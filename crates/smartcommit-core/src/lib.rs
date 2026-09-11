//! `smartcommit-core` — the Git engine and planning logic for
//! SmartCommit, independent of any CLI or UI.
//!
//! This crate must remain usable without AI/an external LLM, and
//! must not depend on `smartcommit-cli`.

mod diff;
mod matcher;
mod message_parser;
mod planner;
mod repository;
mod repository_state;
mod staged_changes;

pub use diff::{get_staged_diffs, DiffError, DiffHunk, DiffLine, FileDiff};
pub use matcher::{match_files_to_messages, MatchScore};
pub use message_parser::{CommitMessage, CommitMessages, ParseCommitMessagesError};
pub use planner::{build_commit_plan, CommitGroup, CommitPlan};
pub use repository::{Repository, RepositoryError, RepositoryInfo};
pub use repository_state::{
    inspect_repository_state, InspectorError, RepositorySafety, RepositoryState,
};
pub use staged_changes::{get_staged_changes, ChangeKind, ChangedFile, StagedChangesError};
