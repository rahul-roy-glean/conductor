pub mod routes;
pub mod sse;

use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

use crate::agent::session::{AgentManager, BroadcastEvent, DispatchMessage};
use crate::agent::worktree;
use crate::db::Database;

pub struct AppState {
    pub db: Database,
    pub agent_manager: AgentManager,
    pub event_tx: broadcast::Sender<BroadcastEvent>,
}

pub async fn run(
    state: Arc<AppState>,
    port: u16,
    dispatch_rx: mpsc::UnboundedReceiver<DispatchMessage>,
) -> anyhow::Result<()> {
    // Spawn the auto-dispatch loop
    let dispatch_state = state.clone();
    tokio::spawn(dispatch_loop(dispatch_state, dispatch_rx));

    let app = routes::create_router(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!("Conductor server listening on port {}", port);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Background loop that auto-dispatches unblocked tasks when agents complete
async fn dispatch_loop(state: Arc<AppState>, mut rx: mpsc::UnboundedReceiver<DispatchMessage>) {
    tracing::info!("Auto-dispatch loop started");

    while let Some(msg) = rx.recv().await {
        let goal_space_id = &msg.goal_space_id;

        // Merge completed branch if present
        if let (Some(branch), Some(repo_path)) = (&msg.branch_to_merge, &msg.repo_path) {
            let repo = std::path::Path::new(repo_path.as_str());
            match worktree::merge_branch_to_main(repo, branch).await {
                Ok(()) => {
                    tracing::info!("Auto-merged branch {} into main", branch);
                    // Record merge event on the agent run
                    if let Some(ref agent_run_id) = msg.agent_run_id {
                        let _ = state.db.insert_agent_event(
                            agent_run_id,
                            "merge_completed",
                            None,
                            &format!("Merged branch {} into main", branch),
                            None,
                            None,
                        );
                    }
                    // Clean up the merged branch
                    if let Err(e) = worktree::delete_branch(repo, branch).await {
                        tracing::warn!("Failed to delete merged branch {}: {}", branch, e);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to auto-merge branch {}: {}", branch, e);
                    if let Some(ref agent_run_id) = msg.agent_run_id {
                        let _ = state.db.insert_agent_event(
                            agent_run_id,
                            "merge_failed",
                            None,
                            &format!("Failed to merge branch {}: {}", branch, e),
                            None,
                            None,
                        );
                    }
                }
            }
        }

        tracing::info!(
            "Auto-dispatching unblocked tasks for goal {}",
            goal_space_id
        );

        let goal = match state.db.get_goal_space(goal_space_id) {
            Ok(Some(g)) => g,
            Ok(None) => {
                tracing::warn!("Goal {} not found for auto-dispatch", goal_space_id);
                continue;
            }
            Err(e) => {
                tracing::error!("Failed to get goal {}: {}", goal_space_id, e);
                continue;
            }
        };

        // Check if goal is already completed
        if goal.status == "completed" || goal.status == "archived" {
            continue;
        }

        let unblocked = match state.db.get_unblocked_tasks(goal_space_id) {
            Ok(tasks) => tasks,
            Err(e) => {
                tracing::error!("Failed to get unblocked tasks: {}", e);
                continue;
            }
        };

        if unblocked.is_empty() {
            // No new tasks to dispatch — check if goal is fully complete
            let _ = crate::goal::space::check_goal_completion(&state.db, goal_space_id);
            continue;
        }

        let mut spawned = 0;
        for task in &unblocked {
            // Merge task-level settings over goal-level settings
            let effective = goal.settings.merge(&task.settings);

            let prompt = format!(
                "You are working on the following task as part of the goal: {}\n\n\
                 Task: {}\n\n\
                 Description: {}\n\n\
                 Work in the current directory. Make your changes, test them, and commit when done.",
                goal.description, task.title, task.description
            );

            match state
                .agent_manager
                .spawn_agent(
                    &task.id,
                    goal_space_id,
                    &prompt,
                    &goal.repo_path,
                    &effective.model(),
                    Some(effective.max_budget_usd()),
                    Some(effective.max_turns()),
                    Some(effective.allowed_tools()),
                    effective.permission_mode(),
                    effective.system_prompt(),
                )
                .await
            {
                Ok(_) => spawned += 1,
                Err(e) => {
                    tracing::error!(
                        "Auto-dispatch: failed to spawn agent for task {}: {}",
                        task.id,
                        e
                    );
                }
            }
        }

        tracing::info!(
            "Auto-dispatch: spawned {} agents for goal {}",
            spawned,
            goal_space_id
        );
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C handler");
    tracing::info!("Shutting down...");
}
