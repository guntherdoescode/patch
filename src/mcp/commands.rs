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
    const ALLOWLIST: &[&str] = &["pwd", "ls", "find", "rg", "grep", "cat", "head", "tail", "wc", "file", "git"];
    if !ALLOWLIST.contains(&program) { return Err(CommandError::NotAllowed); }
    if program == "git" {
        let sub = args.first().map(String::as_str).unwrap_or("");
        if !matches!(sub, "status" | "diff" | "log" | "show" | "branch" | "rev-parse" | "ls-files") {
            return Err(CommandError::NotAllowed);
        }
    }
    // Reject options that turn nominally read-only utilities into execution or
    // mutation primitives. No shell is ever involved.
    if program == "find" && args.iter().any(|a| ["-exec", "-execdir", "-delete", "-ok", "-okdir"].contains(&a.as_str())) {
        return Err(CommandError::NotAllowed);
    }
    if program == "sed" && args.iter().any(|a| a == "-i" || a.starts_with("--in-place")) {
        return Err(CommandError::NotAllowed);
    }
    if program == "rg" && args.iter().any(|a| a == "--pre" || a.starts_with("--pre=")) {
        return Err(CommandError::NotAllowed);
    }
    // Validate path operands separately from search expressions/options so
    // read-only commands cannot be used to read arbitrary host files.
    let path_operands: Vec<&str> = match program {
        "pwd" => {
            if !args.is_empty() { return Err(CommandError::NotAllowed); }
            Vec::new()
        }
        "ls" | "cat" | "head" | "tail" | "wc" | "file" => args.iter()
            .filter(|a| !a.starts_with('-')).map(String::as_str).collect(),
        "find" => args.first().filter(|a| !a.starts_with('-')).map(|a| vec![a.as_str()]).unwrap_or_default(),
        "rg" => {
            let Some(separator) = args.iter().position(|a| a == "--") else { return Err(CommandError::NotAllowed); };
            args.iter().skip(separator + 2).map(String::as_str).collect()
        }
        "grep" => {
            let mut seen_pattern = false;
            args.iter().filter_map(|arg| {
                if arg.starts_with('-') { None }
                else if !seen_pattern { seen_pattern = true; None }
                else { Some(arg.as_str()) }
            }).collect()
        }
        "git" => {
            if args.iter().any(|a| a.starts_with('/') || a.split('/').any(|part| part == "..")
                || ["--no-index", "--ext-diff", "--textconv", "-c"].contains(&a.as_str())) {
                return Err(CommandError::NotAllowed);
            }
            Vec::new()
        }
        _ => return Err(CommandError::NotAllowed),
    };
    for operand in path_operands {
        workspace.resolve(operand).map_err(|_| CommandError::NotAllowed)?;
    }

    let mut command = Command::new(program);
    command.args(args).current_dir(workspace.root())
        .env("GIT_CONFIG_NOSYSTEM", "1").env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_OPTIONAL_LOCKS", "0").env("GIT_PAGER", "cat").env("PAGER", "cat")
        .stdin(Stdio::null()).stderr(Stdio::piped()).stdout(Stdio::piped());
    let output = command.output()?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.status.success() {
        text.push_str(&String::from_utf8_lossy(&output.stderr));
        return Err(CommandError::Failed(text));
    }
    if text.len() > 1_000_000 { text.truncate(1_000_000); text.push_str("\n[output truncated]"); }
    Ok(text)
}
