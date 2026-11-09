//! Ollama-compatible tool-call parsing and model-facing native tool catalogue.
use serde_json::{json, Value};

pub fn native_tool_definitions() -> Value {
    json!([
      {"type":"function","function":{"name":"get_current_directory","description":"Return the active workspace root.","parameters":{"type":"object","properties":{},"required":[]}}},
      {"type":"function","function":{"name":"list_directory","description":"List entries in a workspace-relative directory.","parameters":{"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}}},
      {"type":"function","function":{"name":"read_file","description":"Read a UTF-8 text file inside the active workspace.","parameters":{"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}}},
      {"type":"function","function":{"name":"search_files","description":"Search workspace text files for a literal or regular-expression query.","parameters":{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}}},
      {"type":"function","function":{"name":"run_read_only","description":"Run a permitted read-only command without a shell. Provide an executable and argument array.","parameters":{"type":"object","properties":{"program":{"type":"string"},"args":{"type":"array","items":{"type":"string"}}},"required":["program","args"]}}},
      {"type":"function","function":{"name":"write_file","description":"Create or replace a UTF-8 text file inside the active workspace. Build mode only.","parameters":{"type":"object","properties":{"path":{"type":"string"},"contents":{"type":"string"}},"required":["path","contents"]}}},
      {"type":"function","function":{"name":"delete_file","description":"Delete a file inside the active workspace. Build mode only.","parameters":{"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}}}
    ])
}

pub fn tool_calls(message: &Value) -> Vec<(String, Value)> {
    message.pointer("/message/tool_calls").and_then(Value::as_array).into_iter().flatten()
        .filter_map(|call| {
            let function = call.get("function")?;
            let name = function.get("name")?.as_str()?.to_owned();
            let arguments = function.get("arguments").cloned().unwrap_or_else(|| json!({}));
            Some((name, arguments))
        }).collect()
}
