//! Operation record tracking (Module 12, part 1).
//!
//! After `execute_commit_plan` runs, SmartCommit records what it
//! did under `.git/smartcommit/` — this is what `restore` later
//! reads to know what to undo, and it's what would let a future
//! `smartcommit history` command (not in this MVP) show past
//! operations.
//!
//! Storage layout, per `ARCHITECTURE.md` §30:
//! ```text
//! .git/smartcommit/
//! ├── last-operation.toml   (always the most recent operation)
//! └── operations/
//!     └── <id>.toml         (one file per operation, kept for history)
//! ```
//!
//! No source code or secrets are stored — only commit hashes,
//! branch names, and timestamps (§30: "Do not store source code
//! unnecessarily").

use crate::{Repository, RepositoryError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// How an operation concluded, per `ARCHITECTURE.md` §31's example
/// (`status = "completed"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationStatus {
    /// Every group's commit was created successfully.
    Completed,
    /// At least one commit was created before a failure stopped the
    /// sequence.
    PartialFailure,
    /// The very first attempted commit failed — nothing was created.
    Failed,
}

/// A record of one `smartcommit commit` run: enough to know exactly
/// what changed and how to undo it, without needing to re-derive
/// anything from the (possibly since-modified) working tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationRecord {
    pub id: String,
    pub start_time: DateTime<Utc>,
    pub original_head: String,
    pub final_head: String,
    pub branch: Option<String>,
    pub checkpoint_ref: String,
    pub created_commits: Vec<String>,
    pub status: OperationStatus,
}

