use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::broadcast;

use crate::agent::event_parser::{parse_stream_json_line, ParsedEvent};
use crate::agent::session::BroadcastEvent;
use crate::db::queries::CreateTask;

/// Decompose a goal description into tasks using Claude Code.
/// Streams progress events via the broadcast channel as Claude explores the codebase.
pub async fn decompose_goal(
    description: &str,
    repo_path: &str,
    event_tx: &broadcast::Sender<BroadcastEvent>,
    operation_id: &str,
    goal_space_id: &str,
) -> Result<Vec<CreateTask>> {
    let prompt = format!(
        r#"You are a task decomposition engine. Analyze the codebase and break this goal into tasks.

Goal: {}

You MUST respond with ONLY a JSON object (no markdown, no explanation, no surrounding text).
The JSON must match this exact structure:

{{"tasks": [
  {{"title": "short imperative name", "description": "detailed requirements", "depends_on": []}},
  {{"title": "another task", "description": "details", "depends_on": [0]}}
]}}

Rules for decomposition:
- Maximize parallelism: tasks should be independent where possible
- Minimize file overlap: tasks touching the same files should depend on each other
- Include a test task for each implementation task
- Each task should be completable by a single agent in one session
- Be specific about files, functions, and expected behavior in each description
- depends_on uses 0-based indices into this same array

Output ONLY the JSON object. No other text."#,
        description
    );

    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "tasks": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string" },
                        "description": { "type": "string" },
                        "depends_on": {
                            "type": "array",
                            "items": { "type": "integer" }
                        }
                    },
                    "required": ["title", "description", "depends_on"]
                }
            }
        },
        "required": ["tasks"]
    });

    let mut child = Command::new("claude")
        .arg("-p")
        .arg(&prompt)
        .arg("--verbose")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--json-schema")
        .arg(serde_json::to_string(&schema)?)
        .arg("--max-turns")
        .arg("15")
        .arg("--append-system-prompt")
        .arg("IMPORTANT: Your final response MUST be ONLY a valid JSON object matching the provided schema. Do not include any markdown, explanation, or surrounding text. Output raw JSON only.")
        .arg("--allowedTools")
        .arg("Read")
        .arg("--allowedTools")
        .arg("Grep")
        .arg("--allowedTools")
        .arg("Glob")
        .current_dir(repo_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn claude for decomposition")?;

    let stdout = child
        .stdout
        .take()
        .context("Failed to capture stdout from claude process")?;

    let mut reader = BufReader::new(stdout).lines();
    let mut result_line: Option<String> = None;
    // In stream-json mode with --json-schema, Claude emits the structured output
    // via a "StructuredOutput" tool use, not in the result event's "result" field.
    let mut structured_output: Option<serde_json::Value> = None;

    while let Some(line) = reader
        .next_line()
        .await
        .context("Failed to read stdout line")?
    {
        if line.trim().is_empty() {
            continue;
        }

        if let Some(parsed) = parse_stream_json_line(&line) {
            let message = match &parsed {
                ParsedEvent::ToolUse {
                    tool_name,
                    input_summary,
                } => {
                    if tool_name == "StructuredOutput" {
                        // Extract the raw input from the StructuredOutput tool use
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                            if let Some(blocks) = v
                                .get("message")
                                .and_then(|m| m.get("content"))
                                .and_then(|c| c.as_array())
                            {
                                for block in blocks {
                                    if block.get("name").and_then(|n| n.as_str())
                                        == Some("StructuredOutput")
                                    {
                                        if let Some(input) = block.get("input") {
                                            structured_output = Some(input.clone());
                                        }
                                    }
                                }
                            }
                        }
                        Some("Generating task decomposition...".to_string())
                    } else {
                        Some(input_summary.clone())
                    }
                }
                ParsedEvent::TextMessage { text } => {
                    let truncated = if text.len() > 120 {
                        format!("{}...", &text[..120])
                    } else {
                        text.clone()
                    };
                    Some(truncated)
                }
                ParsedEvent::Error { message } => Some(format!("Error: {}", message)),
                ParsedEvent::Result { .. } => {
                    result_line = Some(line.clone());
                    None
                }
                // Skip TextDelta, ApiRequest, System, ToolResult (too noisy)
                _ => None,
            };

            if let Some(msg) = message {
                let _ = event_tx.send(BroadcastEvent::OperationUpdate {
                    operation_id: operation_id.to_string(),
                    goal_space_id: goal_space_id.to_string(),
                    operation_type: "decompose".to_string(),
                    status: "running".to_string(),
                    message: msg,
                    result: None,
                });
            }
        } else {
            // Check if this unparseable line is the result line (type: "result")
            if line.contains("\"type\":\"result\"") || line.contains("\"type\": \"result\"") {
                result_line = Some(line.clone());
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
        anyhow::bail!(
            "Claude decomposition failed (exit {}): {}",
            status,
            stderr_output
        );
    }

    // Prefer structured output from StructuredOutput tool use, fall back to result line
    if let Some(output) = structured_output {
        tracing::debug!("Decomposition structured output: {}", output);
        let output_str =
            serde_json::to_string(&output).context("Failed to serialize structured output")?;
        parse_decomposition_output(&output_str)
    } else {
        let raw_result = result_line.context("No result event received from Claude stream")?;
        tracing::debug!("Decomposition result line: {}", raw_result);
        parse_decomposition_output(&raw_result)
    }
}

/// Extract tasks from the "result" field of Claude CLI JSON output.
///
/// The result field can be:
/// 1. A JSON string that parses directly: "{\"tasks\":[...]}"
/// 2. A text string with JSON embedded: "Here are the tasks:\n{\"tasks\":[...]}\n"
/// 3. An object with a "tasks" key: {"tasks":[...]}
fn extract_tasks_from_result_field(result_field: &serde_json::Value) -> Result<serde_json::Value> {
    // Case 3: result is already an object with "tasks"
    if let Some(tasks) = result_field.get("tasks") {
        return Ok(tasks.clone());
    }

    // Cases 1 and 2: result is a string
    let result_str = match result_field.as_str() {
        Some(s) => s,
        None => {
            anyhow::bail!(
                "Unexpected 'result' field type (not string or object): {}",
                serde_json::to_string_pretty(result_field)
                    .unwrap_or_else(|_| "<unprintable>".into())
            );
        }
    };

    // Case 1: try direct JSON parse
    if let Ok(inner) = serde_json::from_str::<serde_json::Value>(result_str) {
        if let Some(tasks) = inner.get("tasks") {
            return Ok(tasks.clone());
        }
        // Parsed as JSON but no "tasks" key — maybe it IS the tasks array directly
        if inner.is_array() {
            return Ok(inner);
        }
    }

    // Case 2: scan for embedded JSON object containing "tasks"
    // Look for the outermost {...} that contains "tasks"
    if let Some(tasks) = extract_json_with_tasks(result_str) {
        return Ok(tasks);
    }

    // Nothing worked — provide a useful error with the actual content
    let preview = if result_str.len() > 500 {
        format!("{}...", &result_str[..500])
    } else {
        result_str.to_string()
    };
    anyhow::bail!(
        "Could not find tasks in result string. Content preview:\n{}",
        preview
    )
}

/// Scan a string for an embedded JSON object containing a "tasks" array.
/// Finds the first `{` that leads to a valid JSON object with a "tasks" key.
fn extract_json_with_tasks(s: &str) -> Option<serde_json::Value> {
    for (i, _) in s.match_indices('{') {
        // Try parsing from this brace to the end
        let candidate = &s[i..];
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(candidate) {
            if let Some(tasks) = parsed.get("tasks") {
                return Some(tasks.clone());
            }
        }
        // Try finding the matching closing brace by parsing progressively
        // (handles cases where there's trailing text after the JSON)
        let mut depth = 0i32;
        let mut in_string = false;
        let mut escape_next = false;
        for (j, ch) in candidate.char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }
            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => depth += 1,
                '}' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        let slice = &candidate[..=j];
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(slice) {
                            if let Some(tasks) = parsed.get("tasks") {
                                return Some(tasks.clone());
                            }
                        }
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    None
}

#[derive(serde::Deserialize)]
struct RawTask {
    title: String,
    description: String,
    depends_on: Vec<usize>,
}

/// Parse the raw stdout from `claude -p --output-format json --json-schema` into CreateTask list.
/// Handles both direct output and the `{"result":"<json-string>"}` wrapper format.
pub fn parse_decomposition_output(stdout: &str) -> Result<Vec<CreateTask>> {
    let result: serde_json::Value =
        serde_json::from_str(stdout).context("Failed to parse decomposition output as JSON")?;

    // Check for error conditions from the Claude CLI.
    // When the agent hits max turns or errors out, the response has type:"result"
    // with a subtype like "error_max_turns" but NO "result" field.
    if let Some(is_error) = result.get("is_error").and_then(|v| v.as_bool()) {
        if is_error {
            let subtype = result
                .get("subtype")
                .and_then(|s| s.as_str())
                .unwrap_or("unknown");
            anyhow::bail!("Claude returned an error (subtype: {})", subtype);
        }
    }
    if let Some(subtype) = result.get("subtype").and_then(|s| s.as_str()) {
        if subtype.starts_with("error") {
            let cost = result
                .get("total_cost_usd")
                .and_then(|c| c.as_f64())
                .unwrap_or(0.0);
            let turns = result
                .get("num_turns")
                .and_then(|t| t.as_u64())
                .unwrap_or(0);
            anyhow::bail!(
                "Claude decomposition failed: {} (used {} turns, ${:.2}). \
                 The agent may need more turns to explore the codebase and produce output.",
                subtype,
                turns,
                cost
            );
        }
    }

    let tasks_value = if let Some(tasks) = result.get("tasks") {
        // Direct schema output: {"tasks": [...]}
        tasks.clone()
    } else if let Some(result_field) = result.get("result") {
        extract_tasks_from_result_field(result_field)?
    } else {
        anyhow::bail!(
            "No 'tasks' or 'result' field in decomposition output: {}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
    };

    let raw_tasks: Vec<RawTask> =
        serde_json::from_value(tasks_value).context("Failed to parse tasks array")?;

    let tasks: Vec<CreateTask> = raw_tasks
        .into_iter()
        .map(|t| CreateTask {
            title: t.title,
            description: t.description,
            priority: 0,
            depends_on: t
                .depends_on
                .into_iter()
                .map(|i| format!("__index_{}", i))
                .collect(),
            settings: Default::default(),
        })
        .collect();

    Ok(tasks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_direct_schema_output() {
        let output = r#"{"tasks":[{"title":"Add validation","description":"Add input validation","depends_on":[]},{"title":"Write tests","description":"Write tests for validation","depends_on":[0]}]}"#;
        let tasks = parse_decomposition_output(output).unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].title, "Add validation");
        assert!(tasks[0].depends_on.is_empty());
        assert_eq!(tasks[1].title, "Write tests");
        assert_eq!(tasks[1].depends_on, vec!["__index_0"]);
    }

    #[test]
    fn test_parse_wrapped_result_string() {
        // This is the actual format from `claude -p --output-format json`
        let inner = r#"{"tasks":[{"title":"Task A","description":"Do A","depends_on":[]}]}"#;
        let output = serde_json::json!({
            "type": "result",
            "subtype": "success",
            "cost_usd": 0.05,
            "is_error": false,
            "result": inner,
            "session_id": "sess-123"
        });
        let tasks = parse_decomposition_output(&output.to_string()).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Task A");
    }

    #[test]
    fn test_parse_wrapped_result_object() {
        let output =
            r#"{"result":{"tasks":[{"title":"Task B","description":"Do B","depends_on":[]}]}}"#;
        let tasks = parse_decomposition_output(output).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Task B");
    }

    #[test]
    fn test_parse_no_tasks_field_errors() {
        let output = r#"{"something":"else"}"#;
        let err = parse_decomposition_output(output).unwrap_err();
        assert!(err.to_string().contains("No 'tasks' or 'result' field"));
    }

    #[test]
    fn test_parse_invalid_json_errors() {
        let err = parse_decomposition_output("not json").unwrap_err();
        assert!(err.to_string().contains("Failed to parse"));
    }

    #[test]
    fn test_parse_result_string_without_tasks_errors() {
        let output = r#"{"result":"{\"no_tasks\":true}"}"#;
        let err = parse_decomposition_output(output).unwrap_err();
        assert!(err.to_string().contains("Could not find tasks"));
    }

    #[test]
    fn test_parse_result_with_surrounding_text() {
        // Claude sometimes wraps JSON in explanation text.
        // In the actual JSON envelope, result is a string with real quotes
        // (the JSON serialization handles escaping), so when we read it
        // back via .as_str() we get actual " characters, not \".
        let inner = "Here are the decomposed tasks:\n\n{\"tasks\":[{\"title\":\"Do X\",\"description\":\"Details\",\"depends_on\":[]}]}\n\nLet me know if you need changes.";
        let output = serde_json::json!({
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "result": inner,
            "session_id": "sess-456"
        });
        let tasks = parse_decomposition_output(&output.to_string()).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Do X");
    }

    #[test]
    fn test_parse_result_with_real_embedded_json() {
        // More realistic: actual newlines, not escaped
        let result_str = "Based on the codebase, here are the tasks:\n\n{\"tasks\":[{\"title\":\"Add auth\",\"description\":\"Add authentication middleware\",\"depends_on\":[]},{\"title\":\"Add tests\",\"description\":\"Write auth tests\",\"depends_on\":[0]}]}";
        let output = serde_json::json!({
            "type": "result",
            "result": result_str,
        });
        let tasks = parse_decomposition_output(&output.to_string()).unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].title, "Add auth");
        assert_eq!(tasks[1].depends_on, vec!["__index_0"]);
    }

    #[test]
    fn test_parse_result_plain_text_no_json_errors() {
        let output = serde_json::json!({
            "type": "result",
            "result": "I couldn't decompose this goal because the repository is empty.",
        });
        let err = parse_decomposition_output(&output.to_string()).unwrap_err();
        assert!(err.to_string().contains("Could not find tasks"));
    }

    #[test]
    fn test_parse_dependencies_converted_to_index_ids() {
        let output = r#"{"tasks":[{"title":"A","description":"D","depends_on":[]},{"title":"B","description":"D","depends_on":[0]},{"title":"C","description":"D","depends_on":[0,1]}]}"#;
        let tasks = parse_decomposition_output(output).unwrap();
        assert_eq!(tasks[0].depends_on.len(), 0);
        assert_eq!(tasks[1].depends_on, vec!["__index_0"]);
        assert_eq!(tasks[2].depends_on, vec!["__index_0", "__index_1"]);
    }

    #[test]
    fn test_parse_empty_tasks_array() {
        let output = r#"{"tasks":[]}"#;
        let tasks = parse_decomposition_output(output).unwrap();
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_parse_error_max_turns() {
        let output = serde_json::json!({
            "type": "result",
            "subtype": "error_max_turns",
            "is_error": false,
            "num_turns": 5,
            "total_cost_usd": 0.65,
            "session_id": "sess-123"
        });
        let err = parse_decomposition_output(&output.to_string()).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("error_max_turns"), "got: {}", msg);
        assert!(msg.contains("5 turns"), "got: {}", msg);
    }

    #[test]
    fn test_parse_is_error_true() {
        let output = serde_json::json!({
            "type": "result",
            "subtype": "error",
            "is_error": true,
            "session_id": "sess-123"
        });
        let err = parse_decomposition_output(&output.to_string()).unwrap_err();
        assert!(err.to_string().contains("error"));
    }

    #[test]
    fn test_parse_success_subtype_not_treated_as_error() {
        let inner = r#"{"tasks":[{"title":"T","description":"D","depends_on":[]}]}"#;
        let output = serde_json::json!({
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "result": inner,
            "session_id": "sess-123"
        });
        let tasks = parse_decomposition_output(&output.to_string()).unwrap();
        assert_eq!(tasks.len(), 1);
    }
}
