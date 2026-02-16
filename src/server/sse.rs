use axum::response::sse::{Event, Sse};
use axum::response::IntoResponse;
use std::convert::Infallible;
use std::sync::Arc;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::agent::session::BroadcastEvent;
use crate::server::AppState;

/// SSE endpoint for all agent events (fleet view)
pub async fn global_event_stream(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> impl IntoResponse {
    let rx = state.event_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(ref event) => {
            let event_name = match event {
                BroadcastEvent::AgentEvent { .. } => "agent_event",
                BroadcastEvent::OperationUpdate { .. } => "operation_update",
            };
            let json = serde_json::to_string(&event).unwrap_or_default();
            Some(Ok::<_, Infallible>(
                Event::default().event(event_name).data(json),
            ))
        }
        Err(_) => None,
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("ping"),
    )
}

/// SSE endpoint for a single agent's events
pub async fn agent_event_stream(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    axum::extract::Path(agent_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let rx = state.event_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(move |result| match result {
        Ok(
            ref broadcast @ BroadcastEvent::AgentEvent {
                ref agent_run_id, ..
            },
        ) if *agent_run_id == agent_id => {
            let json = serde_json::to_string(broadcast).unwrap_or_default();
            Some(Ok::<_, Infallible>(
                Event::default().event("agent_event").data(json),
            ))
        }
        _ => None,
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("ping"),
    )
}
