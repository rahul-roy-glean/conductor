use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{broadcast, mpsc, RwLock};

use crate::agent::event_parser::{self, ParsedEvent};
use crate::agent::worktree;
use crate::db::queries::{AgentEvent, AgentRun};
use crate::db::Database;

/// Message sent to the dispatch loop when an agent finishes or dispatch is requested
#[derive(Debug, Clone)]
pub struct DispatchMessage {
    pub goal_space_id: String,
    pub branch_to_merge: Option<String>,
    pub repo_path: Option<String>,
    pub agent_run_id: Option<String>,
}

/// Status of an agent session
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Spawning,
    Running,
    Stalled,
    Done,
    Failed,
    Killed,
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentStatus::Spawning => write!(f, "spawning"),
            AgentStatus::Running => write!(f, "running"),
            AgentStatus::Stalled => write!(f, "stalled"),
            AgentStatus::Done => write!(f, "done"),
            AgentStatus::Failed => write!(f, "failed"),
            AgentStatus::Killed => write!(f, "killed"),
        }
    }
}

/// Live state for an active agent session (in-memory)
struct LiveSession {
    agent_run_id: String,
    claude_session_id: Option<String>,
    process: Child,
    worktree_path: PathBuf,
    repo_path: PathBuf,
    status: AgentStatus,
    cost_usd: f64,
    input_tokens: i64,
    output_tokens: i64,
}

/// SSE event broadcast payload
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind")]
pub enum BroadcastEvent {
    AgentEvent {
        agent_run_id: String,
        event: AgentEvent,
    },
    OperationUpdate {
        operation_id: String,
        goal_space_id: String,
        operation_type: String,
        status: String,
        message: String,
        result: Option<serde_json::Value>,
    },
}

/// Manages all active Claude Code agent sessions
pub struct AgentManager {
    sessions: Arc<RwLock<HashMap<String, LiveSession>>>,
    db: Database,
    event_tx: broadcast::Sender<BroadcastEvent>,
    dispatch_tx: mpsc::UnboundedSender<DispatchMessage>,
}

