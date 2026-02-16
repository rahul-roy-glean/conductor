use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use std::sync::Arc;

use crate::goal::space;
use crate::server::AppState;

/// Payload from Claude Code's Stop hook
#[derive(Debug, serde::Deserialize)]
pub struct StopHookPayload {
    pub session_id: Option<String>,
    pub stop_hook_active: Option<bool>,
}

/// Handle the Stop hook - agent finished
pub async fn handle_stop_hook(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<StopHookPayload>,
) -> impl IntoResponse {
    tracing::info!("Stop hook received: {:?}", payload);

    if let Some(session_id) = &payload.session_id {
        // Find the agent run with this Claude session ID
        let agents = match state.db.list_agent_runs() {
            Ok(a) => a,
            Err(e) => {
                tracing::error!("Failed to list agents: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": e.to_string()})),
                )
                    .into_response();
            }
        };

        if let Some(agent) = agents
            .iter()
            .find(|a| a.claude_session_id.as_deref() == Some(session_id))
        {
            // Mark agent as done
            if let Err(e) = state.db.update_agent_run_status(&agent.id, "done") {
                tracing::error!("Failed to update agent run status to done for {}: {}", agent.id, e);
            }

            // Mark task as done
            if let Err(e) = state.db.update_task(
                &agent.task_id,
                &crate::db::queries::UpdateTask {
                    status: Some("done".to_string()),
                    title: None,
                    description: None,
                    priority: None,
                    depends_on: None,
                ..Default::default()
                },
            ) {
                tracing::error!("Failed to update task {} to done via stop hook: {}", agent.task_id, e);
            }

            if let Err(e) = state.db.insert_goal_history(
                &agent.goal_space_id,
                "task_completed",
                &format!("Task {} completed by agent {}", agent.task_id, agent.id),
                None,
            ) {
                tracing::error!("Failed to insert goal history for goal {}: {}", agent.goal_space_id, e);
            }

            // Check if the goal is now complete
            if let Err(e) = space::check_goal_completion(&state.db, &agent.goal_space_id) {
                tracing::error!("Failed to check goal completion for goal {}: {}", agent.goal_space_id, e);
            }

            // Auto-dispatch newly unblocked tasks
            state.agent_manager.request_dispatch(&agent.goal_space_id);

            tracing::info!(
                "Agent {} completed task {} via stop hook",
                agent.id,
                agent.task_id
            );
        }
    }

    Json(json!({"ok": true})).into_response()
}

/// Handle the SubagentStop hook
pub async fn handle_subagent_stop_hook(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    tracing::info!("Subagent stop hook received: {:?}", payload);
    Json(json!({"ok": true})).into_response()
}
