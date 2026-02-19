use anyhow::Result;
use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

use crate::db::Database;

// ── Goal Space types ──

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct GoalSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_budget_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
}

impl GoalSettings {
    /// Get the resolved model value (with fallback to default)
    pub fn model(&self) -> String {
        self.model.clone().unwrap_or_else(|| "sonnet".to_string())
    }

    /// Get the resolved max_budget_usd value (with fallback to default)
    pub fn max_budget_usd(&self) -> f64 {
        self.max_budget_usd.unwrap_or(5.0)
    }

    /// Get the resolved max_turns value (with fallback to default)
    pub fn max_turns(&self) -> u32 {
        self.max_turns.unwrap_or(50)
    }

    /// Get the resolved allowed_tools value (with fallback to default)
    pub fn allowed_tools(&self) -> Vec<String> {
        self.allowed_tools.clone().unwrap_or_else(|| {
            vec![
                "Bash".to_string(),
                "Read".to_string(),
                "Edit".to_string(),
                "Write".to_string(),
                "Grep".to_string(),
                "Glob".to_string(),
            ]
        })
    }

    /// Get the resolved permission_mode value (returns None if not set)
    pub fn permission_mode(&self) -> Option<String> {
        self.permission_mode.clone()
    }

    /// Get the resolved system_prompt value (returns None if not set)
    pub fn system_prompt(&self) -> Option<String> {
        self.system_prompt.clone()
    }

