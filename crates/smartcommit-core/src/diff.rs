//! Diff extraction (Module 4).
//!
//! Extracts the staged diff (HEAD vs. index) for every changed file
//! as structured hunks, never the unstaged working-tree diff.
//!
//! # Why one `git diff` call, not one per file
//!
//! An earlier version of this module called `git diff --cached -M
//! -- <path>` once per file, mirroring Module 3's per-file
//! `ChangedFile` model. That breaks rename detection: restricting
//! the diff to a single pathspec stops git from pairing the old and
//! new sides of a rename, so a genuine rename gets reported as a
//! plain delete+add instead. Renames can only be detected correctly
//! when git sees the *entire* staged change set at once. So this
//! module runs `git diff --cached -M` exactly once (no pathspec)
//! and splits the resulting unified diff into per-file blocks
//! itself.

use crate::{Repository, RepositoryError};
use std::path::PathBuf;
use std::process::Command;

/// One line inside a diff hunk, with its Git-diff prefix already
/// stripped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLine {
    Context(String),
    Added(String),
    Removed(String),
}

/// One `@@ ... @@` hunk within a file's diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    /// The full hunk header line, e.g. `@@ -1,5 +1,5 @@`.
    pub header: String,
    pub lines: Vec<DiffLine>,
}

/// The staged diff for a single file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    /// The file's current path — for a rename this is the new path,
    /// matching `ChangedFile::path` from Module 3. For a delete,
    /// git repeats the same name on both sides of its `diff --git`
    /// header, so this is simply that file's path.
    pub path: PathBuf,
    pub hunks: Vec<DiffHunk>,
    pub additions: usize,
    pub deletions: usize,
    /// True for a binary file. When true, `hunks` is always empty —
    /// binary content is never parsed as text.
    pub is_binary: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum DiffError {
    #[error(transparent)]
    Repository(#[from] RepositoryError),

    #[error("failed to run git diff: {0}")]
    GitUnavailable(String),

    #[error("git diff returned a failure status: {0}")]
    CommandFailed(String),
}

/// Extracts the staged diff for every currently staged file.
///
/// Uses `--no-color` and `--no-ext-diff` so output is always plain
/// unified-diff text, regardless of the caller's git config (color
/// settings, `diff.external`). Rename detection (`-M`) matches
/// Module 3's `get_staged_changes` so the two stay consistent.
///
/// Returns an empty `Vec` when nothing is staged.
pub fn get_staged_diffs(
    path: impl AsRef<std::path::Path>,
) -> Result<Vec<FileDiff>, DiffError> {
    let repo = Repository::discover(path)?;

    let output = Command::new("git")
        .arg("-C")
        .arg(&repo.root)
        .args(["diff", "--cached", "-M", "--no-color", "--no-ext-diff"])
        .output()
        .map_err(|e| DiffError::GitUnavailable(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(DiffError::CommandFailed(stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_unified_diff(&stdout))
}

/// Parses the full output of `git diff --cached -M --no-color
/// --no-ext-diff` into one `FileDiff` per `diff --git` section.
///
/// # Known limitation
/// The file path is taken from the `diff --git a/<path> b/<path>`
/// header by splitting on the first `" b/"` after stripping the
/// `"diff --git a/"` prefix. A path that legitimately contains the
/// literal substring `" b/"` would split incorrectly. This matches
/// the heuristic most diff parsers use and is not expected to cause
/// problems in practice, but is not a byte-for-byte-safe parse of
/// git's diff header for adversarial filenames.
fn parse_unified_diff(diff_text: &str) -> Vec<FileDiff> {
    let mut files = Vec::new();
    let mut current_file: Option<FileDiff> = None;
    let mut current_hunk: Option<DiffHunk> = None;

    // Finalizes the in-progress hunk (if any) into the in-progress
    // file, and the in-progress file (if any) into `files`. Shared
    // by "we hit the next file/hunk" and "we reached the end".
    fn finalize_hunk(file: &mut Option<FileDiff>, hunk: &mut Option<DiffHunk>) {
        if let (Some(f), Some(h)) = (file.as_mut(), hunk.take()) {
            f.hunks.push(h);
        }
    }

    for line in diff_text.lines() {
        if let Some(rest) = line.strip_prefix("diff --git a/") {
            // Starting a new file's section: close out whatever was
            // in progress first.
            finalize_hunk(&mut current_file, &mut current_hunk);
            if let Some(f) = current_file.take() {
                files.push(f);
            }

            // "a/<path> b/<path>" — see the doc comment above for
            // the splitting heuristic and its limitation.
            let path = rest
                .rsplit_once(" b/")
                .map(|(_a, b)| PathBuf::from(b))
                .unwrap_or_else(|| PathBuf::from(rest));

            current_file = Some(FileDiff {
                path,
                hunks: Vec::new(),
                additions: 0,
                deletions: 0,
                is_binary: false,
            });
            continue;
        }

        if line.starts_with("Binary files ") && line.ends_with(" differ") {
            if let Some(f) = current_file.as_mut() {
                f.is_binary = true;
            }
            continue;
        }

        if let Some(header) = line.strip_prefix("@@ ") {
            // New hunk: close out the previous one first.
            finalize_hunk(&mut current_file, &mut current_hunk);
            current_hunk = Some(DiffHunk {
                header: format!("@@ {header}"),
                lines: Vec::new(),
            });
            continue;
        }

        // Lines below only mean something once we're inside a hunk.
        // Metadata lines before the first hunk of a file (`index
        // ...`, `new file mode ...`, `--- a/X`, `+++ b/Y`,
        // `rename from/to ...`, `similarity index ...`, etc.) are
        // intentionally skipped — Module 3 already captures
        // everything about a file's identity and status that this
        // module needs.
        if current_hunk.is_none() {
            continue;
        }

        if let Some(rest) = line.strip_prefix('+') {
            if let (Some(f), Some(h)) = (current_file.as_mut(), current_hunk.as_mut()) {
                h.lines.push(DiffLine::Added(rest.to_string()));
                f.additions += 1;
            }
        } else if let Some(rest) = line.strip_prefix('-') {
            if let (Some(f), Some(h)) = (current_file.as_mut(), current_hunk.as_mut()) {
                h.lines.push(DiffLine::Removed(rest.to_string()));
                f.deletions += 1;
            }
        } else if let Some(rest) = line.strip_prefix(' ') {
            if let Some(h) = current_hunk.as_mut() {
                h.lines.push(DiffLine::Context(rest.to_string()));
            }
        }
        // A line starting with `\` (e.g. "\ No newline at end of
        // file") is a marker, not content — deliberately not
        // pushed as a DiffLine and not counted.
    }

    finalize_hunk(&mut current_file, &mut current_hunk);
    if let Some(f) = current_file.take() {
        files.push(f);
    }

    files
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
            "smartcommit-core-diff-test-{label}-{}-{n}",
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
    fn extracts_a_text_modification() {
        let dir = init_repo("modification");
        write_file(&dir.join("README.md"), "changed\n");
        git(&dir, &["add", "README.md"]);

        let diffs = get_staged_diffs(&dir).expect("should succeed");
        assert_eq!(diffs.len(), 1);
        let diff = &diffs[0];
        assert_eq!(diff.path, PathBuf::from("README.md"));
        assert!(!diff.is_binary);
        assert_eq!(diff.hunks.len(), 1);
        assert_eq!(diff.additions, 1);
        assert_eq!(diff.deletions, 1);
        assert!(diff.hunks[0].header.starts_with("@@ "));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn extracts_a_new_file() {
        let dir = init_repo("new-file");
        write_file(&dir.join("new.txt"), "line one\nline two\n");
        git(&dir, &["add", "new.txt"]);

        let diffs = get_staged_diffs(&dir).expect("should succeed");
        assert_eq!(diffs.len(), 1);
        let diff = &diffs[0];
        assert_eq!(diff.path, PathBuf::from("new.txt"));
        assert!(!diff.is_binary);
        assert_eq!(diff.additions, 2);
        assert_eq!(diff.deletions, 0);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn extracts_a_deleted_file() {
        let dir = init_repo("deleted-file");
        fs::remove_file(dir.join("README.md")).expect("failed to delete README.md");
        git(&dir, &["add", "README.md"]);

        let diffs = get_staged_diffs(&dir).expect("should succeed");
        assert_eq!(diffs.len(), 1);
        let diff = &diffs[0];
        assert_eq!(diff.path, PathBuf::from("README.md"));
        assert!(!diff.is_binary);
        assert_eq!(diff.additions, 0);
        assert_eq!(diff.deletions, 1);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn extracts_multiple_hunks_in_one_file() {
        let dir = init_repo("multi-hunk");
        let lines: Vec<String> = (1..=30).map(|n| n.to_string()).collect();
        write_file(&dir.join("nums.txt"), &(lines.join("\n") + "\n"));
        git(&dir, &["add", "nums.txt"]);
        git(&dir, &["commit", "--quiet", "-m", "add nums"]);

        let mut lines = lines;
        lines[1] = "CHANGED_TOP".to_string(); // line 2
        lines[28] = "CHANGED_BOTTOM".to_string(); // line 29
        write_file(&dir.join("nums.txt"), &(lines.join("\n") + "\n"));
        git(&dir, &["add", "nums.txt"]);

        let diffs = get_staged_diffs(&dir).expect("should succeed");
        assert_eq!(diffs.len(), 1);
        let diff = &diffs[0];
        assert_eq!(diff.path, PathBuf::from("nums.txt"));
        assert_eq!(
            diff.hunks.len(),
            2,
            "two separated changes should produce two hunks, got {:?}",
            diff.hunks
        );
        assert_eq!(diff.additions, 2);
        assert_eq!(diff.deletions, 2);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detects_a_binary_file() {
        let dir = init_repo("binary");
        // A real NUL byte is what makes git's binary heuristic
        // trigger — a string of visible characters would not.
        fs::write(dir.join("data.bin"), [0u8, 1, 2, 255, 254, 10, 0, 5])
            .expect("failed to write binary fixture");
        git(&dir, &["add", "data.bin"]);

        let diffs = get_staged_diffs(&dir).expect("should succeed");
        assert_eq!(diffs.len(), 1);
        let diff = &diffs[0];
        assert_eq!(diff.path, PathBuf::from("data.bin"));
        assert!(diff.is_binary);
        assert!(
            diff.hunks.is_empty(),
            "a binary file must never have text hunks"
        );
        assert_eq!(diff.additions, 0);
        assert_eq!(diff.deletions, 0);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn line_counts_match_a_known_diff() {
        let dir = init_repo("line-counts");
        write_file(&dir.join("README.md"), "one\ntwo\nthree\n");
        git(&dir, &["add", "README.md"]);
        git(&dir, &["commit", "--quiet", "-m", "three lines"]);

        // Remove the middle line, add two new lines at the end —
        // a known, hand-countable change.
        write_file(&dir.join("README.md"), "one\nthree\nfour\nfive\n");
        git(&dir, &["add", "README.md"]);

        let diffs = get_staged_diffs(&dir).expect("should succeed");
        assert_eq!(diffs.len(), 1);
        let diff = &diffs[0];
        assert_eq!(diff.deletions, 1, "removed exactly one line (\"two\")");
        assert_eq!(diff.additions, 2, "added exactly two lines (\"four\", \"five\")");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn correctly_diffs_a_rename_alongside_other_changes() {
        // Regression test for the pathspec-filtering bug described
        // in this module's doc comment: renames must still be
        // detected as renames (not delete+add) when other staged
        // changes are present in the same commit.
        let dir = init_repo("rename-with-others");
        let body = "line\n".repeat(20);
        write_file(&dir.join("original.txt"), &body);
        git(&dir, &["add", "original.txt"]);
        git(&dir, &["commit", "--quiet", "-m", "add original"]);

        fs::rename(dir.join("original.txt"), dir.join("renamed.txt"))
            .expect("failed to rename");
        write_file(&dir.join("other.txt"), "unrelated\n");
        git(&dir, &["add", "-A"]);

        let diffs = get_staged_diffs(&dir).expect("should succeed");
        assert_eq!(diffs.len(), 2, "expected the rename plus the new file, got {diffs:?}");

        let renamed = diffs
            .iter()
            .find(|d| d.path == PathBuf::from("renamed.txt"))
            .expect("renamed.txt should be present");
        // An unmodified rename has no content diff, so no hunks —
        // it's still correctly identified as renamed.txt rather
        // than showing up as a 20-line delete+add.
        assert!(renamed.hunks.is_empty());

        fs::remove_dir_all(&dir).ok();
    }
}
