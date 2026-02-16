use anyhow::Result;
use serde_json::Value;

use crate::db::queries::AgentEvent;
use crate::db::Database;

/// Parsed event from Claude Code's stream-json output
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ParsedEvent {
    /// Agent is making a tool call
    ToolUse {
        tool_name: String,
        input_summary: String,
    },
    /// Tool returned a result
    ToolResult {
        tool_name: String,
        success: bool,
        summary: String,
    },
    /// Text being streamed from the agent
    TextDelta { text: String },
    /// Full text message completed
    TextMessage { text: String },
    /// API request with cost info
    ApiRequest {
        model: String,
        cost_usd: f64,
        input_tokens: i64,
        output_tokens: i64,
        duration_ms: i64,
    },
    /// Error occurred
    Error { message: String },
    /// Session completed with result
    Result {
        session_id: String,
        result_text: String,
        cost_usd: f64,
        input_tokens: i64,
        output_tokens: i64,
    },
    /// System message from Claude Code
    System { message: String },
}

/// Parse a single NDJSON line from Claude Code's stream-json output
pub fn parse_stream_json_line(line: &str) -> Option<ParsedEvent> {
    let v: Value = serde_json::from_str(line).ok()?;

    // Handle different event types from stream-json output
    let event_type = v.get("type")?.as_str()?;

    match event_type {
        "assistant" => {
            // Assistant message - check for tool_use or text content
            if let Some(content) = v.get("message").and_then(|m| m.get("content")) {
                if let Some(content_arr) = content.as_array() {
                    for block in content_arr {
                        if let Some(block_type) = block.get("type").and_then(|t| t.as_str()) {
                            match block_type {
                                "tool_use" => {
                                    let tool_name = block
                                        .get("name")
                                        .and_then(|n| n.as_str())
                                        .unwrap_or("unknown")
                                        .to_string();
                                    let input = block.get("input").cloned().unwrap_or(Value::Null);
                                    let input_summary = summarize_tool_input(&tool_name, &input);
                                    return Some(ParsedEvent::ToolUse {
                                        tool_name,
                                        input_summary,
                                    });
                                }
                                "text" => {
                                    let text = block
                                        .get("text")
                                        .and_then(|t| t.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    if !text.is_empty() {
                                        return Some(ParsedEvent::TextMessage { text });
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            None
        }

        "content_block_delta" => {
            // Streaming text delta
            if let Some(delta) = v.get("delta") {
                if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                    return Some(ParsedEvent::TextDelta {
                        text: text.to_string(),
                    });
                }
            }
            None
        }

        "result" => {
            // Final result
            let session_id = v
                .get("session_id")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let result_text = v
                .get("result")
                .and_then(|r| r.as_str())
                .unwrap_or("")
                .to_string();
            let cost_usd = v
                .get("cost_usd")
                .or_else(|| v.get("total_cost_usd"))
                .and_then(|c| c.as_f64())
                .unwrap_or(0.0);

            // Extract token counts from usage field
            let input_tokens = v
                .get("usage")
                .and_then(|u| u.get("input_tokens"))
                .and_then(|t| t.as_i64())
                .unwrap_or(0);
            let output_tokens = v
                .get("usage")
                .and_then(|u| u.get("output_tokens"))
                .and_then(|t| t.as_i64())
                .unwrap_or(0);

            Some(ParsedEvent::Result {
                session_id,
                result_text,
                cost_usd,
                input_tokens,
                output_tokens,
            })
        }

        "tool_result" | "tool_output" => {
            let tool_name = v
                .get("tool_name")
                .or_else(|| v.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("unknown")
                .to_string();
            let is_error = v.get("is_error").and_then(|e| e.as_bool()).unwrap_or(false);
            let output = v
                .get("output")
                .or_else(|| v.get("content"))
                .and_then(|o| o.as_str())
                .unwrap_or("")
                .to_string();
            let summary = if output.len() > 200 {
                format!("{}...", &output[..200])
            } else {
                output
            };

            Some(ParsedEvent::ToolResult {
                tool_name,
                success: !is_error,
                summary,
            })
        }

        "error" => {
            let message = v
                .get("error")
                .or_else(|| v.get("message"))
                .and_then(|e| e.as_str())
                .unwrap_or("Unknown error")
                .to_string();
            Some(ParsedEvent::Error { message })
        }

        "system" => {
            let message = v
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("")
                .to_string();
            if !message.is_empty() {
                Some(ParsedEvent::System { message })
            } else {
                None
            }
        }

        _ => None,
    }
}

/// Store a parsed event in the database and return it
pub fn store_event(
    db: &Database,
    agent_run_id: &str,
    event: &ParsedEvent,
    raw_line: &str,
) -> Result<AgentEvent> {
    let (event_type, tool_name, summary, cost_delta) = match event {
        ParsedEvent::ToolUse {
            tool_name,
            input_summary,
        } => (
            "tool_call",
            Some(tool_name.as_str()),
            input_summary.clone(),
            None,
        ),
        ParsedEvent::ToolResult {
            tool_name,
            success,
            summary,
        } => {
            let status = if *success { "OK" } else { "ERROR" };
            (
                "tool_result",
                Some(tool_name.as_str()),
                format!("[{}] {}", status, summary),
                None,
            )
        }
        ParsedEvent::TextDelta { text } => {
            let truncated = if text.len() > 100 {
                format!("{}...", &text[..100])
            } else {
                text.clone()
            };
            ("text_output", None, truncated, None)
        }
        ParsedEvent::TextMessage { text } => {
            let truncated = if text.len() > 200 {
                format!("{}...", &text[..200])
            } else {
                text.clone()
            };
            ("text_output", None, truncated, None)
        }
        ParsedEvent::ApiRequest {
            model,
            cost_usd,
            input_tokens,
            output_tokens,
            ..
        } => (
            "cost_update",
            None,
            format!(
                "API call: {} (in={}, out={}, ${:.4})",
                model, input_tokens, output_tokens, cost_usd
            ),
            Some(*cost_usd),
        ),
        ParsedEvent::Error { message } => ("error", None, message.clone(), None),
        ParsedEvent::Result {
            result_text,
            cost_usd,
            input_tokens,
            output_tokens,
            ..
        } => {
            let truncated = if result_text.len() > 200 {
                format!("{}...", &result_text[..200])
            } else {
                result_text.clone()
            };
            (
                "result",
                None,
                format!(
                    "Completed: {} (in={}, out={})",
                    truncated, input_tokens, output_tokens
                ),
                Some(*cost_usd),
            )
        }
        ParsedEvent::System { message } => ("system", None, message.clone(), None),
    };

    db.insert_agent_event(
        agent_run_id,
        event_type,
        tool_name,
        &summary,
        Some(raw_line),
        cost_delta,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_use_event() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Read","input":{"file_path":"src/main.rs"}}]}}"#;
        let event = parse_stream_json_line(line).unwrap();
        match event {
            ParsedEvent::ToolUse {
                tool_name,
                input_summary,
            } => {
                assert_eq!(tool_name, "Read");
                assert_eq!(input_summary, "Reading src/main.rs");
            }
            _ => panic!("Expected ToolUse, got {:?}", event),
        }
    }

    #[test]
    fn test_parse_tool_use_bash() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"cargo test"}}]}}"#;
        let event = parse_stream_json_line(line).unwrap();
        match event {
            ParsedEvent::ToolUse {
                tool_name,
                input_summary,
            } => {
                assert_eq!(tool_name, "Bash");
                assert_eq!(input_summary, "Running: cargo test");
            }
            _ => panic!("Expected ToolUse, got {:?}", event),
        }
    }

    #[test]
    fn test_parse_tool_use_edit() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Edit","input":{"file_path":"src/lib.rs"}}]}}"#;
        let event = parse_stream_json_line(line).unwrap();
        match event {
            ParsedEvent::ToolUse {
                tool_name,
                input_summary,
            } => {
                assert_eq!(tool_name, "Edit");
                assert_eq!(input_summary, "Editing src/lib.rs");
            }
            _ => panic!("Expected ToolUse"),
        }
    }

    #[test]
    fn test_parse_tool_use_write() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Write","input":{"file_path":"new_file.rs"}}]}}"#;
        let event = parse_stream_json_line(line).unwrap();
        match event {
            ParsedEvent::ToolUse {
                tool_name,
                input_summary,
            } => {
                assert_eq!(tool_name, "Write");
                assert_eq!(input_summary, "Writing new_file.rs");
            }
            _ => panic!("Expected ToolUse"),
        }
    }

    #[test]
    fn test_parse_tool_use_grep() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Grep","input":{"pattern":"fn main"}}]}}"#;
        let event = parse_stream_json_line(line).unwrap();
        match event {
            ParsedEvent::ToolUse {
                tool_name,
                input_summary,
            } => {
                assert_eq!(tool_name, "Grep");
                assert_eq!(input_summary, "Searching for 'fn main'");
            }
            _ => panic!("Expected ToolUse"),
        }
    }

    #[test]
    fn test_parse_tool_use_glob() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Glob","input":{"pattern":"**/*.rs"}}]}}"#;
        let event = parse_stream_json_line(line).unwrap();
        match event {
            ParsedEvent::ToolUse {
                tool_name,
                input_summary,
            } => {
                assert_eq!(tool_name, "Glob");
                assert_eq!(input_summary, "Finding files matching '**/*.rs'");
            }
            _ => panic!("Expected ToolUse"),
        }
    }

    #[test]
    fn test_parse_tool_use_unknown_tool() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"CustomTool","input":{}}]}}"#;
        let event = parse_stream_json_line(line).unwrap();
        match event {
            ParsedEvent::ToolUse {
                tool_name,
                input_summary,
            } => {
                assert_eq!(tool_name, "CustomTool");
                assert_eq!(input_summary, "Using CustomTool");
            }
            _ => panic!("Expected ToolUse"),
        }
    }

    #[test]
    fn test_parse_text_message() {
        let line =
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello world"}]}}"#;
        let event = parse_stream_json_line(line).unwrap();
        match event {
            ParsedEvent::TextMessage { text } => {
                assert_eq!(text, "Hello world");
            }
            _ => panic!("Expected TextMessage, got {:?}", event),
        }
    }

    #[test]
    fn test_parse_empty_text_returns_none() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":""}]}}"#;
        assert!(parse_stream_json_line(line).is_none());
    }

    #[test]
    fn test_parse_text_delta() {
        let line = r#"{"type":"content_block_delta","delta":{"text":"streaming text"}}"#;
        let event = parse_stream_json_line(line).unwrap();
        match event {
            ParsedEvent::TextDelta { text } => {
                assert_eq!(text, "streaming text");
            }
            _ => panic!("Expected TextDelta, got {:?}", event),
        }
    }

    #[test]
    fn test_parse_result() {
        let line = r#"{"type":"result","session_id":"sess-123","result":"Task completed","cost_usd":0.42,"usage":{"input_tokens":100,"output_tokens":50}}"#;
        let event = parse_stream_json_line(line).unwrap();
        match event {
            ParsedEvent::Result {
                session_id,
                result_text,
                cost_usd,
                input_tokens,
                output_tokens,
            } => {
                assert_eq!(session_id, "sess-123");
                assert_eq!(result_text, "Task completed");
                assert!((cost_usd - 0.42).abs() < f64::EPSILON);
                assert_eq!(input_tokens, 100);
                assert_eq!(output_tokens, 50);
            }
            _ => panic!("Expected Result, got {:?}", event),
        }
    }

    #[test]
    fn test_parse_result_missing_fields() {
        let line = r#"{"type":"result"}"#;
        let event = parse_stream_json_line(line).unwrap();
        match event {
            ParsedEvent::Result {
                session_id,
                result_text,
                cost_usd,
                input_tokens,
                output_tokens,
            } => {
                assert_eq!(session_id, "");
                assert_eq!(result_text, "");
                assert!((cost_usd - 0.0).abs() < f64::EPSILON);
                assert_eq!(input_tokens, 0);
                assert_eq!(output_tokens, 0);
            }
            _ => panic!("Expected Result"),
        }
    }

    #[test]
    fn test_parse_tool_result_success() {
        let line =
            r#"{"type":"tool_result","tool_name":"Bash","is_error":false,"output":"test passed"}"#;
        let event = parse_stream_json_line(line).unwrap();
        match event {
            ParsedEvent::ToolResult {
                tool_name,
                success,
                summary,
            } => {
                assert_eq!(tool_name, "Bash");
                assert!(success);
                assert_eq!(summary, "test passed");
            }
            _ => panic!("Expected ToolResult, got {:?}", event),
        }
    }

    #[test]
    fn test_parse_tool_result_error() {
        let line = r#"{"type":"tool_result","tool_name":"Bash","is_error":true,"output":"command failed"}"#;
        let event = parse_stream_json_line(line).unwrap();
        match event {
            ParsedEvent::ToolResult {
                tool_name,
                success,
                summary,
            } => {
                assert_eq!(tool_name, "Bash");
                assert!(!success);
                assert_eq!(summary, "command failed");
            }
            _ => panic!("Expected ToolResult"),
        }
    }

    #[test]
    fn test_parse_tool_result_long_output_truncated() {
        let long_output = "x".repeat(300);
        let line = format!(
            r#"{{"type":"tool_result","tool_name":"Bash","is_error":false,"output":"{}"}}"#,
            long_output
        );
        let event = parse_stream_json_line(&line).unwrap();
        match event {
            ParsedEvent::ToolResult { summary, .. } => {
                assert!(summary.len() <= 204); // 200 + "..."
                assert!(summary.ends_with("..."));
            }
            _ => panic!("Expected ToolResult"),
        }
    }

    #[test]
    fn test_parse_tool_output_variant() {
        let line = r#"{"type":"tool_output","name":"Read","content":"file contents"}"#;
        let event = parse_stream_json_line(line).unwrap();
        match event {
            ParsedEvent::ToolResult {
                tool_name,
                success,
                summary,
            } => {
                assert_eq!(tool_name, "Read");
                assert!(success);
                assert_eq!(summary, "file contents");
            }
            _ => panic!("Expected ToolResult"),
        }
    }

    #[test]
    fn test_parse_error() {
        let line = r#"{"type":"error","error":"Rate limit exceeded"}"#;
        let event = parse_stream_json_line(line).unwrap();
        match event {
            ParsedEvent::Error { message } => {
                assert_eq!(message, "Rate limit exceeded");
            }
            _ => panic!("Expected Error, got {:?}", event),
        }
    }

    #[test]
    fn test_parse_error_with_message_field() {
        let line = r#"{"type":"error","message":"Something went wrong"}"#;
        let event = parse_stream_json_line(line).unwrap();
        match event {
            ParsedEvent::Error { message } => {
                assert_eq!(message, "Something went wrong");
            }
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn test_parse_system_message() {
        let line = r#"{"type":"system","message":"Agent initialized"}"#;
        let event = parse_stream_json_line(line).unwrap();
        match event {
            ParsedEvent::System { message } => {
                assert_eq!(message, "Agent initialized");
            }
            _ => panic!("Expected System, got {:?}", event),
        }
    }

    #[test]
    fn test_parse_empty_system_message_returns_none() {
        let line = r#"{"type":"system","message":""}"#;
        assert!(parse_stream_json_line(line).is_none());
    }

    #[test]
    fn test_parse_unknown_type_returns_none() {
        let line = r#"{"type":"unknown_event","data":"something"}"#;
        assert!(parse_stream_json_line(line).is_none());
    }

    #[test]
    fn test_parse_invalid_json_returns_none() {
        assert!(parse_stream_json_line("not json at all").is_none());
        assert!(parse_stream_json_line("").is_none());
        assert!(parse_stream_json_line("{}").is_none()); // no type field
    }

    #[test]
    fn test_parse_missing_type_returns_none() {
        let line = r#"{"data":"no type field"}"#;
        assert!(parse_stream_json_line(line).is_none());
    }

    #[test]
    fn test_parse_content_block_delta_no_text_returns_none() {
        let line = r#"{"type":"content_block_delta","delta":{"other":"value"}}"#;
        assert!(parse_stream_json_line(line).is_none());
    }

    #[test]
    fn test_parse_bash_long_command_truncated() {
        let long_cmd = "a".repeat(200);
        let line = format!(
            r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","name":"Bash","input":{{"command":"{}"}}}}]}}}}"#,
            long_cmd
        );
        let event = parse_stream_json_line(&line).unwrap();
        match event {
            ParsedEvent::ToolUse { input_summary, .. } => {
                // "Running: " (9 chars) + 80 chars + "..." = 92
                assert!(input_summary.len() <= 93);
                assert!(input_summary.contains("..."));
            }
            _ => panic!("Expected ToolUse"),
        }
    }

    #[test]
    fn test_summarize_tool_input_missing_fields() {
        let input = serde_json::json!({});
        assert_eq!(summarize_tool_input("Read", &input), "Reading ?");
        assert_eq!(summarize_tool_input("Bash", &input), "Running: ?");
        assert_eq!(summarize_tool_input("Grep", &input), "Searching for '?'");
    }

    #[test]
    fn test_parse_assistant_with_usage_still_parses_content() {
        // Assistant messages with usage data should still parse content (text/tool_use),
        // not be hijacked as ApiRequest events. Token tracking happens in session.rs.
        let line = r#"{"type":"assistant","message":{"model":"claude-opus-4-6","id":"msg_test","type":"message","role":"assistant","content":[{"type":"text","text":"Hello"}],"usage":{"input_tokens":100,"cache_creation_input_tokens":1000,"cache_read_input_tokens":500,"output_tokens":50}},"session_id":"sess-123"}"#;
        let event = parse_stream_json_line(line).unwrap();
        match event {
            ParsedEvent::TextMessage { text } => {
                assert_eq!(text, "Hello");
            }
            _ => panic!("Expected TextOutput, got {:?}", event),
        }
    }

    #[test]
    fn test_parse_result_with_usage() {
        let line = r#"{"type":"result","session_id":"sess-123","result":"Done","total_cost_usd":0.42,"usage":{"input_tokens":200,"cache_creation_input_tokens":2000,"cache_read_input_tokens":1000,"output_tokens":100}}"#;
        let event = parse_stream_json_line(line).unwrap();
        match event {
            ParsedEvent::Result {
                session_id,
                result_text,
                cost_usd,
                input_tokens,
                output_tokens,
            } => {
                assert_eq!(session_id, "sess-123");
                assert_eq!(result_text, "Done");
                assert!((cost_usd - 0.42).abs() < f64::EPSILON);
                assert_eq!(input_tokens, 200);
                assert_eq!(output_tokens, 100);
            }
            _ => panic!("Expected Result, got {:?}", event),
        }
    }
}

/// Generate a human-readable summary of a tool's input
fn summarize_tool_input(tool_name: &str, input: &Value) -> String {
    match tool_name {
        "Read" => {
            let path = input
                .get("file_path")
                .and_then(|p| p.as_str())
                .unwrap_or("?");
            format!("Reading {}", path)
        }
        "Edit" => {
            let path = input
                .get("file_path")
                .and_then(|p| p.as_str())
                .unwrap_or("?");
            format!("Editing {}", path)
        }
        "Write" => {
            let path = input
                .get("file_path")
                .and_then(|p| p.as_str())
                .unwrap_or("?");
            format!("Writing {}", path)
        }
        "Bash" => {
            let cmd = input.get("command").and_then(|c| c.as_str()).unwrap_or("?");
            let truncated = if cmd.len() > 80 {
                format!("{}...", &cmd[..80])
            } else {
                cmd.to_string()
            };
            format!("Running: {}", truncated)
        }
        "Grep" => {
            let pattern = input.get("pattern").and_then(|p| p.as_str()).unwrap_or("?");
            format!("Searching for '{}'", pattern)
        }
        "Glob" => {
            let pattern = input.get("pattern").and_then(|p| p.as_str()).unwrap_or("?");
            format!("Finding files matching '{}'", pattern)
        }
        _ => format!("Using {}", tool_name),
    }
}
