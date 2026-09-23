use crate::{
    ChangeKind, ChangedFile, CommitGroup, PlannedCommitTime, Repository, RepositoryError,
    SafetyCheckpoint,
};
use chrono::{DateTime, FixedOffset};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedCommit {
    pub message: String,
    pub commit_oid: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionFailure {
    pub group_index: usize,
    pub message: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionReport {
    pub completed: Vec<CompletedCommit>,
    pub failure: Option<ExecutionFailure>,
    pub not_attempted: Vec<String>,
    pub checkpoint_backup_ref: String,
}

impl ExecutionReport {
    pub fn is_full_success(&self) -> bool {
        self.failure.is_none()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error(transparent)]
    Repository(#[from] RepositoryError),

    #[error("failed to run git: {0}")]
    GitUnavailable(String),

    #[error("failed to read the currently staged index: {0}")]
    IndexReadFailed(String),

    #[error(
        "repository HEAD has moved since the safety checkpoint was created (checkpoint expected {expected}, found {actual}); refusing to proceed"
    )]
    HeadMoved { expected: String, actual: String },

    #[error("could not prepare a working area for commit construction: {0}")]
    TempAreaUnavailable(String),
}

pub fn execute_commit_plan(
    repo_path: impl AsRef<Path>,
    groups: &[CommitGroup],
    timestamps: Option<&[PlannedCommitTime]>,
    checkpoint: &SafetyCheckpoint,
) -> Result<ExecutionReport, ExecutionError> {
    let repo = Repository::discover(&repo_path)?;

    let current_head = run_git_capture(&repo.root, &["rev-parse", "HEAD"], None)
        .map_err(ExecutionError::GitUnavailable)?;
    if current_head != checkpoint.original_head {
        return Err(ExecutionError::HeadMoved {
            expected: checkpoint.original_head.clone(),
            actual: current_head,
        });
    }

    // Captured once, before any temp-index work begins: the blob
    // and mode for every currently-staged path, exactly as the user
    // originally staged it. This is the content every commit in the
    // sequence draws from — never re-read from the working tree,
    // which is what keeps unstaged edits (even ones stacked on top
    // of a staged file) completely out of scope.
    let staged_entries = read_staged_index_entries(&repo.root)?;

    let temp_area = TempIndexArea::new(&repo.git_dir)
        .map_err(|e| ExecutionError::TempAreaUnavailable(e.to_string()))?;

    let mut completed = Vec::new();
    let mut current_parent = checkpoint.original_head.clone();
    let mut failure: Option<ExecutionFailure> = None;

    for (group_index, group) in groups.iter().enumerate() {
        if group.files.is_empty() {
            continue;
        }

        let timestamp = timestamps
            .and_then(|ts| ts.iter().find(|t| t.commit_index == group_index))
            .map(|t| t.timestamp);

        let index_path = temp_area.index_path_for(group_index);

        if let Err(reason) = prepare_temp_index(
            &repo.root,
            &index_path,
            &current_parent,
            group,
            &staged_entries,
        ) {
            failure = Some(ExecutionFailure {
                group_index,
                message: group.message.clone(),
                reason,
            });
            break;
        }

        if let Err(reason) = create_commit(&repo.root, &index_path, &group.message, timestamp) {
            failure = Some(ExecutionFailure {
                group_index,
                message: group.message.clone(),
                reason,
            });
            break;
        }

        // VERIFY: the commit must have actually moved HEAD. Reading
        // via the real index/HEAD (no GIT_INDEX_FILE override) here
        // is deliberate — this confirms what git itself now
        // considers the branch's real state, not just what our temp
        // index thinks happened.
        let new_head = match run_git_capture(&repo.root, &["rev-parse", "HEAD"], None) {
            Ok(h) => h,
            Err(e) => {
                failure = Some(ExecutionFailure {
                    group_index,
                    message: group.message.clone(),
                    reason: format!("could not verify the new commit: {e}"),
                });
                break;
            }
        };
        if new_head == current_parent {
            failure = Some(ExecutionFailure {
                group_index,
                message: group.message.clone(),
                reason: "commit reported success but HEAD did not advance".to_string(),
            });
            break;
        }

        completed.push(CompletedCommit {
            message: group.message.clone(),
            commit_oid: new_head.clone(),
        });
        current_parent = new_head;
    }

    let not_attempted = match &failure {
        Some(f) => groups[(f.group_index + 1)..]
            .iter()
            .map(|g| g.message.clone())
            .collect(),
        None => Vec::new(),
    };

    Ok(ExecutionReport {
        completed,
        failure,
        not_attempted,
        checkpoint_backup_ref: checkpoint.backup_ref.clone(),
    })
}

struct StagedEntry {
    mode: String,
    blob: String,
}

fn read_staged_index_entries(
    repo_root: &Path,
) -> Result<HashMap<PathBuf, StagedEntry>, ExecutionError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["ls-files", "--stage"])
        .output()
        .map_err(|e| ExecutionError::GitUnavailable(e.to_string()))?;

    if !output.status.success() {
        return Err(ExecutionError::IndexReadFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut map = HashMap::new();
    for line in stdout.lines() {
        if let Some((meta, path)) = line.split_once('\t') {
            let mut parts = meta.split_whitespace();
            if let (Some(mode), Some(blob)) = (parts.next(), parts.next()) {
                map.insert(
                    PathBuf::from(path),
                    StagedEntry {
                        mode: mode.to_string(),
                        blob: blob.to_string(),
                    },
                );
            }
        }
    }
    Ok(map)
}

struct TempIndexArea {
    dir: PathBuf,
}

impl TempIndexArea {
    fn new(git_dir: &Path) -> std::io::Result<Self> {
        let dir = git_dir
            .join("smartcommit")
            .join("tmp")
            .join(format!("exec-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    fn index_path_for(&self, group_index: usize) -> PathBuf {
        self.dir.join(format!("index-{group_index}"))
    }
}

impl Drop for TempIndexArea {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn prepare_temp_index(
    repo_root: &Path,
    index_path: &Path,
    parent: &str,
    group: &CommitGroup,
    staged_entries: &HashMap<PathBuf, StagedEntry>,
) -> Result<(), String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .env("GIT_INDEX_FILE", index_path)
        .args(["read-tree", parent])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "failed to prepare index from parent tree: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    for file in &group.files {
        match file.change_kind {
            ChangeKind::Deleted => {
                remove_path(repo_root, index_path, &file.path)?;
            }
            ChangeKind::Renamed => {
                if let Some(old) = &file.old_path {
                    remove_path(repo_root, index_path, old)?;
                }
                add_path(repo_root, index_path, file, staged_entries)?;
            }
            ChangeKind::Added | ChangeKind::Modified => {
                add_path(repo_root, index_path, file, staged_entries)?;
            }
        }
    }

    Ok(())
}

fn add_path(
    repo_root: &Path,
    index_path: &Path,
    file: &ChangedFile,
    staged_entries: &HashMap<PathBuf, StagedEntry>,
) -> Result<(), String> {
    let entry = staged_entries.get(&file.path).ok_or_else(|| {
        format!(
            "\"{}\" is not present in the currently staged index — the plan may be stale (staged changes were modified after the plan was built)",
            file.path.display()
        )
    })?;

    // Three separate arguments (mode, object, path) rather than the
    // comma-joined single-argument form — sidesteps any ambiguity
    // for a path that happens to contain a literal comma. Verified
    // this form works correctly, including with such a path, before
    // relying on it.
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .env("GIT_INDEX_FILE", index_path)
        .args([
            "update-index",
            "--add",
            "--cacheinfo",
            &entry.mode,
            &entry.blob,
        ])
        .arg(&file.path)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "failed to stage \"{}\": {}",
            file.path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

fn remove_path(repo_root: &Path, index_path: &Path, path: &Path) -> Result<(), String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .env("GIT_INDEX_FILE", index_path)
        .args(["update-index", "--force-remove"])
        .arg(path)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "failed to remove \"{}\": {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

fn create_commit(
    repo_root: &Path,
    index_path: &Path,
    message: &str,
    timestamp: Option<DateTime<FixedOffset>>,
) -> Result<(), String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(repo_root)
        .env("GIT_INDEX_FILE", index_path)
        .args(["commit", "-m", message]);

    if let Some(ts) = timestamp {
        // Both env vars are required: `--date` alone (the porcelain
        // flag) only sets the *author* date and leaves the committer
        // date at the real current time — confirmed empirically.
        let rfc3339 = ts.to_rfc3339();
        cmd.env("GIT_AUTHOR_DATE", &rfc3339);
        cmd.env("GIT_COMMITTER_DATE", &rfc3339);
    }

    let output = cmd.output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // A hook rejection typically writes its message to stderr,
        // but some hooks write to stdout — include whichever is
        // non-empty (preferring stderr) rather than silently
        // dropping a hook's actual rejection reason.
        return Err(if !stderr.is_empty() { stderr } else { stdout });
    }
    Ok(())
}

fn run_git_capture(
    repo_root: &Path,
    args: &[&str],
    index_file: Option<&Path>,
) -> Result<String, String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(repo_root).args(args);
    if let Some(idx) = index_file {
        cmd.env("GIT_INDEX_FILE", idx);
    }
    let output = cmd.output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create_safety_checkpoint;
    use std::fs;
    use std::process::Command as StdCommand;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(label: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "smartcommit-core-executor-test-{label}-{}-{n}",
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
            fs::create_dir_all(parent).expect("failed to create fixture parent dir");
        }
        fs::write(path, contents).expect("failed to write fixture file");
    }

    fn checkpoint_for(dir: &Path) -> SafetyCheckpoint {
        let now: DateTime<chrono::Utc> = "2026-09-01T09:00:00Z".parse().unwrap();
        create_safety_checkpoint(dir, now).expect("checkpoint creation should succeed")
    }

    fn changed(path: &str, kind: ChangeKind) -> ChangedFile {
        ChangedFile {
            path: PathBuf::from(path),
            change_kind: kind,
            old_path: None,
        }
    }

    fn group(message: &str, files: Vec<ChangedFile>) -> CommitGroup {
        CommitGroup {
            message: message.to_string(),
            files,
            confidence: 1.0,
        }
    }

    #[test]
    fn two_commits_from_two_file_groups() {
        let dir = temp_dir("two-groups");
        git(&dir, &["init", "--quiet"]);
        write_file(&dir.join("a.txt"), "a1\n");
        write_file(&dir.join("b.txt"), "b1\n");
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "--quiet", "-m", "init"]);

        write_file(&dir.join("a.txt"), "a2\n");
        write_file(&dir.join("b.txt"), "b2\n");
        git(&dir, &["add", "-A"]);

        let checkpoint = checkpoint_for(&dir);
        let groups = vec![
            group("Update a", vec![changed("a.txt", ChangeKind::Modified)]),
            group("Update b", vec![changed("b.txt", ChangeKind::Modified)]),
        ];

        let report = execute_commit_plan(&dir, &groups, None, &checkpoint).expect("should succeed");

        assert!(report.is_full_success());
        assert_eq!(report.completed.len(), 2);
        assert_eq!(report.completed[0].message, "Update a");
        assert_eq!(report.completed[1].message, "Update b");

        let log = git_output(&dir, &["log", "--oneline", "--reverse"]);
        assert_eq!(log.lines().count(), 3, "init + 2 new commits, got: {log}");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn three_commits_in_sequence() {
        let dir = temp_dir("three-groups");
        git(&dir, &["init", "--quiet"]);
        write_file(&dir.join("a.txt"), "1\n");
        write_file(&dir.join("b.txt"), "1\n");
        write_file(&dir.join("c.txt"), "1\n");
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "--quiet", "-m", "init"]);

        write_file(&dir.join("a.txt"), "2\n");
        write_file(&dir.join("b.txt"), "2\n");
        write_file(&dir.join("c.txt"), "2\n");
        git(&dir, &["add", "-A"]);

        let checkpoint = checkpoint_for(&dir);
        let groups = vec![
            group("A", vec![changed("a.txt", ChangeKind::Modified)]),
            group("B", vec![changed("b.txt", ChangeKind::Modified)]),
            group("C", vec![changed("c.txt", ChangeKind::Modified)]),
        ];

        let report = execute_commit_plan(&dir, &groups, None, &checkpoint).expect("should succeed");

        assert!(report.is_full_success());
        assert_eq!(report.completed.len(), 3);

        // Each commit's tree must contain exactly the intended
        // content up to that point — b.txt and c.txt still "1" in
        // commit A's tree, only a.txt updated.
        let tree_after_a = git_output(
            &dir,
            &["show", &format!("{}:b.txt", report.completed[0].commit_oid)],
        );
        assert_eq!(tree_after_a, "1");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn applies_planned_timestamps() {
        let dir = temp_dir("timestamps");
        git(&dir, &["init", "--quiet"]);
        write_file(&dir.join("a.txt"), "1\n");
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "--quiet", "-m", "init"]);
        write_file(&dir.join("a.txt"), "2\n");
        git(&dir, &["add", "-A"]);

        let checkpoint = checkpoint_for(&dir);
        let groups = vec![group(
            "Update a",
            vec![changed("a.txt", ChangeKind::Modified)],
        )];
        let ts: DateTime<FixedOffset> = "2026-03-01T09:00:00+00:00".parse().unwrap();
        let timestamps = vec![PlannedCommitTime {
            commit_index: 0,
            timestamp: ts,
        }];

        let report = execute_commit_plan(&dir, &groups, Some(&timestamps), &checkpoint)
            .expect("should succeed");

        let commit_oid = &report.completed[0].commit_oid;
        let author_date = git_output(
            &dir,
            &[
                "show",
                "-s",
                "--format=%ad",
                "--date=iso-strict",
                commit_oid,
            ],
        );
        let committer_date = git_output(
            &dir,
            &[
                "show",
                "-s",
                "--format=%cd",
                "--date=iso-strict",
                commit_oid,
            ],
        );
        // Compare parsed instants, not raw strings: different git
        // versions render a zero UTC offset differently ("Z" vs
        // "+00:00" — both valid ISO 8601 for the same instant), so a
        // string comparison is version-fragile in a way the actual
        // behavior isn't. This was caught by a real test failure on
        // a newer git version than the sandbox's, not anticipated
        // upfront.
        let parsed_author: DateTime<FixedOffset> = author_date
            .parse()
            .expect("author date should be valid RFC3339/ISO8601");
        let parsed_committer: DateTime<FixedOffset> = committer_date
            .parse()
            .expect("committer date should be valid RFC3339/ISO8601");
        assert_eq!(parsed_author, ts);
        assert_eq!(parsed_committer, ts);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn preserves_unstaged_changes_including_ones_stacked_on_a_staged_file() {
        let dir = temp_dir("preserve-unstaged");
        git(&dir, &["init", "--quiet"]);
        write_file(&dir.join("a.txt"), "1\n");
        write_file(&dir.join("c.txt"), "1\n");
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "--quiet", "-m", "init"]);

        // a.txt: staged change, then an additional UNstaged edit on
        // top of it.
        write_file(&dir.join("a.txt"), "1\n2-staged\n");
        git(&dir, &["add", "a.txt"]);
        write_file(&dir.join("a.txt"), "1\n2-staged\n3-unstaged\n");
        // c.txt: unstaged-only edit, never part of the plan at all.
        write_file(&dir.join("c.txt"), "1\nunstaged-only\n");

        let checkpoint = checkpoint_for(&dir);
        let groups = vec![group(
            "Update a",
            vec![changed("a.txt", ChangeKind::Modified)],
        )];

        execute_commit_plan(&dir, &groups, None, &checkpoint).expect("should succeed");

        let a_on_disk = fs::read_to_string(dir.join("a.txt")).unwrap();
        assert_eq!(a_on_disk, "1\n2-staged\n3-unstaged\n");
        let c_on_disk = fs::read_to_string(dir.join("c.txt")).unwrap();
        assert_eq!(c_on_disk, "1\nunstaged-only\n");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn staged_content_is_correct_in_the_resulting_commit() {
        let dir = temp_dir("staged-content-correct");
        git(&dir, &["init", "--quiet"]);
        write_file(&dir.join("a.txt"), "1\n");
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "--quiet", "-m", "init"]);

        // Staged content differs from what's later on disk — the
        // commit must reflect the STAGED version, not the working
        // tree at execution time.
        write_file(&dir.join("a.txt"), "staged-version\n");
        git(&dir, &["add", "a.txt"]);
        write_file(&dir.join("a.txt"), "working-tree-version-after-staging\n");

        let checkpoint = checkpoint_for(&dir);
        let groups = vec![group(
            "Update a",
            vec![changed("a.txt", ChangeKind::Modified)],
        )];

        let report = execute_commit_plan(&dir, &groups, None, &checkpoint).expect("should succeed");

        let committed_content = git_output(
            &dir,
            &["show", &format!("{}:a.txt", report.completed[0].commit_oid)],
        );
        assert_eq!(committed_content, "staged-version");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn failure_midway_reports_completed_and_not_attempted_and_stops() {
        let dir = temp_dir("failure-midway");
        git(&dir, &["init", "--quiet"]);
        write_file(&dir.join("a.txt"), "1\n");
        write_file(&dir.join("b.txt"), "1\n");
        write_file(&dir.join("c.txt"), "1\n");
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "--quiet", "-m", "init"]);

        write_file(&dir.join("a.txt"), "2\n");
        write_file(&dir.join("b.txt"), "2\n");
        write_file(&dir.join("c.txt"), "2\n");
        git(&dir, &["add", "-A"]);

        let checkpoint = checkpoint_for(&dir);
        // "b.txt" is deliberately NOT in the staged-entries map's
        // reality: we describe a group referencing a path that was
        // never staged, forcing add_path's lookup to fail — this is
        // the module's own internal consistency check, exercised
        // directly without needing to install a real Git hook.
        let groups = vec![
            group("A", vec![changed("a.txt", ChangeKind::Modified)]),
            group(
                "Bad group",
                vec![changed("does_not_exist.txt", ChangeKind::Modified)],
            ),
            group("C", vec![changed("c.txt", ChangeKind::Modified)]),
        ];

        let report = execute_commit_plan(&dir, &groups, None, &checkpoint).expect("should succeed");

        assert!(!report.is_full_success());
        assert_eq!(report.completed.len(), 1);
        assert_eq!(report.completed[0].message, "A");
        let failure = report.failure.expect("should have a failure");
        assert_eq!(failure.group_index, 1);
        assert_eq!(failure.message, "Bad group");
        assert!(failure.reason.contains("does_not_exist.txt"));
        assert_eq!(report.not_attempted, vec!["C".to_string()]);

        // HEAD must have advanced exactly once (commit A), not moved
        // further.
        let log = git_output(&dir, &["log", "--oneline", "--reverse"]);
        assert_eq!(log.lines().count(), 2, "init + commit A only, got: {log}");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn hook_rejection_is_reported_as_a_failure_and_head_does_not_move() {
        let dir = temp_dir("hook-rejection");
        git(&dir, &["init", "--quiet"]);
        write_file(&dir.join("a.txt"), "1\n");
        write_file(&dir.join("b.txt"), "1\n");
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "--quiet", "-m", "init"]);

        write_file(&dir.join("a.txt"), "2\n");
        write_file(&dir.join("b.txt"), "2\n");
        git(&dir, &["add", "-A"]);

        // A hook that rejects any commit touching b.txt.
        let hooks_dir = dir.join(".git").join("hooks");
        fs::create_dir_all(&hooks_dir).unwrap();
        let hook_path = hooks_dir.join("pre-commit");
        write_file(
            &hook_path,
            "#!/bin/sh\nif git diff --cached --name-only | grep -q b.txt; then\n  echo 'REJECTED: b.txt is blocked' >&2\n  exit 1\nfi\nexit 0\n",
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&hook_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&hook_path, perms).unwrap();
        }

        let checkpoint = checkpoint_for(&dir);
        let groups = vec![
            group("Update a", vec![changed("a.txt", ChangeKind::Modified)]),
            group("Update b", vec![changed("b.txt", ChangeKind::Modified)]),
        ];

        let report = execute_commit_plan(&dir, &groups, None, &checkpoint).expect("should succeed");

        assert_eq!(report.completed.len(), 1);
        assert_eq!(report.completed[0].message, "Update a");
        let failure = report.failure.expect("should have a failure");
        assert_eq!(failure.group_index, 1);
        assert!(
            failure.reason.contains("REJECTED"),
            "expected the hook's own message in the failure reason, got: {}",
            failure.reason
        );

        let head = git_output(&dir, &["rev-parse", "HEAD"]);
        assert_eq!(
            head, report.completed[0].commit_oid,
            "HEAD must not have moved past the successful commit"
        );

        // The real index must still show b.txt staged — the
        // rejected commit must not have consumed it.
        let index = git_output(&dir, &["diff", "--cached", "--name-only"]);
        assert_eq!(index, "b.txt");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn handles_deleted_files() {
        let dir = temp_dir("deleted-files");
        git(&dir, &["init", "--quiet"]);
        write_file(&dir.join("gone.txt"), "bye\n");
        write_file(&dir.join("keep.txt"), "keep\n");
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "--quiet", "-m", "init"]);

        git(&dir, &["rm", "--quiet", "gone.txt"]);

        let checkpoint = checkpoint_for(&dir);
        let groups = vec![group(
            "Remove gone.txt",
            vec![changed("gone.txt", ChangeKind::Deleted)],
        )];

        let report = execute_commit_plan(&dir, &groups, None, &checkpoint).expect("should succeed");

        assert!(report.is_full_success());
        let tree = git_output(
            &dir,
            &["ls-tree", "--name-only", &report.completed[0].commit_oid],
        );
        assert_eq!(
            tree, "keep.txt",
            "gone.txt must be absent from the resulting tree"
        );
        assert!(dir.join("keep.txt").exists());
        assert!(
            !dir.join("gone.txt").exists(),
            "gone.txt was already removed from the working tree by `git rm`, before the executor ran"
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn handles_added_files() {
        let dir = temp_dir("added-files");
        git(&dir, &["init", "--quiet"]);
        write_file(&dir.join("README.md"), "hello\n");
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "--quiet", "-m", "init"]);

        write_file(&dir.join("new.txt"), "brand new\n");
        git(&dir, &["add", "-A"]);

        let checkpoint = checkpoint_for(&dir);
        let groups = vec![group(
            "Add new.txt",
            vec![changed("new.txt", ChangeKind::Added)],
        )];

        let report = execute_commit_plan(&dir, &groups, None, &checkpoint).expect("should succeed");

        assert!(report.is_full_success());
        let content = git_output(
            &dir,
            &[
                "show",
                &format!("{}:new.txt", report.completed[0].commit_oid),
            ],
        );
        assert_eq!(content, "brand new");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn empty_groups_are_skipped_without_being_reported_as_failures() {
        let dir = temp_dir("empty-group");
        git(&dir, &["init", "--quiet"]);
        write_file(&dir.join("a.txt"), "1\n");
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "--quiet", "-m", "init"]);
        write_file(&dir.join("a.txt"), "2\n");
        git(&dir, &["add", "-A"]);

        let checkpoint = checkpoint_for(&dir);
        let groups = vec![
            group("Empty message", vec![]),
            group("Update a", vec![changed("a.txt", ChangeKind::Modified)]),
        ];

        let report = execute_commit_plan(&dir, &groups, None, &checkpoint).expect("should succeed");

        assert!(report.is_full_success());
        assert_eq!(report.completed.len(), 1);
        assert_eq!(report.completed[0].message, "Update a");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn refuses_to_run_if_head_moved_since_the_checkpoint() {
        let dir = temp_dir("head-moved");
        git(&dir, &["init", "--quiet"]);
        write_file(&dir.join("a.txt"), "1\n");
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "--quiet", "-m", "init"]);
        write_file(&dir.join("a.txt"), "2\n");
        git(&dir, &["add", "-A"]);

        let checkpoint = checkpoint_for(&dir);

        // Something else advances HEAD after the checkpoint was taken.
        write_file(&dir.join("b.txt"), "unexpected\n");
        git(&dir, &["add", "b.txt"]);
        git(
            &dir,
            &[
                "commit",
                "--quiet",
                "-m",
                "an unexpected intervening commit",
            ],
        );

        let groups = vec![group(
            "Update a",
            vec![changed("a.txt", ChangeKind::Modified)],
        )];
        let err = execute_commit_plan(&dir, &groups, None, &checkpoint).unwrap_err();

        assert!(matches!(err, ExecutionError::HeadMoved { .. }));

        fs::remove_dir_all(&dir).ok();
    }
}
