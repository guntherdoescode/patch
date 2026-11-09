//! Agent "hands": native workspace tools, command execution, policy enforcement,
//! MCP stdio transport, and model tool-call orchestration.
pub mod agent;
pub mod commands;
pub mod policy;
pub mod protocol;
pub mod tools;
pub mod workspace;

pub use policy::{AccessDecision, ExecutionMode, Operation};
pub use workspace::Workspace;
