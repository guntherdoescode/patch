//! Provider-neutral tool-call parsing and the built-in workspace tool catalogue.
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

impl ToolDefinition {
    fn new(name: &str, description: &str, input_schema: Value) -> Self {
        Self { name: name.to_owned(), description: description.to_owned(), input_schema }
    }
}

/// Provider-neutral definitions for Patch's built-in workspace tools. External
/// MCP servers can contribute the same shape directly from tools/list without
/// being coupled to a particular model API.
pub fn native_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition::new("get_current_directory", "Return the active workspace root.", json!({"type":"object","properties":{},"required":[]})),
        ToolDefinition::new("list_directory", "List entries in a workspace-relative directory.", json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]})),
        ToolDefinition::new("read_file", "Read a UTF-8 text file inside the active workspace.", json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]})),
        ToolDefinition::new("search_files", "Search workspace text files for a literal or regular-expression query.", json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]})),
        ToolDefinition::new("run_read_only", "Run a permitted read-only command without a shell. Provide an executable and argument array.", json!({"type":"object","properties":{"program":{"type":"string"},"args":{"type":"array","items":{"type":"string"}}},"required":["program","args"]})),
        ToolDefinition::new("write_file", "Create or replace a UTF-8 text file inside the active workspace. Build mode only.", json!({"type":"object","properties":{"path":{"type":"string"},"contents":{"type":"string"}},"required":["path","contents"]})),
        ToolDefinition::new("delete_file", "Delete a file inside the active workspace. Build mode only.", json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]})),
    ]
}

/// Extract structured calls from Ollama's native tool_calls field. Some
/// tool-capable models incorrectly print the same call as a JSON object in
/// message.content; accept that form only when it is an exact, known tool
/// envelope, never by scraping arbitrary prose.
pub fn tool_calls(message: &Value, allowed_tools: &[String]) -> Vec<(String, Value)> {
    if let Some(calls) = message.pointer("/message/tool_calls").and_then(Value::as_array) {
        let parsed = calls.iter().filter_map(|call| {
            let function = call.get("function")?;
            let name = function.get("name")?.as_str()?.to_owned();
            if !allowed_tools.iter().any(|allowed| allowed == &name) { return None; }
            let arguments = function.get("arguments").cloned().unwrap_or_else(|| json!({}));
            Some((name, arguments))
        }).collect::<Vec<_>>();
        if !parsed.is_empty() { return parsed; }
    }

    let Some(content) = message.pointer("/message/content").and_then(Value::as_str) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(content.trim()) else {
        return Vec::new();
    };
    let name = value.get("name").and_then(Value::as_str);
    let arguments = value.get("arguments");
    match (name, arguments) {
        (Some(name), Some(arguments)) if allowed_tools.iter().any(|allowed| allowed == name) && arguments.is_object() => {
            vec![(name.to_owned(), arguments.clone())]
        }
        _ => Vec::new(),
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_content_tool_call() {
        let message = json!({"message":{"content":r#"{"name":"read_file","arguments":{"path":"MCP.txt"}}"#}});
        assert_eq!(tool_calls(&message, &["read_file".to_owned()]), vec![("read_file".to_owned(), json!({"path":"MCP.txt"}))]);
    }

    #[test]
    fn ignores_unknown_json_content_tool() {
        let message = json!({"message":{"content":r#"{"name":"run_shell","arguments":{"command":"id"}}"#}});
        assert!(tool_calls(&message, &["read_file".to_owned()]).is_empty());
    }

    #[test]
    fn parses_native_ollama_tool_call() {
        let message = json!({"message":{"tool_calls":[{"function":{"name":"read_file","arguments":{"path":"MCP.txt"}}}] }});
        assert_eq!(tool_calls(&message, &["read_file".to_owned()]), vec![("read_file".to_owned(), json!({"path":"MCP.txt"}))]);
    }
}


#[cfg(test)]
mod dynamic_tool_tests {
    use super::*;

    #[test]
    fn accepts_any_tool_advertised_by_the_active_catalogue() {
        let message = json!({"message":{"content":r#"{"name":"git.status","arguments":{"path":"."}}"#}});
        assert_eq!(
            tool_calls(&message, &["git.status".to_owned()]),
            vec![("git.status".to_owned(), json!({"path":"."}))]
        );
    }

    #[test]
    fn rejects_tools_not_in_the_active_catalogue() {
        let message = json!({"message":{"tool_calls":[{"function":{"name":"unlisted_tool","arguments":{}}}]}});
        assert!(tool_calls(&message, &["read_file".to_owned()]).is_empty());
    }
}
