//! Integration tests for the Conductor REST API.
//!
//! These tests spin up the full axum server with an in-memory SQLite database
//! and test the HTTP endpoints without spawning actual Claude Code processes.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use conductor::agent::session::AgentManager;
use conductor::db::Database;
use conductor::server::routes::create_router;
use conductor::server::AppState;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

fn test_state() -> Arc<AppState> {
    let db = Database::open_in_memory().unwrap();
    db.run_migrations().unwrap();
    let (event_tx, _) = tokio::sync::broadcast::channel(1024);
    let (dispatch_tx, _dispatch_rx) = tokio::sync::mpsc::unbounded_channel();
    let agent_manager = AgentManager::new(db.clone(), event_tx.clone(), dispatch_tx);
    Arc::new(AppState {
        db,
        agent_manager,
        event_tx,
    })
}

async fn json_body(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

// ── Goal Space API Tests ──

#[tokio::test]
async fn test_create_goal() {
    let state = test_state();
    let app = create_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/goals")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Test Goal",
                        "description": "Test description",
                        "repo_path": "/tmp/test"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = json_body(resp).await;
    assert_eq!(body["name"], "Test Goal");
    assert_eq!(body["status"], "active");
    assert!(body["id"].as_str().is_some());
}

#[tokio::test]
async fn test_list_goals_empty() {
    let state = test_state();
    let app = create_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/goals")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_create_and_list_goals() {
    let state = test_state();

    // Create a goal directly via DB for setup
    state
        .db
        .create_goal_space(&conductor::db::queries::CreateGoalSpace {
            name: "Goal 1".into(),
            description: "Desc".into(),
            repo_path: "/tmp".into(),
        })
        .unwrap();
    state
        .db
        .create_goal_space(&conductor::db::queries::CreateGoalSpace {
            name: "Goal 2".into(),
            description: "Desc".into(),
            repo_path: "/tmp".into(),
        })
        .unwrap();

    let app = create_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/goals")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_get_goal() {
    let state = test_state();
    let goal = state
        .db
        .create_goal_space(&conductor::db::queries::CreateGoalSpace {
            name: "Find Me".into(),
            description: "Desc".into(),
            repo_path: "/tmp".into(),
        })
        .unwrap();

    let app = create_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri(&format!("/api/goals/{}", goal.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["name"], "Find Me");
    assert_eq!(body["id"], goal.id);
}

#[tokio::test]
async fn test_get_goal_not_found() {
    let state = test_state();
    let app = create_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/goals/nonexistent-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_goal() {
    let state = test_state();
    let goal = state
        .db
        .create_goal_space(&conductor::db::queries::CreateGoalSpace {
            name: "Original".into(),
            description: "Desc".into(),
            repo_path: "/tmp".into(),
        })
        .unwrap();

    let app = create_router(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/goals/{}", goal.id))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"name": "Updated Name"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    // Verify the update
    let updated = state.db.get_goal_space(&goal.id).unwrap().unwrap();
    assert_eq!(updated.name, "Updated Name");
}

#[tokio::test]
async fn test_delete_goal_archives() {
    let state = test_state();
    let goal = state
        .db
        .create_goal_space(&conductor::db::queries::CreateGoalSpace {
            name: "To Archive".into(),
            description: "Desc".into(),
            repo_path: "/tmp".into(),
        })
        .unwrap();

    let app = create_router(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/goals/{}", goal.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let archived = state.db.get_goal_space(&goal.id).unwrap().unwrap();
    assert_eq!(archived.status, "archived");
}

// ── Task API Tests ──

#[tokio::test]
async fn test_create_task() {
    let state = test_state();
    let goal = state
        .db
        .create_goal_space(&conductor::db::queries::CreateGoalSpace {
            name: "G".into(),
            description: "D".into(),
            repo_path: "/tmp".into(),
        })
        .unwrap();

    let app = create_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/goals/{}/tasks", goal.id))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "title": "New Task",
                        "description": "Do the thing",
                        "priority": 3
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = json_body(resp).await;
    assert_eq!(body["title"], "New Task");
    assert_eq!(body["priority"], 3);
    assert_eq!(body["status"], "pending");
}

#[tokio::test]
async fn test_list_tasks() {
    let state = test_state();
    let goal = state
        .db
        .create_goal_space(&conductor::db::queries::CreateGoalSpace {
            name: "G".into(),
            description: "D".into(),
            repo_path: "/tmp".into(),
        })
        .unwrap();

    state
        .db
        .create_task(
            &goal.id,
            &conductor::db::queries::CreateTask {
                title: "T1".into(),
                description: "D".into(),
                priority: 0,
                depends_on: vec![],
            },
        )
        .unwrap();
    state
        .db
        .create_task(
            &goal.id,
            &conductor::db::queries::CreateTask {
                title: "T2".into(),
                description: "D".into(),
                priority: 0,
                depends_on: vec![],
            },
        )
        .unwrap();

    let app = create_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri(&format!("/api/goals/{}/tasks", goal.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_update_task() {
    let state = test_state();
    let goal = state
        .db
        .create_goal_space(&conductor::db::queries::CreateGoalSpace {
            name: "G".into(),
            description: "D".into(),
            repo_path: "/tmp".into(),
        })
        .unwrap();
    let task = state
        .db
        .create_task(
            &goal.id,
            &conductor::db::queries::CreateTask {
                title: "Original".into(),
                description: "D".into(),
                priority: 0,
                depends_on: vec![],
            },
        )
        .unwrap();

    let app = create_router(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/tasks/{}", task.id))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"status": "running"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let updated = state.db.get_task(&task.id).unwrap().unwrap();
    assert_eq!(updated.status, "running");
}

#[tokio::test]
async fn test_retry_task() {
    let state = test_state();
    let goal = state
        .db
        .create_goal_space(&conductor::db::queries::CreateGoalSpace {
            name: "G".into(),
            description: "D".into(),
            repo_path: "/tmp".into(),
        })
        .unwrap();
    let task = state
        .db
        .create_task(
            &goal.id,
            &conductor::db::queries::CreateTask {
                title: "Failed Task".into(),
                description: "D".into(),
                priority: 0,
                depends_on: vec![],
            },
        )
        .unwrap();

    // Mark as failed
    state
        .db
        .update_task(
            &task.id,
            &conductor::db::queries::UpdateTask {
                status: Some("failed".into()),
                title: None,
                description: None,
                priority: None,
                depends_on: None,
            },
        )
        .unwrap();

    let app = create_router(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/tasks/{}/retry", task.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let retried = state.db.get_task(&task.id).unwrap().unwrap();
    assert_eq!(retried.status, "pending");
}

// ── Agent API Tests ──

#[tokio::test]
async fn test_list_agents_empty() {
    let state = test_state();
    let app = create_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/agents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_get_agent_not_found() {
    let state = test_state();
    let app = create_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/agents/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_agent_with_data() {
    let state = test_state();
    let goal = state
        .db
        .create_goal_space(&conductor::db::queries::CreateGoalSpace {
            name: "G".into(),
            description: "D".into(),
            repo_path: "/tmp".into(),
        })
        .unwrap();
    let task = state
        .db
        .create_task(
            &goal.id,
            &conductor::db::queries::CreateTask {
                title: "T".into(),
                description: "D".into(),
                priority: 0,
                depends_on: vec![],
            },
        )
        .unwrap();
    let run = state
        .db
        .create_agent_run(&task.id, &goal.id, None, None, "sonnet", Some(5.0))
        .unwrap();

    let app = create_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri(&format!("/api/agents/{}", run.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["id"], run.id);
    assert_eq!(body["model"], "sonnet");
    assert_eq!(body["max_budget_usd"], 5.0);
}

#[tokio::test]
async fn test_get_agent_events() {
    let state = test_state();
    let goal = state
        .db
        .create_goal_space(&conductor::db::queries::CreateGoalSpace {
            name: "G".into(),
            description: "D".into(),
            repo_path: "/tmp".into(),
        })
        .unwrap();
    let task = state
        .db
        .create_task(
            &goal.id,
            &conductor::db::queries::CreateTask {
                title: "T".into(),
                description: "D".into(),
                priority: 0,
                depends_on: vec![],
            },
        )
        .unwrap();
    let run = state
        .db
        .create_agent_run(&task.id, &goal.id, None, None, "sonnet", None)
        .unwrap();

    state
        .db
        .insert_agent_event(&run.id, "tool_call", Some("Read"), "Reading file", None, None)
        .unwrap();
    state
        .db
        .insert_agent_event(&run.id, "text_output", None, "Output text", None, None)
        .unwrap();

    let app = create_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri(&format!("/api/agents/{}/events", run.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let events = body.as_array().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["event_type"], "tool_call");
    assert_eq!(events[0]["tool_name"], "Read");
    assert_eq!(events[1]["event_type"], "text_output");
}

#[tokio::test]
async fn test_nudge_agent_no_message() {
    let state = test_state();
    let app = create_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents/some-id/nudge")
                .header("content-type", "application/json")
                .body(Body::from(json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_nudge_agent_empty_message() {
    let state = test_state();
    let app = create_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents/some-id/nudge")
                .header("content-type", "application/json")
                .body(Body::from(json!({"message": ""}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ── Stats API Tests ──

#[tokio::test]
async fn test_stats_empty() {
    let state = test_state();
    let app = create_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["active_agents"], 0);
    assert_eq!(body["total_cost_usd"], 0.0);
    assert_eq!(body["tasks_completed"], 0);
    assert_eq!(body["tasks_total"], 0);
    assert_eq!(body["goals_active"], 0);
}

#[tokio::test]
async fn test_stats_with_data() {
    let state = test_state();

    let goal = state
        .db
        .create_goal_space(&conductor::db::queries::CreateGoalSpace {
            name: "G".into(),
            description: "D".into(),
            repo_path: "/tmp".into(),
        })
        .unwrap();
    let task = state
        .db
        .create_task(
            &goal.id,
            &conductor::db::queries::CreateTask {
                title: "T".into(),
                description: "D".into(),
                priority: 0,
                depends_on: vec![],
            },
        )
        .unwrap();
    let run = state
        .db
        .create_agent_run(&task.id, &goal.id, None, None, "sonnet", None)
        .unwrap();
    state
        .db
        .update_agent_run_status(&run.id, "running")
        .unwrap();
    state
        .db
        .update_agent_run_cost(&run.id, 1.50, 3000, 1000)
        .unwrap();

    let app = create_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["active_agents"], 1);
    assert_eq!(body["total_cost_usd"], 1.5);
    assert_eq!(body["tasks_total"], 1);
    assert_eq!(body["goals_active"], 1);
}

// ── Hook API Tests ──

#[tokio::test]
async fn test_stop_hook() {
    let state = test_state();
    let app = create_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/hooks/stop")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"session_id": "test-session"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["ok"], true);
}

#[tokio::test]
async fn test_subagent_stop_hook() {
    let state = test_state();
    let app = create_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/hooks/subagent-stop")
                .header("content-type", "application/json")
                .body(Body::from(json!({"data": "test"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_stop_hook_marks_agent_done() {
    let state = test_state();

    // Set up a goal, task, and agent run with a session ID
    let goal = state
        .db
        .create_goal_space(&conductor::db::queries::CreateGoalSpace {
            name: "G".into(),
            description: "D".into(),
            repo_path: "/tmp".into(),
        })
        .unwrap();
    let task = state
        .db
        .create_task(
            &goal.id,
            &conductor::db::queries::CreateTask {
                title: "T".into(),
                description: "D".into(),
                priority: 0,
                depends_on: vec![],
            },
        )
        .unwrap();
    let run = state
        .db
        .create_agent_run(&task.id, &goal.id, None, None, "sonnet", None)
        .unwrap();
    state
        .db
        .update_agent_run_status(&run.id, "running")
        .unwrap();
    state
        .db
        .update_agent_run_session_id(&run.id, "claude-sess-42")
        .unwrap();

    let app = create_router(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/hooks/stop")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"session_id": "claude-sess-42"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    // Agent run should be marked done
    let updated_run = state.db.get_agent_run(&run.id).unwrap().unwrap();
    assert_eq!(updated_run.status, "done");

    // Task should be marked done
    let updated_task = state.db.get_task(&task.id).unwrap().unwrap();
    assert_eq!(updated_task.status, "done");
}