    /// Merge task-level settings over goal-level settings.
    /// Task settings override goal settings where present.
    pub fn merge(&self, task_settings: &GoalSettings) -> GoalSettings {
        GoalSettings {
            model: task_settings.model.clone().or_else(|| self.model.clone()),
            max_budget_usd: task_settings.max_budget_usd.or(self.max_budget_usd),
            max_turns: task_settings.max_turns.or(self.max_turns),
            allowed_tools: task_settings
                .allowed_tools
                .clone()
                .or_else(|| self.allowed_tools.clone()),
            permission_mode: task_settings
                .permission_mode
                .clone()
                .or_else(|| self.permission_mode.clone()),
            system_prompt: task_settings
                .system_prompt
                .clone()
                .or_else(|| self.system_prompt.clone()),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GoalSpace {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub repo_path: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub settings: GoalSettings,
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateGoalSpace {
    pub name: String,
    pub description: String,
    pub repo_path: String,
    #[serde(default)]
    pub settings: GoalSettings,
}

// ── Task types ──

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Task {
    pub id: String,
    pub goal_space_id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: i32,
    pub depends_on: Vec<String>,
    pub settings: GoalSettings,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateTask {
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub settings: GoalSettings,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct UpdateTask {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<i32>,
    pub depends_on: Option<Vec<String>>,
    pub settings: Option<GoalSettings>,
}

// ── Agent Run types ──

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentRun {
    pub id: String,
    pub task_id: String,
    pub goal_space_id: String,
    pub claude_session_id: Option<String>,
    pub worktree_path: Option<String>,
    pub branch: Option<String>,
    pub status: String,
    pub model: String,
    pub cost_usd: f64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub max_budget_usd: Option<f64>,
    pub started_at: String,
    pub last_activity_at: Option<String>,
    pub finished_at: Option<String>,
}

// ── Agent Event types ──

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentEvent {
    pub id: i64,
    pub agent_run_id: String,
    pub event_type: String,
    pub tool_name: Option<String>,
    pub summary: String,
    pub raw_json: Option<String>,
    pub cost_delta_usd: Option<f64>,
    pub created_at: String,
}

// ── Project types ──

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Project {
    pub id: String,
    pub path: String,
    pub display_name: String,
    pub sort_order: i32,
    #[serde(default)]
    pub settings: GoalSettings,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateProject {
    pub path: String,
    pub display_name: String,
    #[serde(default)]
    pub sort_order: i32,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct UpdateProject {
    pub path: Option<String>,
    pub display_name: Option<String>,
    pub sort_order: Option<i32>,
    pub settings: Option<GoalSettings>,
}

// ── Goal Message types ──

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GoalMessage {
    pub id: String,
    pub goal_space_id: String,
    pub role: String,
    pub content: String,
    pub message_type: String,
    pub metadata_json: String,
    pub created_at: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateGoalMessage {
    pub goal_space_id: String,
    pub role: String,
    pub content: String,
    #[serde(default = "default_message_type")]
    pub message_type: String,
    #[serde(default = "default_metadata_json")]
    pub metadata_json: String,
}

fn default_message_type() -> String {
    "text".to_string()
}

fn default_metadata_json() -> String {
    "{}".to_string()
}

// ── Stats ──

#[derive(Debug, serde::Serialize)]
pub struct Stats {
    pub active_agents: i64,
    pub total_cost_usd: f64,
    pub tasks_completed: i64,
    pub tasks_total: i64,
    pub goals_active: i64,
}

// ── Goal Space Queries ──

impl Database {
    pub fn create_goal_space(&self, input: &CreateGoalSpace) -> Result<GoalSpace> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let settings_json = serde_json::to_string(&input.settings)?;

        {
            let conn = self.conn();
            conn.execute(
                "INSERT INTO goal_spaces (id, name, description, status, repo_path, created_at, updated_at, settings)
                 VALUES (?1, ?2, ?3, 'active', ?4, ?5, ?6, ?7)",
                params![id, input.name, input.description, input.repo_path, now, now, settings_json],
            )?;
        }

        self.insert_goal_history(
            &id,
            "created",
            &format!("Goal space '{}' created", input.name),
            None,
        )?;

        Ok(GoalSpace {
            id,
            name: input.name.clone(),
            description: input.description.clone(),
            status: "active".to_string(),
            repo_path: input.repo_path.clone(),
            created_at: now.clone(),
            updated_at: now,
            settings: input.settings.clone(),
        })
    }

    pub fn list_goal_spaces(&self) -> Result<Vec<GoalSpace>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, status, repo_path, created_at, updated_at, settings
             FROM goal_spaces ORDER BY created_at DESC",
        )?;

        let goals = stmt
            .query_map([], |row| {
                let settings_str: String = row.get(7)?;
                let settings: GoalSettings =
                    serde_json::from_str(&settings_str).unwrap_or_default();
                Ok(GoalSpace {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    status: row.get(3)?,
                    repo_path: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    settings,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(goals)
    }

    pub fn get_goal_space(&self, id: &str) -> Result<Option<GoalSpace>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, status, repo_path, created_at, updated_at, settings
             FROM goal_spaces WHERE id = ?1",
        )?;

        let goal = stmt
            .query_row(params![id], |row| {
                let settings_str: String = row.get(7)?;
                let settings: GoalSettings =
                    serde_json::from_str(&settings_str).unwrap_or_default();
                Ok(GoalSpace {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    status: row.get(3)?,
                    repo_path: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    settings,
                })
            })
            .optional()?;

        Ok(goal)
    }

    pub fn update_goal_space(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn();
        let now = Utc::now().to_rfc3339();

        if let Some(name) = name {
            conn.execute(
                "UPDATE goal_spaces SET name = ?1, updated_at = ?2 WHERE id = ?3",
                params![name, now, id],
            )?;
        }
        if let Some(description) = description {
            conn.execute(
                "UPDATE goal_spaces SET description = ?1, updated_at = ?2 WHERE id = ?3",
                params![description, now, id],
            )?;
        }
        if let Some(status) = status {
            conn.execute(
                "UPDATE goal_spaces SET status = ?1, updated_at = ?2 WHERE id = ?3",
                params![status, now, id],
            )?;
        }

        Ok(())
    }

    pub fn update_goal_settings(&self, id: &str, settings: &GoalSettings) -> Result<()> {
        let conn = self.conn();
        let now = Utc::now().to_rfc3339();
        let settings_json = serde_json::to_string(settings)?;

        conn.execute(
            "UPDATE goal_spaces SET settings = ?1, updated_at = ?2 WHERE id = ?3",
            params![settings_json, now, id],
        )?;

        Ok(())
    }

    /// Atomically mark a goal as completed if and only if all its tasks are done.
    /// Returns true if the goal was marked completed, false if not (because there are pending tasks or no tasks).
    pub fn mark_goal_completed_if_all_tasks_done(&self, goal_space_id: &str) -> Result<bool> {
        let conn = self.conn();
        let now = Utc::now().to_rfc3339();

        // Use a single atomic UPDATE statement that only updates if:
        // 1. There is at least one task
        // 2. All tasks are done
        // 3. The goal is not already completed
        let rows_affected = conn.execute(
            "UPDATE goal_spaces
             SET status = 'completed', updated_at = ?1
             WHERE id = ?2
               AND status != 'completed'
               AND EXISTS (SELECT 1 FROM tasks WHERE goal_space_id = ?2)
               AND NOT EXISTS (SELECT 1 FROM tasks WHERE goal_space_id = ?2 AND status != 'done')",
            params![now, goal_space_id],
        )?;

        Ok(rows_affected > 0)
    }

    pub fn delete_goal_space(&self, id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn();
        conn.execute(
            "UPDATE goal_spaces SET status = 'archived', updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    // ── Task Queries ──

    pub fn create_task(&self, goal_space_id: &str, input: &CreateTask) -> Result<Task> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let depends_on_json = serde_json::to_string(&input.depends_on)?;
        let settings_json = serde_json::to_string(&input.settings)?;

        {
            let conn = self.conn();
            conn.execute(
                "INSERT INTO tasks (id, goal_space_id, title, description, status, priority, depends_on, settings, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6, ?7, ?8, ?9)",
                params![id, goal_space_id, input.title, input.description, input.priority, depends_on_json, settings_json, now, now],
            )?;
        }

        self.insert_goal_history(
            goal_space_id,
            "task_added",
            &format!("Task '{}' added", input.title),
            None,
        )?;

        Ok(Task {
            id,
            goal_space_id: goal_space_id.to_string(),
            title: input.title.clone(),
            description: input.description.clone(),
            status: "pending".to_string(),
            priority: input.priority,
            depends_on: input.depends_on.clone(),
            settings: input.settings.clone(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn list_tasks(&self, goal_space_id: &str) -> Result<Vec<Task>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, goal_space_id, title, description, status, priority, depends_on, settings, created_at, updated_at
             FROM tasks WHERE goal_space_id = ?1 ORDER BY priority DESC, created_at ASC",
        )?;

        let tasks = stmt
            .query_map(params![goal_space_id], |row| {
                let depends_on_str: String = row.get(6)?;
                let depends_on: Vec<String> =
                    serde_json::from_str(&depends_on_str).unwrap_or_default();
                let settings_str: String = row.get(7)?;
                let settings: GoalSettings =
                    serde_json::from_str(&settings_str).unwrap_or_default();
                Ok(Task {
                    id: row.get(0)?,
                    goal_space_id: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    status: row.get(4)?,
                    priority: row.get(5)?,
                    depends_on,
                    settings,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(tasks)
    }

    pub fn get_task(&self, id: &str) -> Result<Option<Task>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, goal_space_id, title, description, status, priority, depends_on, settings, created_at, updated_at
             FROM tasks WHERE id = ?1",
        )?;

        let task = stmt
            .query_row(params![id], |row| {
                let depends_on_str: String = row.get(6)?;
                let depends_on: Vec<String> =
                    serde_json::from_str(&depends_on_str).unwrap_or_default();
                let settings_str: String = row.get(7)?;
                let settings: GoalSettings =
                    serde_json::from_str(&settings_str).unwrap_or_default();
                Ok(Task {
                    id: row.get(0)?,
                    goal_space_id: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    status: row.get(4)?,
                    priority: row.get(5)?,
                    depends_on,
                    settings,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })
            .optional()?;

        Ok(task)
    }

    pub fn update_task(&self, id: &str, input: &UpdateTask) -> Result<()> {
        let conn = self.conn();
        let now = Utc::now().to_rfc3339();

        if let Some(ref title) = input.title {
            conn.execute(
                "UPDATE tasks SET title = ?1, updated_at = ?2 WHERE id = ?3",
                params![title, now, id],
            )?;
        }
        if let Some(ref description) = input.description {
            conn.execute(
                "UPDATE tasks SET description = ?1, updated_at = ?2 WHERE id = ?3",
                params![description, now, id],
            )?;
        }
        if let Some(ref status) = input.status {
            conn.execute(
                "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
                params![status, now, id],
            )?;
        }
        if let Some(priority) = input.priority {
            conn.execute(
                "UPDATE tasks SET priority = ?1, updated_at = ?2 WHERE id = ?3",
                params![priority, now, id],
            )?;
        }
        if let Some(ref depends_on) = input.depends_on {
            let json = serde_json::to_string(depends_on)?;
            conn.execute(
                "UPDATE tasks SET depends_on = ?1, updated_at = ?2 WHERE id = ?3",
                params![json, now, id],
            )?;
        }
        if let Some(ref settings) = input.settings {
            let json = serde_json::to_string(settings)?;
            conn.execute(
                "UPDATE tasks SET settings = ?1, updated_at = ?2 WHERE id = ?3",
                params![json, now, id],
            )?;
        }

        Ok(())
    }

    pub fn get_unblocked_tasks(&self, goal_space_id: &str) -> Result<Vec<Task>> {
        let all_tasks = self.list_tasks(goal_space_id)?;

        let done_ids: std::collections::HashSet<String> = all_tasks
            .iter()
            .filter(|t| t.status == "done")
            .map(|t| t.id.clone())
            .collect();

        let unblocked: Vec<Task> = all_tasks
            .into_iter()
            .filter(|t| {
                t.status == "pending" && t.depends_on.iter().all(|dep| done_ids.contains(dep))
            })
            .collect();

        Ok(unblocked)
    }

    // ── Agent Run Queries ──

    pub fn create_agent_run(
        &self,
        task_id: &str,
        goal_space_id: &str,
        worktree_path: Option<&str>,
        branch: Option<&str>,
        model: &str,
        max_budget_usd: Option<f64>,
    ) -> Result<AgentRun> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        {
            let conn = self.conn();
            conn.execute(
                "INSERT INTO agent_runs (id, task_id, goal_space_id, worktree_path, branch, status, model, max_budget_usd, started_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'spawning', ?6, ?7, ?8)",
                params![id, task_id, goal_space_id, worktree_path, branch, model, max_budget_usd, now],
            )?;
        }

        self.insert_goal_history(
            goal_space_id,
            "agent_spawned",
            &format!("Agent {} spawned for task {}", id, task_id),
            None,
        )?;

        Ok(AgentRun {
            id,
            task_id: task_id.to_string(),
            goal_space_id: goal_space_id.to_string(),
            claude_session_id: None,
            worktree_path: worktree_path.map(String::from),
            branch: branch.map(String::from),
            status: "spawning".to_string(),
            model: model.to_string(),
            cost_usd: 0.0,
            input_tokens: 0,
            output_tokens: 0,
            max_budget_usd,
            started_at: now,
            last_activity_at: None,
            finished_at: None,
        })
    }

    pub fn get_agent_run(&self, id: &str) -> Result<Option<AgentRun>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, goal_space_id, claude_session_id, worktree_path, branch,
                    status, model, cost_usd, input_tokens, output_tokens, max_budget_usd,
                    started_at, last_activity_at, finished_at
             FROM agent_runs WHERE id = ?1",
        )?;

        let run = stmt
            .query_row(params![id], |row| {
                Ok(AgentRun {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    goal_space_id: row.get(2)?,
                    claude_session_id: row.get(3)?,
                    worktree_path: row.get(4)?,
                    branch: row.get(5)?,
                    status: row.get(6)?,
                    model: row.get(7)?,
                    cost_usd: row.get(8)?,
                    input_tokens: row.get(9)?,
                    output_tokens: row.get(10)?,
                    max_budget_usd: row.get(11)?,
                    started_at: row.get(12)?,
                    last_activity_at: row.get(13)?,
                    finished_at: row.get(14)?,
                })
            })
            .optional()?;

        Ok(run)
    }

    pub fn list_agent_runs(&self) -> Result<Vec<AgentRun>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, goal_space_id, claude_session_id, worktree_path, branch,
                    status, model, cost_usd, input_tokens, output_tokens, max_budget_usd,
                    started_at, last_activity_at, finished_at
             FROM agent_runs ORDER BY started_at DESC",
        )?;

        let runs = stmt
            .query_map([], |row| {
                Ok(AgentRun {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    goal_space_id: row.get(2)?,
                    claude_session_id: row.get(3)?,
                    worktree_path: row.get(4)?,
                    branch: row.get(5)?,
                    status: row.get(6)?,
                    model: row.get(7)?,
                    cost_usd: row.get(8)?,
                    input_tokens: row.get(9)?,
                    output_tokens: row.get(10)?,
                    max_budget_usd: row.get(11)?,
                    started_at: row.get(12)?,
                    last_activity_at: row.get(13)?,
                    finished_at: row.get(14)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(runs)
    }

    pub fn list_active_agent_runs(&self) -> Result<Vec<AgentRun>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, goal_space_id, claude_session_id, worktree_path, branch,
                    status, model, cost_usd, input_tokens, output_tokens, max_budget_usd,
                    started_at, last_activity_at, finished_at
             FROM agent_runs WHERE status IN ('spawning', 'running', 'stalled')
             ORDER BY started_at DESC",
        )?;

        let runs = stmt
            .query_map([], |row| {
                Ok(AgentRun {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    goal_space_id: row.get(2)?,
                    claude_session_id: row.get(3)?,
                    worktree_path: row.get(4)?,
                    branch: row.get(5)?,
                    status: row.get(6)?,
                    model: row.get(7)?,
                    cost_usd: row.get(8)?,
                    input_tokens: row.get(9)?,
                    output_tokens: row.get(10)?,
                    max_budget_usd: row.get(11)?,
                    started_at: row.get(12)?,
                    last_activity_at: row.get(13)?,
                    finished_at: row.get(14)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(runs)
    }

    pub fn update_agent_run_status(&self, id: &str, status: &str) -> Result<()> {
        let conn = self.conn();
        let now = Utc::now().to_rfc3339();

        if status == "done" || status == "failed" || status == "killed" {
            conn.execute(
                "UPDATE agent_runs SET status = ?1, finished_at = ?2 WHERE id = ?3",
                params![status, now, id],
            )?;
        } else {
            conn.execute(
                "UPDATE agent_runs SET status = ?1 WHERE id = ?2",
                params![status, id],
            )?;
        }

        Ok(())
    }

    pub fn update_agent_run_session_id(&self, id: &str, session_id: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE agent_runs SET claude_session_id = ?1 WHERE id = ?2",
            params![session_id, id],
        )?;
        Ok(())
    }

    pub fn update_agent_run_cost(
        &self,
        id: &str,
        cost_usd: f64,
        input_tokens: i64,
        output_tokens: i64,
    ) -> Result<()> {
        let conn = self.conn();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE agent_runs SET cost_usd = ?1, input_tokens = ?2, output_tokens = ?3, last_activity_at = ?4 WHERE id = ?5",
            params![cost_usd, input_tokens, output_tokens, now, id],
        )?;
        Ok(())
    }

    pub fn update_agent_run_activity(&self, id: &str) -> Result<()> {
        let conn = self.conn();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE agent_runs SET last_activity_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    // ── Agent Event Queries ──

    pub fn insert_agent_event(
        &self,
        agent_run_id: &str,
        event_type: &str,
        tool_name: Option<&str>,
        summary: &str,
        raw_json: Option<&str>,
        cost_delta_usd: Option<f64>,
    ) -> Result<AgentEvent> {
        let conn = self.conn();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO agent_events (agent_run_id, event_type, tool_name, summary, raw_json, cost_delta_usd, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![agent_run_id, event_type, tool_name, summary, raw_json, cost_delta_usd, now],
        )?;

        let id = conn.last_insert_rowid();

        Ok(AgentEvent {
            id,
            agent_run_id: agent_run_id.to_string(),
            event_type: event_type.to_string(),
            tool_name: tool_name.map(String::from),
            summary: summary.to_string(),
            raw_json: raw_json.map(String::from),
            cost_delta_usd,
            created_at: now,
        })
    }

    pub fn list_agent_events(&self, agent_run_id: &str) -> Result<Vec<AgentEvent>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, agent_run_id, event_type, tool_name, summary, raw_json, cost_delta_usd, created_at
             FROM agent_events WHERE agent_run_id = ?1 ORDER BY id ASC",
        )?;

        let events = stmt
            .query_map(params![agent_run_id], |row| {
                Ok(AgentEvent {
                    id: row.get(0)?,
                    agent_run_id: row.get(1)?,
                    event_type: row.get(2)?,
                    tool_name: row.get(3)?,
                    summary: row.get(4)?,
                    raw_json: row.get(5)?,
                    cost_delta_usd: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(events)
    }

    // ── Goal Space History ──

    pub fn insert_goal_history(
        &self,
        goal_space_id: &str,
        event_type: &str,
        description: &str,
        metadata: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO goal_space_history (goal_space_id, event_type, description, metadata, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![goal_space_id, event_type, description, metadata, now],
        )?;

        Ok(())
    }

    // ── Stats ──

    pub fn get_stats(&self) -> Result<Stats> {
        let conn = self.conn();

        let active_agents: i64 = conn.query_row(
            "SELECT COUNT(*) FROM agent_runs WHERE status IN ('spawning', 'running', 'stalled')",
            [],
            |row| row.get(0),
        )?;

        let total_cost_usd: f64 = conn.query_row(
            "SELECT COALESCE(SUM(cost_usd), 0.0) FROM agent_runs",
            [],
            |row| row.get(0),
        )?;

        let tasks_completed: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE status = 'done'",
            [],
            |row| row.get(0),
        )?;

        let tasks_total: i64 =
            conn.query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))?;

        let goals_active: i64 = conn.query_row(
            "SELECT COUNT(*) FROM goal_spaces WHERE status = 'active'",
            [],
            |row| row.get(0),
        )?;

        Ok(Stats {
            active_agents,
            total_cost_usd,
            tasks_completed,
            tasks_total,
            goals_active,
        })
    }

    // ── Project Queries ──

    pub fn create_project(&self, input: &CreateProject) -> Result<Project> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let settings_json = serde_json::to_string(&GoalSettings::default())?;

        let conn = self.conn();
        conn.execute(
            "INSERT INTO projects (id, path, display_name, sort_order, settings, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, input.path, input.display_name, input.sort_order, settings_json, now, now],
        )?;

        Ok(Project {
            id,
            path: input.path.clone(),
            display_name: input.display_name.clone(),
            sort_order: input.sort_order,
            settings: GoalSettings::default(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, path, display_name, sort_order, settings, created_at, updated_at
             FROM projects ORDER BY sort_order ASC, created_at ASC",
        )?;

        let projects = stmt
            .query_map([], |row| {
                let settings_str: String = row.get(4)?;
                let settings: GoalSettings =
                    serde_json::from_str(&settings_str).unwrap_or_default();
                Ok(Project {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    display_name: row.get(2)?,
                    sort_order: row.get(3)?,
                    settings,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(projects)
    }

    pub fn get_project(&self, id: &str) -> Result<Option<Project>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, path, display_name, sort_order, settings, created_at, updated_at
             FROM projects WHERE id = ?1",
        )?;

        let project = stmt
            .query_row(params![id], |row| {
                let settings_str: String = row.get(4)?;
                let settings: GoalSettings =
                    serde_json::from_str(&settings_str).unwrap_or_default();
                Ok(Project {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    display_name: row.get(2)?,
                    sort_order: row.get(3)?,
                    settings,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .optional()?;

        Ok(project)
    }

    pub fn update_project(&self, id: &str, input: &UpdateProject) -> Result<()> {
        let conn = self.conn();
        let now = Utc::now().to_rfc3339();

        if let Some(ref path) = input.path {
            conn.execute(
                "UPDATE projects SET path = ?1, updated_at = ?2 WHERE id = ?3",
                params![path, now, id],
            )?;
        }
        if let Some(ref display_name) = input.display_name {
            conn.execute(
                "UPDATE projects SET display_name = ?1, updated_at = ?2 WHERE id = ?3",
                params![display_name, now, id],
            )?;
        }
        if let Some(sort_order) = input.sort_order {
            conn.execute(
                "UPDATE projects SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
                params![sort_order, now, id],
            )?;
        }
        if let Some(ref settings) = input.settings {
            let json = serde_json::to_string(settings).map_err(|e| anyhow::anyhow!(e))?;
            conn.execute(
                "UPDATE projects SET settings = ?1, updated_at = ?2 WHERE id = ?3",
                params![json, now, id],
            )?;
        }

        Ok(())
    }

    pub fn delete_project(&self, id: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn list_goals_by_project(&self, project_id: &str) -> Result<Vec<GoalSpace>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, status, repo_path, created_at, updated_at, settings
             FROM goal_spaces WHERE project_id = ?1 ORDER BY created_at DESC",
        )?;

        let goals = stmt
            .query_map(params![project_id], |row| {
                let settings_str: String = row.get(7)?;
                let settings: GoalSettings =
                    serde_json::from_str(&settings_str).unwrap_or_default();
                Ok(GoalSpace {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    status: row.get(3)?,
                    repo_path: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    settings,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(goals)
    }

    // ── Goal Message Queries ──

    pub fn create_goal_message(&self, input: &CreateGoalMessage) -> Result<GoalMessage> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let conn = self.conn();
        conn.execute(
            "INSERT INTO goal_messages (id, goal_space_id, role, content, message_type, metadata_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, input.goal_space_id, input.role, input.content, input.message_type, input.metadata_json, now],
        )?;

        Ok(GoalMessage {
            id,
            goal_space_id: input.goal_space_id.clone(),
            role: input.role.clone(),
            content: input.content.clone(),
            message_type: input.message_type.clone(),
            metadata_json: input.metadata_json.clone(),
            created_at: now,
        })
    }

    pub fn list_goal_messages(&self, goal_space_id: &str) -> Result<Vec<GoalMessage>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, goal_space_id, role, content, message_type, metadata_json, created_at
             FROM goal_messages WHERE goal_space_id = ?1 ORDER BY created_at ASC",
        )?;

        let messages = stmt
            .query_map(params![goal_space_id], |row| {
                Ok(GoalMessage {
                    id: row.get(0)?,
                    goal_space_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    message_type: row.get(4)?,
                    metadata_json: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(messages)
    }

    pub fn delete_goal_messages(&self, goal_space_id: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "DELETE FROM goal_messages WHERE goal_space_id = ?1",
            params![goal_space_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn test_db() -> Database {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations().unwrap();
        db
    }

    // ── Goal Space tests ──

    #[test]
    fn test_create_goal_space() {
        let db = test_db();
        let input = CreateGoalSpace {
            name: "Test Goal".into(),
            description: "Test description".into(),
            repo_path: "/tmp/test".into(),
            settings: Default::default(),
        };
        let goal = db.create_goal_space(&input).unwrap();
        assert_eq!(goal.name, "Test Goal");
        assert_eq!(goal.status, "active");
        assert!(!goal.id.is_empty());
    }

    #[test]
    fn test_list_goal_spaces() {
        let db = test_db();
        // Empty initially
        let goals = db.list_goal_spaces().unwrap();
        assert!(goals.is_empty());

        // Create two
        db.create_goal_space(&CreateGoalSpace {
            name: "Goal 1".into(),
            description: "Desc 1".into(),
            repo_path: "/tmp/1".into(),
            settings: Default::default(),
        })
        .unwrap();
        db.create_goal_space(&CreateGoalSpace {
            name: "Goal 2".into(),
            description: "Desc 2".into(),
            repo_path: "/tmp/2".into(),
            settings: Default::default(),
        })
        .unwrap();

        let goals = db.list_goal_spaces().unwrap();
        assert_eq!(goals.len(), 2);
    }

    #[test]
    fn test_get_goal_space() {
        let db = test_db();
        let created = db
            .create_goal_space(&CreateGoalSpace {
                name: "Find Me".into(),
                description: "Desc".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();

        let found = db.get_goal_space(&created.id).unwrap().unwrap();
        assert_eq!(found.name, "Find Me");
        assert_eq!(found.id, created.id);

        // Not found
        let missing = db.get_goal_space("nonexistent").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_update_goal_space() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "Original".into(),
                description: "Desc".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();

        db.update_goal_space(&goal.id, Some("Updated"), None, None)
            .unwrap();
        let updated = db.get_goal_space(&goal.id).unwrap().unwrap();
        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.description, "Desc"); // unchanged

        db.update_goal_space(&goal.id, None, None, Some("completed"))
            .unwrap();
        let completed = db.get_goal_space(&goal.id).unwrap().unwrap();
        assert_eq!(completed.status, "completed");
    }

    #[test]
    fn test_delete_goal_space_archives() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "To Archive".into(),
                description: "Desc".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();

        db.delete_goal_space(&goal.id).unwrap();
        let archived = db.get_goal_space(&goal.id).unwrap().unwrap();
        assert_eq!(archived.status, "archived");
    }

    // ── Task tests ──

    #[test]
    fn test_create_task() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();

        let task = db
            .create_task(
                &goal.id,
                &CreateTask {
                    title: "Task 1".into(),
                    description: "Do something".into(),
                    priority: 5,
                    depends_on: vec![],
                    settings: Default::default(),
                },
            )
            .unwrap();

        assert_eq!(task.title, "Task 1");
        assert_eq!(task.status, "pending");
        assert_eq!(task.priority, 5);
        assert!(task.depends_on.is_empty());
    }

    #[test]
    fn test_list_tasks_ordered_by_priority() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();

        db.create_task(
            &goal.id,
            &CreateTask {
                title: "Low".into(),
                description: "D".into(),
                priority: 1,
                depends_on: vec![],
                settings: Default::default(),
            },
        )
        .unwrap();
        db.create_task(
            &goal.id,
            &CreateTask {
                title: "High".into(),
                description: "D".into(),
                priority: 10,
                depends_on: vec![],
                settings: Default::default(),
            },
        )
        .unwrap();
        db.create_task(
            &goal.id,
            &CreateTask {
                title: "Med".into(),
                description: "D".into(),
                priority: 5,
                depends_on: vec![],
                settings: Default::default(),
            },
        )
        .unwrap();

        let tasks = db.list_tasks(&goal.id).unwrap();
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].title, "High");
        assert_eq!(tasks[1].title, "Med");
        assert_eq!(tasks[2].title, "Low");
    }

    #[test]
    fn test_get_task() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();
        let task = db
            .create_task(
                &goal.id,
                &CreateTask {
                    title: "T".into(),
                    description: "D".into(),
                    priority: 0,
                    depends_on: vec![],
                    settings: Default::default(),
                },
            )
            .unwrap();

        let found = db.get_task(&task.id).unwrap().unwrap();
        assert_eq!(found.title, "T");

        let missing = db.get_task("nope").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_update_task() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();
        let task = db
            .create_task(
                &goal.id,
                &CreateTask {
                    title: "Original".into(),
                    description: "D".into(),
                    priority: 0,
                    depends_on: vec![],
                    settings: Default::default(),
                },
            )
            .unwrap();

        db.update_task(
            &task.id,
            &UpdateTask {
                title: Some("Updated".into()),
                status: Some("running".into()),
                description: None,
                priority: None,
                depends_on: None,
                ..Default::default()
            },
        )
        .unwrap();

        let updated = db.get_task(&task.id).unwrap().unwrap();
        assert_eq!(updated.title, "Updated");
        assert_eq!(updated.status, "running");
    }

    #[test]
    fn test_task_with_dependencies() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();

        let t1 = db
            .create_task(
                &goal.id,
                &CreateTask {
                    title: "T1".into(),
                    description: "D".into(),
                    priority: 0,
                    depends_on: vec![],
                    settings: Default::default(),
                },
            )
            .unwrap();

        let t2 = db
            .create_task(
                &goal.id,
                &CreateTask {
                    title: "T2".into(),
                    description: "D".into(),
                    priority: 0,
                    depends_on: vec![t1.id.clone()],
                    settings: Default::default(),
                },
            )
            .unwrap();

        let found = db.get_task(&t2.id).unwrap().unwrap();
        assert_eq!(found.depends_on, vec![t1.id]);
    }

    #[test]
    fn test_get_unblocked_tasks() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();

        let t1 = db
            .create_task(
                &goal.id,
                &CreateTask {
                    title: "Independent".into(),
                    description: "D".into(),
                    priority: 0,
                    depends_on: vec![],
                    settings: Default::default(),
                },
            )
            .unwrap();

        let t2 = db
            .create_task(
                &goal.id,
                &CreateTask {
                    title: "Depends on T1".into(),
                    description: "D".into(),
                    priority: 0,
                    depends_on: vec![t1.id.clone()],
                    settings: Default::default(),
                },
            )
            .unwrap();

        // Initially only t1 is unblocked
        let unblocked = db.get_unblocked_tasks(&goal.id).unwrap();
        assert_eq!(unblocked.len(), 1);
        assert_eq!(unblocked[0].title, "Independent");

        // Mark t1 as done
        db.update_task(
            &t1.id,
            &UpdateTask {
                status: Some("done".into()),
                title: None,
                description: None,
                priority: None,
                depends_on: None,
                ..Default::default()
            },
        )
        .unwrap();

        // Now t2 is unblocked
        let unblocked = db.get_unblocked_tasks(&goal.id).unwrap();
        assert_eq!(unblocked.len(), 1);
        assert_eq!(unblocked[0].id, t2.id);
    }

    #[test]
    fn test_get_unblocked_tasks_all_independent() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();

        db.create_task(
            &goal.id,
            &CreateTask {
                title: "A".into(),
                description: "D".into(),
                priority: 0,
                depends_on: vec![],
                settings: Default::default(),
            },
        )
        .unwrap();
        db.create_task(
            &goal.id,
            &CreateTask {
                title: "B".into(),
                description: "D".into(),
                priority: 0,
                depends_on: vec![],
                settings: Default::default(),
            },
        )
        .unwrap();
        db.create_task(
            &goal.id,
            &CreateTask {
                title: "C".into(),
                description: "D".into(),
                priority: 0,
                depends_on: vec![],
                settings: Default::default(),
            },
        )
        .unwrap();

        let unblocked = db.get_unblocked_tasks(&goal.id).unwrap();
        assert_eq!(unblocked.len(), 3);
    }

    #[test]
    fn test_get_unblocked_excludes_non_pending() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();

        let t = db
            .create_task(
                &goal.id,
                &CreateTask {
                    title: "Already running".into(),
                    description: "D".into(),
                    priority: 0,
                    depends_on: vec![],
                    settings: Default::default(),
                },
            )
            .unwrap();

        db.update_task(
            &t.id,
            &UpdateTask {
                status: Some("running".into()),
                title: None,
                description: None,
                priority: None,
                depends_on: None,
                ..Default::default()
            },
        )
        .unwrap();

        let unblocked = db.get_unblocked_tasks(&goal.id).unwrap();
        assert!(unblocked.is_empty());
    }

    // ── Agent Run tests ──

    #[test]
    fn test_create_and_get_agent_run() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();
        let task = db
            .create_task(
                &goal.id,
                &CreateTask {
                    title: "T".into(),
                    description: "D".into(),
                    priority: 0,
                    depends_on: vec![],
                    settings: Default::default(),
                },
            )
            .unwrap();

        let run = db
            .create_agent_run(
                &task.id,
                &goal.id,
                Some("/tmp/wt"),
                Some("branch-1"),
                "sonnet",
                Some(5.0),
            )
            .unwrap();

        assert_eq!(run.status, "spawning");
        assert_eq!(run.model, "sonnet");
        assert_eq!(run.cost_usd, 0.0);
        assert_eq!(run.max_budget_usd, Some(5.0));

        let found = db.get_agent_run(&run.id).unwrap().unwrap();
        assert_eq!(found.id, run.id);
        assert_eq!(found.worktree_path, Some("/tmp/wt".into()));
    }

    #[test]
    fn test_update_agent_run_status() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();
        let task = db
            .create_task(
                &goal.id,
                &CreateTask {
                    title: "T".into(),
                    description: "D".into(),
                    priority: 0,
                    depends_on: vec![],
                    settings: Default::default(),
                },
            )
            .unwrap();
        let run = db
            .create_agent_run(&task.id, &goal.id, None, None, "sonnet", None)
            .unwrap();

        db.update_agent_run_status(&run.id, "running").unwrap();
        let updated = db.get_agent_run(&run.id).unwrap().unwrap();
        assert_eq!(updated.status, "running");
        assert!(updated.finished_at.is_none());

        db.update_agent_run_status(&run.id, "done").unwrap();
        let done = db.get_agent_run(&run.id).unwrap().unwrap();
        assert_eq!(done.status, "done");
        assert!(done.finished_at.is_some());
    }

    #[test]
    fn test_update_agent_run_cost() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();
        let task = db
            .create_task(
                &goal.id,
                &CreateTask {
                    title: "T".into(),
                    description: "D".into(),
                    priority: 0,
                    depends_on: vec![],
                    settings: Default::default(),
                },
            )
            .unwrap();
        let run = db
            .create_agent_run(&task.id, &goal.id, None, None, "sonnet", None)
            .unwrap();

        db.update_agent_run_cost(&run.id, 1.23, 1000, 500).unwrap();
        let updated = db.get_agent_run(&run.id).unwrap().unwrap();
        assert!((updated.cost_usd - 1.23).abs() < f64::EPSILON);
        assert_eq!(updated.input_tokens, 1000);
        assert_eq!(updated.output_tokens, 500);
        assert!(updated.last_activity_at.is_some());
    }

    #[test]
    fn test_update_agent_run_session_id() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();
        let task = db
            .create_task(
                &goal.id,
                &CreateTask {
                    title: "T".into(),
                    description: "D".into(),
                    priority: 0,
                    depends_on: vec![],
                    settings: Default::default(),
                },
            )
            .unwrap();
        let run = db
            .create_agent_run(&task.id, &goal.id, None, None, "sonnet", None)
            .unwrap();

        assert!(run.claude_session_id.is_none());

        db.update_agent_run_session_id(&run.id, "sess-abc-123")
            .unwrap();
        let updated = db.get_agent_run(&run.id).unwrap().unwrap();
        assert_eq!(updated.claude_session_id, Some("sess-abc-123".into()));
    }

    #[test]
    fn test_list_agent_runs() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();
        let task = db
            .create_task(
                &goal.id,
                &CreateTask {
                    title: "T".into(),
                    description: "D".into(),
                    priority: 0,
                    depends_on: vec![],
                    settings: Default::default(),
                },
            )
            .unwrap();

        assert!(db.list_agent_runs().unwrap().is_empty());

        db.create_agent_run(&task.id, &goal.id, None, None, "sonnet", None)
            .unwrap();
        db.create_agent_run(&task.id, &goal.id, None, None, "opus", None)
            .unwrap();

        let runs = db.list_agent_runs().unwrap();
        assert_eq!(runs.len(), 2);
    }

    #[test]
    fn test_list_active_agent_runs() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();
        let task = db
            .create_task(
                &goal.id,
                &CreateTask {
                    title: "T".into(),
                    description: "D".into(),
                    priority: 0,
                    depends_on: vec![],
                    settings: Default::default(),
                },
            )
            .unwrap();

        let r1 = db
            .create_agent_run(&task.id, &goal.id, None, None, "sonnet", None)
            .unwrap();
        let r2 = db
            .create_agent_run(&task.id, &goal.id, None, None, "sonnet", None)
            .unwrap();

        // r1 is spawning (active), r2 we'll mark done
        db.update_agent_run_status(&r2.id, "done").unwrap();

        let active = db.list_active_agent_runs().unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, r1.id);
    }

    // ── Agent Event tests ──

    #[test]
    fn test_insert_and_list_events() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();
        let task = db
            .create_task(
                &goal.id,
                &CreateTask {
                    title: "T".into(),
                    description: "D".into(),
                    priority: 0,
                    depends_on: vec![],
                    settings: Default::default(),
                },
            )
            .unwrap();
        let run = db
            .create_agent_run(&task.id, &goal.id, None, None, "sonnet", None)
            .unwrap();

        db.insert_agent_event(
            &run.id,
            "tool_call",
            Some("Read"),
            "Reading src/main.rs",
            None,
            None,
        )
        .unwrap();
        db.insert_agent_event(&run.id, "cost_update", None, "API call", None, Some(0.05))
            .unwrap();
        db.insert_agent_event(&run.id, "text_output", None, "Hello", None, None)
            .unwrap();

        let events = db.list_agent_events(&run.id).unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].event_type, "tool_call");
        assert_eq!(events[0].tool_name, Some("Read".into()));
        assert_eq!(events[1].cost_delta_usd, Some(0.05));
        assert_eq!(events[2].summary, "Hello");
    }

    #[test]
    fn test_events_ordered_by_id() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();
        let task = db
            .create_task(
                &goal.id,
                &CreateTask {
                    title: "T".into(),
                    description: "D".into(),
                    priority: 0,
                    depends_on: vec![],
                    settings: Default::default(),
                },
            )
            .unwrap();
        let run = db
            .create_agent_run(&task.id, &goal.id, None, None, "sonnet", None)
            .unwrap();

        db.insert_agent_event(&run.id, "a", None, "first", None, None)
            .unwrap();
        db.insert_agent_event(&run.id, "b", None, "second", None, None)
            .unwrap();
        db.insert_agent_event(&run.id, "c", None, "third", None, None)
            .unwrap();

        let events = db.list_agent_events(&run.id).unwrap();
        assert!(events[0].id < events[1].id);
        assert!(events[1].id < events[2].id);
        assert_eq!(events[0].summary, "first");
        assert_eq!(events[2].summary, "third");
    }

    // ── Stats tests ──

    #[test]
    fn test_stats_empty() {
        let db = test_db();
        let stats = db.get_stats().unwrap();
        assert_eq!(stats.active_agents, 0);
        assert!((stats.total_cost_usd - 0.0).abs() < f64::EPSILON);
        assert_eq!(stats.tasks_completed, 0);
        assert_eq!(stats.tasks_total, 0);
        assert_eq!(stats.goals_active, 0);
    }

    #[test]
    fn test_stats_with_data() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();

        let t1 = db
            .create_task(
                &goal.id,
                &CreateTask {
                    title: "T1".into(),
                    description: "D".into(),
                    priority: 0,
                    depends_on: vec![],
                    settings: Default::default(),
                },
            )
            .unwrap();
        let t2 = db
            .create_task(
                &goal.id,
                &CreateTask {
                    title: "T2".into(),
                    description: "D".into(),
                    priority: 0,
                    depends_on: vec![],
                    settings: Default::default(),
                },
            )
            .unwrap();

        // Mark t1 done
        db.update_task(
            &t1.id,
            &UpdateTask {
                status: Some("done".into()),
                title: None,
                description: None,
                priority: None,
                depends_on: None,
                ..Default::default()
            },
        )
        .unwrap();

        // Create an active agent
        let run = db
            .create_agent_run(&t2.id, &goal.id, None, None, "sonnet", None)
            .unwrap();
        db.update_agent_run_status(&run.id, "running").unwrap();
        db.update_agent_run_cost(&run.id, 2.50, 5000, 2000).unwrap();

        let stats = db.get_stats().unwrap();
        assert_eq!(stats.active_agents, 1);
        assert!((stats.total_cost_usd - 2.50).abs() < f64::EPSILON);
        assert_eq!(stats.tasks_completed, 1);
        assert_eq!(stats.tasks_total, 2);
        assert_eq!(stats.goals_active, 1);
    }

    // ── Goal History tests ──

    #[test]
    fn test_goal_history_created_on_goal_creation() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();

        // History should have a "created" entry from create_goal_space
        let conn = db.conn();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM goal_space_history WHERE goal_space_id = ?1 AND event_type = 'created'",
                rusqlite::params![goal.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_goal_history_on_task_creation() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();

        db.create_task(
            &goal.id,
            &CreateTask {
                title: "Task".into(),
                description: "D".into(),
                priority: 0,
                depends_on: vec![],
                settings: Default::default(),
            },
        )
        .unwrap();

        let conn = db.conn();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM goal_space_history WHERE goal_space_id = ?1 AND event_type = 'task_added'",
                rusqlite::params![goal.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    // ── Project tests ──

    #[test]
    fn test_create_project() {
        let db = test_db();
        let project = db
            .create_project(&CreateProject {
                path: "/tmp/myproject".into(),
                display_name: "My Project".into(),
                sort_order: 0,
            })
            .unwrap();
        assert_eq!(project.display_name, "My Project");
        assert_eq!(project.path, "/tmp/myproject");
        assert!(!project.id.is_empty());
    }

    #[test]
    fn test_list_projects() {
        let db = test_db();
        assert!(db.list_projects().unwrap().is_empty());

        db.create_project(&CreateProject {
            path: "/tmp/a".into(),
            display_name: "A".into(),
            sort_order: 1,
        })
        .unwrap();
        db.create_project(&CreateProject {
            path: "/tmp/b".into(),
            display_name: "B".into(),
            sort_order: 0,
        })
        .unwrap();

        let projects = db.list_projects().unwrap();
        assert_eq!(projects.len(), 2);
        // Ordered by sort_order ASC
        assert_eq!(projects[0].display_name, "B");
        assert_eq!(projects[1].display_name, "A");
    }

    #[test]
    fn test_get_project() {
        let db = test_db();
        let created = db
            .create_project(&CreateProject {
                path: "/tmp/find".into(),
                display_name: "Find Me".into(),
                sort_order: 0,
            })
            .unwrap();

        let found = db.get_project(&created.id).unwrap().unwrap();
        assert_eq!(found.display_name, "Find Me");

        let missing = db.get_project("nonexistent").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_update_project() {
        let db = test_db();
        let project = db
            .create_project(&CreateProject {
                path: "/tmp/orig".into(),
                display_name: "Original".into(),
                sort_order: 0,
            })
            .unwrap();

        db.update_project(
            &project.id,
            &UpdateProject {
                display_name: Some("Updated".into()),
                path: None,
                sort_order: None,
                settings: None,
            },
        )
        .unwrap();

        let updated = db.get_project(&project.id).unwrap().unwrap();
        assert_eq!(updated.display_name, "Updated");
        assert_eq!(updated.path, "/tmp/orig"); // unchanged
    }

    #[test]
    fn test_delete_project() {
        let db = test_db();
        let project = db
            .create_project(&CreateProject {
                path: "/tmp/del".into(),
                display_name: "Delete Me".into(),
                sort_order: 0,
            })
            .unwrap();

        db.delete_project(&project.id).unwrap();
        let deleted = db.get_project(&project.id).unwrap();
        assert!(deleted.is_none());
    }

    #[test]
    fn test_list_goals_by_project() {
        let db = test_db();
        let project = db
            .create_project(&CreateProject {
                path: "/tmp/proj".into(),
                display_name: "Proj".into(),
                sort_order: 0,
            })
            .unwrap();

        // Create a goal and manually set its project_id
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp/proj".into(),
                settings: Default::default(),
            })
            .unwrap();
        {
            let conn = db.conn();
            conn.execute(
                "UPDATE goal_spaces SET project_id = ?1 WHERE id = ?2",
                rusqlite::params![project.id, goal.id],
            )
            .unwrap();
        }

        let goals = db.list_goals_by_project(&project.id).unwrap();
        assert_eq!(goals.len(), 1);
        assert_eq!(goals[0].name, "G");

        // No goals for a different project
        let empty = db.list_goals_by_project("other").unwrap();
        assert!(empty.is_empty());
    }

    // ── Goal Message tests ──

    #[test]
    fn test_create_goal_message() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();

        let msg = db
            .create_goal_message(&CreateGoalMessage {
                goal_space_id: goal.id.clone(),
                role: "user".into(),
                content: "Hello".into(),
                message_type: "text".into(),
                metadata_json: "{}".into(),
            })
            .unwrap();

        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");
        assert!(!msg.id.is_empty());
    }

    #[test]
    fn test_list_goal_messages() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();

        db.create_goal_message(&CreateGoalMessage {
            goal_space_id: goal.id.clone(),
            role: "user".into(),
            content: "First".into(),
            message_type: "text".into(),
            metadata_json: "{}".into(),
        })
        .unwrap();
        db.create_goal_message(&CreateGoalMessage {
            goal_space_id: goal.id.clone(),
            role: "assistant".into(),
            content: "Second".into(),
            message_type: "text".into(),
            metadata_json: "{}".into(),
        })
        .unwrap();

        let messages = db.list_goal_messages(&goal.id).unwrap();
        assert_eq!(messages.len(), 2);
        // Ordered by created_at ASC
        assert_eq!(messages[0].content, "First");
        assert_eq!(messages[1].content, "Second");
    }

    #[test]
    fn test_delete_goal_messages() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
                settings: Default::default(),
            })
            .unwrap();

        db.create_goal_message(&CreateGoalMessage {
            goal_space_id: goal.id.clone(),
            role: "user".into(),
            content: "Msg".into(),
            message_type: "text".into(),
            metadata_json: "{}".into(),
        })
        .unwrap();

        assert_eq!(db.list_goal_messages(&goal.id).unwrap().len(), 1);
        db.delete_goal_messages(&goal.id).unwrap();
        assert!(db.list_goal_messages(&goal.id).unwrap().is_empty());
    }
}

// Add the optional() helper for rusqlite
trait OptionalExt<T> {
    fn optional(self) -> std::result::Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for std::result::Result<T, rusqlite::Error> {
    fn optional(self) -> std::result::Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
