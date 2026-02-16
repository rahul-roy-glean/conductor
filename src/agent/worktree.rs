use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub const WORKTREE_BASE: &str = "/tmp/conductor/worktrees";

/// Create a git worktree for an agent
pub async fn create_worktree(
    repo_path: &Path,
    agent_id: &str,
    branch_name: &str,
) -> Result<PathBuf> {
    let worktree_path = PathBuf::from(WORKTREE_BASE).join(agent_id);

    // Ensure base directory exists
    tokio::fs::create_dir_all(WORKTREE_BASE)
        .await
        .context("Failed to create worktree base directory")?;

    // Remove existing worktree if it exists
    if worktree_path.exists() {
        if let Err(e) = remove_worktree(repo_path, &worktree_path).await {
            tracing::warn!(
                "Failed to remove existing worktree at {}: {}",
                worktree_path.display(),
                e
            );
        }
    }

    let wt_str = worktree_path
        .to_str()
        .context("Worktree path contains invalid UTF-8")?;

    // Create the worktree with a new branch
    let output = Command::new("git")
        .args(["worktree", "add", wt_str, "-b", branch_name])
        .current_dir(repo_path)
        .output()
        .await
        .context("Failed to run git worktree add")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::debug!(
            "git worktree add -b failed (branch may exist): {}",
            stderr.trim()
        );

        // Branch might already exist, try without -b
        let output = Command::new("git")
            .args(["worktree", "add", wt_str, branch_name])
            .current_dir(repo_path)
            .output()
            .await
            .context("Failed to run git worktree add (existing branch)")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("git worktree add failed: {}", stderr);
        }
    }

    tracing::info!(
        "Created worktree at {} on branch {}",
        worktree_path.display(),
        branch_name
    );

    Ok(worktree_path)
}

/// Remove a git worktree and clean up stale metadata
pub async fn remove_worktree(repo_path: &Path, worktree_path: &Path) -> Result<()> {
    let wt_str = worktree_path
        .to_str()
        .context("Worktree path contains invalid UTF-8")?;

    let output = Command::new("git")
        .args(["worktree", "remove", "--force", wt_str])
        .current_dir(repo_path)
        .output()
        .await
        .context("Failed to run git worktree remove")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("git worktree remove failed: {}", stderr.trim());

        // Fallback: manual cleanup of the directory
        if worktree_path.exists() {
            if let Err(e) = tokio::fs::remove_dir_all(worktree_path).await {
                tracing::error!(
                    "Failed to manually remove worktree directory {}: {}",
                    worktree_path.display(),
                    e
                );
            }
        }
    }

    // Prune stale worktree metadata from the git repo
    let prune_output = Command::new("git")
        .args(["worktree", "prune"])
        .current_dir(repo_path)
        .output()
        .await;

    if let Err(e) = prune_output {
        tracing::warn!("git worktree prune failed: {}", e);
    }

    tracing::info!("Removed worktree at {}", worktree_path.display());
    Ok(())
}

/// Merge a completed agent branch into the repo's current branch (typically main).
/// Returns Ok(true) on success, Ok(false) if there was nothing to merge.
/// On conflict, aborts the merge and returns an error.
pub async fn merge_branch_to_main(repo_path: &Path, branch: &str) -> Result<()> {
    // Detect the default branch
    let head_output = Command::new("git")
        .args(["symbolic-ref", "--short", "HEAD"])
        .current_dir(repo_path)
        .output()
        .await
        .context("Failed to detect default branch")?;

    let default_branch = String::from_utf8_lossy(&head_output.stdout)
        .trim()
        .to_string();

    if default_branch.is_empty() {
        anyhow::bail!("Could not determine default branch");
    }

    tracing::info!(
        "Merging branch {} into {} in {}",
        branch,
        default_branch,
        repo_path.display()
    );

    // Perform the merge
    let merge_output = Command::new("git")
        .args([
            "merge",
            "--no-ff",
            branch,
            "-m",
            &format!("Merge {}", branch),
        ])
        .current_dir(repo_path)
        .output()
        .await
        .context("Failed to run git merge")?;

    if !merge_output.status.success() {
        let stderr = String::from_utf8_lossy(&merge_output.stderr);
        tracing::warn!("Merge conflict for branch {}: {}", branch, stderr.trim());

        // Abort the failed merge to leave the repo clean
        let _ = Command::new("git")
            .args(["merge", "--abort"])
            .current_dir(repo_path)
            .output()
            .await;

        anyhow::bail!("Merge conflict for branch {}: {}", branch, stderr.trim());
    }

    tracing::info!("Successfully merged branch {} into {}", branch, default_branch);
    Ok(())
}

