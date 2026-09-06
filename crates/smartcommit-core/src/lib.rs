//! `smartcommit-core` — the Git engine and planning logic for
//! SmartCommit, independent of any CLI or UI.
//!
//! This crate must remain usable without AI/an external LLM, and
//! must not depend on `smartcommit-cli`.

mod repository;
mod repository_state;
mod staged_changes;

pub use repository::{Repository, RepositoryError, RepositoryInfo};
pub use repository_state::{
    inspect_repository_state, InspectorError, RepositorySafety, RepositoryState,
};
pub use staged_changes::{get_staged_changes, ChangeKind, ChangedFile, StagedChangesError};

/// Placeholder for one parsed, developer-supplied commit message.
/// Fields land in Module 5 — Multiline Commit Message Parser.
pub struct CommitMessage;

/// Placeholder for a proposed set of commits (groups + unassigned
/// files). Fields land in Module 7 — Commit Planner.
pub struct CommitPlan;

#[cfg(test)]
mod tests {
    // Compile-time smoke test: the remaining placeholder types exist
    // and are constructible. RepositoryInfo, RepositoryState, and
    // ChangedFile are no longer placeholders — see their own modules
    // for tests.
    use super::*;

    #[test]
    fn placeholder_types_are_constructible() {
        let _ = CommitMessage;
        let _ = CommitPlan;
    }
}

