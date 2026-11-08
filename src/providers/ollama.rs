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
