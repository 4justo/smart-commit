//! Git repository detection (Module 1).
//!
//! Shells out to the system `git` binary rather than linking
//! `libgit2`/`git2` — consistent with the rest of the MVP engine,
//! and avoids pulling in a heavier dependency before a module
//! actually needs the extra control `git2` would offer (e.g.
//! Module 10's index manipulation).

use std::path::{Path, PathBuf};
use std::process::Command;

/// Structured result of a successful repository discovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryInfo {
    /// The working-tree root. For a bare repository (which has no
    /// working tree) this is the bare repository's own directory —
    /// there is nothing else it could sensibly mean.
    pub root: PathBuf,
    /// Absolute path to the `.git` directory (or, for a bare
    /// repository, the repository directory itself).
    pub git_dir: PathBuf,
    /// Whether this is a bare repository (no working tree).
    pub is_bare: bool,
}

/// Errors `Repository::discover` can return. Every variant here
/// represents an expected, non-panicking outcome — an invalid or
/// inaccessible path, or a path that simply isn't a Git repository,
/// is normal user input, not a bug.
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("path does not exist: {0}")]
    PathNotFound(PathBuf),

    #[error("path is not accessible: {path} ({reason})")]
    PathNotAccessible { path: PathBuf, reason: String },

    #[error("not a Git repository: {0}")]
    NotARepository(PathBuf),

    #[error("could not run 'git': {0}")]
    GitUnavailable(String),
}

/// Namespace for repository discovery. A unit struct rather than a
/// module-level free function so call sites read as `Repository::discover(..)`,
/// matching the API shape modules.md suggests and leaving room for
/// this to grow instance methods later (e.g. once Module 2 adds
/// state inspection) without a breaking API change.
pub struct Repository;

impl Repository {
    /// Discovers the Git repository containing `path`.
    ///
    /// `path` may be the repository root, any directory nested
    /// inside it, or an explicit `--repo` path supplied by the CLI.
    /// Git's own upward directory search (via `git -C <path>`)
    /// handles the "nested directory" case without any extra logic
    /// here.
    pub fn discover(path: impl AsRef<Path>) -> Result<RepositoryInfo, RepositoryError> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(RepositoryError::PathNotFound(path.to_path_buf()));
        }

        let canonical =
            std::fs::canonicalize(path).map_err(|e| RepositoryError::PathNotAccessible {
                path: path.to_path_buf(),
                reason: e.to_string(),
            })?;

        // --absolute-git-dir and --is-bare-repository can be
        // combined and both succeed for bare AND non-bare repos.
        // --show-toplevel cannot be combined here: it hard-fails
        // the whole invocation on a bare repo (no working tree to
        // report), even when other flags in the same call would
        // have succeeded. So the work-tree root is queried
        // separately, only once we know we're not bare.
        let probe = Command::new("git")
            .arg("-C")
            .arg(&canonical)
            .args(["rev-parse", "--absolute-git-dir", "--is-bare-repository"])
            .output()
            .map_err(|e| RepositoryError::GitUnavailable(e.to_string()))?;

        if !probe.status.success() {
            return Err(RepositoryError::NotARepository(canonical));
        }

        let probe_stdout = String::from_utf8_lossy(&probe.stdout);
        let mut probe_lines = probe_stdout.lines();
        let git_dir = probe_lines
            .next()
            .map(PathBuf::from)
            .ok_or_else(|| RepositoryError::NotARepository(canonical.clone()))?;
        let is_bare = probe_lines.next() == Some("true");

        let root = if is_bare {
            // No working tree — the bare repository directory
            // itself is the only sensible "root".
            git_dir.clone()
        } else {
            let toplevel = Command::new("git")
                .arg("-C")
                .arg(&canonical)
                .args(["rev-parse", "--show-toplevel"])
                .output()
                .map_err(|e| RepositoryError::GitUnavailable(e.to_string()))?;

            if !toplevel.status.success() {
                return Err(RepositoryError::NotARepository(canonical));
            }

            String::from_utf8_lossy(&toplevel.stdout)
                .lines()
                .next()
                .map(PathBuf::from)
                .ok_or_else(|| RepositoryError::NotARepository(canonical.clone()))?
        };

        Ok(RepositoryInfo {
            root,
            git_dir,
            is_bare,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command as StdCommand;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn make_temp_dir(label: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "smartcommit-core-repository-test-{label}-{}-{n}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("failed to create temp dir for test");
        dir
    }

    fn run_git(dir: &Path, args: &[&str]) {
        let status = StdCommand::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .unwrap_or_else(|e| panic!("failed to run git {args:?} for test fixture: {e}"));
        assert!(status.success(), "git {args:?} failed in test fixture");
    }

    #[test]
    fn discovers_a_valid_repository_root() {
        let dir = make_temp_dir("valid-root");
        run_git(&dir, &["init", "--quiet"]);

        let info = Repository::discover(&dir).expect("expected discovery to succeed");
        assert_eq!(
            info.root,
            fs::canonicalize(&dir).unwrap(),
            "root should match the repo directory"
        );
        assert!(!info.is_bare);
        assert!(info.git_dir.ends_with(".git"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn discovers_repository_from_a_nested_directory() {
        let dir = make_temp_dir("nested");
        run_git(&dir, &["init", "--quiet"]);
        let nested = dir.join("src").join("deep").join("nesting");
        fs::create_dir_all(&nested).expect("failed to create nested dirs");

        let info = Repository::discover(&nested).expect("expected discovery to succeed");
        assert_eq!(
            info.root,
            fs::canonicalize(&dir).unwrap(),
            "root should resolve up to the repo root, not the nested path"
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_a_non_repository_directory() {
        let dir = make_temp_dir("non-repo");

        let result = Repository::discover(&dir);
        assert!(
            matches!(result, Err(RepositoryError::NotARepository(_))),
            "expected NotARepository, got {result:?}"
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_an_invalid_path() {
        let missing = std::env::temp_dir().join("smartcommit-core-this-path-does-not-exist-abc123");
        assert!(!missing.exists(), "test fixture path should not exist");

        let result = Repository::discover(&missing);
        assert!(
            matches!(result, Err(RepositoryError::PathNotFound(_))),
            "expected PathNotFound, got {result:?}"
        );
    }

    #[test]
    fn discovers_a_bare_repository() {
        let parent = make_temp_dir("bare-parent");
        let bare_dir = parent.join("repo.git");
        run_git(&parent, &["init", "--quiet", "--bare", "repo.git"]);

        let info = Repository::discover(&bare_dir).expect("expected discovery to succeed");
        assert!(
            info.is_bare,
            "expected a bare repository to be detected as bare"
        );
        assert_eq!(
            info.root,
            fs::canonicalize(&bare_dir).unwrap(),
            "for a bare repo, root should be the bare repo directory itself"
        );

        fs::remove_dir_all(&parent).ok();
    }
}
