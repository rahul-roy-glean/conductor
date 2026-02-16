use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "conductor", about = "Orchestrate multiple Claude Code agents")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the backend server
    Server {
        /// Port to listen on
        #[arg(short, long, default_value = "3001")]
        port: u16,
    },
    /// Start backend and open React UI in browser
    Ui {
        /// Port to listen on
        #[arg(short, long, default_value = "3001")]
        port: u16,
    },
    /// Manage goal spaces
    Goal {
        #[command(subcommand)]
        command: GoalCommands,
    },
    /// Show fleet status overview
    Status,
    /// Inspect a specific agent
    Inspect {
        /// Agent run ID
        agent_id: String,
    },
    /// Send a message to a running agent
    Nudge {
        /// Agent run ID
        agent_id: String,
        /// Message to send
        message: String,
    },
    /// Terminate an agent
    Kill {
        /// Agent run ID
        agent_id: String,
    },
    /// Tail agent event logs
    Logs {
        /// Agent run ID
        agent_id: String,
    },
    /// Clean up stale worktrees, orphaned branches, and stuck agent runs
    Cleanup,
}

#[derive(Subcommand)]
pub enum GoalCommands {
    /// Create a new goal space
    Create {
        /// Goal description
        description: String,
        /// Path to the git repository
        #[arg(long)]
        repo: String,
        /// Goal name (defaults to first line of description)
        #[arg(long)]
        name: Option<String>,
    },
    /// List all goal spaces
    List,
    /// Decompose a goal into tasks
    Decompose {
        /// Goal space ID
        goal_id: String,
    },
    /// Start dispatching tasks to agents
    Dispatch {
        /// Goal space ID
        goal_id: String,
    },
}

const DEFAULT_API_BASE: &str = "http://localhost:3001";

pub async fn handle_goal_command(command: GoalCommands) -> Result<()> {
    let client = reqwest::Client::new();

    match command {
        GoalCommands::Create {
            description,
            repo,
            name,
        } => {
            let name = name.unwrap_or_else(|| {
                description
                    .lines()
                    .next()
                    .unwrap_or(&description)
                    .chars()
                    .take(80)
                    .collect()
            });

            let resp = client
                .post(format!("{}/api/goals", DEFAULT_API_BASE))
                .json(&serde_json::json!({
                    "name": name,
                    "description": description,
                    "repo_path": repo,
                }))
                .send()
                .await?;

            if resp.status().is_success() {
                let goal: serde_json::Value = resp.json().await?;
                println!("Created goal space: {}", goal["id"]);
                println!("  Name: {}", goal["name"]);
            } else {
                let err = resp.text().await?;
                anyhow::bail!("Failed to create goal: {}", err);
            }
        }
        GoalCommands::List => {
            let resp = client
                .get(format!("{}/api/goals", DEFAULT_API_BASE))
                .send()
                .await?;

            if resp.status().is_success() {
                let goals: Vec<serde_json::Value> = resp.json().await?;
                if goals.is_empty() {
                    println!("No goal spaces found.");
                } else {
                    println!(
                        "{:<38} {:<10} {:<40} {}",
                        "ID", "STATUS", "NAME", "CREATED"
                    );
                    println!("{}", "-".repeat(100));
                    for goal in goals {
                        println!(
                            "{:<38} {:<10} {:<40} {}",
                            goal["id"].as_str().unwrap_or(""),
                            goal["status"].as_str().unwrap_or(""),
                            goal["name"].as_str().unwrap_or(""),
                            goal["created_at"].as_str().unwrap_or(""),
                        );
                    }
                }
            } else {
                let err = resp.text().await?;
                anyhow::bail!("Failed to list goals: {}", err);
            }
        }
        GoalCommands::Decompose { goal_id } => {
            println!("Decomposing goal {}...", goal_id);
            let resp = client
                .post(format!("{}/api/goals/{}/decompose", DEFAULT_API_BASE, goal_id))
                .send()
                .await?;

            if resp.status().is_success() {
                let tasks: Vec<serde_json::Value> = resp.json().await?;
                println!("Proposed {} tasks:", tasks.len());
                for (i, task) in tasks.iter().enumerate() {
                    println!(
                        "  {}. {} (depends on: {:?})",
                        i + 1,
                        task["title"].as_str().unwrap_or(""),
                        task["depends_on"]
                    );
                    if let Some(desc) = task["description"].as_str() {
                        for line in desc.lines().take(3) {
                            println!("     {}", line);
                        }
                    }
                }
            } else {
                let err = resp.text().await?;
                anyhow::bail!("Failed to decompose goal: {}", err);
            }
        }
        GoalCommands::Dispatch { goal_id } => {
            println!("Dispatching tasks for goal {}...", goal_id);
            let resp = client
                .post(format!("{}/api/goals/{}/dispatch", DEFAULT_API_BASE, goal_id))
                .send()
                .await?;

            if resp.status().is_success() {
                let result: serde_json::Value = resp.json().await?;
                println!(
                    "Dispatched {} agents",
                    result["agents_spawned"].as_u64().unwrap_or(0)
                );
            } else {
                let err = resp.text().await?;
                anyhow::bail!("Failed to dispatch: {}", err);
            }
        }
    }

    Ok(())
}

