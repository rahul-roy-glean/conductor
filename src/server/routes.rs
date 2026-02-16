use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Router,
};
use serde_json::json;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

use crate::agent::session::BroadcastEvent;
use crate::db::queries::{CreateGoalSpace, CreateTask, UpdateTask};
use crate::hooks;
use crate::server::sse;
use crate::server::AppState;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Goal Spaces
        .route("/api/goals", post(create_goal).get(list_goals))
        .route(
            "/api/goals/{id}",
            get(get_goal).put(update_goal).delete(delete_goal),
        )
        .route("/api/goals/{id}/decompose", post(decompose_goal))
        .route("/api/goals/{id}/dispatch", post(dispatch_goal))
        .route(
            "/api/goals/{id}/tasks",
            get(list_tasks).post(create_task),
        )
        // Tasks
        .route("/api/tasks/{id}", put(update_task))
        .route("/api/tasks/{id}/retry", post(retry_task))
        .route("/api/tasks/{id}/dispatch", post(dispatch_task))
        .route("/api/goals/{id}/retry-failed", post(retry_all_failed))
        // Agents
        .route("/api/agents", get(list_agents))
        .route("/api/agents/{id}", get(get_agent))
        .route("/api/agents/{id}/nudge", post(nudge_agent))
        .route("/api/agents/{id}/kill", post(kill_agent))
        .route("/api/agents/{id}/events", get(get_agent_events))
        // SSE
        .route("/api/events", get(sse::global_event_stream))
        .route("/api/agents/{id}/stream", get(sse::agent_event_stream))
        // Hooks
        .route("/api/hooks/stop", post(hooks::handler::handle_stop_hook))
        .route(
            "/api/hooks/subagent-stop",
            post(hooks::handler::handle_subagent_stop_hook),
        )
        // Stats
        .route("/api/stats", get(get_stats))
        .layer(
            CorsLayer::permissive(), // Allow frontend dev server
        )
        .with_state(state)
}

// ── Goal Space Handlers ──

