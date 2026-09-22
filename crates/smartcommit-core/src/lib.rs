//! `smartcommit-core` — the Git engine and planning logic for
//! SmartCommit, independent of any CLI or UI.
//!
//! This crate must remain usable without AI/an external LLM, and
//! must not depend on `smartcommit-cli`.

mod checkpoint;
mod config;
mod diff;
mod executor;
mod matcher;
mod message_parser;
mod operation_record;
mod parser;
mod planner;
mod repository;
mod repository_state;
mod restore;
mod staged_changes;
mod timeline;

// AI module (optional, can be disabled if needed)
#[cfg(feature = "ai")]
pub mod ai;

pub use checkpoint::{create_safety_checkpoint, CheckpointError, SafetyCheckpoint};
pub use config::{
    load_config, CommitConfig, ConfigError, MatchingConfig, SmartCommitConfig, TimelineConfig,
};
pub use diff::{get_staged_diffs, DiffError, DiffHunk, DiffLine, FileDiff, StructureInfo, StructureKind};
pub use executor::{
    execute_commit_plan, CompletedCommit, ExecutionError, ExecutionFailure, ExecutionReport,
};
pub use matcher::{match_files_to_messages, MatchScore};
pub use message_parser::{CommitMessage, CommitMessages, ParseCommitMessagesError};
pub use operation_record::{
    load_last_operation_record, save_operation_record, OperationRecord, OperationRecordError,
    OperationStatus,
};
pub use planner::{build_commit_plan, CommitGroup, CommitPlan};
pub use repository::{Repository, RepositoryError, RepositoryInfo};
pub use repository_state::{
    inspect_repository_state, InspectorError, RepositorySafety, RepositoryState,
};
pub use restore::{
    execute_restore, manual_recovery_instructions, plan_restore, RestoreError, RestorePlan,
};
pub use parser::{detect_language, find_structure_at_line, parse_file_structures};
pub use staged_changes::{get_staged_changes, ChangeKind, ChangedFile, StagedChangesError};
pub use timeline::{
    apply_timeline_to_plan, generate_timeline, ApplyTimelineError, PlannedCommitTime, RandomSource,
    SeededRandomSource, SystemRandomSource, TimedCommitGroup, TimedCommitPlan, TimelineError,
    TimelineMode,
};

// AI exports (only available when ai feature is enabled)
#[cfg(feature = "ai")]
pub use ai::{
    analyze_changes_semantically, analyze_hunks, create_provider, create_provider_with_fallback,
    generate_commit_messages, load_ai_config_from_env, validate_ai_config, AIConfig, AIProvider,
    AISemanticAnalysis, ChangeCategory, CombinedResult, DeterministicAnalysis,
    DeterministicCategory, DiffAnalysis, GenerationError, HunkAnalysis, HunkAnalysisError,
    MessageRequest, MessageResponse, ModelSelection, ProviderType, RiskLevel, SemanticAnalysis,
    SemanticAnalysisError, SemanticCategory,
};

// Re-exported so downstream crates (smartcommit-cli and later
// modules) can construct/match on chrono-typed fields (timeline
// timestamps, checkpoint creation time) without adding chrono as
// their own separate dependency.
pub use chrono::{DateTime, Duration, FixedOffset, Utc};
