use crate::{
    inspect_repository_state, ChangedFile, InspectorError, Repository, RepositoryError,
    RepositoryState,
};
use chrono::{DateTime, Utc};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub struct SafetyCheckpoint {
    pub original_head: String,
    pub branch: Option<String>,
    pub backup_ref: String,
    pub created_at: DateTime<Utc>,
    pub repository_state: RepositoryState,
    pub staged_files: Vec<ChangedFile>,
}

#[derive(Debug, thiserror::Error)]
pub enum CheckpointError {
    #[error(transparent)]
    Repository(#[from] RepositoryError),

    #[error("failed to read repository state: {0}")]
    StateUnavailable(#[from] InspectorError),

    #[error("failed to read staged changes: {0}")]
    StagedChangesUnavailable(#[from] crate::StagedChangesError),

    #[error("repository has no commits yet (HEAD is unborn) — nothing to checkpoint")]
    NoCommits,

    #[error("failed to run git: {0}")]
    GitUnavailable(String),

    #[error("could not find an available checkpoint ref name after {attempts} attempts")]
    NameExhausted { attempts: u32 },

    #[error("failed to create the backup ref: {0}")]
    RefCreationFailed(String),
}

pub fn create_safety_checkpoint(
    path: impl AsRef<Path>,
    now: DateTime<Utc>,
) -> Result<SafetyCheckpoint, CheckpointError> {
    let repo = Repository::discover(&path)?;

    let original_head = read_head(&repo.root)?;
    let branch = read_branch(&repo.root)?;
    let repository_state = inspect_repository_state(&repo.root)?;
    let staged_files = crate::get_staged_changes(&repo.root)?;

    let backup_ref = create_backup_ref(&repo.root, &original_head, now)?;

    Ok(SafetyCheckpoint {
        original_head,
        branch,
        backup_ref,
        created_at: now,
        repository_state,
        staged_files,
    })
}

pub(crate) fn read_head(repo_root: &Path) -> Result<String, CheckpointError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["rev-parse", "--verify", "-q", "HEAD"])
        .output()
        .map_err(|e| CheckpointError::GitUnavailable(e.to_string()))?;

    if !output.status.success() {
        return Err(CheckpointError::NoCommits);
    }

    let head = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if head.is_empty() {
        return Err(CheckpointError::NoCommits);
    }
    Ok(head)
}

pub(crate) fn read_branch(repo_root: &Path) -> Result<Option<String>, CheckpointError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["symbolic-ref", "--short", "-q", "HEAD"])
        .output()
        .map_err(|e| CheckpointError::GitUnavailable(e.to_string()))?;

    if !output.status.success() {
        return Ok(None);
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(if branch.is_empty() {
        None
    } else {
        Some(branch)
    })
}