/// Delete a branch after it has been successfully merged
pub async fn delete_branch(repo_path: &Path, branch: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["branch", "-d", branch])
        .current_dir(repo_path)
        .output()
        .await
        .context("Failed to run git branch -d")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("Failed to delete branch {}: {}", branch, stderr.trim());
    } else {
        tracing::info!("Deleted merged branch {}", branch);
    }

    Ok(())
}

/// List all conductor worktrees for a repo
pub async fn list_worktrees(repo_path: &Path) -> Result<Vec<WorktreeInfo>> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .await
        .context("Failed to run git worktree list")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut worktrees = Vec::new();
    let mut current_path = None;
    let mut current_branch = None;

    for line in stdout.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            // Save previous worktree if it's a conductor one
            if let Some(prev_path) = current_path.take() {
                let prev_path_str: String = prev_path;
                if prev_path_str.starts_with(WORKTREE_BASE) {
                    worktrees.push(WorktreeInfo {
                        path: PathBuf::from(&prev_path_str),
                        branch: current_branch.take().unwrap_or_default(),
                    });
                }
            }
            current_path = Some(path.to_string());
            current_branch = None;
        } else if let Some(branch) = line.strip_prefix("branch refs/heads/") {
            current_branch = Some(branch.to_string());
        }
    }

    // Don't forget the last one
    if let Some(path) = current_path {
        if path.starts_with(WORKTREE_BASE) {
            worktrees.push(WorktreeInfo {
                path: PathBuf::from(path),
                branch: current_branch.unwrap_or_default(),
            });
        }
    }

    Ok(worktrees)
}

#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: String,
}

/// Clean up all stale worktrees, orphaned worktree directories, and conductor branches
/// for agent runs that are no longer active.
///
/// This should be called on server startup to recover from crashes, and can also
/// be invoked manually via `conductor cleanup`.
pub async fn cleanup_stale(
    db: &crate::db::Database,
    active_run_ids: &[String],
) -> Result<CleanupReport> {
    let mut report = CleanupReport::default();

    // 1. Mark any "running"/"spawning" agent runs as failed (they're dead if we're here)
    let stale_runs = db.list_active_agent_runs()?;
    for run in &stale_runs {
        if active_run_ids.contains(&run.id) {
            continue;
        }
        tracing::info!(
            "Marking stale agent run {} as failed (was {})",
            run.id,
            run.status
        );
        let _ = db.update_agent_run_status(&run.id, "failed");
        // Also reset the task back to pending so it can be retried
        let _ = db.update_task(
            &run.task_id,
            &crate::db::queries::UpdateTask {
                status: Some("pending".to_string()),
                title: None,
                description: None,
                priority: None,
                depends_on: None,
            ..Default::default()
            },
        );
        report.runs_marked_failed += 1;
    }

    // 2. Collect all repo paths from goal spaces so we can prune worktrees
    let goals = db.list_goal_spaces()?;
    let mut repo_paths: Vec<String> = goals.iter().map(|g| g.repo_path.clone()).collect();
    repo_paths.sort();
    repo_paths.dedup();

    for repo_path_str in &repo_paths {
        let repo_path = Path::new(repo_path_str);
        if !repo_path.exists() {
            continue;
        }

        // Prune stale worktree metadata
        let _ = Command::new("git")
            .args(["worktree", "prune"])
            .current_dir(repo_path)
            .output()
            .await;

        // Delete conductor/* branches that are no longer needed
        let branch_output = Command::new("git")
            .args(["branch", "--list", "conductor/*"])
            .current_dir(repo_path)
            .output()
            .await;

        if let Ok(output) = branch_output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let branch = line.trim().trim_start_matches("* ");
                if branch.is_empty() || !branch.starts_with("conductor/") {
                    continue;
                }
                // Try to delete — will fail if not fully merged, which is fine
                let del = Command::new("git")
                    .args(["branch", "-d", branch])
                    .current_dir(repo_path)
                    .output()
                    .await;
                match del {
                    Ok(o) if o.status.success() => {
                        tracing::info!("Deleted stale branch {}", branch);
                        report.branches_deleted += 1;
                    }
                    Ok(o) => {
                        let stderr = String::from_utf8_lossy(&o.stderr);
                        // If not merged, try force-delete only if the agent run is done/failed
                        if stderr.contains("not fully merged") {
                            // Check if any agent run references this branch and is terminal
                            let runs = db.list_agent_runs()?;
                            let is_terminal = runs.iter().any(|r| {
                                r.branch.as_deref() == Some(branch)
                                    && (r.status == "done"
                                        || r.status == "failed"
                                        || r.status == "killed")
                            });
                            if is_terminal {
                                tracing::warn!(
                                    "Branch {} is not merged but agent is terminal — keeping for manual review",
                                    branch
                                );
                                report.unmerged_branches.push(branch.to_string());
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to delete branch {}: {}", branch, e);
                    }
                }
            }
        }
    }

    // 3. Remove orphaned worktree directories from disk
    let worktree_base = Path::new(WORKTREE_BASE);
    if worktree_base.exists() {
        if let Ok(mut entries) = tokio::fs::read_dir(worktree_base).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let dir_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                // Check if any active run owns this worktree
                let is_active = active_run_ids.iter().any(|id| dir_name.contains(id));
                if !is_active {
                    tracing::info!("Removing orphaned worktree directory: {}", path.display());
                    if let Err(e) = tokio::fs::remove_dir_all(&path).await {
                        tracing::warn!(
                            "Failed to remove orphaned worktree {}: {}",
                            path.display(),
                            e
                        );
                    } else {
                        report.worktrees_removed += 1;
                    }
                }
            }
        }
    }

    Ok(report)
}

