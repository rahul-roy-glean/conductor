use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::broadcast;

use crate::agent::event_parser::{parse_stream_json_line, ParsedEvent};
use crate::agent::session::BroadcastEvent;
use crate::db::queries::CreateGoalMessage;
use crate::db::Database;

/// Run a chat interaction for a goal space.
/// Saves the user message, spawns `claude -p` to generate a response,
/// streams chunks via SSE, and saves the assistant reply.
pub async fn run_goal_chat(
    db: &Database,
    goal_space_id: &str,
    message: &str,
    event_tx: &broadcast::Sender<BroadcastEvent>,
    operation_id: &str,
) -> Result<()> {
    // Load goal context
    let goal = db
        .get_goal_space(goal_space_id)?
        .context("Goal space not found")?;

    // Save user message
    db.create_goal_message(&CreateGoalMessage {
        goal_space_id: goal_space_id.to_string(),
        role: "user".to_string(),
        content: message.to_string(),
        message_type: "text".to_string(),
        metadata_json: "{}".to_string(),
    })?;

    // Load previous messages for context
    let history = db.list_goal_messages(goal_space_id)?;
    let mut context_parts = Vec::new();
    // Include up to the last 20 messages for context
    let recent = if history.len() > 20 {
        &history[history.len() - 20..]
    } else {
        &history
    };
    for msg in recent {
        // Skip the current user message (just saved)
        if msg.role == "user" || msg.role == "assistant" {
            context_parts.push(format!("{}: {}", msg.role, msg.content));
        }
    }

    // Build system prompt with goal context
    let system_prompt = format!(
        "You are an AI assistant helping with the goal: {}\n\
         Description: {}\n\
         Repository: {}\n\n\
         You are having a conversation about this goal. Help the user plan, \
         understand, and make decisions about this goal. Be concise and helpful.",
        goal.name, goal.description, goal.repo_path
    );

    // Build the prompt with conversation context
    let prompt = if context_parts.len() > 1 {
        // There's prior conversation history (more than just the current message)
        let history_text = context_parts[..context_parts.len() - 1].join("\n\n");
        format!(
            "Previous conversation:\n{}\n\nUser's latest message: {}",
            history_text, message
        )
    } else {
        message.to_string()
    };

    let mut child = Command::new("claude")
        .arg("-p")
        .arg(&prompt)
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--max-turns")
        .arg("3")
        .arg("--append-system-prompt")
        .arg(&system_prompt)
        .arg("--permission-mode")
        .arg("plan")
        .arg("--allowedTools")
        .arg("Read")
        .arg("--allowedTools")
        .arg("Grep")
        .arg("--allowedTools")
        .arg("Glob")
        .current_dir(&goal.repo_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn claude for chat")?;

    let stdout = child
        .stdout
        .take()
        .context("Failed to capture stdout from claude process")?;

    let mut reader = BufReader::new(stdout).lines();
    let mut full_response = String::new();

    while let Some(line) = reader
        .next_line()
        .await
        .context("Failed to read stdout line")?
    {
        if line.trim().is_empty() {
            continue;
        }

        if let Some(parsed) = parse_stream_json_line(&line) {
            match &parsed {
                ParsedEvent::TextDelta { text } => {
                    full_response.push_str(text);
                    let _ = event_tx.send(BroadcastEvent::ChatChunk {
                        operation_id: operation_id.to_string(),
                        goal_space_id: goal_space_id.to_string(),
                        chunk: text.clone(),
                        done: false,
                    });
                }
                ParsedEvent::TextMessage { text } => {
                    // Claude Code CLI emits complete text in "assistant" events
                    // rather than incremental deltas. Broadcast the full text
                    // as a chunk so the frontend sees it immediately.
                    if !text.is_empty() {
                        full_response.push_str(text);
                        let _ = event_tx.send(BroadcastEvent::ChatChunk {
                            operation_id: operation_id.to_string(),
                            goal_space_id: goal_space_id.to_string(),
                            chunk: text.clone(),
                            done: false,
                        });
                    }
                }
                ParsedEvent::Result { result_text, .. } => {
                    // Final result — if we didn't get text from assistant events,
                    // use the result text as the response
                    if full_response.is_empty() && !result_text.is_empty() {
                        full_response = result_text.clone();
                        let _ = event_tx.send(BroadcastEvent::ChatChunk {
                            operation_id: operation_id.to_string(),
                            goal_space_id: goal_space_id.to_string(),
                            chunk: result_text.clone(),
                            done: false,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    let status = child
        .wait()
        .await
        .context("Failed to wait on claude process")?;

    if !status.success() {
        let stderr_output = if let Some(mut stderr) = child.stderr.take() {
            let mut buf = String::new();
            tokio::io::AsyncReadExt::read_to_string(&mut stderr, &mut buf)
                .await
                .unwrap_or(0);
            buf
        } else {
            String::new()
        };
        anyhow::bail!("Claude chat failed (exit {}): {}", status, stderr_output);
    }

    // Save assistant response
    if !full_response.is_empty() {
        db.create_goal_message(&CreateGoalMessage {
            goal_space_id: goal_space_id.to_string(),
            role: "assistant".to_string(),
            content: full_response,
            message_type: "text".to_string(),
            metadata_json: "{}".to_string(),
        })?;
    }

    // Send completion event
    let _ = event_tx.send(BroadcastEvent::ChatChunk {
        operation_id: operation_id.to_string(),
        goal_space_id: goal_space_id.to_string(),
        chunk: String::new(),
        done: true,
    });

    Ok(())
}
