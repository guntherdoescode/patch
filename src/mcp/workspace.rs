use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Workspace {
    root: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("workspace root is not a directory")]
    NotDirectory,
    #[error("path is outside the active workspace")]
    OutsideWorkspace,
    #[error("path is invalid or cannot be resolved: {0}")]
    InvalidPath(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl Workspace {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, WorkspaceError> {
        let root = std::fs::canonicalize(root)?;
        if !root.is_dir() { return Err(WorkspaceError::NotDirectory); }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path { &self.root }

    /// Resolve a user/model-supplied path and reject traversal and symlink escapes.
    /// For a new file, canonicalise the existing parent before appending its name.
    pub fn resolve(&self, input: impl AsRef<Path>) -> Result<PathBuf, WorkspaceError> {
        let input = input.as_ref();
        if input.components().any(|c| matches!(c, Component::ParentDir)) {
            return Err(WorkspaceError::OutsideWorkspace);
        }
        let candidate = if input.is_absolute() { input.to_path_buf() } else { self.root.join(input) };
        let resolved = canonicalize_allow_missing(&candidate)?;
        if !resolved.starts_with(&self.root) {
            return Err(WorkspaceError::OutsideWorkspace);
        }
        Ok(resolved)
    }

    /// Resolve a path after an explicit external-access grant. Canonicalisation
    /// still occurs before the caller touches the target.
    pub fn resolve_with_external(&self, input: impl AsRef<Path>, allow_external: bool) -> Result<PathBuf, WorkspaceError> {
        let input = input.as_ref();
        if !allow_external { return self.resolve(input); }
        let candidate = if input.is_absolute() { input.to_path_buf() } else { self.root.join(input) };
        canonicalize_allow_missing(&candidate)
    }

    pub fn contains(&self, path: impl AsRef<Path>) -> bool {
        self.resolve(path).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_parent_traversal() {
        let root = std::env::current_dir().unwrap();
        let ws = Workspace::open(root).unwrap();
        assert!(matches!(ws.resolve("../outside"), Err(WorkspaceError::OutsideWorkspace)));
    }
}

fn canonicalize_allow_missing(candidate: &Path) -> Result<PathBuf, WorkspaceError> {
    let mut ancestor = candidate;
    let mut suffix = Vec::new();
    loop {
        match std::fs::canonicalize(ancestor) {
            Ok(mut resolved) => {
                for component in suffix.iter().rev() { resolved.push(component); }
                return Ok(resolved);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let name = ancestor.file_name().ok_or(WorkspaceError::OutsideWorkspace)?;
                suffix.push(name.to_os_string());
                ancestor = ancestor.parent().ok_or(WorkspaceError::OutsideWorkspace)?;
            }
            Err(error) => return Err(WorkspaceError::Io(error)),
        }
    }
}
