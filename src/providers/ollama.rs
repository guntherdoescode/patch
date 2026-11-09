//! Ollama local model discovery and model metadata inspection.

use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub modified_at: String,
}

#[derive(Debug, Deserialize)]
struct TagsResponse {
    #[serde(default)]
    models: Vec<OllamaModel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OllamaModelInfo {
    pub context_length: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Default)]
pub struct ChatUsage {
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
}

#[derive(Debug, thiserror::Error)]
pub enum ChatError {
    #[error("Request failed: {0}")]
    Request(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Ollama error: {0}")]
    Provider(String),
}

#[derive(Debug, thiserror::Error)]
pub enum OllamaError {
    #[error("Could not reach Ollama at {0}")]
    Unreachable(String),
    #[error("Ollama returned an invalid response: {0}")]
    InvalidResponse(String),
}

pub struct OllamaClient {
    base_url: String,
    http: Client,
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new("http://127.0.0.1:11434")
    }
}


/// Translate provider-neutral tool definitions into Ollama's function-tool
/// schema. The MCP layer never needs to know this wire representation.
pub fn format_tools(definitions: &[crate::mcp::agent::ToolDefinition]) -> Value {
    Value::Array(definitions.iter().map(|tool| serde_json::json!({
        "type": "function",
        "function": {
            "name": tool.name,
            "description": tool.description,
            "parameters": tool.input_schema
        }
    })).collect())
}