fn create_backup_ref(
    repo_root: &Path,
    head_oid: &str,
    now: DateTime<Utc>,
) -> Result<String, CheckpointError> {
    let timestamp = now.format("%Y%m%dT%H%M%SZ");

    const MAX_ATTEMPTS: u32 = 1000;
    for attempt in 0..MAX_ATTEMPTS {
        let candidate = if attempt == 0 {
            format!("refs/smc/backups/{timestamp}")
        } else {
            format!("refs/smc/backups/{timestamp}-{attempt}")
        };

        let output = Command::new("git")
            .arg("-C")
            .arg(repo_root)
            .args(["update-ref", "--create-reflog", &candidate, head_oid, ""])
            .output()
            .map_err(|e| CheckpointError::GitUnavailable(e.to_string()))?;

        if output.status.success() {
            return Ok(candidate);
        }

        // Only "already exists" is expected and worth retrying with
        // the next candidate name; any other failure (invalid oid,
        // permissions, corrupt repo, ...) is a real error and should
        // surface immediately rather than being masked by 1000
        // silent retries.
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("already exists") {
            return Err(CheckpointError::RefCreationFailed(
                stderr.trim().to_string(),
            ));
        }
    }

    Err(CheckpointError::NameExhausted {
        attempts: MAX_ATTEMPTS,
    })
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
            "smartcommit-core-checkpoint-test-{label}-{}-{n}",
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

    fn git_output(dir: &Path, args: &[&str]) -> String {
        let output = StdCommand::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap_or_else(|e| panic!("failed to run git {args:?}: {e}"));
        assert!(output.status.success(), "git {args:?} failed");
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn write_file(path: &Path, contents: &str) {
        fs::write(path, contents).expect("failed to write fixture file");
    }

    fn init_repo(label: &str) -> PathBuf {
        let dir = temp_dir(label);
        git(&dir, &["init", "--quiet"]);
        write_file(&dir.join("README.md"), "initial\n");
        git(&dir, &["add", "README.md"]);
        git(&dir, &["commit", "--quiet", "-m", "initial commit"]);
        dir
    }

    fn fixed_now() -> DateTime<Utc> {
        "2026-09-01T09:00:00Z".parse().unwrap()
    }

    #[test]
    fn checkpoint_backup_points_to_correct_head() {
        let dir = init_repo("correct-head");
        let expected_head = git_output(&dir, &["rev-parse", "HEAD"]);

        let checkpoint = create_safety_checkpoint(&dir, fixed_now()).expect("should succeed");

        assert_eq!(checkpoint.original_head, expected_head);
        let ref_points_to = git_output(&dir, &["rev-parse", &checkpoint.backup_ref]);
        assert_eq!(
            ref_points_to, expected_head,
            "the backup ref itself must resolve to the same commit as original_head"
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn checkpoint_captures_branch_and_snapshots() {
        let dir = init_repo("branch-and-snapshots");
        write_file(&dir.join("staged.txt"), "hello\n");
        git(&dir, &["add", "staged.txt"]);

        let checkpoint = create_safety_checkpoint(&dir, fixed_now()).expect("should succeed");

        assert_eq!(checkpoint.branch.as_deref(), Some("master"));
        assert_eq!(checkpoint.staged_files.len(), 1);
        assert_eq!(
            checkpoint.staged_files[0].path,
            std::path::PathBuf::from("staged.txt")
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn checkpoint_detects_detached_head() {
        let dir = init_repo("detached");
        let head = git_output(&dir, &["rev-parse", "HEAD"]);
        git(&dir, &["checkout", "--quiet", "--detach", &head]);

        let checkpoint = create_safety_checkpoint(&dir, fixed_now()).expect("should succeed");

        assert_eq!(checkpoint.branch, None);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn backup_remains_valid_after_further_commits() {
        let dir = init_repo("remains-valid");
        let checkpoint = create_safety_checkpoint(&dir, fixed_now()).expect("should succeed");
        let original_head = checkpoint.original_head.clone();

        // Simulate SmartCommit going on to make new commits after
        // the checkpoint.
        write_file(&dir.join("more.txt"), "more content\n");
        git(&dir, &["add", "more.txt"]);
        git(&dir, &["commit", "--quiet", "-m", "a later commit"]);

        let new_head = git_output(&dir, &["rev-parse", "HEAD"]);
        assert_ne!(new_head, original_head, "HEAD should have moved forward");

        // The checkpoint ref must still exist and still point at the
        // pre-operation commit, unaffected by history moving on.
        let ref_points_to = git_output(&dir, &["rev-parse", &checkpoint.backup_ref]);
        assert_eq!(ref_points_to, original_head);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn failure_on_a_repo_with_no_commits_does_not_modify_the_repository() {
        let dir = temp_dir("no-commits");
        git(&dir, &["init", "--quiet"]);
        // No commits at all — HEAD is unborn.

        let before = fs::read_dir(dir.join(".git"))
            .map(|entries| entries.count())
            .unwrap_or(0);

        let err = create_safety_checkpoint(&dir, fixed_now()).unwrap_err();
        assert!(matches!(err, CheckpointError::NoCommits));

        // No refs/smc ref should have been created.
        let refs_output = StdCommand::new("git")
            .args(["for-each-ref", "refs/smc"])
            .current_dir(&dir)
            .output()
            .expect("failed to run git for-each-ref");
        assert!(
            String::from_utf8_lossy(&refs_output.stdout).is_empty(),
            "no backup ref should exist after a failed checkpoint"
        );

        let after = fs::read_dir(dir.join(".git"))
            .map(|entries| entries.count())
            .unwrap_or(0);
        assert_eq!(
            before, after,
            ".git directory entry count should be unchanged"
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn naming_collision_is_handled_by_suffixing() {
        let dir = init_repo("collision");
        let now = fixed_now();

        // Two checkpoints requested with the exact same timestamp —
        // forces the base name to collide.
        let first = create_safety_checkpoint(&dir, now).expect("first checkpoint should succeed");
        let second =
            create_safety_checkpoint(&dir, now).expect("second checkpoint should also succeed");

        assert_ne!(
            first.backup_ref, second.backup_ref,
            "colliding timestamps must produce distinct ref names"
        );
        assert!(
            first.backup_ref.ends_with("Z"),
            "first ref: {}",
            first.backup_ref
        );
        assert!(
            second.backup_ref.ends_with("-1"),
            "second ref should get a -1 suffix, got: {}",
            second.backup_ref
        );

        // Both refs must independently resolve, and both to the same
        // commit (nothing changed between the two calls).
        let first_points_to = git_output(&dir, &["rev-parse", &first.backup_ref]);
        let second_points_to = git_output(&dir, &["rev-parse", &second.backup_ref]);
        assert_eq!(first_points_to, second_points_to);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn backup_ref_does_not_appear_in_branch_listing() {
        let dir = init_repo("no-branch-clutter");
        create_safety_checkpoint(&dir, fixed_now()).expect("should succeed");

        let branches = git_output(&dir, &["branch", "--list"]);
        assert!(
            !branches.contains("smc"),
            "backup refs must not clutter normal branch listings, got: {branches}"
        );

        fs::remove_dir_all(&dir).ok();
    }
}
