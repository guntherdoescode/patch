use super::workspace::{Workspace, WorkspaceError};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode { Plan, Build }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation { Read, CommandReadOnly, Write, Delete, Execute }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessDecision { Allow, NeedsExternalApproval, Deny }

/// Central, deny-by-default policy. Plan is read-only. Build may mutate inside
/// the active workspace. Anything outside it requires an explicit UI grant.
pub fn authorise(workspace: &Workspace, mode: ExecutionMode, operation: Operation, path: Option<&Path>, external_granted: bool) -> AccessDecision {
    if mode == ExecutionMode::Plan && !matches!(operation, Operation::Read | Operation::CommandReadOnly) {
        return AccessDecision::Deny;
    }
    if let Some(path) = path {
        match workspace.resolve(path) {
            Ok(_) => AccessDecision::Allow,
            Err(WorkspaceError::OutsideWorkspace) if external_granted => AccessDecision::Allow,
            Err(WorkspaceError::OutsideWorkspace) => AccessDecision::NeedsExternalApproval,
            Err(_) => AccessDecision::Deny,
        }
    } else if operation == Operation::Execute && mode == ExecutionMode::Plan {
        AccessDecision::Deny
    } else {
        AccessDecision::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plan_denies_mutation_even_if_external_grant_exists() {
        let ws = Workspace::open(std::env::current_dir().unwrap()).unwrap();
        assert_eq!(authorise(&ws, ExecutionMode::Plan, Operation::Write, Some(Path::new("x")), true), AccessDecision::Deny);
    }
    #[test]
    fn build_allows_in_tree_write() {
        let ws = Workspace::open(std::env::current_dir().unwrap()).unwrap();
        assert_eq!(authorise(&ws, ExecutionMode::Build, Operation::Write, Some(Path::new("Cargo.toml")), false), AccessDecision::Allow);
    }
}