impl OllamaClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            http: Client::builder()
                .timeout(Duration::from_secs(8))
                .build()
                .expect("reqwest client configuration is valid"),
        }
    }

    pub fn list_models(&self) -> Result<Vec<OllamaModel>, OllamaError> {
        let response = self.http
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .map_err(|error| OllamaError::Unreachable(error.to_string()))?
            .error_for_status()
            .map_err(|error| OllamaError::Unreachable(error.to_string()))?;

        response.json::<TagsResponse>()
            .map(|body| body.models)
            .map_err(|error| OllamaError::InvalidResponse(error.to_string()))
    }

    /// Stream a chat completion from Ollama's newline-delimited JSON endpoint.
    /// Each content fragment is sent immediately; final usage is returned after
    /// Ollama marks the response as done.
    pub fn chat_stream(
        &self,
        model: &str,
        messages: &[ChatMessage],
        mut on_chunk: impl FnMut(String),
    ) -> Result<ChatUsage, ChatError> {
        use std::io::{BufRead, BufReader};

        let response = self.http
            .post(format!("{}/api/chat", self.base_url))
            .json(&serde_json::json!({
                "model": model,
                "messages": messages.iter().map(|message| serde_json::json!({
                    "role": message.role,
                    "content": message.content
                })).collect::<Vec<_>>(),
                "stream": true
            }))
            .timeout(Duration::from_secs(300))
            .send()
            .map_err(|error| ChatError::Request(error.to_string()))?
            .error_for_status()
            .map_err(|error| ChatError::Request(error.to_string()))?;

        let mut usage = ChatUsage::default();
        let reader = BufReader::new(response);
        for line in reader.lines() {
            let line = line.map_err(|error| ChatError::Request(error.to_string()))?;
            if line.trim().is_empty() {
                continue;
            }
            let value: Value = serde_json::from_str(&line)
                .map_err(|error| ChatError::InvalidResponse(error.to_string()))?;
            if let Some(error) = value.get("error").and_then(Value::as_str) {
                return Err(ChatError::Provider(error.to_owned()));
            }
            if let Some(fragment) = value.pointer("/message/content").and_then(Value::as_str) {
                if !fragment.is_empty() {
                    on_chunk(fragment.to_owned());
                }
            }
            if value.get("done").and_then(Value::as_bool) == Some(true) {
                usage.prompt_tokens = value.get("prompt_eval_count").and_then(Value::as_u64);
                usage.completion_tokens = value.get("eval_count").and_then(Value::as_u64);
                break;
            }
        }
        Ok(usage)
    }

    /// Run a bounded tool-calling turn. Tool execution is delegated to the
    /// caller so the central permission layer remains authoritative.
    pub fn chat_with_tools(
        &self,
        model: &str,
        messages: &[ChatMessage],
        tools: &Value,
        mut execute: impl FnMut(&str, Value) -> Result<String, String>,
        mut on_chunk: impl FnMut(String),
    ) -> Result<ChatUsage, ChatError> {
        let mut conversation = messages.iter().map(|message| serde_json::json!({
            "role": message.role, "content": message.content
        })).collect::<Vec<_>>();
        let mut usage = ChatUsage::default();

        for _ in 0..8 {
            let response = self.http
                .post(format!("{}/api/chat", self.base_url))
                .json(&serde_json::json!({
                    "model": model, "messages": conversation, "tools": tools, "stream": false
                }))
                .timeout(Duration::from_secs(300))
                .send()
                .map_err(|error| ChatError::Request(error.to_string()))?
                .error_for_status()
                .map_err(|error| ChatError::Request(error.to_string()))?;
            let value: Value = response.json()
                .map_err(|error| ChatError::InvalidResponse(error.to_string()))?;
            if let Some(error) = value.get("error").and_then(Value::as_str) {
                return Err(ChatError::Provider(error.to_owned()));
            }
            if let Some(count) = value.get("prompt_eval_count").and_then(Value::as_u64) {
                usage.prompt_tokens = Some(usage.prompt_tokens.unwrap_or(0).saturating_add(count));
            }
            if let Some(count) = value.get("eval_count").and_then(Value::as_u64) {
                usage.completion_tokens = Some(usage.completion_tokens.unwrap_or(0).saturating_add(count));
            }
            let message = value.get("message").cloned()
                .ok_or_else(|| ChatError::InvalidResponse("missing message".into()))?;
            // Only accept calls for tools advertised in this request. The MCP
            // catalogue can contain arbitrary server-defined names; the provider
            // adapter must not maintain its own hard-coded tool-name whitelist.
            let allowed_tools = tools.as_array().into_iter().flatten()
                .filter_map(|tool| tool.pointer("/function/name").and_then(Value::as_str))
                .map(str::to_owned)
                .collect::<Vec<_>>();
            let parsed_calls = crate::mcp::agent::tool_calls(
                &serde_json::json!({"message": message}),
                &allowed_tools,
            );
            if parsed_calls.is_empty() {
                if let Some(content) = message.get("content").and_then(Value::as_str) {
                    if !content.is_empty() { on_chunk(content.to_owned()); }
                }
                return Ok(usage);
            }

            // Ollama expects an assistant tool_calls message followed by one
            // tool message per call. Some models emit a JSON envelope in plain
            // content instead; convert that into the same canonical shape rather
            // than replaying the JSON as ordinary assistant prose.
            let mut assistant_message = message.clone();
            if assistant_message.get("tool_calls").and_then(Value::as_array).is_none() {
                assistant_message["content"] = Value::String(String::new());
                assistant_message["tool_calls"] = Value::Array(
                    parsed_calls.iter().enumerate().map(|(index, (name, arguments))| {
                        serde_json::json!({
                            "type": "function",
                            "function": {
                                "index": index,
                                "name": name,
                                "arguments": arguments
                            }
                        })
                    }).collect()
                );
            }
            conversation.push(assistant_message);

            for (name, arguments) in parsed_calls {
                let result = execute(&name, arguments)
                    .unwrap_or_else(|error| format!("Tool error: {error}"));
                // tool_name is part of Ollama's documented message format.
                conversation.push(serde_json::json!({
                    "role": "tool",
                    "tool_name": name,
                    "content": result
                }));
            }
        }
        Err(ChatError::Provider("tool-call limit reached (8 rounds)".into()))
    }

    pub fn model_info(&self, model: &str) -> Result<OllamaModelInfo, OllamaError> {
        let response = self.http
            .post(format!("{}/api/show", self.base_url))
            .json(&serde_json::json!({ "name": model }))
            .send()
            .map_err(|error| OllamaError::Unreachable(error.to_string()))?
            .error_for_status()
            .map_err(|error| OllamaError::Unreachable(error.to_string()))?;

        let body = response.json::<Value>()
            .map_err(|error| OllamaError::InvalidResponse(error.to_string()))?;

        // Ollama model_info metadata commonly exposes general.context_length.
        // Search nested metadata instead of assuming one architecture key.
        let context_length = find_numeric_key(&body, "context_length");
        Ok(OllamaModelInfo { context_length })
    }
}

fn find_numeric_key(value: &Value, wanted: &str) -> Option<u64> {
    match value {
        Value::Object(map) => {
            if let Some(found) = map.iter()
                .find(|(key, _)| key.as_str() == wanted || key.ends_with(".context_length"))
                .and_then(|(_, value)| value.as_u64())
            {
                return Some(found);
            }
            map.values().find_map(|child| find_numeric_key(child, wanted))
        }
        Value::Array(items) => items.iter().find_map(|child| find_numeric_key(child, wanted)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_context_length_in_nested_model_metadata() {
        let value = serde_json::json!({
            "model_info": { "llama.context_length": 32768 },
            "details": {}
        });
        assert_eq!(find_numeric_key(&value, "context_length"), Some(32768));
    }

    #[test]
    fn does_not_invent_missing_context_length() {
        let value = serde_json::json!({ "details": { "family": "llama" } });
        assert_eq!(find_numeric_key(&value, "context_length"), None);
    }
}