async fn create_goal(
    State(state): State<Arc<AppState>>,
    Json(input): Json<CreateGoalSpace>,
) -> impl IntoResponse {
    match state.db.create_goal_space(&input) {
        Ok(goal) => (StatusCode::CREATED, Json(json!(goal))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn list_goals(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.db.list_goal_spaces() {
        Ok(goals) => Json(json!(goals)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn get_goal(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.db.get_goal_space(&id) {
        Ok(Some(goal)) => Json(json!(goal)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"error": "Not found"}))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn update_goal(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(input): Json<serde_json::Value>,
) -> impl IntoResponse {
    let name = input.get("name").and_then(|v| v.as_str());
    let description = input.get("description").and_then(|v| v.as_str());
    let status = input.get("status").and_then(|v| v.as_str());

    match state.db.update_goal_space(&id, name, description, status) {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn delete_goal(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.db.delete_goal_space(&id) {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn decompose_goal(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let goal = match state.db.get_goal_space(&id) {
        Ok(Some(g)) => g,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(json!({"error": "Goal not found"}))).into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    let operation_id = uuid::Uuid::new_v4().to_string();
    let goal_space_id = id.clone();

    // Broadcast initial running status
    let _ = state.event_tx.send(BroadcastEvent::OperationUpdate {
        operation_id: operation_id.clone(),
        goal_space_id: goal_space_id.clone(),
        operation_type: "decompose".to_string(),
        status: "running".to_string(),
        message: "Decomposing goal...".to_string(),
        result: None,
    });

    // Spawn background task
    let op_id = operation_id.clone();
    let gs_id = goal_space_id.clone();
    let state = Arc::clone(&state);
    tokio::spawn(async move {
        match crate::goal::decompose::decompose_goal(
            &goal.description,
            &goal.repo_path,
            &state.event_tx,
            &op_id,
            &gs_id,
        ).await {
            Ok(tasks) => {
                let mut created_tasks = Vec::new();
                let mut failed = false;
                // Map __index_N placeholders to real task UUIDs as we create them
                let mut index_to_id: std::collections::HashMap<String, String> =
                    std::collections::HashMap::new();

                for (i, task_input) in tasks.iter().enumerate() {
                    // Resolve __index_N dependencies to actual task IDs
                    let resolved = CreateTask {
                        title: task_input.title.clone(),
                        description: task_input.description.clone(),
                        priority: task_input.priority,
                        depends_on: task_input
                            .depends_on
                            .iter()
                            .filter_map(|dep| {
                                index_to_id.get(dep).cloned().or_else(|| {
                                    tracing::warn!(
                                        "Unresolved dependency '{}' for task '{}'",
                                        dep,
                                        task_input.title
                                    );
                                    None
                                })
                            })
                            .collect(),
                    };

                    match state.db.create_task(&gs_id, &resolved) {
                        Ok(task) => {
                            index_to_id
                                .insert(format!("__index_{}", i), task.id.clone());
                            created_tasks.push(task);
                        }
                        Err(e) => {
                            let _ = state.event_tx.send(BroadcastEvent::OperationUpdate {
                                operation_id: op_id.clone(),
                                goal_space_id: gs_id.clone(),
                                operation_type: "decompose".to_string(),
                                status: "failed".to_string(),
                                message: format!("Failed to create task: {}", e),
                                result: None,
                            });
                            failed = true;
                            break;
                        }
                    }
                }
                if !failed {
                    let _ = state.event_tx.send(BroadcastEvent::OperationUpdate {
                        operation_id: op_id,
                        goal_space_id: gs_id.clone(),
                        operation_type: "decompose".to_string(),
                        status: "completed".to_string(),
                        message: format!("Created {} tasks", created_tasks.len()),
                        result: Some(json!(created_tasks)),
                    });
                }
            }
            Err(e) => {
                let _ = state.event_tx.send(BroadcastEvent::OperationUpdate {
                    operation_id: op_id,
                    goal_space_id: gs_id.clone(),
                    operation_type: "decompose".to_string(),
                    status: "failed".to_string(),
                    message: e.to_string(),
                    result: None,
                });
            }
        }
    });

    (
        StatusCode::ACCEPTED,
        Json(json!({ "operation_id": operation_id, "status": "running" })),
    )
        .into_response()
}

async fn dispatch_goal(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let goal = match state.db.get_goal_space(&id) {
        Ok(Some(g)) => g,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(json!({"error": "Goal not found"}))).into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    let operation_id = uuid::Uuid::new_v4().to_string();
    let goal_space_id = id.clone();

    // Broadcast running status
    let _ = state.event_tx.send(BroadcastEvent::OperationUpdate {
        operation_id: operation_id.clone(),
        goal_space_id: goal_space_id.clone(),
        operation_type: "dispatch".to_string(),
        status: "running".to_string(),
        message: "Dispatching agents...".to_string(),
        result: None,
    });

    // Spawn background task
    let op_id = operation_id.clone();
    let state = Arc::clone(&state);
    tokio::spawn(async move {
        let unblocked = match state.db.get_unblocked_tasks(&goal_space_id) {
            Ok(tasks) => tasks,
            Err(e) => {
                let _ = state.event_tx.send(BroadcastEvent::OperationUpdate {
                    operation_id: op_id,
                    goal_space_id: goal_space_id.clone(),
                    operation_type: "dispatch".to_string(),
                    status: "failed".to_string(),
                    message: format!("Failed to get tasks: {}", e),
                    result: None,
                });
                return;
            }
        };

        let mut agents_spawned = 0;

        for task in &unblocked {
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
                    &goal_space_id,
                    &prompt,
                    &goal.repo_path,
                    "sonnet",
                    Some(5.0),
                    Some(50),
                    Some(vec![
                        "Bash".to_string(),
                        "Read".to_string(),
                        "Edit".to_string(),
                        "Write".to_string(),
                        "Grep".to_string(),
                        "Glob".to_string(),
                    ]),
                )
                .await
            {
                Ok(_) => agents_spawned += 1,
                Err(e) => {
                    tracing::error!("Failed to spawn agent for task {}: {}", task.id, e);
                }
            }
        }

        let _ = state.event_tx.send(BroadcastEvent::OperationUpdate {
            operation_id: op_id,
            goal_space_id: goal_space_id.clone(),
            operation_type: "dispatch".to_string(),
            status: "completed".to_string(),
            message: format!("Spawned {} agents for {} tasks", agents_spawned, unblocked.len()),
            result: Some(json!({"agents_spawned": agents_spawned, "tasks_available": unblocked.len()})),
        });
    });

    (
        StatusCode::ACCEPTED,
        Json(json!({ "operation_id": operation_id, "status": "running" })),
    )
        .into_response()
}

// ── Task Handlers ──

async fn list_tasks(
    State(state): State<Arc<AppState>>,
    Path(goal_id): Path<String>,
) -> impl IntoResponse {
    match state.db.list_tasks(&goal_id) {
        Ok(tasks) => Json(json!(tasks)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn create_task(
    State(state): State<Arc<AppState>>,
    Path(goal_id): Path<String>,
    Json(input): Json<CreateTask>,
) -> impl IntoResponse {
    match state.db.create_task(&goal_id, &input) {
        Ok(task) => (StatusCode::CREATED, Json(json!(task))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn update_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(input): Json<UpdateTask>,
) -> impl IntoResponse {
    match state.db.update_task(&id, &input) {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn retry_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Reset task to pending so it can be dispatched again
    let update = UpdateTask {
        status: Some("pending".to_string()),
        title: None,
        description: None,
        priority: None,
        depends_on: None,
    };

    match state.db.update_task(&id, &update) {
        Ok(()) => {
            // Find the goal_space_id for this task and trigger dispatch
            if let Ok(Some(task)) = state.db.get_task(&id) {
                state.agent_manager.request_dispatch(&task.goal_space_id);
            }
            Json(json!({"ok": true, "status": "pending"})).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn retry_all_failed(
    State(state): State<Arc<AppState>>,
    Path(goal_id): Path<String>,
) -> impl IntoResponse {
    let tasks = match state.db.list_tasks(&goal_id) {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    let mut retried = 0;
    for task in &tasks {
        if task.status == "failed" {
            let update = UpdateTask {
                status: Some("pending".to_string()),
                title: None,
                description: None,
                priority: None,
                depends_on: None,
            };
            if state.db.update_task(&task.id, &update).is_ok() {
                retried += 1;
            }
        }
    }

    // Trigger auto-dispatch for the newly-pending tasks
    if retried > 0 {
        state.agent_manager.request_dispatch(&goal_id);
    }

    Json(json!({"ok": true, "retried": retried})).into_response()
}

async fn dispatch_task(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
) -> impl IntoResponse {
    // Get the task to find its goal_space_id and other info
    let task = match state.db.get_task(&task_id) {
        Ok(Some(t)) => t,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(json!({"error": "Task not found"}))).into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    // Get the goal space to find the repo_path and description
    let goal = match state.db.get_goal_space(&task.goal_space_id) {
        Ok(Some(g)) => g,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(json!({"error": "Goal space not found"}))).into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    let operation_id = uuid::Uuid::new_v4().to_string();
    let goal_space_id = task.goal_space_id.clone();

    // Broadcast running status
    let _ = state.event_tx.send(BroadcastEvent::OperationUpdate {
        operation_id: operation_id.clone(),
        goal_space_id: goal_space_id.clone(),
        operation_type: "dispatch".to_string(),
        status: "running".to_string(),
        message: format!("Dispatching agent for task '{}'...", task.title),
        result: None,
    });

    // Spawn background task
    let op_id = operation_id.clone();
    let state = Arc::clone(&state);
    tokio::spawn(async move {
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
                &goal_space_id,
                &prompt,
                &goal.repo_path,
                "sonnet",
                Some(5.0),
                Some(50),
                Some(vec![
                    "Bash".to_string(),
                    "Read".to_string(),
                    "Edit".to_string(),
                    "Write".to_string(),
                    "Grep".to_string(),
                    "Glob".to_string(),
                ]),
            )
            .await
        {
            Ok(_) => {
                let _ = state.event_tx.send(BroadcastEvent::OperationUpdate {
                    operation_id: op_id,
                    goal_space_id: goal_space_id.clone(),
                    operation_type: "dispatch".to_string(),
                    status: "completed".to_string(),
                    message: format!("Agent spawned for task '{}'", task.title),
                    result: Some(json!({"task_id": task.id})),
                });
            }
            Err(e) => {
                tracing::error!("Failed to spawn agent for task {}: {}", task.id, e);
                let _ = state.event_tx.send(BroadcastEvent::OperationUpdate {
                    operation_id: op_id,
                    goal_space_id: goal_space_id.clone(),
                    operation_type: "dispatch".to_string(),
                    status: "failed".to_string(),
                    message: format!("Failed to spawn agent: {}", e),
                    result: None,
                });
            }
        }
    });

    (
        StatusCode::ACCEPTED,
        Json(json!({ "operation_id": operation_id, "status": "running" })),
    )
        .into_response()
}

// ── Agent Handlers ──

async fn list_agents(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.db.list_agent_runs() {
        Ok(agents) => Json(json!(agents)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn get_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.db.get_agent_run(&id) {
        Ok(Some(agent)) => Json(json!(agent)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"error": "Not found"}))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn nudge_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(input): Json<serde_json::Value>,
) -> impl IntoResponse {
    let message = input
        .get("message")
        .and_then(|m| m.as_str())
        .unwrap_or("");

    if message.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Message required"})),
        )
            .into_response();
    }

    match state.agent_manager.nudge_agent(&id, message).await {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn kill_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.agent_manager.kill_agent(&id).await {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn get_agent_events(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.db.list_agent_events(&id) {
        Ok(events) => Json(json!(events)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

// ── Stats Handler ──

async fn get_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.db.get_stats() {
        Ok(stats) => Json(json!(stats)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}