impl AgentManager {
    pub fn new(
        db: Database,
        event_tx: broadcast::Sender<BroadcastEvent>,
        dispatch_tx: mpsc::UnboundedSender<DispatchMessage>,
    ) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            db,
            event_tx,
            dispatch_tx,
        }
    }

    /// Request auto-dispatch of unblocked tasks for a goal space (no merge needed)
    pub fn request_dispatch(&self, goal_space_id: &str) {
        let _ = self.dispatch_tx.send(DispatchMessage {
            goal_space_id: goal_space_id.to_string(),
            branch_to_merge: None,
            repo_path: None,
            agent_run_id: None,
        });
    }

    /// Spawn a new agent for a task
    pub async fn spawn_agent(
        &self,
        task_id: &str,
        goal_space_id: &str,
        prompt: &str,
        repo_path: &str,
        model: &str,
        max_budget_usd: Option<f64>,
        max_turns: Option<u32>,
        allowed_tools: Option<Vec<String>>,
        permission_mode: Option<String>,
        system_prompt: Option<String>,
    ) -> Result<AgentRun> {
        let agent_run_id = uuid::Uuid::new_v4().to_string();

        // Get task title for branch name
        let task = self.db.get_task(task_id)?;
        let task_title = task.map(|t| t.title).unwrap_or_else(|| "task".to_string());

        // Create branch name and worktree
        let branch = worktree::branch_name(&agent_run_id, &task_title);
        let repo = std::path::Path::new(repo_path);
        let worktree_path = worktree::create_worktree(repo, &agent_run_id, &branch).await?;

        // Drop guard to ensure cleanup if we fail after creating the worktree
        struct CleanupGuard {
            repo_path: PathBuf,
            worktree_path: PathBuf,
            agent_run_id: Option<String>,
            db: Database,
            should_cleanup: bool,
        }

        impl Drop for CleanupGuard {
            fn drop(&mut self) {
                if self.should_cleanup {
                    let repo = self.repo_path.clone();
                    let wt = self.worktree_path.clone();
                    let run_id = self.agent_run_id.clone();
                    let db = self.db.clone();

                    tokio::spawn(async move {
                        tracing::warn!(
                            "Cleaning up failed spawn: removing worktree at {}",
                            wt.display()
                        );
                        if let Err(e) = worktree::remove_worktree(&repo, &wt).await {
                            tracing::error!("Failed to cleanup worktree during failed spawn: {}", e);
                        }

                        // If DB record was created, mark it as failed
                        if let Some(id) = run_id {
                            if let Err(e) = db.update_agent_run_status(&id, "failed") {
                                tracing::error!("Failed to mark agent run as failed during cleanup: {}", e);
                            }
                        }
                    });
                }
            }
        }

        let mut cleanup_guard = CleanupGuard {
            repo_path: repo.to_path_buf(),
            worktree_path: worktree_path.clone(),
            agent_run_id: None,
            db: self.db.clone(),
            should_cleanup: true,
        };

        // Create agent run in DB
        let agent_run = self.db.create_agent_run(
            task_id,
            goal_space_id,
            Some(worktree_path.to_str().unwrap()),
            Some(&branch),
            model,
            max_budget_usd,
        )?;

        // Store agent_run_id so cleanup can mark it as failed if needed
        cleanup_guard.agent_run_id = Some(agent_run.id.clone());

        // Mark task as running
        self.db.update_task(
            task_id,
            &crate::db::queries::UpdateTask {
                status: Some("running".to_string()),
                title: None,
                description: None,
                priority: None,
                depends_on: None,
            },
        )?;

        // Build claude command
        let mut cmd = Command::new("claude");
        cmd.arg("-p")
            .arg(prompt)
            .arg("--output-format")
            .arg("stream-json")
            .arg("--verbose");

        if let Some(budget) = max_budget_usd {
            cmd.arg("--max-budget-usd").arg(budget.to_string());
        }

        if let Some(turns) = max_turns {
            cmd.arg("--max-turns").arg(turns.to_string());
        }

        if let Some(ref tools) = allowed_tools {
            for tool in tools {
                cmd.arg("--allowedTools").arg(tool);
            }
        }

        if let Some(ref mode) = permission_mode {
            cmd.arg("--permission-mode").arg(mode);
        }

        if let Some(ref prompt) = system_prompt {
            cmd.arg("--append-system-prompt").arg(prompt);
        }

        cmd.arg("--model").arg(model);

        cmd.current_dir(&worktree_path);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        tracing::info!(
            "Spawning agent {} for task {} in {}",
            agent_run.id,
            task_id,
            worktree_path.display()
        );

        let mut child = cmd.spawn().context("Failed to spawn claude process")?;

        // Take stdout and stderr for reading events
        let stdout = child.stdout.take().context("Failed to get stdout")?;
        let stderr = child.stderr.take().context("Failed to get stderr")?;

        // Store live session
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(
                agent_run.id.clone(),
                LiveSession {
                    agent_run_id: agent_run.id.clone(),
                    claude_session_id: None,
                    process: child,
                    worktree_path: worktree_path.clone(),
                    repo_path: repo.to_path_buf(),
                    status: AgentStatus::Running,
                    cost_usd: 0.0,
                    input_tokens: 0,
                    output_tokens: 0,
                },
            );
        }

        // Update status to running
        self.db
            .update_agent_run_status(&agent_run.id, "running")?;

        // Spawn succeeded - disable cleanup guard
        cleanup_guard.should_cleanup = false;

        // Spawn background task to read stdout events
        let db = self.db.clone();
        let event_tx = self.event_tx.clone();
        let sessions = self.sessions.clone();
        let dispatch_tx = self.dispatch_tx.clone();
        let run_id = agent_run.id.clone();
        let task_id_owned = task_id.to_string();
        let goal_space_id_owned = goal_space_id.to_string();

        tokio::spawn(async move {
            // Spawn a task to collect stderr in the background
            let stderr_handle = tokio::spawn(async move {
                let mut stderr_reader = BufReader::new(stderr);
                let mut stderr_buf = String::new();
                loop {
                    let mut line = String::new();
                    match stderr_reader.read_line(&mut line).await {
                        Ok(0) => break,
                        Ok(_) => stderr_buf.push_str(&line),
                        Err(_) => break,
                    }
                }
                stderr_buf
            });

            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            // Staleness and timeout tracking
            let mut last_event_time = std::time::Instant::now();
            let start_time = std::time::Instant::now();
            let stall_timeout = std::time::Duration::from_secs(10 * 60); // 10 minutes
            let hard_timeout = std::time::Duration::from_secs(20 * 60); // 20 minutes
            let mut watchdog_interval = tokio::time::interval(std::time::Duration::from_secs(30));
            watchdog_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            // Get max_budget_usd for this run
            let max_budget = match db.get_agent_run(&run_id) {
                Ok(Some(agent_run)) => agent_run.max_budget_usd,
                _ => None,
            };

            let mut timed_out = false;
            let mut budget_exceeded = false;
            let mut stalled = false;

            loop {
                tokio::select! {
                    result = lines.next_line() => {
                        match result {
                            Ok(Some(line)) => {
                                if line.trim().is_empty() {
                                    continue;
                                }

                                // Update last event time
                                last_event_time = std::time::Instant::now();

                                // Clear stalled status if previously set
                                if stalled {
                                    stalled = false;
                                    let mut sessions = sessions.write().await;
                                    if let Some(session) = sessions.get_mut(&run_id) {
                                        session.status = AgentStatus::Running;
                                        if let Err(e) = db.update_agent_run_status(&run_id, "running") {
                                            tracing::error!("Failed to update agent run status to running for {}: {}", run_id, e);
                                        }
                                    }
                                }

                                if let Some(parsed) = event_parser::parse_stream_json_line(&line) {
                                    // Store in DB
                                    if let Ok(agent_event) =
                                        event_parser::store_event(&db, &run_id, &parsed, &line)
                                    {
                                        // Broadcast to SSE subscribers
                                        let _ = event_tx.send(BroadcastEvent::AgentEvent {
                                            agent_run_id: run_id.clone(),
                                            event: agent_event,
                                        });
                                    }

                                    // Update cost accumulators
                                    match &parsed {
                                        ParsedEvent::ApiRequest {
                                            cost_usd,
                                            input_tokens,
                                            output_tokens,
                                            ..
                                        } => {
                                            let mut sessions = sessions.write().await;
                                            if let Some(session) = sessions.get_mut(&run_id) {
                                                session.cost_usd += cost_usd;
                                                session.input_tokens += input_tokens;
                                                session.output_tokens += output_tokens;
                                                if let Err(e) = db.update_agent_run_cost(
                                                    &run_id,
                                                    session.cost_usd,
                                                    session.input_tokens,
                                                    session.output_tokens,
                                                ) {
                                                    tracing::error!("Failed to update agent run cost for {}: {}", run_id, e);
                                                }

                                                // Enforce max_budget_usd server-side
                                                if let Some(budget) = max_budget {
                                                    if session.cost_usd > budget {
                                                        tracing::warn!(
                                                            "Agent {} exceeded budget: ${:.4} > ${:.4}",
                                                            run_id,
                                                            session.cost_usd,
                                                            budget
                                                        );
                                                        budget_exceeded = true;
                                                        session.status = AgentStatus::Killed;
                                                        if let Err(e) = db.update_agent_run_status(&run_id, "killed") {
                                                            tracing::error!("Failed to update agent run status to killed for {}: {}", run_id, e);
                                                        }
                                                        if let Err(e) = db.insert_agent_event(
                                                            &run_id,
                                                            "error",
                                                            None,
                                                            &format!("Budget exceeded: ${:.4} > ${:.4}", session.cost_usd, budget),
                                                            None,
                                                            None,
                                                        ) {
                                                            tracing::error!("Failed to insert budget exceeded event for {}: {}", run_id, e);
                                                        }
                                                        session.process.kill().await.ok();
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                        ParsedEvent::Result {
                                            session_id,
                                            cost_usd,
                                            ..
                                        } => {
                                            let mut sessions = sessions.write().await;
                                            if let Some(session) = sessions.get_mut(&run_id) {
                                                session.claude_session_id = Some(session_id.clone());
                                                session.cost_usd = *cost_usd;
                                                if let Err(e) = db.update_agent_run_session_id(&run_id, session_id) {
                                                    tracing::error!("Failed to update agent run session ID for {}: {}", run_id, e);
                                                }
                                                if let Err(e) = db.update_agent_run_cost(
                                                    &run_id,
                                                    session.cost_usd,
                                                    session.input_tokens,
                                                    session.output_tokens,
                                                ) {
                                                    tracing::error!("Failed to update agent run cost for {}: {}", run_id, e);
                                                }
                                            }
                                        }
                                        _ => {
                                            if let Err(e) = db.update_agent_run_activity(&run_id) {
                                                tracing::error!("Failed to update agent run activity for {}: {}", run_id, e);
                                            }
                                        }
                                    }
                                }
                            }
                            Ok(None) => {
                                // EOF - process exited normally
                                break;
                            }
                            Err(e) => {
                                tracing::error!("Error reading stdout for agent {}: {}", run_id, e);
                                break;
                            }
                        }
                    }
                    _ = watchdog_interval.tick() => {
                        let elapsed_since_last_event = last_event_time.elapsed();
                        let total_elapsed = start_time.elapsed();

                        // Check hard timeout (20 minutes total)
                        if total_elapsed >= hard_timeout {
                            tracing::warn!("Agent {} hard timeout after {:?}", run_id, total_elapsed);
                            timed_out = true;
                            let mut sessions = sessions.write().await;
                            if let Some(session) = sessions.get_mut(&run_id) {
                                session.status = AgentStatus::Failed;
                                if let Err(e) = db.update_agent_run_status(&run_id, "failed") {
                                    tracing::error!("Failed to update agent run status to failed for {}: {}", run_id, e);
                                }
                                if let Err(e) = db.insert_agent_event(
                                    &run_id,
                                    "error",
                                    None,
                                    &format!("Hard timeout after {:?}", total_elapsed),
                                    None,
                                    None,
                                ) {
                                    tracing::error!("Failed to insert hard timeout event for {}: {}", run_id, e);
                                }
                                session.process.kill().await.ok();
                            }
                            break;
                        }

                        // Check staleness (10 minutes without events)
                        if elapsed_since_last_event >= stall_timeout {
                            if !stalled {
                                tracing::warn!("Agent {} stalled - no events for {:?}", run_id, elapsed_since_last_event);
                                stalled = true;
                                let mut sessions = sessions.write().await;
                                if let Some(session) = sessions.get_mut(&run_id) {
                                    session.status = AgentStatus::Stalled;
                                    if let Err(e) = db.update_agent_run_status(&run_id, "stalled") {
                                        tracing::error!("Failed to update agent run status to stalled for {}: {}", run_id, e);
                                    }
                                    if let Err(e) = db.insert_agent_event(
                                        &run_id,
                                        "warning",
                                        None,
                                        &format!("Agent stalled - no events for {:?}", elapsed_since_last_event),
                                        None,
                                        None,
                                    ) {
                                        tracing::error!("Failed to insert stalled event for {}: {}", run_id, e);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Collect stderr output
            let stderr_output = stderr_handle.await.unwrap_or_default();
            if !stderr_output.trim().is_empty() {
                tracing::warn!("Agent {} stderr: {}", run_id, stderr_output.trim());
                // Store stderr as an error event so it's visible in the UI
                let summary = if stderr_output.len() > 500 {
                    format!("{}...", &stderr_output[..500])
                } else {
                    stderr_output.clone()
                };
                if let Err(e) = db.insert_agent_event(
                    &run_id,
                    "error",
                    None,
                    &summary,
                    None,
                    None,
                ) {
                    tracing::error!("Failed to insert stderr event for {}: {}", run_id, e);
                }
            }

            // Process exited - determine final status
            let final_status = {
                let mut sessions = sessions.write().await;
                if let Some(session) = sessions.get_mut(&run_id) {
                    // Determine final status based on exit conditions
                    let final_status = if timed_out {
                        if let Err(e) = db.update_task(
                            &task_id_owned,
                            &crate::db::queries::UpdateTask {
                                status: Some("failed".to_string()),
                                title: None,
                                description: None,
                                priority: None,
                                depends_on: None,
                            },
                        ) {
                            tracing::error!("Failed to update task {} to failed (timeout) for agent {}: {}", task_id_owned, run_id, e);
                        }
                        "failed"
                    } else if budget_exceeded {
                        if let Err(e) = db.update_task(
                            &task_id_owned,
                            &crate::db::queries::UpdateTask {
                                status: Some("failed".to_string()),
                                title: None,
                                description: None,
                                priority: None,
                                depends_on: None,
                            },
                        ) {
                            tracing::error!("Failed to update task {} to failed (budget exceeded) for agent {}: {}", task_id_owned, run_id, e);
                        }
                        "killed"
                    } else {
                        // Normal exit - check process status
                        let exit_status = session.process.try_wait();
                        match exit_status {
                            Ok(Some(status)) if status.success() => {
                                // Only mark as done if the agent actually did work
                                // ($0.00 cost means Claude exited without making any API calls)
                                if session.cost_usd > 0.0 {
                                    if let Err(e) = db.update_task(
                                        &task_id_owned,
                                        &crate::db::queries::UpdateTask {
                                            status: Some("done".to_string()),
                                            title: None,
                                            description: None,
                                            priority: None,
                                            depends_on: None,
                                        },
                                    ) {
                                        tracing::error!("Failed to update task {} to done for agent {}: {}", task_id_owned, run_id, e);
                                    }
                                    "done"
                                } else {
                                    tracing::warn!(
                                        "Agent {} exited successfully but cost $0.00 — no work was done, marking as failed",
                                        run_id
                                    );
                                    if let Err(e) = db.update_task(
                                        &task_id_owned,
                                        &crate::db::queries::UpdateTask {
                                            status: Some("failed".to_string()),
                                            title: None,
                                            description: None,
                                            priority: None,
                                            depends_on: None,
                                        },
                                    ) {
                                        tracing::error!("Failed to update task {} to failed (no work done) for agent {}: {}", task_id_owned, run_id, e);
                                    }
                                    "failed"
                                }
                            }
                            _ => {
                                if let Err(e) = db.update_task(
                                    &task_id_owned,
                                    &crate::db::queries::UpdateTask {
                                        status: Some("failed".to_string()),
                                        title: None,
                                        description: None,
                                        priority: None,
                                        depends_on: None,
                                    },
                                ) {
                                    tracing::error!("Failed to update task {} to failed (exit error) for agent {}: {}", task_id_owned, run_id, e);
                                }
                                "failed"
                            }
                        }
                    };

                    if let Err(e) = db.update_agent_run_status(&run_id, final_status) {
                        tracing::error!("Failed to update agent run status to {} for {}: {}", final_status, run_id, e);
                    }
                    session.status = match final_status {
                        "done" => AgentStatus::Done,
                        "killed" => AgentStatus::Killed,
                        _ => AgentStatus::Failed,
                    };

                    // Cleanup worktree
                    if let Err(e) = worktree::remove_worktree(&session.repo_path, &session.worktree_path).await {
                        tracing::error!("Failed to remove worktree {} for agent {}: {}", session.worktree_path.display(), run_id, e);
                    }

                    // Remove from live sessions
                    sessions.remove(&run_id);

                    Some(final_status)
                } else {
                    sessions.remove(&run_id);
                    None
                }
            }; // sessions write lock dropped here

            tracing::info!("Agent {} finished with status {:?}", run_id, final_status);

            // Auto-dispatch next unblocked tasks for this goal
            if final_status == Some("done") {
                // Look up the branch from the DB so we can merge it
                let branch_to_merge = match db.get_agent_run(&run_id) {
                    Ok(Some(ar)) => ar.branch,
                    _ => None,
                };

                // Resolve the actual repo_path from the goal space
                let repo_path = match db.get_goal_space(&goal_space_id_owned) {
                    Ok(Some(g)) => Some(g.repo_path),
                    _ => None,
                };

                let _ = dispatch_tx.send(DispatchMessage {
                    goal_space_id: goal_space_id_owned,
                    branch_to_merge,
                    repo_path,
                    agent_run_id: Some(run_id.clone()),
                });
            }
        });

        Ok(agent_run)
    }

    /// Send a nudge message to a running agent via --resume
    pub async fn nudge_agent(&self, agent_run_id: &str, message: &str) -> Result<()> {
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(agent_run_id)
            .context("Agent not found or not running")?;

        let session_id = session
            .claude_session_id
            .as_ref()
            .context("Agent has no Claude session ID yet")?;

        // Spawn a new claude process with --resume
        let mut cmd = Command::new("claude");
        cmd.arg("-p")
            .arg(message)
            .arg("--resume")
            .arg(session_id)
            .arg("--output-format")
            .arg("stream-json");

        cmd.current_dir(&session.worktree_path);
        // We don't need the nudge output, so set stdout/stderr to null to prevent pipe deadlock
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());

        let child = cmd.spawn().context("Failed to spawn claude resume")?;

        tracing::info!("Nudged agent {} with message", agent_run_id);

        // Spawn a task to await the child and log its exit status
        let agent_run_id_owned = agent_run_id.to_string();
        tokio::spawn(async move {
            match child.wait_with_output().await {
                Ok(output) => {
                    if output.status.success() {
                        tracing::debug!("Nudge for agent {} completed successfully", agent_run_id_owned);
                    } else {
                        tracing::warn!(
                            "Nudge for agent {} exited with status: {}",
                            agent_run_id_owned,
                            output.status
                        );
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to wait for nudge process for agent {}: {}", agent_run_id_owned, e);
                }
            }
        });

        Ok(())
    }

    /// Kill a running agent
    pub async fn kill_agent(&self, agent_run_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(agent_run_id)
            .context("Agent not found or not running")?;

        // Send SIGTERM
        session.process.kill().await.ok();
        session.status = AgentStatus::Killed;

        self.db
            .update_agent_run_status(agent_run_id, "killed")?;

        // Cleanup worktree
        worktree::remove_worktree(&session.repo_path, &session.worktree_path).await?;

        sessions.remove(agent_run_id);

        tracing::info!("Killed agent {}", agent_run_id);

        Ok(())
    }

    /// Get IDs of all active sessions
    pub async fn active_session_ids(&self) -> Vec<String> {
        let sessions = self.sessions.read().await;
        sessions.keys().cloned().collect()
    }

    /// Check if an agent is active
    pub async fn is_active(&self, agent_run_id: &str) -> bool {
        let sessions = self.sessions.read().await;
        sessions.contains_key(agent_run_id)
    }
}