impl OperationRecord {
    /// Builds a record from a completed execution run. `start_time`
    /// is passed in rather than read from the system clock — same
    /// testability rationale used throughout (Modules 8 and 9): a
    /// caller can supply a fixed time for deterministic tests, and
    /// it doubles as the record's `id` (formatted the same way
    /// Module 9's checkpoint ref timestamps are), so operations sort
    /// naturally by filename in the `operations/` directory.
    pub fn from_execution(
        checkpoint: &crate::SafetyCheckpoint,
        report: &crate::ExecutionReport,
        start_time: DateTime<Utc>,
    ) -> Self {
        let status = if report.failure.is_none() {
            OperationStatus::Completed
        } else if report.completed.is_empty() {
            OperationStatus::Failed
        } else {
            OperationStatus::PartialFailure
        };

        let final_head = report
            .completed
            .last()
            .map(|c| c.commit_oid.clone())
            .unwrap_or_else(|| checkpoint.original_head.clone());

        OperationRecord {
            id: start_time.format("%Y%m%dT%H%M%SZ").to_string(),
            start_time,
            original_head: checkpoint.original_head.clone(),
            final_head,
            branch: checkpoint.branch.clone(),
            checkpoint_ref: checkpoint.backup_ref.clone(),
            created_commits: report
                .completed
                .iter()
                .map(|c| c.commit_oid.clone())
                .collect(),
            status,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OperationRecordError {
    #[error(transparent)]
    Repository(#[from] RepositoryError),

    #[error("failed to serialize operation record: {0}")]
    SerializeFailed(String),

    #[error("failed to deserialize operation record: {0}")]
    DeserializeFailed(String),

    #[error("failed to write operation record: {0}")]
    WriteFailed(String),

    #[error("failed to read operation record: {0}")]
    ReadFailed(String),

    #[error("no SmartCommit operation record was found for this repository")]
    NotFound,
}

/// Writes `record` to both `last-operation.toml` (overwritten every
/// time) and `operations/<id>.toml` (kept, one per operation).
pub fn save_operation_record(
    repo_path: impl AsRef<Path>,
    record: &OperationRecord,
) -> Result<(), OperationRecordError> {
    let repo = Repository::discover(repo_path)?;
    let smartcommit_dir = repo.git_dir.join("smartcommit");
    let operations_dir = smartcommit_dir.join("operations");
    std::fs::create_dir_all(&operations_dir)
        .map_err(|e| OperationRecordError::WriteFailed(e.to_string()))?;

    let toml_text = toml::to_string_pretty(record)
        .map_err(|e| OperationRecordError::SerializeFailed(e.to_string()))?;

    std::fs::write(smartcommit_dir.join("last-operation.toml"), &toml_text)
        .map_err(|e| OperationRecordError::WriteFailed(e.to_string()))?;
    std::fs::write(
        operations_dir.join(format!("{}.toml", record.id)),
        &toml_text,
    )
    .map_err(|e| OperationRecordError::WriteFailed(e.to_string()))?;

    Ok(())
}

/// Loads the most recent operation record. This is what `restore`
/// (with no operation ID given) acts on — browsing older entries in
/// `operations/` is not yet exposed by any command in this MVP,
/// though they're written for future use.
pub fn load_last_operation_record(
    repo_path: impl AsRef<Path>,
) -> Result<OperationRecord, OperationRecordError> {
    let repo = Repository::discover(repo_path)?;
    let path = repo.git_dir.join("smartcommit").join("last-operation.toml");

    if !path.exists() {
        return Err(OperationRecordError::NotFound);
    }

    let text = std::fs::read_to_string(&path)
        .map_err(|e| OperationRecordError::ReadFailed(e.to_string()))?;
    toml::from_str(&text).map_err(|e| OperationRecordError::DeserializeFailed(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command as StdCommand;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(label: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "smartcommit-core-operation-record-test-{label}-{}-{n}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("failed to create temp dir");
        dir
    }

    fn git(dir: &Path, args: &[&str]) {
        let status = StdCommand::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_AUTHOR_NAME", "Test User")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "Test User")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .status()
            .unwrap_or_else(|e| panic!("failed to run git {args:?}: {e}"));
        assert!(status.success(), "git {args:?} failed");
    }

    fn init_repo(label: &str) -> PathBuf {
        let dir = temp_dir(label);
        git(&dir, &["init", "--quiet"]);
        fs::write(dir.join("a.txt"), "1\n").unwrap();
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "--quiet", "-m", "init"]);
        dir
    }

    fn sample_record(id: &str) -> OperationRecord {
        OperationRecord {
            id: id.to_string(),
            start_time: "2026-09-01T09:00:00Z".parse().unwrap(),
            original_head: "abc123".to_string(),
            final_head: "def456".to_string(),
            branch: Some("main".to_string()),
            checkpoint_ref: "refs/smartcommit/backups/20260901T090000Z".to_string(),
            created_commits: vec!["def456".to_string()],
            status: OperationStatus::Completed,
        }
    }

    #[test]
    fn round_trips_a_record_through_toml() {
        let dir = init_repo("round-trip");
        let record = sample_record("20260901T090000Z");

        save_operation_record(&dir, &record).expect("save should succeed");
        let loaded = load_last_operation_record(&dir).expect("load should succeed");

        assert_eq!(loaded, record);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn writes_both_last_operation_and_a_history_entry() {
        let dir = init_repo("history-entry");
        let record = sample_record("20260901T090000Z");

        save_operation_record(&dir, &record).expect("save should succeed");

        assert!(dir.join(".git/smartcommit/last-operation.toml").exists());
        assert!(dir
            .join(".git/smartcommit/operations/20260901T090000Z.toml")
            .exists());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn last_operation_is_overwritten_by_a_newer_one() {
        let dir = init_repo("overwritten");
        save_operation_record(&dir, &sample_record("first")).expect("save 1 should succeed");
        save_operation_record(&dir, &sample_record("second")).expect("save 2 should succeed");

        let loaded = load_last_operation_record(&dir).expect("load should succeed");
        assert_eq!(loaded.id, "second");

        // Both history entries must still exist independently.
        assert!(dir.join(".git/smartcommit/operations/first.toml").exists());
        assert!(dir.join(".git/smartcommit/operations/second.toml").exists());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_record_is_reported_as_not_found() {
        let dir = init_repo("missing");
        let err = load_last_operation_record(&dir).unwrap_err();
        assert!(matches!(err, OperationRecordError::NotFound));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn from_execution_derives_completed_status() {
        let checkpoint = crate::SafetyCheckpoint {
            original_head: "orig".to_string(),
            branch: Some("main".to_string()),
            backup_ref: "refs/smartcommit/backups/x".to_string(),
            created_at: Utc::now(),
            repository_state: crate::RepositoryState::default(),
            staged_files: vec![],
        };
        let report = crate::ExecutionReport {
            completed: vec![crate::CompletedCommit {
                message: "A".to_string(),
                commit_oid: "final".to_string(),
            }],
            failure: None,
            not_attempted: vec![],
            checkpoint_backup_ref: checkpoint.backup_ref.clone(),
        };

        let record = OperationRecord::from_execution(&checkpoint, &report, Utc::now());
        assert_eq!(record.status, OperationStatus::Completed);
        assert_eq!(record.final_head, "final");
        assert_eq!(record.created_commits, vec!["final".to_string()]);
    }

    #[test]
    fn from_execution_derives_failed_status_when_nothing_completed() {
        let checkpoint = crate::SafetyCheckpoint {
            original_head: "orig".to_string(),
            branch: Some("main".to_string()),
            backup_ref: "refs/smartcommit/backups/x".to_string(),
            created_at: Utc::now(),
            repository_state: crate::RepositoryState::default(),
            staged_files: vec![],
        };
        let report = crate::ExecutionReport {
            completed: vec![],
            failure: Some(crate::ExecutionFailure {
                group_index: 0,
                message: "A".to_string(),
                reason: "rejected".to_string(),
            }),
            not_attempted: vec![],
            checkpoint_backup_ref: checkpoint.backup_ref.clone(),
        };

        let record = OperationRecord::from_execution(&checkpoint, &report, Utc::now());
        assert_eq!(record.status, OperationStatus::Failed);
        assert_eq!(
            record.final_head, "orig",
            "with nothing completed, final_head should fall back to original_head"
        );
    }

    #[test]
    fn from_execution_derives_partial_failure_status() {
        let checkpoint = crate::SafetyCheckpoint {
            original_head: "orig".to_string(),
            branch: Some("main".to_string()),
            backup_ref: "refs/smartcommit/backups/x".to_string(),
            created_at: Utc::now(),
            repository_state: crate::RepositoryState::default(),
            staged_files: vec![],
        };
        let report = crate::ExecutionReport {
            completed: vec![crate::CompletedCommit {
                message: "A".to_string(),
                commit_oid: "commit-a".to_string(),
            }],
            failure: Some(crate::ExecutionFailure {
                group_index: 1,
                message: "B".to_string(),
                reason: "rejected".to_string(),
            }),
            not_attempted: vec![],
            checkpoint_backup_ref: checkpoint.backup_ref.clone(),
        };

        let record = OperationRecord::from_execution(&checkpoint, &report, Utc::now());
        assert_eq!(record.status, OperationStatus::PartialFailure);
        assert_eq!(record.final_head, "commit-a");
    }
}
