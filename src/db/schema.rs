use anyhow::Result;
use rusqlite::Connection;

pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS goal_spaces (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'active',
            repo_path TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            goal_space_id TEXT NOT NULL REFERENCES goal_spaces(id),
            title TEXT NOT NULL,
            description TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            priority INTEGER NOT NULL DEFAULT 0,
            depends_on TEXT NOT NULL DEFAULT '[]',
            settings TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS agent_runs (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL REFERENCES tasks(id),
            goal_space_id TEXT NOT NULL REFERENCES goal_spaces(id),
            claude_session_id TEXT,
            worktree_path TEXT,
            branch TEXT,
            status TEXT NOT NULL DEFAULT 'spawning',
            model TEXT NOT NULL DEFAULT 'sonnet',
            cost_usd REAL NOT NULL DEFAULT 0.0,
            input_tokens INTEGER NOT NULL DEFAULT 0,
            output_tokens INTEGER NOT NULL DEFAULT 0,
            max_budget_usd REAL,
            started_at TEXT NOT NULL,
            last_activity_at TEXT,
            finished_at TEXT
        );

        CREATE TABLE IF NOT EXISTS agent_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            agent_run_id TEXT NOT NULL REFERENCES agent_runs(id),
            event_type TEXT NOT NULL,
            tool_name TEXT,
            summary TEXT NOT NULL,
            raw_json TEXT,
            cost_delta_usd REAL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS goal_space_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            goal_space_id TEXT NOT NULL REFERENCES goal_spaces(id),
            event_type TEXT NOT NULL,
            description TEXT NOT NULL,
            metadata TEXT,
            created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_tasks_goal ON tasks(goal_space_id);
        CREATE INDEX IF NOT EXISTS idx_agent_runs_task ON agent_runs(task_id);
        CREATE INDEX IF NOT EXISTS idx_agent_runs_goal ON agent_runs(goal_space_id);
        CREATE INDEX IF NOT EXISTS idx_agent_runs_status ON agent_runs(status);
        CREATE INDEX IF NOT EXISTS idx_agent_events_run ON agent_events(agent_run_id);
        CREATE INDEX IF NOT EXISTS idx_goal_history_goal ON goal_space_history(goal_space_id);
        ",
    )?;

    // Migration: Add settings column to goal_spaces table
    // This is safe to run multiple times - it will only add the column if it doesn't exist
    let table_info: Vec<String> = conn
        .prepare("PRAGMA table_info(goal_spaces)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    if !table_info.contains(&"settings".to_string()) {
        conn.execute(
            "ALTER TABLE goal_spaces ADD COLUMN settings TEXT NOT NULL DEFAULT '{}'",
            [],
        )?;
    }

    // Migration: Add settings column to tasks table
    let task_info: Vec<String> = conn
        .prepare("PRAGMA table_info(tasks)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    if !task_info.contains(&"settings".to_string()) {
        conn.execute(
            "ALTER TABLE tasks ADD COLUMN settings TEXT NOT NULL DEFAULT '{}'",
            [],
        )?;
    }

    Ok(())
}
