use crate::{Repository, RepositoryError};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedFile {
    pub path: PathBuf,
    pub change_kind: ChangeKind,
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

pub fn get_staged_changes(
    path: impl AsRef<std::path::Path>,
) -> Result<Vec<ChangedFile>, StagedChangesError> {
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
        write_file(&dir.join("never_added.txt"), "hello\n");

        let result = get_staged_changes(&dir).expect("should succeed");
        assert!(
            result.is_empty(),
            "an untracked file must not appear in staged changes, got {result:?}"
        );

        fs::remove_dir_all(&dir).ok();
    }
}
