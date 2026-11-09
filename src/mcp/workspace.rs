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
        let resolved = match std::fs::canonicalize(&candidate) {
            Ok(path) => path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let parent = candidate.parent().ok_or(WorkspaceError::OutsideWorkspace)?;
                let name = candidate.file_name().ok_or(WorkspaceError::OutsideWorkspace)?;
                std::fs::canonicalize(parent)?.join(name)
            }
            Err(error) => return Err(WorkspaceError::Io(error)),
        };
        if !resolved.starts_with(&self.root) {
            return Err(WorkspaceError::OutsideWorkspace);
        }
        Ok(resolved)
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
