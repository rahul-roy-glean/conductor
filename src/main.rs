mod agent;
mod cli;
mod db;
mod goal;
mod hooks;
mod server;

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

use crate::cli::{Cli, Commands};
use crate::db::Database;
use crate::server::AppState;

/// Returns the path to the conductor database, creating the directory if needed.
/// Uses `~/.conductor/conductor.db` by default, overridable with `CONDUCTOR_DB`.
fn db_path() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("CONDUCTOR_DB") {
        return Ok(PathBuf::from(p));
    }
    let dir = dirs::home_dir()
        .context("Could not determine home directory")?
        .join(".conductor");
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create {}", dir.display()))?;
    Ok(dir.join("conductor.db"))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("conductor=info".parse()?))
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Server { port } => {
            let (state, dispatch_rx) = init_app_state().await?;
            server::run(state, port, dispatch_rx).await?;
        }
        Commands::Ui { port } => {
            let (state, dispatch_rx) = init_app_state().await?;
            // Open browser
            let url = format!("http://localhost:{}", port);
            tracing::info!("Opening UI at {}", url);
            let _ = open::that(&url);
            server::run(state, port, dispatch_rx).await?;
        }
        Commands::Goal { command } => {
            cli::handle_goal_command(command).await?;
        }
        Commands::Status => {
            cli::handle_status().await?;
        }
        Commands::Inspect { agent_id } => {
            cli::handle_inspect(&agent_id).await?;
        }
        Commands::Nudge { agent_id, message } => {
            cli::handle_nudge(&agent_id, &message).await?;
        }
        Commands::Kill { agent_id } => {
            cli::handle_kill(&agent_id).await?;
        }
        Commands::Logs { agent_id } => {
            cli::handle_logs(&agent_id).await?;
        }
        Commands::Cleanup => {
            let db = Database::open(&db_path()?)?;
            db.run_migrations()?;
            cli::handle_cleanup(&db).await?;
        }
    }

    Ok(())
}

async fn init_app_state() -> Result<(
    Arc<AppState>,
    tokio::sync::mpsc::UnboundedReceiver<agent::session::DispatchMessage>,
)> {
    let path = db_path()?;
    tracing::info!("Using database at {}", path.display());
    let db = Database::open(&path)?;
    db.run_migrations()?;

    // Clean up stale state from previous runs (crashed agents, orphaned worktrees)
    match agent::worktree::cleanup_stale(&db, &[]).await {
        Ok(report) => {
            if report.runs_marked_failed > 0
                || report.branches_deleted > 0
                || report.worktrees_removed > 0
            {
                tracing::info!("Startup {}", report);
            }
        }
        Err(e) => tracing::warn!("Startup cleanup failed (non-fatal): {}", e),
    }

    let (event_tx, _) = tokio::sync::broadcast::channel(1024);
    let (dispatch_tx, dispatch_rx) = tokio::sync::mpsc::unbounded_channel();

    let agent_manager = agent::session::AgentManager::new(db.clone(), event_tx.clone(), dispatch_tx);

    Ok((
        Arc::new(AppState {
            db,
            agent_manager,
            event_tx,
        }),
        dispatch_rx,
    ))
}
