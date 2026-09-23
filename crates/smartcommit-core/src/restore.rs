use crate::checkpoint::{read_branch, read_head};
use crate::{
    inspect_repository_state, load_last_operation_record, InspectorError, OperationRecord,
    OperationRecordError, Repository, RepositoryError,
};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub struct RestorePlan {
    pub record: OperationRecord,
    pub current_head: String,
    pub current_branch: Option<String>,
    pub is_safe: bool,
    pub unsafe_reasons: Vec<String>,
    pub working_tree_has_changes: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum RestoreError {
    #[error(transparent)]
    Repository(#[from] RepositoryError),

    #[error(transparent)]
    OperationRecord(#[from] OperationRecordError),

    #[error(transparent)]
    Inspector(#[from] InspectorError),

    #[error("failed to run git: {0}")]
    GitUnavailable(String),

    #[error("refusing to restore: this plan was judged unsafe")]
    RefusedUnsafePlan,

    #[error("failed to move the ref during restore: {0}")]
    RefUpdateFailed(String),
}

pub fn plan_restore(repo_path: impl AsRef<Path>) -> Result<RestorePlan, RestoreError> {
    let repo = Repository::discover(&repo_path)?;
    let record = load_last_operation_record(&repo.root)?;

    let current_head =
        read_head(&repo.root).map_err(|e| RestoreError::GitUnavailable(e.to_string()))?;
    let current_branch =
        read_branch(&repo.root).map_err(|e| RestoreError::GitUnavailable(e.to_string()))?;

    let mut unsafe_reasons = Vec::new();

    if current_head != record.final_head {
        unsafe_reasons.push(format!(
            "current HEAD ({current_head}) does not match this operation's recorded final HEAD ({}) — something else has happened since, and automatic restore would risk losing it",
            record.final_head
        ));
    }

    if current_branch != record.branch {
        unsafe_reasons.push(format!(
            "current branch ({}) does not match the operation's recorded branch ({})",
            current_branch.as_deref().unwrap_or("(detached)"),
            record.branch.as_deref().unwrap_or("(detached)"),
        ));
    }

    match checkpoint_still_valid(&repo.root, &record) {
        Ok(true) => {}
        Ok(false) => unsafe_reasons.push(format!(
            "the checkpoint ref {} no longer points to the recorded original HEAD ({})",
            record.checkpoint_ref, record.original_head
        )),
        Err(e) => unsafe_reasons.push(format!(
            "could not verify the checkpoint ref {}: {e}",
            record.checkpoint_ref
        )),
    }

    let state = inspect_repository_state(&repo.root)?;
    let working_tree_has_changes =
        state.has_staged_changes || state.has_unstaged_changes || state.has_untracked_files;

    Ok(RestorePlan {
        is_safe: unsafe_reasons.is_empty(),
        unsafe_reasons,
        record,
        current_head,
        current_branch,
        working_tree_has_changes,
    })
}

pub fn execute_restore(
    repo_path: impl AsRef<Path>,
    plan: &RestorePlan,
) -> Result<(), RestoreError> {
    if !plan.is_safe {
        return Err(RestoreError::RefusedUnsafePlan);
    }

    let repo = Repository::discover(&repo_path)?;

    let target_ref = match &plan.record.branch {
        Some(branch) => format!("refs/heads/{branch}"),
        None => "HEAD".to_string(),
    };

    let output = Command::new("git")
        .arg("-C")
        .arg(&repo.root)
        .args(["update-ref", &target_ref, &plan.record.original_head])
        .output()
        .map_err(|e| RestoreError::GitUnavailable(e.to_string()))?;

    if !output.status.success() {
        return Err(RestoreError::RefUpdateFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    Ok(())
}

pub fn manual_recovery_instructions(record: &OperationRecord) -> String {
    let target_ref = match &record.branch {
        Some(branch) => format!("refs/heads/{branch}"),
        None => "HEAD".to_string(),
    };
    format!(
        "git update-ref {target_ref} {}\n\nThis moves the branch pointer back to the pre-operation commit \
         without touching your working tree or staged changes. Only run this if you've reviewed the \
         situation above and are sure it's what you want — it does not merge in or preserve any commits \
         made after the SmartCommit operation on this branch.",
        record.original_head
    )
}

fn checkpoint_still_valid(repo_root: &Path, record: &OperationRecord) -> Result<bool, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["rev-parse", "--verify", "-q", &record.checkpoint_ref])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Ok(false);
    }

    let resolved = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(resolved == record.original_head)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        create_safety_checkpoint, execute_commit_plan, save_operation_record, ChangeKind,
        ChangedFile, CommitGroup,
    };
    use chrono::Utc;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command as StdCommand;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(label: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "smartcommit-core-restore-test-{label}-{}-{n}",
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
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn run_a_real_operation(label: &str) -> (PathBuf, String) {
        let dir = temp_dir(label);
        git(&dir, &["init", "--quiet"]);
        write_file(&dir.join("a.txt"), "1\n");
        write_file(&dir.join("b.txt"), "1\n");
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "--quiet", "-m", "init"]);
        let original_head = git_output(&dir, &["rev-parse", "HEAD"]);

        write_file(&dir.join("a.txt"), "2\n");
        write_file(&dir.join("b.txt"), "2\n");
        git(&dir, &["add", "-A"]);

        let now = Utc::now();
        let checkpoint = create_safety_checkpoint(&dir, now).expect("checkpoint should succeed");
        let groups = vec![
            CommitGroup {
                message: "A".to_string(),
                files: vec![ChangedFile {
                    path: PathBuf::from("a.txt"),
                    change_kind: ChangeKind::Modified,
                    old_path: None,
                }],
                confidence: 1.0,
            },
            CommitGroup {
                message: "B".to_string(),
                files: vec![ChangedFile {
                    path: PathBuf::from("b.txt"),
                    change_kind: ChangeKind::Modified,
                    old_path: None,
                }],
                confidence: 1.0,
            },
        ];
        let report = execute_commit_plan(&dir, &groups, None, &checkpoint)
            .expect("execution should succeed");

        let record = OperationRecord::from_execution(&checkpoint, &report, now);
        save_operation_record(&dir, &record).expect("save should succeed");

        (dir, original_head)
    }

    #[test]
    fn successful_restore_moves_head_back_and_preserves_index() {
        let (dir, original_head) = run_a_real_operation("successful");

        let plan = plan_restore(&dir).expect("plan_restore should succeed");
        assert!(plan.is_safe, "reasons: {:?}", plan.unsafe_reasons);

        execute_restore(&dir, &plan).expect("execute_restore should succeed");

        let head_after = git_output(&dir, &["rev-parse", "HEAD"]);
        assert_eq!(head_after, original_head);

        // The real index was never touched throughout the whole
        // operation, so both originally-staged files should be
        // staged again relative to the restored HEAD.
        let staged = git_output(&dir, &["diff", "--cached", "--name-only"]);
        let mut staged_files: Vec<&str> = staged.lines().collect();
        staged_files.sort();
        assert_eq!(staged_files, vec!["a.txt", "b.txt"]);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn operation_metadata_matches_what_actually_happened() {
        let (dir, original_head) = run_a_real_operation("metadata");

        let record = crate::load_last_operation_record(&dir).expect("load should succeed");
        assert_eq!(record.original_head, original_head);
        assert_eq!(record.status, crate::OperationStatus::Completed);
        assert_eq!(record.created_commits.len(), 2);
        assert_eq!(record.branch.as_deref(), Some("master"));

        let head_now = git_output(&dir, &["rev-parse", "HEAD"]);
        assert_eq!(record.final_head, head_now);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn restore_after_multiple_commits_returns_to_the_true_original_head() {
        let (dir, original_head) = run_a_real_operation("multi-commit");

        let log = git_output(&dir, &["log", "--oneline"]);
        assert_eq!(log.lines().count(), 3, "init + 2 new commits, got: {log}");

        let plan = plan_restore(&dir).expect("plan_restore should succeed");
        execute_restore(&dir, &plan).expect("execute_restore should succeed");

        let log_after = git_output(&dir, &["log", "--oneline"]);
        assert_eq!(log_after.lines().count(), 1, "should be back to just init");
        assert_eq!(git_output(&dir, &["rev-parse", "HEAD"]), original_head);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn changed_working_tree_after_operation_is_reported_but_does_not_block_restore() {
        let (dir, _original_head) = run_a_real_operation("changed-working-tree");

        // Simulate the developer editing something (unstaged) after
        // the operation completed.
        write_file(&dir.join("a.txt"), "2\nan extra unstaged line\n");
        write_file(&dir.join("new_untracked.txt"), "untracked\n");

        let plan = plan_restore(&dir).expect("plan_restore should succeed");
        assert!(
            plan.is_safe,
            "an unstaged/untracked change alone must not block restore"
        );
        assert!(plan.working_tree_has_changes);

        execute_restore(&dir, &plan).expect("execute_restore should succeed");

        // The unstaged edit and untracked file must survive restore
        // untouched — restore never touches the working tree.
        assert_eq!(
            fs::read_to_string(dir.join("a.txt")).unwrap(),
            "2\nan extra unstaged line\n"
        );
        assert!(dir.join("new_untracked.txt").exists());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unsafe_restore_is_refused_when_a_later_commit_exists() {
        let (dir, _original_head) = run_a_real_operation("unsafe-later-commit");

        // Something else commits to the branch after the SmartCommit
        // operation finished.
        write_file(&dir.join("c.txt"), "unrelated work\n");
        git(&dir, &["add", "c.txt"]);
        git(
            &dir,
            &["commit", "--quiet", "-m", "an unrelated later commit"],
        );

        let plan = plan_restore(&dir).expect("plan_restore should succeed");
        assert!(!plan.is_safe);
        assert!(plan
            .unsafe_reasons
            .iter()
            .any(|r| r.contains("does not match this operation's recorded final HEAD")));

        let err = execute_restore(&dir, &plan).unwrap_err();
        assert!(matches!(err, RestoreError::RefusedUnsafePlan));

        // HEAD must be completely unaffected by the refused attempt.
        let log = git_output(&dir, &["log", "--oneline"]);
        assert_eq!(
            log.lines().count(),
            4,
            "init + 2 operation commits + 1 unrelated, got: {log}"
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn manual_instructions_are_provided_for_an_unsafe_plan() {
        let (dir, _original_head) = run_a_real_operation("manual-instructions");
        write_file(&dir.join("c.txt"), "unrelated\n");
        git(&dir, &["add", "c.txt"]);
        git(&dir, &["commit", "--quiet", "-m", "unrelated"]);

        let plan = plan_restore(&dir).expect("plan_restore should succeed");
        assert!(!plan.is_safe);

        let instructions = manual_recovery_instructions(&plan.record);
        assert!(instructions.contains("git update-ref"));
        assert!(instructions.contains(&plan.record.original_head));

        fs::remove_dir_all(&dir).ok();
    }
}
