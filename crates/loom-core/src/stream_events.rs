use loom_adapters::command_runner::now_string;
use loom_types::api::{AgentStreamEvent, LiveDiff};
use serde_json::Value;
use uuid::Uuid;

pub fn parse_stream_line(agent_id: &str, line: &str) -> Option<AgentStreamEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let value: Value = serde_json::from_str(trimmed).ok()?;
    parse_claude_stream(agent_id, &value).or_else(|| parse_codex_stream(agent_id, &value))
}

pub fn event_kind(event: &AgentStreamEvent) -> &'static str {
    match event {
        AgentStreamEvent::Reasoning { .. } => "reasoning",
        AgentStreamEvent::ToolUse { .. } => "tool_use",
        AgentStreamEvent::ToolResult { .. } => "tool_result",
        AgentStreamEvent::AgentMessage { .. } => "agent_message",
        AgentStreamEvent::Done { .. } => "done",
        AgentStreamEvent::McpCall { .. } => "mcp_call",
    }
}

pub fn event_live_diff(event: &AgentStreamEvent) -> Option<LiveDiff> {
    match event {
        AgentStreamEvent::ToolUse { input, .. } => input
            .get("live_diff")
            .and_then(|value| serde_json::from_value(value.clone()).ok()),
        _ => None,
    }
}

fn parse_claude_stream(agent_id: &str, value: &Value) -> Option<AgentStreamEvent> {
    let type_name = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match type_name {
        "assistant" => {
            let content = value
                .get("message")
                .and_then(|message| message.get("content"))
                .and_then(Value::as_array)?;
            first_content_event(agent_id, content)
        }
        "user" => {
            let content = value
                .get("message")
                .and_then(|message| message.get("content"))
                .and_then(Value::as_array)?;
            content.iter().find_map(|part| {
                if part.get("type").and_then(Value::as_str) == Some("tool_result") {
                    let success = !part
                        .get("is_error")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    Some(tool_result_event(
                        agent_id,
                        part.get("name")
                            .or_else(|| part.get("tool"))
                            .and_then(Value::as_str)
                            .unwrap_or("tool"),
                        success,
                        text_from_value(part.get("content").unwrap_or(&Value::Null)),
                    ))
                } else {
                    None
                }
            })
        }
        "tool_use" => Some(tool_use_event(
            agent_id,
            value.get("name").and_then(Value::as_str).unwrap_or("tool"),
            value.get("input").cloned().unwrap_or(Value::Null),
        )),
        "tool_result" => Some(tool_result_event(
            agent_id,
            value
                .get("name")
                .or_else(|| value.get("tool"))
                .and_then(Value::as_str)
                .unwrap_or("tool"),
            !value
                .get("is_error")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            text_from_value(value.get("content").unwrap_or(&Value::Null)),
        )),
        "result" => Some(done_event(
            agent_id,
            value
                .get("exit_code")
                .and_then(Value::as_i64)
                .unwrap_or_else(|| {
                    if value.get("subtype").and_then(Value::as_str) == Some("success") {
                        0
                    } else {
                        1
                    }
                }) as i32,
        )),
        _ => None,
    }
}

