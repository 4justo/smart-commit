//! `smartcommit-core` — the Git engine and planning logic for
//! SmartCommit, independent of any CLI or UI.
//!
//! This crate must remain usable without AI/an external LLM, and
//! must not depend on `smartcommit-cli`.

mod checkpoint;
mod diff;
mod matcher;
mod message_parser;
mod planner;
mod repository;
mod repository_state;
mod staged_changes;
mod timeline;

pub use checkpoint::{create_safety_checkpoint, CheckpointError, SafetyCheckpoint};
pub use diff::{get_staged_diffs, DiffError, DiffHunk, DiffLine, FileDiff};
pub use matcher::{match_files_to_messages, MatchScore};
pub use message_parser::{CommitMessage, CommitMessages, ParseCommitMessagesError};
pub use planner::{build_commit_plan, CommitGroup, CommitPlan};
pub use repository::{Repository, RepositoryError, RepositoryInfo};
pub use repository_state::{
    inspect_repository_state, InspectorError, RepositorySafety, RepositoryState,
};
pub use staged_changes::{get_staged_changes, ChangeKind, ChangedFile, StagedChangesError};
pub use timeline::{
    apply_timeline_to_plan, generate_timeline, ApplyTimelineError, PlannedCommitTime,
    RandomSource, SeededRandomSource, SystemRandomSource, TimedCommitGroup, TimedCommitPlan,
    TimelineError, TimelineMode,
};

// Re-exported so downstream crates (smartcommit-cli and later
// modules) can construct/match on chrono-typed fields (timeline
// timestamps, checkpoint creation time) without adding chrono as
// their own separate dependency.
pub use chrono::{DateTime, Duration, FixedOffset, Utc};
