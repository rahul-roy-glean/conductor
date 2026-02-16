use crate::db::Database;
use anyhow::Result;

/// Check if all tasks in a goal space are done, and if so mark the goal as completed
pub fn check_goal_completion(db: &Database, goal_space_id: &str) -> Result<bool> {
    let tasks = db.list_tasks(goal_space_id)?;

    if tasks.is_empty() {
        return Ok(false);
    }

    let all_done = tasks.iter().all(|t| t.status == "done");

    if all_done {
        db.update_goal_space(goal_space_id, None, None, Some("completed"))?;
        db.insert_goal_history(goal_space_id, "goal_completed", "All tasks completed", None)?;
        tracing::info!("Goal space {} completed", goal_space_id);
    }

    Ok(all_done)
}

/// Get summary stats for a goal space
pub fn goal_summary(
    db: &Database,
    goal_space_id: &str,
) -> Result<GoalSummary> {
    let tasks = db.list_tasks(goal_space_id)?;
    let total = tasks.len();
    let done = tasks.iter().filter(|t| t.status == "done").count();
    let running = tasks.iter().filter(|t| t.status == "running").count();
    let failed = tasks.iter().filter(|t| t.status == "failed").count();
    let pending = tasks.iter().filter(|t| t.status == "pending").count();
    let blocked = tasks
        .iter()
        .filter(|t| t.status == "blocked")
        .count();

    Ok(GoalSummary {
        total,
        done,
        running,
        failed,
        pending,
        blocked,
    })
}

#[derive(Debug, serde::Serialize)]
pub struct GoalSummary {
    pub total: usize,
    pub done: usize,
    pub running: usize,
    pub failed: usize,
    pub pending: usize,
    pub blocked: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::queries::{CreateGoalSpace, CreateTask, UpdateTask};

    fn test_db() -> Database {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations().unwrap();
        db
    }

    #[test]
    fn test_check_goal_completion_no_tasks() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
            })
            .unwrap();

        let completed = check_goal_completion(&db, &goal.id).unwrap();
        assert!(!completed);

        // Status should still be active
        let g = db.get_goal_space(&goal.id).unwrap().unwrap();
        assert_eq!(g.status, "active");
    }

    #[test]
    fn test_check_goal_completion_all_done() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
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
                },
            )
            .unwrap();

        // Mark both done
        db.update_task(&t1.id, &UpdateTask { status: Some("done".into()), title: None, description: None, priority: None, depends_on: None }).unwrap();
        db.update_task(&t2.id, &UpdateTask { status: Some("done".into()), title: None, description: None, priority: None, depends_on: None }).unwrap();

        let completed = check_goal_completion(&db, &goal.id).unwrap();
        assert!(completed);

        let g = db.get_goal_space(&goal.id).unwrap().unwrap();
        assert_eq!(g.status, "completed");
    }

    #[test]
    fn test_check_goal_completion_not_all_done() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
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
                },
            )
            .unwrap();
        db.create_task(
            &goal.id,
            &CreateTask {
                title: "T2".into(),
                description: "D".into(),
                priority: 0,
                depends_on: vec![],
            },
        )
        .unwrap();

        // Only mark t1 done
        db.update_task(&t1.id, &UpdateTask { status: Some("done".into()), title: None, description: None, priority: None, depends_on: None }).unwrap();

        let completed = check_goal_completion(&db, &goal.id).unwrap();
        assert!(!completed);

        let g = db.get_goal_space(&goal.id).unwrap().unwrap();
        assert_eq!(g.status, "active"); // Not changed
    }

    #[test]
    fn test_goal_summary_counts() {
        let db = test_db();
        let goal = db
            .create_goal_space(&CreateGoalSpace {
                name: "G".into(),
                description: "D".into(),
                repo_path: "/tmp".into(),
            })
            .unwrap();

        let t1 = db.create_task(&goal.id, &CreateTask { title: "T1".into(), description: "D".into(), priority: 0, depends_on: vec![] }).unwrap();
        let t2 = db.create_task(&goal.id, &CreateTask { title: "T2".into(), description: "D".into(), priority: 0, depends_on: vec![] }).unwrap();
        db.create_task(&goal.id, &CreateTask { title: "T3".into(), description: "D".into(), priority: 0, depends_on: vec![] }).unwrap();

        db.update_task(&t1.id, &UpdateTask { status: Some("done".into()), title: None, description: None, priority: None, depends_on: None }).unwrap();
        db.update_task(&t2.id, &UpdateTask { status: Some("running".into()), title: None, description: None, priority: None, depends_on: None }).unwrap();

        let summary = goal_summary(&db, &goal.id).unwrap();
        assert_eq!(summary.total, 3);
        assert_eq!(summary.done, 1);
        assert_eq!(summary.running, 1);
        assert_eq!(summary.pending, 1);
        assert_eq!(summary.failed, 0);
    }
}