pub async fn handle_status() -> Result<()> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/api/agents", DEFAULT_API_BASE))
        .send()
        .await?;

    if resp.status().is_success() {
        let agents: Vec<serde_json::Value> = resp.json().await?;
        if agents.is_empty() {
            println!("No active agents.");
        } else {
            println!(
                "{:<38} {:<10} {:<10} {:<10} {:<30}",
                "ID", "STATUS", "MODEL", "COST", "TASK"
            );
            println!("{}", "-".repeat(100));
            for agent in agents {
                println!(
                    "{:<38} {:<10} {:<10} ${:<9.4} {:<30}",
                    agent["id"].as_str().unwrap_or(""),
                    agent["status"].as_str().unwrap_or(""),
                    agent["model"].as_str().unwrap_or(""),
                    agent["cost_usd"].as_f64().unwrap_or(0.0),
                    agent["task_id"].as_str().unwrap_or(""),
                );
            }
        }
    } else {
        let err = resp.text().await?;
        anyhow::bail!("Failed to get status: {}", err);
    }

    Ok(())
}

pub async fn handle_inspect(agent_id: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/api/agents/{}", DEFAULT_API_BASE, agent_id))
        .send()
        .await?;

    if resp.status().is_success() {
        let agent: serde_json::Value = resp.json().await?;
        println!("Agent: {}", agent["id"]);
        println!("  Status:     {}", agent["status"]);
        println!("  Model:      {}", agent["model"]);
        println!("  Cost:       ${:.4}", agent["cost_usd"].as_f64().unwrap_or(0.0));
        println!("  Task:       {}", agent["task_id"]);
        println!("  Session:    {}", agent["claude_session_id"]);
        println!("  Worktree:   {}", agent["worktree_path"]);
        println!("  Branch:     {}", agent["branch"]);
        println!("  Started:    {}", agent["started_at"]);
        println!("  Last Active:{}", agent["last_activity_at"]);

        // Fetch recent events
        let events_resp = client
            .get(format!(
                "{}/api/agents/{}/events",
                DEFAULT_API_BASE, agent_id
            ))
            .send()
            .await?;

        if events_resp.status().is_success() {
            let events: Vec<serde_json::Value> = events_resp.json().await?;
            println!("\n  Recent Events:");
            for event in events.iter().rev().take(20).rev() {
                println!(
                    "    [{}] {} {}",
                    event["created_at"].as_str().unwrap_or(""),
                    event["event_type"].as_str().unwrap_or(""),
                    event["summary"].as_str().unwrap_or(""),
                );
            }
        }
    } else {
        let err = resp.text().await?;
        anyhow::bail!("Agent not found: {}", err);
    }

    Ok(())
}

pub async fn handle_nudge(agent_id: &str, message: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/api/agents/{}/nudge", DEFAULT_API_BASE, agent_id))
        .json(&serde_json::json!({ "message": message }))
        .send()
        .await?;

    if resp.status().is_success() {
        println!("Nudged agent {}", agent_id);
    } else {
        let err = resp.text().await?;
        anyhow::bail!("Failed to nudge: {}", err);
    }

    Ok(())
}

pub async fn handle_kill(agent_id: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/api/agents/{}/kill", DEFAULT_API_BASE, agent_id))
        .send()
        .await?;

    if resp.status().is_success() {
        println!("Killed agent {}", agent_id);
    } else {
        let err = resp.text().await?;
        anyhow::bail!("Failed to kill: {}", err);
    }

    Ok(())
}

pub async fn handle_cleanup(db: &crate::db::Database) -> Result<()> {
    println!("Running cleanup...");
    let report = crate::agent::worktree::cleanup_stale(db, &[]).await?;
    println!("{}", report);
    Ok(())
}

pub async fn handle_logs(agent_id: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!(
            "{}/api/agents/{}/events",
            DEFAULT_API_BASE, agent_id
        ))
        .send()
        .await?;

    if resp.status().is_success() {
        let events: Vec<serde_json::Value> = resp.json().await?;
        for event in &events {
            println!(
                "[{}] {}: {}",
                event["created_at"].as_str().unwrap_or(""),
                event["event_type"].as_str().unwrap_or(""),
                event["summary"].as_str().unwrap_or(""),
            );
        }
        if events.is_empty() {
            println!("No events yet for agent {}", agent_id);
        }
    } else {
        let err = resp.text().await?;
        anyhow::bail!("Failed to get logs: {}", err);
    }

    Ok(())
}
