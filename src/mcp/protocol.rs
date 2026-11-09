//! Minimal MCP JSON-RPC 2.0 stdio client primitives. Each message is one JSON
//! line, matching the MCP stdio transport. Server lifecycle/handshake is kept
//! explicit so untrusted server output is never treated as instructions.
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

pub struct McpProcess { child: Child, input: ChildStdin, output: BufReader<ChildStdout>, next_id: u64 }

#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("MCP process could not start: {0}")] Start(#[from] std::io::Error),
    #[error("MCP protocol error: {0}")] Protocol(String),
}

impl McpProcess {
    pub fn spawn(program: &str, args: &[String]) -> Result<Self, McpError> {
        let mut child = Command::new(program).args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn()?;
        let input = child.stdin.take().ok_or_else(|| McpError::Protocol("missing stdin".into()))?;
        let output = child.stdout.take().ok_or_else(|| McpError::Protocol("missing stdout".into()))?;
        Ok(Self { child, input, output: BufReader::new(output), next_id: 1 })
    }

    pub fn request(&mut self, method: &str, params: Value) -> Result<Value, McpError> {
        let id = self.next_id; self.next_id += 1;
        let message = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        writeln!(self.input, "{}", message).map_err(|e| McpError::Protocol(e.to_string()))?;
        self.input.flush().map_err(|e| McpError::Protocol(e.to_string()))?;
        loop {
            let mut line = String::new();
            self.output.read_line(&mut line).map_err(|e| McpError::Protocol(e.to_string()))?;
            if line.is_empty() { return Err(McpError::Protocol("server closed stdout".into())); }
            let value: Value = serde_json::from_str(&line).map_err(|e| McpError::Protocol(e.to_string()))?;
            if value.get("id").and_then(Value::as_u64) != Some(id) { continue; }
            if let Some(error) = value.get("error") { return Err(McpError::Protocol(error.to_string())); }
            return Ok(value.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    pub fn initialise(&mut self) -> Result<Value, McpError> {
        let result = self.request("initialize", json!({"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"Patch","version":env!("CARGO_PKG_VERSION")}}))?;
        writeln!(self.input, "{}", json!({"jsonrpc":"2.0","method":"notifications/initialized"})).map_err(|e| McpError::Protocol(e.to_string()))?;
        Ok(result)
    }

    pub fn list_tools(&mut self) -> Result<Value, McpError> { self.request("tools/list", json!({})) }
    pub fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value, McpError> {
        self.request("tools/call", json!({"name":name,"arguments":arguments}))
    }
}

impl Drop for McpProcess { fn drop(&mut self) { let _ = self.child.kill(); let _ = self.child.wait(); } }
