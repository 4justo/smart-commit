//! `smartcommit-core` — the Git engine and planning logic for
//! SmartCommit, independent of any CLI or UI.
//!
//! This crate must remain usable without AI/an external LLM, and
//! must not depend on `smartcommit-cli`.

mod diff;
mod matcher;
mod message_parser;
mod repository;
mod repository_state;
mod staged_changes;

pub use diff::{get_staged_diffs, DiffError, DiffHunk, DiffLine, FileDiff};
pub use matcher::{match_files_to_messages, MatchScore};
pub use message_parser::{CommitMessage, CommitMessages, ParseCommitMessagesError};
pub use repository::{Repository, RepositoryError, RepositoryInfo};
pub use repository_state::{
    inspect_repository_state, InspectorError, RepositorySafety, RepositoryState,
};
pub use staged_changes::{get_staged_changes, ChangeKind, ChangedFile, StagedChangesError};

/// Placeholder for a proposed set of commits (groups + unassigned
/// files). Fields land in Module 7 — Commit Planner.
pub struct CommitPlan;

#[cfg(test)]
mod tests {
    // Compile-time smoke test: the remaining placeholder type exists
    // and is constructible. Everything else has its own tests in its
    // own module now.
    use super::*;

    #[test]
    fn placeholder_types_are_constructible() {
        let _ = CommitPlan;
    }
}