fn parse_codex_stream(agent_id: &str, value: &Value) -> Option<AgentStreamEvent> {
    let type_name = value
        .get("type")
        .or_else(|| value.get("event"))
        .or_else(|| value.get("kind"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    match type_name {
        "reasoning" | "agent_reasoning" | "reasoning_delta" => Some(reasoning_event(
            value
                .get("text")
                .or_else(|| value.get("message"))
                .and_then(Value::as_str)
                .unwrap_or_default(),
        )),
        "agent_message" | "message" | "assistant_message" => Some(agent_message_event(
            agent_id,
            value
                .get("text")
                .or_else(|| value.get("message"))
                .and_then(Value::as_str)
                .unwrap_or_default(),
        )),
        "tool_call" | "tool_use" | "function_call" => Some(tool_use_event(
            agent_id,
            value
                .get("tool")
                .or_else(|| value.get("tool_name"))
                .or_else(|| value.get("tool_id"))
                .or_else(|| value.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("tool"),
            value
                .get("input")
                .or_else(|| value.get("arguments"))
                .cloned()
                .unwrap_or(Value::Null),
        )),
        "tool_result" | "function_result" => Some(tool_result_event(
            agent_id,
            value
                .get("tool")
                .or_else(|| value.get("tool_name"))
                .or_else(|| value.get("tool_id"))
                .or_else(|| value.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("tool"),
            value
                .get("success")
                .and_then(Value::as_bool)
                .unwrap_or_else(|| value.get("error").is_none()),
            text_from_value(
                value
                    .get("output")
                    .or_else(|| value.get("content"))
                    .unwrap_or(&Value::Null),
            ),
        )),
        "done" | "completed" => Some(done_event(
            agent_id,
            value
                .get("exit_code")
                .or_else(|| value.get("code"))
                .and_then(Value::as_i64)
                .unwrap_or(0) as i32,
        )),
        _ => parts_event(agent_id, value),
    }
}

fn first_content_event(agent_id: &str, content: &[Value]) -> Option<AgentStreamEvent> {
    content
        .iter()
        .find_map(|part| match part.get("type").and_then(Value::as_str) {
            Some("text") => part
                .get("text")
                .and_then(Value::as_str)
                .filter(|text| !text.trim().is_empty())
                .map(|text| agent_message_event(agent_id, text)),
            Some("tool_use") => Some(tool_use_event(
                agent_id,
                part.get("name").and_then(Value::as_str).unwrap_or("tool"),
                part.get("input").cloned().unwrap_or(Value::Null),
            )),
            _ => None,
        })
}

fn parts_event(agent_id: &str, value: &Value) -> Option<AgentStreamEvent> {
    let parts = value.get("parts").and_then(Value::as_array)?;
    parts
        .iter()
        .find_map(|part| match part.get("type").and_then(Value::as_str) {
            Some("text") | Some("output_text") => part
                .get("text")
                .and_then(Value::as_str)
                .map(|text| agent_message_event(agent_id, text)),
            Some("tool_call") | Some("function_call") => Some(tool_use_event(
                agent_id,
                part.get("tool")
                    .or_else(|| part.get("tool_name"))
                    .or_else(|| part.get("tool_id"))
                    .or_else(|| part.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or("tool"),
                part.get("input")
                    .or_else(|| part.get("arguments"))
                    .cloned()
                    .unwrap_or(Value::Null),
            )),
            Some("tool_result") | Some("function_result") => Some(tool_result_event(
                agent_id,
                part.get("tool")
                    .or_else(|| part.get("tool_name"))
                    .or_else(|| part.get("tool_id"))
                    .or_else(|| part.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or("tool"),
                part.get("success").and_then(Value::as_bool).unwrap_or(true),
                text_from_value(
                    part.get("output")
                        .or_else(|| part.get("content"))
                        .unwrap_or(&Value::Null),
                ),
            )),
            _ => None,
        })
}

fn reasoning_event(text: &str) -> AgentStreamEvent {
    AgentStreamEvent::Reasoning {
        id: Uuid::new_v4(),
        at: now_string(),
        text: bounded(text, 4096),
    }
}

fn agent_message_event(agent_id: &str, text: &str) -> AgentStreamEvent {
    AgentStreamEvent::AgentMessage {
        id: Uuid::new_v4(),
        at: now_string(),
        agent_id: agent_id.to_string(),
        text: bounded(text, 4096),
    }
}

fn tool_use_event(agent_id: &str, tool: &str, mut input: Value) -> AgentStreamEvent {
    if let Some((server, normalized_tool)) = mcp_tool(tool) {
        return AgentStreamEvent::McpCall {
            id: Uuid::new_v4(),
            at: now_string(),
            agent_id: agent_id.to_string(),
            tool: normalized_tool,
            server,
            input,
            output: None,
        };
    }
    if let Some(diff) = live_diff_for_tool(tool, &input) {
        ensure_object(&mut input).insert(
            "live_diff".to_string(),
            serde_json::to_value(diff).unwrap_or(Value::Null),
        );
    }
    AgentStreamEvent::ToolUse {
        id: Uuid::new_v4(),
        at: now_string(),
        agent_id: agent_id.to_string(),
        tool: tool.to_string(),
        input,
    }
}

fn mcp_tool(tool: &str) -> Option<(String, String)> {
    if let Some(rest) = tool.strip_prefix("mcp__") {
        let mut parts = rest.splitn(2, "__");
        let server = parts.next()?.replace('_', "-");
        let name = parts.next().unwrap_or(rest).replace("__", ".");
        return Some((server, name));
    }
    if let Some(rest) = tool.strip_prefix("mcp.") {
        let mut parts = rest.splitn(2, '.');
        let server = parts.next().unwrap_or("mcp").to_string();
        let name = parts.next().unwrap_or(rest).to_string();
        return Some((server, name));
    }
    None
}

fn tool_result_event(
    agent_id: &str,
    tool: &str,
    success: bool,
    snippet: Option<String>,
) -> AgentStreamEvent {
    AgentStreamEvent::ToolResult {
        id: Uuid::new_v4(),
        at: now_string(),
        agent_id: agent_id.to_string(),
        tool: tool.to_string(),
        success,
        snippet: snippet.map(|text| bounded(&text, 1024)),
    }
}

fn done_event(agent_id: &str, exit_code: i32) -> AgentStreamEvent {
    AgentStreamEvent::Done {
        id: Uuid::new_v4(),
        at: now_string(),
        agent_id: agent_id.to_string(),
        exit_code,
    }
}

fn live_diff_for_tool(tool: &str, input: &Value) -> Option<LiveDiff> {
    let lowered = tool.to_ascii_lowercase();
    if !(lowered.contains("edit") || lowered.contains("write")) {
        return None;
    }
    let path = string_field(input, &["file_path", "path", "filename", "relative_path"])?;
    let before = string_field(input, &["old_string", "old_str", "before"]);
    let after = string_field(
        input,
        &["new_string", "new_str", "replacement", "content", "after"],
    );
    if before.is_none() && after.is_none() {
        return None;
    }
    let unified_diff = unified_diff(
        &path,
        before.as_deref().unwrap_or(""),
        after.as_deref().unwrap_or(""),
    );
    Some(LiveDiff {
        schema_version: "x07.studio.live_diff@0.1.0".to_string(),
        path,
        before: before.map(|text| bounded(&text, 4096)),
        after: after.map(|text| bounded(&text, 4096)),
        unified_diff,
    })
}

fn string_field(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .map(str::to_string)
}

fn text_from_value(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(items) => Some(
            items
                .iter()
                .filter_map(text_from_value)
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .filter(|text| !text.trim().is_empty()),
        Value::Object(map) => map
            .get("text")
            .or_else(|| map.get("content"))
            .and_then(text_from_value),
        _ => None,
    }
}

fn ensure_object(value: &mut Value) -> &mut serde_json::Map<String, Value> {
    if !value.is_object() {
        *value = Value::Object(Default::default());
    }
    value.as_object_mut().expect("object was just installed")
}

fn bounded(text: &str, max_chars: usize) -> String {
    let mut out = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        out.push_str("\n...");
    }
    out
}

fn unified_diff(path: &str, before: &str, after: &str) -> String {
    let mut out = format!("--- a/{path}\n+++ b/{path}\n");
    out.push_str("@@\n");
    for line in before.lines().take(80) {
        out.push('-');
        out.push_str(line);
        out.push('\n');
    }
    for line in after.lines().take(80) {
        out.push('+');
        out.push_str(line);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{event_live_diff, parse_stream_line};

    #[test]
    fn parses_claude_tool_use_with_live_diff() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Write","input":{"file_path":"src/main.x07.json","content":"{\"kind\":\"module\"}"}}]}}"#;

        let event = parse_stream_line("claude-code", line).expect("event");
        let diff = event_live_diff(&event).expect("diff");

        assert_eq!(diff.path, "src/main.x07.json");
        assert!(diff.unified_diff.contains("+++ b/src/main.x07.json"));
    }

    #[test]
    fn parses_codex_parts_tool_result() {
        let line = r#"{"type":"turn","parts":[{"type":"tool_result","tool_name":"Edit","success":true,"output":"patched src/main.x07.json"}]}"#;

        let event = parse_stream_line("openai-codex", line).expect("event");

        match event {
            loom_types::api::AgentStreamEvent::ToolResult {
                tool,
                success,
                snippet,
                ..
            } => {
                assert_eq!(tool, "Edit");
                assert!(success);
                assert_eq!(snippet.as_deref(), Some("patched src/main.x07.json"));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn parses_mcp_tool_use_as_transparent_call() {
        let line =
            r#"{"type":"tool_call","tool_id":"mcp.x07.search_v1","input":{"query":"trust"}}"#;

        let event = parse_stream_line("openai-codex", line).expect("event");

        match event {
            loom_types::api::AgentStreamEvent::McpCall {
                server,
                tool,
                input,
                ..
            } => {
                assert_eq!(server, "x07");
                assert_eq!(tool, "search_v1");
                assert_eq!(input["query"], "trust");
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
