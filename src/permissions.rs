//! Central execution policy. The UI must never be the security boundary.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    Plan,
    Build,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolAction {
    ReadFile,
    ListDirectory,
    SearchFiles,
    ReadOnlyCommand,
    WriteFile,
    DeleteFile,
    ApplyPatch,
    MutatingCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Approval {
    NotRequired,
    Granted,
    Denied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionDecision {
    Allow,
    RequireApproval,
    Deny,
}

/// Decide whether an operation may execute.
///
/// Commands are deliberately classified by the caller. Patch must not infer
/// that an arbitrary shell command is harmless merely because it was requested
/// while Plan mode is active.
pub fn authorise(
    mode: ExecutionMode,
    action: ToolAction,
    approval: Approval,
) -> PermissionDecision {
    let read_only = matches!(
        action,
        ToolAction::ReadFile
            | ToolAction::ListDirectory
            | ToolAction::SearchFiles
            | ToolAction::ReadOnlyCommand
    );

    if read_only {
        return PermissionDecision::Allow;
    }

    match mode {
        ExecutionMode::Plan => PermissionDecision::Deny,
        ExecutionMode::Build => match approval {
            Approval::Granted => PermissionDecision::Allow,
            Approval::NotRequired => PermissionDecision::RequireApproval,
            Approval::Denied => PermissionDecision::Deny,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_allows_reads() {
        for action in [
            ToolAction::ReadFile,
            ToolAction::ListDirectory,
            ToolAction::SearchFiles,
            ToolAction::ReadOnlyCommand,
        ] {
            assert_eq!(
                authorise(ExecutionMode::Plan, action, Approval::NotRequired),
                PermissionDecision::Allow
            );
        }
    }

    #[test]
    fn plan_never_allows_mutations_even_with_approval() {
        for action in [
            ToolAction::WriteFile,
            ToolAction::DeleteFile,
            ToolAction::ApplyPatch,
            ToolAction::MutatingCommand,
        ] {
            assert_eq!(
                authorise(ExecutionMode::Plan, action, Approval::Granted),
                PermissionDecision::Deny
            );
        }
    }

    #[test]
    fn build_requires_explicit_approval_for_mutations() {
        assert_eq!(
            authorise(
                ExecutionMode::Build,
                ToolAction::WriteFile,
                Approval::NotRequired
            ),
            PermissionDecision::RequireApproval
        );
        assert_eq!(
            authorise(
                ExecutionMode::Build,
                ToolAction::WriteFile,
                Approval::Granted
            ),
            PermissionDecision::Allow
        );
        assert_eq!(
            authorise(
                ExecutionMode::Build,
                ToolAction::WriteFile,
                Approval::Denied
            ),
            PermissionDecision::Deny
        );
    }
}