#[derive(Debug, Default)]
pub struct CleanupReport {
    pub runs_marked_failed: usize,
    pub branches_deleted: usize,
    pub worktrees_removed: usize,
    pub unmerged_branches: Vec<String>,
}

impl std::fmt::Display for CleanupReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cleanup: {} runs marked failed, {} branches deleted, {} worktree dirs removed",
            self.runs_marked_failed, self.branches_deleted, self.worktrees_removed
        )?;
        if !self.unmerged_branches.is_empty() {
            write!(
                f,
                "\nUnmerged branches (need manual review): {}",
                self.unmerged_branches.join(", ")
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch_name_simple() {
        let name = branch_name("abcdef12-3456-7890-abcd-ef1234567890", "Add login page");
        assert_eq!(name, "conductor/abcdef12/add-login-page");
    }

    #[test]
    fn test_branch_name_special_chars() {
        let name = branch_name("abcdef12-xxxx", "Fix bug: handle NULL pointers!");
        assert_eq!(name, "conductor/abcdef12/fix-bug--handle-null-pointers");
    }

    #[test]
    fn test_branch_name_long_title_truncated() {
        let long_title = "a".repeat(100);
        let name = branch_name("abcdef12-xxxx", &long_title);
        // "conductor/abcdef12/" prefix + max 40 chars of title
        assert!(name.len() <= 19 + 40);
    }

    #[test]
    fn test_branch_name_short_agent_id() {
        let name = branch_name("abc", "task");
        assert_eq!(name, "conductor/abc/task");
    }

    #[test]
    fn test_branch_name_uppercase_lowered() {
        let name = branch_name("ABCDEF12-xxxx", "UPPER Case Title");
        assert!(name.contains("upper-case-title"));
    }

    #[test]
    fn test_branch_name_all_special_chars() {
        let name = branch_name("abcdef12-xxxx", "!!!@@@###");
        // All get replaced with -, then trimmed
        assert_eq!(name, "conductor/abcdef12/");
    }

    #[test]
    fn test_branch_name_hyphens_preserved() {
        let name = branch_name("abcdef12-xxxx", "already-hyphenated-name");
        assert_eq!(name, "conductor/abcdef12/already-hyphenated-name");
    }
}

/// Generate a branch name for an agent's task
pub fn branch_name(agent_id: &str, task_title: &str) -> String {
    let sanitized: String = task_title
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>()
        .to_lowercase();

    let sanitized = sanitized.trim_matches('-');
    let truncated = if sanitized.len() > 40 {
        &sanitized[..40]
    } else {
        sanitized
    };

    format!(
        "conductor/{}/{}",
        &agent_id[..8.min(agent_id.len())],
        truncated
    )
}
