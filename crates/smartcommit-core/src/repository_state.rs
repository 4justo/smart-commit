//! Repository state inspection (Module 2).
//!
//! Determines whether SmartCommit can safely operate on the current
//! repository: branch, HEAD, detached-HEAD, in-progress merge/
//! rebase/cherry-pick, conflicts, and staged/unstaged/untracked
//! file status — plus a Safe/Warning/Blocked classification.
//!
//! Builds on [`crate::Repository::discover`] (Module 1) for
//! repository discovery, then shells out to `git status
//! --porcelain=v2 --branch` for everything else.

use crate::{Repository, RepositoryError};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RepositoryState {
    pub branch: Option<String>,
    pub head: Option<String>,
    pub detached_head: bool,
    pub merge_in_progress: bool,
    pub rebase_in_progress: bool,
    pub cherry_pick_in_progress: bool,
    pub has_conflicts: bool,
    pub has_staged_changes: bool,
    pub has_unstaged_changes: bool,
    pub has_untracked_files: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositorySafety {
    Safe,
    Warning,
    Blocked,
}

impl RepositoryState {
    // Summarize whether SmartCommit should proceed on this repo state.
    // This is intentionally conservative: anything risky becomes Warning
    // or Blocked rather than guessing.
    pub fn safety(&self) -> RepositorySafety {
        // In-progress Git operations mean the repository is not safe to touch.
        if self.merge_in_progress
            || self.rebase_in_progress
            || self.cherry_pick_in_progress
            || self.has_conflicts
        {
            return RepositorySafety::Blocked;
        }

        // A repo is only "Safe" when there is staged work and no unstable
        // side conditions that could confuse later commit planning.
        if !self.has_staged_changes || self.has_unstaged_changes || self.has_untracked_files {
            return RepositorySafety::Warning;
        }

        RepositorySafety::Safe
    }
}

// Errors from Module 2 inspection.
// Keep this narrow: only repository discovery and Git status failures.
#[derive(Debug, thiserror::Error)]
pub enum InspectorError {
    #[error(transparent)]
    Repository(#[from] RepositoryError),

    #[error("failed to run git status: {0}")]
    GitUnavailable(String),

    #[error("git status returned a failure status")]
    StatusFailed,
}

// Inspect the current repository state by asking Git for porcelain v2 output.
// This builds the Module 2 model without mutating the repository.
pub fn inspect_repository_state(
    path: impl AsRef<Path>,
) -> Result<RepositoryState, InspectorError> {
    // Module 1 handles repo discovery and path validation.
    let repo = Repository::discover(path)?;

    let mut state = RepositoryState::default();

    // Ask Git for branch metadata plus tracked/untracked file state in one call.
    let output = Command::new("git")
        .arg("-C")
        .arg(&repo.root)
        .args(["status", "--porcelain=v2", "--branch"])
        .output()
        .map_err(|e| InspectorError::GitUnavailable(e.to_string()))?;

    // A non-zero exit code means Git could not produce a trustworthy status.
    if !output.status.success() {
        return Err(InspectorError::StatusFailed);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        // The branch line tells us whether HEAD is attached or detached.
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            if rest == "(detached)" {
                state.detached_head = true;
            } else {
                state.branch = Some(rest.to_string());
            }
            continue;
        }

        // The oid line gives the current HEAD commit when Git knows it.
        if let Some(rest) = line.strip_prefix("# branch.oid ") {
            if rest != "(initial)" {
                state.head = Some(rest.to_string());
            }
            continue;
        }

        // Record type `1` is a normal tracked file entry.
        if let Some(rest) = line.strip_prefix("1 ") {
            let mut parts = rest.split_whitespace();
            if let Some(xy) = parts.next() {
                let mut chars = xy.chars();
                let index = chars.next().unwrap_or('.');
                let worktree = chars.next().unwrap_or('.');

                if index != '.' {
                    state.has_staged_changes = true;
                }
                if worktree != '.' {
                    state.has_unstaged_changes = true;
                }
            }
            continue;
        }

        // Record type `2` is a renamed or copied tracked file.
        if let Some(rest) = line.strip_prefix("2 ") {
            let mut parts = rest.split_whitespace();
            if let Some(xy) = parts.next() {
                let mut chars = xy.chars();
                let index = chars.next().unwrap_or('.');
                let worktree = chars.next().unwrap_or('.');

                if index != '.' {
                    state.has_staged_changes = true;
                }
                if worktree != '.' {
                    state.has_unstaged_changes = true;
                }
            }
            continue;
        }

        // `?` means Git sees an untracked file in the working tree.
        if line.starts_with("? ") {
            state.has_untracked_files = true;
            continue;
        }

        // `u` means Git has an unresolved conflict for this path.
        if line.starts_with("u ") {
            state.has_conflicts = true;
            state.has_staged_changes = true;
        }
    }

    // These special files are reliable markers for in-progress operations.
    state.merge_in_progress = repo.git_dir.join("MERGE_HEAD").exists();
    state.cherry_pick_in_progress = repo.git_dir.join("CHERRY_PICK_HEAD").exists();

    // Rebase uses either `rebase-apply` or `rebase-merge`, depending on how
    // Git started the operation.
    state.rebase_in_progress = repo.git_dir.join("rebase-apply").exists()
        || repo.git_dir.join("rebase-merge").exists();

    Ok(state)
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
            "smartcommit-core-repository-state-test-{label}-{}-{n}",
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

    #[test]
    fn detects_clean_repository() {
        let dir = temp_dir("clean");
        git(&dir, &["init", "--quiet"]);

        let state = inspect_repository_state(&dir).expect("inspection should succeed");
        assert!(state.branch.is_some());
        assert!(!state.detached_head);
        assert!(!state.has_staged_changes);
        assert!(!state.has_unstaged_changes);
        assert!(!state.has_untracked_files);
        assert_eq!(state.safety(), RepositorySafety::Warning);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detects_staged_changes() {
        let dir = temp_dir("staged");
        git(&dir, &["init", "--quiet"]);
        write_file(&dir.join("file.txt"), "hello\n");
        git(&dir, &["add", "file.txt"]);

        let state = inspect_repository_state(&dir).expect("inspection should succeed");
        assert!(state.has_staged_changes);
        assert!(!state.has_unstaged_changes);
        assert!(!state.has_untracked_files);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detects_unstaged_changes() {
        let dir = temp_dir("unstaged");
        git(&dir, &["init", "--quiet"]);
        write_file(&dir.join("file.txt"), "hello\n");
        git(&dir, &["add", "file.txt"]);
        git(&dir, &["commit", "--quiet", "-m", "init"]);
        write_file(&dir.join("file.txt"), "changed\n");

        let state = inspect_repository_state(&dir).expect("inspection should succeed");
        assert!(state.has_unstaged_changes);
        assert!(!state.has_staged_changes);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detects_untracked_files() {
        let dir = temp_dir("untracked");
        git(&dir, &["init", "--quiet"]);
        write_file(&dir.join("new.txt"), "hello\n");

        let state = inspect_repository_state(&dir).expect("inspection should succeed");
        assert!(state.has_untracked_files);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detects_detached_head() {
        let dir = temp_dir("detached");
        git(&dir, &["init", "--quiet"]);
        write_file(&dir.join("file.txt"), "hello\n");
        git(&dir, &["add", "file.txt"]);
        git(&dir, &["commit", "--quiet", "-m", "init"]);
        git(&dir, &["checkout", "--quiet", "--detach"]);

        let state = inspect_repository_state(&dir).expect("inspection should succeed");
        assert!(state.detached_head);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detects_merge_conflicts() {
        let dir = temp_dir("conflict");
        git(&dir, &["init", "--quiet"]);
        write_file(&dir.join("conflict.txt"), "base\n");
        git(&dir, &["add", "conflict.txt"]);
        git(&dir, &["commit", "--quiet", "-m", "base"]);
        git(&dir, &["checkout", "--quiet", "-b", "feature"]);
        write_file(&dir.join("conflict.txt"), "feature\n");
        git(&dir, &["commit", "--quiet", "-am", "feature"]);
        git(&dir, &["checkout", "--quiet", "-"]);
        write_file(&dir.join("conflict.txt"), "main\n");
        git(&dir, &["commit", "--quiet", "-am", "main"]);

        let status = StdCommand::new("git")
            .args(["merge", "feature"])
            .current_dir(&dir)
            .env("GIT_AUTHOR_NAME", "Test User")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "Test User")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .status()
            .expect("failed to run merge");
        assert!(!status.success(), "merge should conflict");

        let state = inspect_repository_state(&dir).expect("inspection should succeed");
        assert!(state.merge_in_progress);
        assert!(state.has_conflicts);
        assert_eq!(state.safety(), RepositorySafety::Blocked);

        fs::remove_dir_all(&dir).ok();
    }
}
