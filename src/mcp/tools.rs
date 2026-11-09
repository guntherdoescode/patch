use super::{commands, workspace::{Workspace, WorkspaceError}};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error(transparent)] Workspace(#[from] WorkspaceError),
    #[error("operation is not permitted in the current mode")] Denied,
    #[error("external path access requires approval")] NeedsApproval,
    #[error("I/O error: {0}")] Io(#[from] std::io::Error),
    #[error("command error: {0}")] Command(String),
}

pub fn current_directory(ws: &Workspace) -> String { ws.root().display().to_string() }

pub fn list_directory(ws: &Workspace, path: &str) -> Result<Vec<String>, ToolError> {
    let path = ws.resolve(path)?;
    let mut entries = std::fs::read_dir(path)?.filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned()).collect::<Vec<_>>();
    entries.sort();
    Ok(entries)
}

pub fn read_file(ws: &Workspace, path: &str) -> Result<String, ToolError> {
    let path = ws.resolve(path)?;
    let metadata = std::fs::metadata(&path)?;
    if !metadata.is_file() || metadata.len() > 2_000_000 { return Err(ToolError::Denied); }
    Ok(std::fs::read_to_string(path)?)
}

pub fn search_files(ws: &Workspace, query: &str) -> Result<Vec<String>, ToolError> {
    let output = commands::run_read_only(ws, "rg", &["--line-number".into(), "--hidden".into(), "--glob".into(), "!.git".into(), query.into(), ".".into()])
        .map_err(|e| ToolError::Command(e.to_string()))?;
    Ok(output.lines().take(500).map(str::to_owned).collect())
}

pub fn write_file(ws: &Workspace, path: &str, contents: &str) -> Result<(), ToolError> {
    let path = ws.resolve(Path::new(path))?;
    if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
    std::fs::write(path, contents)?;
    Ok(())
}

pub fn delete_file(ws: &Workspace, path: &str) -> Result<(), ToolError> {
    let path = ws.resolve(path)?;
    if path.is_dir() { return Err(ToolError::Denied); }
    std::fs::remove_file(path)?;
    Ok(())
}
