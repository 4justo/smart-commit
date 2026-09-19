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
///
/// Named `Added`/`Removed` (not `Addition`/`Deletion`) to match the
/// names already in use throughout the rest of the codebase (Module
/// 6's matcher, the CLI's `--verbose` renderer) — extending the
/// existing model rather than introducing a parallel one with
/// different names for the same concept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLine {
    Context(String),
    Added(String),
    Removed(String),
}

/// Per-hunk line counts (Module 16). `total_changed_lines` is
/// `additions + deletions` — context lines don't count as "changed."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HunkStats {
    pub additions: usize,
    pub deletions: usize,
    pub context: usize,
    pub total_changed_lines: usize,
}

/// One `@@ ... @@` hunk within a file's diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    /// Deterministic identifier: `"<file path>#<index>"`, where
    /// `index` is this hunk's 0-based position among its file's
    /// hunks (hunks within one file never overlap and are always
    /// produced in order, so this is unique and stable without
    /// needing a hash — two hunks with byte-identical content still
    /// get different IDs whenever they're in different files or at
    /// different positions in the same file).
    pub id: String,
    /// Same as the parent `FileDiff::path` — carried onto the hunk
    /// itself so a hunk can be handled independently of its file
    /// (e.g. a flattened list of hunks across files) without losing
    /// track of which file it belongs to.
    pub file_path: PathBuf,
    pub old_start: usize,
    pub old_count: usize,
    pub new_start: usize,
    pub new_count: usize,
    /// The full hunk header line, e.g. `@@ -1,5 +1,5 @@`.
    pub header: String,
    pub lines: Vec<DiffLine>,
    /// The hunk exactly as it appeared in `git diff` output — the
    /// header line plus every body line, each still carrying its
    /// original `+`/`-`/` ` prefix, joined with `\n`. One caveat:
    /// this is captured via the same line-splitting the rest of the
    /// parser uses, which (per Rust's `str::lines()`) treats a
    /// trailing `\r` immediately before `\n` as part of the line
    /// terminator rather than content — so a file using CRLF line
    /// endings internally will have that `\r` silently absent here
    /// too. Parsing itself is unaffected (verified — see
    /// `extracts_a_hunk_from_a_crlf_file` below); only the literal
    /// carriage return in `raw_content` and each `DiffLine`'s text
    /// is not byte-for-byte preserved.
    pub raw_content: String,
    pub stats: HunkStats,
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
pub fn get_staged_diffs(path: impl AsRef<std::path::Path>) -> Result<Vec<FileDiff>, DiffError> {
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

/// Parses one `@@ -a[,b] +c[,d] @@ ...` header into
/// `(old_start, old_count, new_start, new_count)`. Handles an
/// omitted count (implying `1`, per the unified diff format — git
/// omits `,1` when a range is exactly one line) and a trailing
/// "function context" hint git sometimes appends after the closing
/// `@@` (e.g. `@@ -3 +3 @@ fn foo() {` — confirmed empirically this
/// appears even for a single-line change).
fn parse_hunk_header_ranges(header: &str) -> Option<(usize, usize, usize, usize)> {
    let rest = header.strip_prefix("@@ ")?;
    let close_idx = rest.find("@@")?;
    let ranges_part = rest[..close_idx].trim();

    let mut parts = ranges_part.split_whitespace();
    let old_spec = parts.next()?.strip_prefix('-')?;
    let new_spec = parts.next()?.strip_prefix('+')?;

    let (old_start, old_count) = parse_range(old_spec)?;
    let (new_start, new_count) = parse_range(new_spec)?;

    Some((old_start, old_count, new_start, new_count))
}

/// Parses one side of a hunk range: `"start,count"` or, when the
/// count is omitted (a one-line range), just `"start"` — in which
/// case the count is `1`.
fn parse_range(spec: &str) -> Option<(usize, usize)> {
    match spec.split_once(',') {
        Some((start, count)) => Some((start.parse().ok()?, count.parse().ok()?)),
        None => Some((spec.parse().ok()?, 1)),
    }
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
    // In-progress hunk state, kept separately from `DiffHunk` itself
    // since several of its fields (id, stats, parsed ranges) can
    // only be computed once the hunk is complete.
    let mut current_hunk: Option<(String, Vec<DiffLine>, Vec<String>)> = None;
    // 0-based index of the next hunk within the *current file* —
    // reset to 0 every time a new `diff --git` section starts. This
    // is what makes hunk IDs deterministic and stable regardless of
    // hunk content.
    let mut hunk_index_in_file: usize = 0;

    fn finalize_hunk(
        file: &mut Option<FileDiff>,
        hunk: &mut Option<(String, Vec<DiffLine>, Vec<String>)>,
        hunk_index_in_file: &mut usize,
    ) {
        let (Some(f), Some((header, lines, raw_lines))) = (file.as_mut(), hunk.take()) else {
            return;
        };

        let (old_start, old_count, new_start, new_count) =
            parse_hunk_header_ranges(&header).unwrap_or((0, 0, 0, 0));

        let mut stats = HunkStats::default();
        for line in &lines {
            match line {
                DiffLine::Added(_) => stats.additions += 1,
                DiffLine::Removed(_) => stats.deletions += 1,
                DiffLine::Context(_) => stats.context += 1,
            }
        }
        stats.total_changed_lines = stats.additions + stats.deletions;

        let id = format!("{}#{}", f.path.display(), *hunk_index_in_file);
        *hunk_index_in_file += 1;

        f.hunks.push(DiffHunk {
            id,
            file_path: f.path.clone(),
            old_start,
            old_count,
            new_start,
            new_count,
            header,
            lines,
            raw_content: raw_lines.join("\n"),
            stats,
        });
    }

    for line in diff_text.lines() {
        if let Some(rest) = line.strip_prefix("diff --git a/") {
            // Starting a new file's section: close out whatever was
            // in progress first.
            finalize_hunk(
                &mut current_file,
                &mut current_hunk,
                &mut hunk_index_in_file,
            );
            if let Some(f) = current_file.take() {
                files.push(f);
            }
            hunk_index_in_file = 0;

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
            finalize_hunk(
                &mut current_file,
                &mut current_hunk,
                &mut hunk_index_in_file,
            );
            current_hunk = Some((format!("@@ {header}"), Vec::new(), vec![line.to_string()]));
            continue;
        }

        // Lines below only mean something once we're inside a hunk.
        // Metadata lines before the first hunk of a file (`index
        // ...`, `new file mode ...`, `--- a/X`, `+++ b/Y`,
        // `rename from/to ...`, `similarity index ...`, etc.) are
        // intentionally skipped — Module 3 already captures
        // everything about a file's identity and status that this
        // module needs.
        let Some((_, lines, raw_lines)) = current_hunk.as_mut() else {
            continue;
        };

        if let Some(rest) = line.strip_prefix('+') {
            if let Some(f) = current_file.as_mut() {
                lines.push(DiffLine::Added(rest.to_string()));
                raw_lines.push(line.to_string());
                f.additions += 1;
            }
        } else if let Some(rest) = line.strip_prefix('-') {
            if let Some(f) = current_file.as_mut() {
                lines.push(DiffLine::Removed(rest.to_string()));
                raw_lines.push(line.to_string());
                f.deletions += 1;
            }
        } else if let Some(rest) = line.strip_prefix(' ') {
            lines.push(DiffLine::Context(rest.to_string()));
            raw_lines.push(line.to_string());
        }
        // A line starting with `\` (e.g. "\ No newline at end of
        // file") is a marker, not content — deliberately not
        // pushed as a DiffLine, not counted, and not added to
        // raw_lines (raw_content is "the hunk's actual diff lines,"
        // not every line of surrounding git output).
    }

    finalize_hunk(
        &mut current_file,
        &mut current_hunk,
        &mut hunk_index_in_file,
    );
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
        assert_eq!(
            diff.additions, 2,
            "added exactly two lines (\"four\", \"five\")"
        );

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

        fs::rename(dir.join("original.txt"), dir.join("renamed.txt")).expect("failed to rename");
        write_file(&dir.join("other.txt"), "unrelated\n");
        git(&dir, &["add", "-A"]);

        let diffs = get_staged_diffs(&dir).expect("should succeed");
        assert_eq!(
            diffs.len(),
            2,
            "expected the rename plus the new file, got {diffs:?}"
        );

        let renamed = diffs
            .iter()
            .find(|d| d.path == Path::new("renamed.txt"))
            .expect("renamed.txt should be present");
        // An unmodified rename has no content diff, so no hunks —
        // it's still correctly identified as renamed.txt rather
        // than showing up as a 20-line delete+add.
        assert!(renamed.hunks.is_empty());

        fs::remove_dir_all(&dir).ok();
    }

    // --- Module 16: hunk-level fields ---

    #[test]
    fn hunk_exposes_correct_ranges_and_stats() {
        let dir = init_repo("hunk-ranges-and-stats");
        write_file(&dir.join("README.md"), "one\ntwo\nthree\n");
        git(&dir, &["add", "README.md"]);
        git(&dir, &["commit", "--quiet", "-m", "three lines"]);

        write_file(&dir.join("README.md"), "one\nCHANGED\nthree\nfour\n");
        git(&dir, &["add", "README.md"]);

        let diffs = get_staged_diffs(&dir).expect("should succeed");
        let hunk = &diffs[0].hunks[0];

        assert_eq!(hunk.old_start, 1);
        assert_eq!(hunk.old_count, 3);
        assert_eq!(hunk.new_start, 1);
        assert_eq!(hunk.new_count, 4);
        assert_eq!(hunk.stats.additions, 2, "\"CHANGED\" and \"four\"");
        assert_eq!(hunk.stats.deletions, 1, "\"two\"");
        assert_eq!(hunk.stats.context, 2, "\"one\" and \"three\"");
        assert_eq!(hunk.stats.total_changed_lines, 3);
        assert_eq!(hunk.file_path, PathBuf::from("README.md"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn hunk_ids_are_deterministic_across_repeated_parses() {
        let dir = init_repo("deterministic-ids");
        write_file(&dir.join("README.md"), "changed\n");
        git(&dir, &["add", "README.md"]);

        let first = get_staged_diffs(&dir).expect("should succeed");
        let second = get_staged_diffs(&dir).expect("should succeed");

        assert_eq!(
            first[0].hunks[0].id, second[0].hunks[0].id,
            "the same diff parsed twice must produce the same hunk id"
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn identical_looking_hunks_in_different_files_get_different_ids() {
        let dir = init_repo("identical-looking-hunks");
        write_file(&dir.join("a.txt"), "x\n");
        write_file(&dir.join("b.txt"), "x\n");
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "--quiet", "-m", "two identical files"]);

        // Same single-line change in both files — byte-identical
        // hunk header and content, apart from the file path.
        write_file(&dir.join("a.txt"), "y\n");
        write_file(&dir.join("b.txt"), "y\n");
        git(&dir, &["add", "-A"]);

        let diffs = get_staged_diffs(&dir).expect("should succeed");
        assert_eq!(diffs.len(), 2);

        let hunk_a = &diffs
            .iter()
            .find(|d| d.path == Path::new("a.txt"))
            .unwrap()
            .hunks[0];
        let hunk_b = &diffs
            .iter()
            .find(|d| d.path == Path::new("b.txt"))
            .unwrap()
            .hunks[0];

        assert_eq!(
            hunk_a.header, hunk_b.header,
            "sanity check: headers really are identical"
        );
        assert_ne!(
            hunk_a.id, hunk_b.id,
            "identical-content hunks in different files must still get different ids"
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn hunk_order_is_preserved_and_ids_reflect_position() {
        let dir = init_repo("preserved-order");
        let lines: Vec<String> = (1..=30).map(|n| n.to_string()).collect();
        write_file(&dir.join("nums.txt"), &(lines.join("\n") + "\n"));
        git(&dir, &["add", "nums.txt"]);
        git(&dir, &["commit", "--quiet", "-m", "add nums"]);

        let mut lines = lines;
        lines[1] = "CHANGED_TOP".to_string();
        lines[28] = "CHANGED_BOTTOM".to_string();
        write_file(&dir.join("nums.txt"), &(lines.join("\n") + "\n"));
        git(&dir, &["add", "nums.txt"]);

        let diffs = get_staged_diffs(&dir).expect("should succeed");
        let hunks = &diffs[0].hunks;
        assert_eq!(hunks.len(), 2);

        assert_eq!(hunks[0].id, "nums.txt#0");
        assert_eq!(hunks[1].id, "nums.txt#1");
        assert!(
            hunks[0].old_start < hunks[1].old_start,
            "hunks must stay in the order they appear in the file"
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn omitted_hunk_count_is_treated_as_one() {
        let dir = init_repo("omitted-count");
        // A one-line file guarantees the hunk's old/new spans are
        // exactly 1 line regardless of git's default 3-line context
        // window — there's nothing else in the file to pull in as
        // context. That's what actually triggers the omitted-count
        // header form (confirmed empirically: "@@ -1 +1 @@").
        write_file(&dir.join("f.txt"), "original\n");
        git(&dir, &["add", "f.txt"]);
        git(&dir, &["commit", "--quiet", "-m", "one line"]);

        write_file(&dir.join("f.txt"), "changed\n");
        git(&dir, &["add", "f.txt"]);

        let diffs = get_staged_diffs(&dir).expect("should succeed");
        let hunk = &diffs[0].hunks[0];

        assert_eq!(
            hunk.header, "@@ -1 +1 @@",
            "sanity check: git really did omit both counts"
        );
        assert_eq!(hunk.old_count, 1);
        assert_eq!(hunk.new_count, 1);
        assert_eq!(hunk.old_start, 1);
        assert_eq!(hunk.new_start, 1);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn handles_unicode_content() {
        let dir = init_repo("unicode");
        write_file(&dir.join("u.txt"), "café\n日本語\n");
        git(&dir, &["add", "u.txt"]);
        git(&dir, &["commit", "--quiet", "-m", "unicode base"]);

        write_file(&dir.join("u.txt"), "café résumé\n日本語テスト\n");
        git(&dir, &["add", "u.txt"]);

        let diffs = get_staged_diffs(&dir).expect("should succeed");
        let hunk = &diffs[0].hunks[0];

        assert_eq!(hunk.stats.additions, 2);
        assert_eq!(hunk.stats.deletions, 2);
        assert!(hunk
            .lines
            .iter()
            .any(|l| matches!(l, DiffLine::Added(t) if t.contains("résumé"))));
        assert!(hunk
            .lines
            .iter()
            .any(|l| matches!(l, DiffLine::Added(t) if t.contains("テスト"))));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn handles_crlf_content_without_corrupting_the_parse() {
        let dir = init_repo("crlf");
        fs::write(dir.join("c.txt"), "line1\r\nline2\r\n").expect("failed to write CRLF fixture");
        git(&dir, &["add", "c.txt"]);
        git(&dir, &["commit", "--quiet", "-m", "crlf base"]);

        fs::write(dir.join("c.txt"), "line1\r\nCHANGED\r\n").expect("failed to write CRLF fixture");
        git(&dir, &["add", "c.txt"]);

        let diffs = get_staged_diffs(&dir).expect("should succeed");
        assert_eq!(diffs.len(), 1);
        let hunk = &diffs[0].hunks[0];

        // Parsing must not choke on or miscount CRLF content — one
        // context line, one deletion, one addition. Note: the literal
        // \r is not preserved in the captured text (see raw_content's
        // doc comment) since Rust's `str::lines()` treats it as part
        // of the line terminator, not content; this test intentionally
        // does not assert the \r survives, only that parsing stays
        // correct despite it.
        assert_eq!(hunk.stats.context, 1);
        assert_eq!(hunk.stats.deletions, 1);
        assert_eq!(hunk.stats.additions, 1);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn raw_content_includes_header_and_prefixed_body_lines() {
        let dir = init_repo("raw-content");
        write_file(&dir.join("README.md"), "changed\n");
        git(&dir, &["add", "README.md"]);

        let diffs = get_staged_diffs(&dir).expect("should succeed");
        let hunk = &diffs[0].hunks[0];

        assert!(hunk.raw_content.starts_with(&hunk.header));
        assert!(hunk.raw_content.contains("-initial"));
        assert!(hunk.raw_content.contains("+changed"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn multiple_files_each_index_their_own_hunks_from_zero() {
        let dir = init_repo("multi-file-indexing");
        write_file(&dir.join("a.txt"), "1\n");
        write_file(&dir.join("b.txt"), "1\n");
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "--quiet", "-m", "two files"]);

        write_file(&dir.join("a.txt"), "2\n");
        write_file(&dir.join("b.txt"), "2\n");
        git(&dir, &["add", "-A"]);

        let diffs = get_staged_diffs(&dir).expect("should succeed");
        assert_eq!(diffs.len(), 2);
        for diff in &diffs {
            assert_eq!(
                diff.hunks[0].id,
                format!("{}#0", diff.path.display()),
                "each file's first hunk should be index 0 within that file"
            );
        }

        fs::remove_dir_all(&dir).ok();
    }
}
