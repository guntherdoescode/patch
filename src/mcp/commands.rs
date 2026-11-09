use std::process::{Command, Stdio};
use super::workspace::Workspace;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("command is not on the read-only allowlist")]
    NotAllowed,
    #[error("command failed: {0}")]
    Failed(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Deliberately shell-free. Arguments are passed directly to an executable;
/// pipes, redirects, substitutions and shell built-ins are never interpreted.
pub fn run_read_only(workspace: &Workspace, program: &str, args: &[String]) -> Result<String, CommandError> {
    const ALLOWLIST: &[&str] = &["pwd", "ls", "find", "rg", "grep", "cat", "sed", "head", "tail", "wc", "file", "git"];
    if !ALLOWLIST.contains(&program) { return Err(CommandError::NotAllowed); }
    if program == "git" {
        let sub = args.first().map(String::as_str).unwrap_or("");
        if !matches!(sub, "status" | "diff" | "log" | "show" | "branch" | "rev-parse" | "ls-files") {
            return Err(CommandError::NotAllowed);
        }
    }
    let mut command = Command::new(program);
    command.args(args).current_dir(workspace.root()).stdin(Stdio::null()).stderr(Stdio::piped()).stdout(Stdio::piped());
    let output = command.output()?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.status.success() {
        text.push_str(&String::from_utf8_lossy(&output.stderr));
        return Err(CommandError::Failed(text));
    }
    if text.len() > 1_000_000 { text.truncate(1_000_000); text.push_str("\n[output truncated]"); }
    Ok(text)
}
