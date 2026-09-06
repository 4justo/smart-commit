//! Staged change discovery (Module 3).
//!
//! Reads the Git index — the set of changes staged for the next
//! commit — via `git diff --cached --name-status`. This is
//! deliberately the diff between HEAD and the index, never the
//! working-tree diff: SmartCommit's MVP operates on staged changes
//! only, and must not accidentally treat unstaged or untracked
//! files as part of that set.
//!
//! Builds on [`crate::Repository::discover`] (Module 1) for
//! repository discovery/validation, matching the pattern
//! `repository_state.rs` (Module 2) already established.

use crate::{Repository, RepositoryError};
use std::path::PathBuf;
use std::process::Command;

/// The kind of change a staged file represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
}

/// A single file with staged changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedFile {
    /// Current path (post-change). For a rename this is the new
    /// path; `old_path` holds the source.
    pub path: PathBuf,
    pub change_kind: ChangeKind,
    /// Set only when `change_kind` is `Renamed`. Without this a
    /// rename entry would be indistinguishable from an add — the
    /// field earns its place because `Renamed` is otherwise not
    /// actionable information.
    pub old_path: Option<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum StagedChangesError {
    #[error(transparent)]
    Repository(#[from] RepositoryError),

    #[error("failed to run git diff: {0}")]
    GitUnavailable(String),

    #[error("git diff returned a failure status: {0}")]
    CommandFailed(String),
}

/// Lists files currently staged for commit (index vs. HEAD), with
/// their change kind.
///
/// Uses `-z` so filenames are NUL-delimited rather than
/// newline-delimited: this avoids ambiguity for filenames containing
/// spaces, and never needs unquoting for special characters. Rename
/// detection is requested explicitly with `-M` so `Renamed` results
/// don't depend on the caller's `diff.renames` git config.
///
/// Returns an empty `Vec` (not an error) when nothing is staged.
pub fn get_staged_changes(
    path: impl AsRef<std::path::Path>,
) -> Result<Vec<ChangedFile>, StagedChangesError> {
    // Module 1 handles repo discovery and path validation.
    let repo = Repository::discover(path)?;

    let output = Command::new("git")
        .arg("-C")
        .arg(&repo.root)
        .args(["diff", "--cached", "--name-status", "-M", "-z"])
        .output()
        .map_err(|e| StagedChangesError::GitUnavailable(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(StagedChangesError::CommandFailed(stderr));
    }

    // With -z, records are NUL-separated and the trailing separator
    // leaves one empty field at the end — filter it out. A record is
    // either [status, path] or, for renames, [status, old_path,
    // new_path].
    let stdout = String::from_utf8_lossy(&output.stdout);
    let fields: Vec<&str> = stdout.split('\0').filter(|s| !s.is_empty()).collect();

    let mut result = Vec::new();
    let mut i = 0;
    while i < fields.len() {
        let status_code = fields[i];
        i += 1;
        let first = status_code.chars().next().unwrap_or('?');

        if first == 'R' {
            let old_path = fields.get(i).map(PathBuf::from);
            i += 1;
            let path = fields.get(i).map(PathBuf::from).unwrap_or_default();
            i += 1;
            result.push(ChangedFile {
                path,
                change_kind: ChangeKind::Renamed,
                old_path,
            });
        } else {
            let path = fields.get(i).map(PathBuf::from).unwrap_or_default();
            i += 1;
            // A/M/D map directly. Any other single-letter status
            // (e.g. T for a type change, U for an unresolved merge
            // conflict) isn't in this module's scope per modules.md,
            // but the file still has a real staged change — folding
            // it into Modified keeps the file visible rather than
            // silently dropping it, which matters more than getting
            // an edge-case label exactly right at this stage.
            let change_kind = match first {
                'A' => ChangeKind::Added,
                'D' => ChangeKind::Deleted,
                _ => ChangeKind::Modified,
            };
            result.push(ChangedFile {
                path,
                change_kind,
                old_path: None,
            });
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::process::Command as StdCommand;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(label: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "smartcommit-core-staged-changes-test-{label}-{}-{n}",
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

    fn write_file(path: &Path, contents: &str) {
        fs::write(path, contents).expect("failed to write fixture file");
    }

    /// Repo with an initial commit, so there's a HEAD to diff the
    /// index against.
    fn init_repo(label: &str) -> PathBuf {
        let dir = temp_dir(label);
        git(&dir, &["init", "--quiet"]);
        write_file(&dir.join("README.md"), "initial\n");
        git(&dir, &["add", "README.md"]);
        git(&dir, &["commit", "--quiet", "-m", "initial commit"]);
        dir
    }

    #[test]
    fn detects_a_staged_added_file() {
        let dir = init_repo("added");
        write_file(&dir.join("new_file.txt"), "hello\n");
        git(&dir, &["add", "new_file.txt"]);

        let result = get_staged_changes(&dir).expect("should succeed");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("new_file.txt"));
        assert_eq!(result[0].change_kind, ChangeKind::Added);
        assert_eq!(result[0].old_path, None);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detects_a_staged_modified_file() {
        let dir = init_repo("modified");
        write_file(&dir.join("README.md"), "changed\n");
        git(&dir, &["add", "README.md"]);

        let result = get_staged_changes(&dir).expect("should succeed");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("README.md"));
        assert_eq!(result[0].change_kind, ChangeKind::Modified);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detects_a_staged_deleted_file() {
        let dir = init_repo("deleted");
        fs::remove_file(dir.join("README.md")).expect("failed to delete README.md");
        git(&dir, &["add", "README.md"]);

        let result = get_staged_changes(&dir).expect("should succeed");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("README.md"));
        assert_eq!(result[0].change_kind, ChangeKind::Deleted);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detects_a_staged_renamed_file() {
        let dir = init_repo("renamed");
        // Give the original file enough content that git's
        // similarity heuristic reliably calls this a rename rather
        // than a delete+add when moved unmodified.
        let body = "line\n".repeat(20);
        write_file(&dir.join("README.md"), &body);
        git(&dir, &["add", "README.md"]);
        git(&dir, &["commit", "--quiet", "-m", "bulk up README"]);

        fs::rename(dir.join("README.md"), dir.join("RENAMED.md"))
            .expect("failed to rename README.md");
        git(&dir, &["add", "-A"]);

        let result = get_staged_changes(&dir).expect("should succeed");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("RENAMED.md"));
        assert_eq!(result[0].change_kind, ChangeKind::Renamed);
        assert_eq!(result[0].old_path, Some(PathBuf::from("README.md")));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detects_multiple_staged_files_of_different_kinds() {
        let dir = init_repo("multi");
        write_file(&dir.join("added.txt"), "new\n");
        write_file(&dir.join("README.md"), "changed\n");
        git(&dir, &["add", "added.txt", "README.md"]);

        let mut result = get_staged_changes(&dir).expect("should succeed");
        result.sort_by(|a, b| a.path.cmp(&b.path));

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].path, PathBuf::from("README.md"));
        assert_eq!(result[0].change_kind, ChangeKind::Modified);
        assert_eq!(result[1].path, PathBuf::from("added.txt"));
        assert_eq!(result[1].change_kind, ChangeKind::Added);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn does_not_include_an_unstaged_modification() {
        let dir = init_repo("unstaged-only");
        // Modify a tracked file but never `git add` it.
        write_file(&dir.join("README.md"), "changed but not staged\n");

        let result = get_staged_changes(&dir).expect("should succeed");
        assert!(
            result.is_empty(),
            "an unstaged modification must not appear in staged changes, got {result:?}"
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn does_not_include_an_untracked_file() {
        let dir = init_repo("untracked-only");
        // Create a new file but never `git add` it — untracked, not staged.
        write_file(&dir.join("never_added.txt"), "hello\n");

        let result = get_staged_changes(&dir).expect("should succeed");
        assert!(
            result.is_empty(),
            "an untracked file must not appear in staged changes, got {result:?}"
        );

        fs::remove_dir_all(&dir).ok();
    }
}
