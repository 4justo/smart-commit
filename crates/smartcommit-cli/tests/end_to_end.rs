//! Module 14 — End-to-End MVP Test Suite.
//!
//! Proves the complete SmartCommit MVP pipeline works safely, as required
//! by `modules.md` Module 14.
//!
//! Every test uses a temporary Git repository and never touches the real
//! SmartCommit repository. Tests are independent and self-contained.
//!
//! The scenarios correspond directly to the Module 14 test list in
//! `modules.md`:
//!
//!  1. Main scenario — three files, three messages, three correct commits
//!  2. No repository
//!  3. No staged changes
//!  4. One message
//!  5. Multiple messages
//!  6. Ambiguous file left unassigned
//!  7. Unassigned file is never committed
//!  8. Added file
//!  9. Deleted file
//! 10. Unstaged changes are preserved end to end
//! 11. Timeline timestamps are applied
//! 12. Dry run makes no Git modification
//! 13. User cancellation exits with code 2
//! 14. Recovery checkpoint is created before commits
//! 15. Restore reverts the operation
//! 16. Binary file in staged set does not crash the tool
//! 17. Invalid config file is rejected before any Git work

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

// ── Shared helpers ────────────────────────────────────────────────────────────

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Path to the built `smartcommit` binary.
fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_smartcommit"))
}

/// Creates a unique temporary directory for one test.
fn temp_dir(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("sc-e2e-{label}-{}-{n}", std::process::id()));
    fs::create_dir_all(&dir).expect("failed to create temp dir");
    dir
}

/// Writes `content` to `path`, creating parent directories as needed.
fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, content).expect("write file");
}

/// Writes a binary file (PNG magic bytes) so git detects it as binary.
fn write_binary_file(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    // PNG magic bytes + NUL bytes — enough for git to label this binary.
    let bytes: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
    fs::write(path, bytes).expect("write binary file");
}

/// Runs a `git` command inside `dir` with standard identity env vars.
fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "Test User")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test User")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .status()
        .unwrap_or_else(|e| panic!("git {args:?}: {e}"));
    assert!(status.success(), "git {args:?} failed in {}", dir.display());
}

/// Runs a `git` command and returns trimmed stdout.
fn git_output(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("git {args:?}: {e}"));
    assert!(
        out.status.success(),
        "git {args:?} failed in {}",
        dir.display()
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Writes a messages file to a temp path and returns that path.
fn messages_file(content: &str, label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let path =
        std::env::temp_dir().join(format!("sc-e2e-msg-{label}-{}-{n}.txt", std::process::id()));
    fs::write(&path, content).expect("write messages file");
    path
}

/// Runs `smartcommit --repo <dir> <args...>` and returns its output.
fn smartcommit(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(bin())
        .arg("--repo")
        .arg(dir)
        .args(args)
        .output()
        .expect("failed to run smartcommit")
}

/// Runs `smartcommit --repo <dir> <args...>`, piping `stdin_bytes` to stdin.
fn smartcommit_with_stdin(dir: &Path, args: &[&str], stdin_bytes: &[u8]) -> std::process::Output {
    let mut child = Command::new(bin())
        .arg("--repo")
        .arg(dir)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn smartcommit");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin_bytes)
        .expect("write stdin");

    child.wait_with_output().expect("wait_with_output")
}

/// Initialises a repo with one initial commit and returns its path.
fn init_repo(label: &str) -> PathBuf {
    let dir = temp_dir(label);
    git(&dir, &["init", "--quiet"]);
    write_file(&dir.join("README.md"), "initial\n");
    git(&dir, &["add", "README.md"]);
    git(&dir, &["commit", "--quiet", "-m", "initial commit"]);
    dir
}

// ── Scenario 1 — Main scenario ────────────────────────────────────────────────

/// Three named files staged, three matching messages. Verifies:
/// - exactly three commits are created (on top of the initial one)
/// - commit messages are in the input order
/// - each commit contains only its own file, not the others
#[test]
fn s01_main_scenario_three_files_three_commits() {
    let repo = init_repo("s01");

    write_file(&repo.join("src/auth.rs"), "fn authenticate() {}\n");
    write_file(&repo.join("src/dashboard.rs"), "fn render_dashboard() {}\n");
    write_file(&repo.join("README.md"), "updated docs\n");
    git(&repo, &["add", "-A"]);

    let mf = messages_file(
        "Fix authentication\n\nImprove dashboard\n\nUpdate documentation\n",
        "s01",
    );

    let out = smartcommit(
        &repo,
        &["commit", "--messages-file", mf.to_str().unwrap(), "--yes"],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "s01 stdout: {stdout}");
    assert!(
        stdout.contains("All commits created successfully."),
        "s01 stdout: {stdout}"
    );

    let log = git_output(&repo, &["log", "--oneline", "--reverse"]);
    let lines: Vec<&str> = log.lines().collect();
    // initial commit + 3 new commits
    assert_eq!(lines.len(), 4, "expected 4 commits total, got:\n{log}");
    assert!(lines[1].contains("Fix authentication"), "log:\n{log}");
    assert!(lines[2].contains("Improve dashboard"), "log:\n{log}");
    assert!(lines[3].contains("Update documentation"), "log:\n{log}");

    // Each commit must contain only its intended file.
    let auth_files = git_output(
        &repo,
        &["diff-tree", "--no-commit-id", "-r", "--name-only", "HEAD^^"],
    );
    assert!(
        auth_files.contains("src/auth.rs"),
        "auth files: {auth_files}"
    );
    assert!(
        !auth_files.contains("src/dashboard.rs"),
        "dashboard.rs leaked into auth commit: {auth_files}"
    );

    let dash_files = git_output(
        &repo,
        &["diff-tree", "--no-commit-id", "-r", "--name-only", "HEAD^"],
    );
    assert!(
        dash_files.contains("src/dashboard.rs"),
        "dash files: {dash_files}"
    );
    assert!(
        !dash_files.contains("src/auth.rs"),
        "auth.rs leaked into dashboard commit: {dash_files}"
    );

    fs::remove_dir_all(&repo).ok();
}

// ── Scenario 2 — No repository ───────────────────────────────────────────────

/// Outside any Git repository the tool must exit non-zero with a
/// meaningful error, not panic.
#[test]
fn s02_no_repository() {
    let not_a_repo = temp_dir("s02");
    let mf = messages_file("Fix something\n", "s02");

    let out = smartcommit(
        &not_a_repo,
        &["plan", "--messages-file", mf.to_str().unwrap()],
    );
    assert!(!out.status.success(), "s02: must fail outside a repository");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.to_lowercase().contains("repository")
            || combined.to_lowercase().contains("git")
            || combined.to_lowercase().contains("not a"),
        "s02: expected a git/repository error, combined output: {combined}"
    );

    fs::remove_dir_all(&not_a_repo).ok();
}

// ── Scenario 3 — No staged changes ───────────────────────────────────────────

#[test]
fn s03_no_staged_changes() {
    let repo = init_repo("s03");
    // Nothing new staged after the initial commit.
    let mf = messages_file("Fix something\n", "s03");

    let out = smartcommit(
        &repo,
        &["commit", "--messages-file", mf.to_str().unwrap(), "--yes"],
    );
    assert!(
        !out.status.success(),
        "s03: must fail with no staged changes"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No staged changes found."),
        "s03 stdout: {stdout}"
    );

    fs::remove_dir_all(&repo).ok();
}

// ── Scenario 4 — One message ─────────────────────────────────────────────────

#[test]
fn s04_one_message_one_commit() {
    let repo = init_repo("s04");

    write_file(&repo.join("src/auth/login.rs"), "fn login() {}\n");
    git(&repo, &["add", "-A"]);

    let mf = messages_file("Fix authentication\n", "s04");

    let out = smartcommit(
        &repo,
        &["commit", "--messages-file", mf.to_str().unwrap(), "--yes"],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "s04 stdout: {stdout}");

    let log = git_output(&repo, &["log", "--oneline"]);
    assert!(log.contains("Fix authentication"), "s04 log: {log}");

    fs::remove_dir_all(&repo).ok();
}

// ── Scenario 5 — Multiple messages ───────────────────────────────────────────

#[test]
fn s05_multiple_messages_produce_multiple_commits() {
    let repo = init_repo("s05");

    write_file(&repo.join("src/auth.rs"), "fn auth() {}\n");
    write_file(&repo.join("src/payment.rs"), "fn pay() {}\n");
    write_file(&repo.join("docs/guide.md"), "# Guide\n");
    git(&repo, &["add", "-A"]);

    let mf = messages_file(
        "Fix authentication\n\nAdd payment validation\n\nUpdate documentation\n",
        "s05",
    );

    let out = smartcommit(
        &repo,
        &["commit", "--messages-file", mf.to_str().unwrap(), "--yes"],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "s05 stdout: {stdout}");

    let log = git_output(&repo, &["log", "--oneline", "--reverse"]);
    assert!(log.contains("Fix authentication"), "s05 log: {log}");
    assert!(log.contains("Add payment validation"), "s05 log: {log}");
    assert!(log.contains("Update documentation"), "s05 log: {log}");

    fs::remove_dir_all(&repo).ok();
}

// ── Scenario 6 — Ambiguous file left unassigned ───────────────────────────────

/// A generic shared-utility file that matches two messages almost equally
/// must appear in `Unassigned:` in the plan, and must never be committed.
#[test]
fn s06_ambiguous_file_is_left_unassigned() {
    let repo = init_repo("s06");

    write_file(&repo.join("src/auth/login.rs"), "fn login() {}\n");
    write_file(&repo.join("src/dashboard/widgets.rs"), "fn widgets() {}\n");
    // Generic name — matcher will not strongly prefer one message.
    write_file(&repo.join("src/shared/utils.rs"), "fn util() {}\n");
    git(&repo, &["add", "-A"]);

    let mf = messages_file(
        "Fix authentication in login module\n\nImprove dashboard widgets\n",
        "s06",
    );

    let plan_out = smartcommit(&repo, &["plan", "--messages-file", mf.to_str().unwrap()]);
    let plan_stdout = String::from_utf8_lossy(&plan_out.stdout);
    assert!(plan_out.status.success(), "s06 plan stdout: {plan_stdout}");
    assert!(
        plan_stdout.contains("Unassigned:"),
        "s06: expected Unassigned section, stdout: {plan_stdout}"
    );
    assert!(
        plan_stdout.contains("src/shared/utils.rs"),
        "s06: utils.rs must appear in Unassigned, stdout: {plan_stdout}"
    );

    // Run commit and verify utils.rs was never committed.
    let commit_out = smartcommit(
        &repo,
        &["commit", "--messages-file", mf.to_str().unwrap(), "--yes"],
    );
    let commit_stdout = String::from_utf8_lossy(&commit_out.stdout);
    assert!(
        commit_out.status.success(),
        "s06 commit stdout: {commit_stdout}"
    );

    let staged = git_output(&repo, &["diff", "--cached", "--name-only"]);
    assert!(
        staged.contains("src/shared/utils.rs"),
        "s06: utils.rs must still be staged after commit, staged: {staged}"
    );

    fs::remove_dir_all(&repo).ok();
}

// ── Scenario 7 — Unassigned file is never committed ───────────────────────────

#[test]
fn s07_unassigned_file_remains_staged_and_not_committed() {
    let repo = init_repo("s07");

    write_file(&repo.join("src/auth/login.rs"), "fn login() {}\n");
    write_file(&repo.join("random_unrelated.rs"), "fn random() {}\n");
    git(&repo, &["add", "-A"]);

    let mf = messages_file("Fix authentication\n", "s07");

    let out = smartcommit(
        &repo,
        &["commit", "--messages-file", mf.to_str().unwrap(), "--yes"],
    );
    assert!(out.status.success());

    // Must still be staged after the operation.
    let staged = git_output(&repo, &["diff", "--cached", "--name-only"]);
    assert!(
        staged.contains("random_unrelated.rs"),
        "s07: unrelated file must still be staged, staged: {staged}"
    );

    // Must not appear in any commit's tree.
    let all_added = git_output(
        &repo,
        &[
            "log",
            "--all",
            "--name-only",
            "--format=",
            "--diff-filter=A",
        ],
    );
    assert!(
        !all_added.contains("random_unrelated.rs"),
        "s07: unrelated file must never be in any commit, all added files: {all_added}"
    );

    fs::remove_dir_all(&repo).ok();
}

// ── Scenario 8 — Added file ───────────────────────────────────────────────────

#[test]
fn s08_added_file_is_committed_correctly() {
    let repo = init_repo("s08");

    write_file(&repo.join("src/payment/validate.rs"), "fn validate() {}\n");
    git(&repo, &["add", "-A"]);

    let mf = messages_file("Add payment validation\n", "s08");

    let out = smartcommit(
        &repo,
        &["commit", "--messages-file", mf.to_str().unwrap(), "--yes"],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "s08 stdout: {stdout}");

    let files_in_commit = git_output(
        &repo,
        &["diff-tree", "--no-commit-id", "-r", "--name-only", "HEAD"],
    );
    assert!(
        files_in_commit.contains("src/payment/validate.rs"),
        "s08: added file missing from commit, files: {files_in_commit}"
    );

    fs::remove_dir_all(&repo).ok();
}

// ── Scenario 9 — Deleted file ─────────────────────────────────────────────────

#[test]
fn s09_deleted_file_is_committed_correctly() {
    let repo = init_repo("s09");

    // Put the file into history first.
    write_file(&repo.join("src/legacy.rs"), "fn old() {}\n");
    git(&repo, &["add", "src/legacy.rs"]);
    git(&repo, &["commit", "--quiet", "-m", "add legacy"]);

    // Stage its deletion.
    git(&repo, &["rm", "--quiet", "src/legacy.rs"]);

    let mf = messages_file("Remove legacy module\n", "s09");

    let out = smartcommit(
        &repo,
        &["commit", "--messages-file", mf.to_str().unwrap(), "--yes"],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "s09 stdout: {stdout}");

    let log = git_output(&repo, &["log", "--oneline"]);
    assert!(log.contains("Remove legacy module"), "s09 log: {log}");

    assert!(
        !repo.join("src/legacy.rs").exists(),
        "s09: deleted file still exists on disk"
    );

    fs::remove_dir_all(&repo).ok();
}

// ── Scenario 10 — Unstaged changes preserved ─────────────────────────────────

/// Unstaged changes — including one stacked on a staged file — must survive
/// the entire `smartcommit commit` operation completely untouched.
#[test]
fn s10_unstaged_changes_are_preserved_end_to_end() {
    let repo = init_repo("s10");

    // v1 goes into history.
    write_file(&repo.join("src/auth/login.rs"), "fn login() { /* v1 */ }\n");
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "--quiet", "-m", "add login"]);

    // Stage v2.
    write_file(
        &repo.join("src/auth/login.rs"),
        "fn login() { /* v2 staged */ }\n",
    );
    git(&repo, &["add", "src/auth/login.rs"]);

    // Leave v3 as an unstaged edit on top of v2.
    write_file(
        &repo.join("src/auth/login.rs"),
        "fn login() { /* v3 unstaged */ }\n",
    );

    // Also add a completely separate unstaged file.
    write_file(&repo.join("scratch.txt"), "work in progress\n");

    let mf = messages_file("Fix authentication\n", "s10");

    let out = smartcommit(
        &repo,
        &["commit", "--messages-file", mf.to_str().unwrap(), "--yes"],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "s10 stdout: {stdout}");

    // Working-tree version must still be v3.
    let wt = fs::read_to_string(repo.join("src/auth/login.rs")).expect("read login.rs");
    assert!(
        wt.contains("v3 unstaged"),
        "s10: working-tree edit was destroyed, content: {wt}"
    );

    // Scratch file must be untouched.
    let scratch = fs::read_to_string(repo.join("scratch.txt")).expect("read scratch.txt");
    assert_eq!(scratch, "work in progress\n");

    // The commit itself must contain the staged (v2) version.
    let committed = git_output(&repo, &["show", "HEAD:src/auth/login.rs"]);
    assert!(
        committed.contains("v2 staged"),
        "s10: commit must contain the staged version, got: {committed}"
    );

    fs::remove_dir_all(&repo).ok();
}

// ── Scenario 11 — Timeline timestamps ────────────────────────────────────────

/// With `--start` and `--interval-minutes`, author dates must match the
/// computed timeline exactly.
#[test]
fn s11_timeline_timestamps_are_applied_to_commits() {
    let repo = init_repo("s11");

    write_file(&repo.join("src/auth/login.rs"), "fn login() {}\n");
    write_file(&repo.join("src/dashboard/widgets.rs"), "fn widgets() {}\n");
    git(&repo, &["add", "-A"]);

    let mf = messages_file(
        "Fix authentication in login module\n\nImprove dashboard widgets\n",
        "s11",
    );

    let out = smartcommit(
        &repo,
        &[
            "commit",
            "--messages-file",
            mf.to_str().unwrap(),
            "--start",
            "2026-01-01T09:00:00+00:00",
            "--interval-minutes",
            "15",
            "--yes",
        ],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "s11 stdout: {stdout}");

    let date1 = git_output(&repo, &["log", "-1", "--format=%aI", "HEAD^"]);
    let date2 = git_output(&repo, &["log", "-1", "--format=%aI", "HEAD"]);

    assert!(date1.starts_with("2026-01-01"), "s11 date1: {date1}");
    assert!(date2.starts_with("2026-01-01"), "s11 date2: {date2}");

    let gap = parse_hhmm(&date2) - parse_hhmm(&date1);
    assert_eq!(
        gap, 15,
        "s11: expected 15-minute gap, got {gap} (date1={date1}, date2={date2})"
    );

    fs::remove_dir_all(&repo).ok();
}

/// Parses `HH:MM` from an ISO 8601 string and returns minutes since midnight.
fn parse_hhmm(iso: &str) -> i64 {
    let time_part = iso.split('T').nth(1).unwrap_or("00:00:00");
    let mut parts = time_part.splitn(3, ':');
    let h: i64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
    let m: i64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
    h * 60 + m
}

// ── Scenario 12 — Dry run ────────────────────────────────────────────────────

#[test]
fn s12_dry_run_makes_no_git_modification() {
    let repo = init_repo("s12");

    write_file(&repo.join("src/auth/login.rs"), "fn login() {}\n");
    write_file(&repo.join("src/dashboard/widgets.rs"), "fn widgets() {}\n");
    git(&repo, &["add", "-A"]);

    let log_before = git_output(&repo, &["log", "--oneline"]);
    let status_before = git_output(&repo, &["status", "--short"]);

    let mf = messages_file(
        "Fix authentication in login module\n\nImprove dashboard widgets\n",
        "s12",
    );

    let out = smartcommit(
        &repo,
        &[
            "commit",
            "--messages-file",
            mf.to_str().unwrap(),
            "--dry-run",
        ],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "s12 stdout: {stdout}");
    assert!(
        stdout.contains("Dry run: no changes were made."),
        "s12 stdout: {stdout}"
    );
    assert!(stdout.contains("SMARTCOMMIT PLAN"), "s12 stdout: {stdout}");

    let log_after = git_output(&repo, &["log", "--oneline"]);
    let status_after = git_output(&repo, &["status", "--short"]);
    assert_eq!(
        log_before, log_after,
        "s12: dry run must not create commits"
    );
    assert_eq!(
        status_before, status_after,
        "s12: dry run must not change staged/unstaged state"
    );

    fs::remove_dir_all(&repo).ok();
}

// ── Scenario 13 — User cancellation ──────────────────────────────────────────

/// Pressing Enter (empty line) at the `[y/N]` prompt must exit with code 2
/// and must not create any commits.
#[test]
fn s13_user_cancellation_exits_with_code_2() {
    let repo = init_repo("s13");

    write_file(&repo.join("src/auth/login.rs"), "fn login() {}\n");
    git(&repo, &["add", "-A"]);

    let log_before = git_output(&repo, &["log", "--oneline"]);

    let mf = messages_file("Fix authentication\n", "s13");

    let out = smartcommit_with_stdin(
        &repo,
        &["commit", "--messages-file", mf.to_str().unwrap()],
        b"\n",
    );
    assert_eq!(
        out.status.code(),
        Some(2),
        "s13: expected exit code 2, got: {:?}",
        out.status.code()
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Cancelled."), "s13 stdout: {stdout}");

    let log_after = git_output(&repo, &["log", "--oneline"]);
    assert_eq!(
        log_before, log_after,
        "s13: cancellation must not create commits"
    );

    fs::remove_dir_all(&repo).ok();
}

// ── Scenario 14 — Recovery checkpoint ────────────────────────────────────────

/// A safety checkpoint ref under `refs/smartcommit/backups/` must be created
/// before any commit, pointing to the original HEAD, and must not appear in
/// `git branch`.
#[test]
fn s14_recovery_checkpoint_is_created_before_commits() {
    let repo = init_repo("s14");

    write_file(&repo.join("src/auth/login.rs"), "fn login() {}\n");
    git(&repo, &["add", "-A"]);

    let original_head = git_output(&repo, &["rev-parse", "HEAD"]);

    let mf = messages_file("Fix authentication\n", "s14");

    let out = smartcommit(
        &repo,
        &["commit", "--messages-file", mf.to_str().unwrap(), "--yes"],
    );
    assert!(out.status.success());

    // At least one backup ref must exist.
    let refs = git_output(&repo, &["for-each-ref", "refs/smartcommit/backups/"]);
    assert!(
        !refs.is_empty(),
        "s14: expected backup ref under refs/smartcommit/backups/"
    );

    // It must resolve to the original HEAD.
    let backup_ref = refs
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(2)
        .expect("ref name from for-each-ref");
    let backup_oid = git_output(&repo, &["rev-parse", backup_ref]);
    assert_eq!(
        backup_oid, original_head,
        "s14: backup ref must point to original HEAD"
    );

    // Must not appear in normal branch listing.
    let branches = git_output(&repo, &["branch", "--list"]);
    assert!(
        !branches.contains("smartcommit"),
        "s14: backup ref must not appear in git branch, branches: {branches}"
    );

    fs::remove_dir_all(&repo).ok();
}

// ── Scenario 15 — Restore ────────────────────────────────────────────────────

/// After a successful `commit`, `restore --yes` must move HEAD back to the
/// original commit and leave the originally-staged files staged again.
#[test]
fn s15_restore_reverts_operation_and_restages_files() {
    let repo = init_repo("s15");

    write_file(&repo.join("src/auth/login.rs"), "fn login() {}\n");
    write_file(&repo.join("src/dashboard/widgets.rs"), "fn widgets() {}\n");
    git(&repo, &["add", "-A"]);

    let original_head = git_output(&repo, &["rev-parse", "HEAD"]);

    let mf = messages_file(
        "Fix authentication in login module\n\nImprove dashboard widgets\n",
        "s15",
    );

    let commit_out = smartcommit(
        &repo,
        &["commit", "--messages-file", mf.to_str().unwrap(), "--yes"],
    );
    assert!(commit_out.status.success());

    // Two commits must now exist on top of the initial one.
    assert_eq!(
        git_output(&repo, &["log", "--oneline", "--reverse"])
            .lines()
            .count(),
        3
    );

    let restore_out = smartcommit(&repo, &["restore", "--yes"]);
    let restore_stdout = String::from_utf8_lossy(&restore_out.stdout);
    assert!(
        restore_out.status.success(),
        "s15: restore should succeed, stdout: {restore_stdout}"
    );
    assert!(
        restore_stdout.contains("SMARTCOMMIT RESTORE"),
        "s15 stdout: {restore_stdout}"
    );
    assert!(
        restore_stdout.contains("Safety: OK"),
        "s15 stdout: {restore_stdout}"
    );
    assert!(
        restore_stdout.contains(&format!("Restored to {original_head}")),
        "s15 stdout: {restore_stdout}"
    );

    // HEAD must be back at the original commit.
    let head_after = git_output(&repo, &["rev-parse", "HEAD"]);
    assert_eq!(head_after, original_head, "s15: HEAD was not restored");

    // Both files must be staged again (index was never touched during execution).
    let staged = git_output(&repo, &["diff", "--cached", "--name-only"]);
    let mut staged_files: Vec<&str> = staged.lines().collect();
    staged_files.sort_unstable();
    assert_eq!(
        staged_files,
        vec!["src/auth/login.rs", "src/dashboard/widgets.rs"],
        "s15: both files must be staged after restore"
    );

    fs::remove_dir_all(&repo).ok();
}

// ── Scenario 16 — Binary file ─────────────────────────────────────────────────

/// A binary file in the staged set must not crash the tool. Both `plan` and
/// `commit` must complete without a Rust panic.
#[test]
fn s16_binary_file_does_not_crash_the_tool() {
    let repo = init_repo("s16");

    write_file(&repo.join("src/auth/login.rs"), "fn login() {}\n");
    write_binary_file(&repo.join("assets/logo.png"));
    git(&repo, &["add", "-A"]);

    let mf = messages_file("Fix authentication\n", "s16");

    // plan must not panic.
    let plan_out = smartcommit(&repo, &["plan", "--messages-file", mf.to_str().unwrap()]);
    let plan_stderr = String::from_utf8_lossy(&plan_out.stderr);
    assert!(
        !plan_stderr.contains("thread 'main' panicked"),
        "s16: plan panicked, stderr: {plan_stderr}"
    );
    let plan_stdout = String::from_utf8_lossy(&plan_out.stdout);
    assert!(
        plan_out.status.success(),
        "s16: plan should succeed, stdout: {plan_stdout}, stderr: {plan_stderr}"
    );

    // commit must not panic.
    let commit_out = smartcommit(
        &repo,
        &["commit", "--messages-file", mf.to_str().unwrap(), "--yes"],
    );
    let commit_stderr = String::from_utf8_lossy(&commit_out.stderr);
    assert!(
        !commit_stderr.contains("thread 'main' panicked"),
        "s16: commit panicked, stderr: {commit_stderr}"
    );

    fs::remove_dir_all(&repo).ok();
}

// ── Scenario 17 — Invalid config ─────────────────────────────────────────────

/// An invalid `.smartcommit.toml` must be caught before any Git work.
/// No commits may be created. No panic is acceptable.
#[test]
fn s17_invalid_config_is_rejected_before_any_git_work() {
    let repo = init_repo("s17");

    write_file(&repo.join("src/auth/login.rs"), "fn login() {}\n");
    git(&repo, &["add", "-A"]);

    let original_head = git_output(&repo, &["rev-parse", "HEAD"]);
    let mf = messages_file("Fix authentication\n", "s17");

    // Case A — syntactically broken TOML.
    write_file(
        &repo.join(".smartcommit.toml"),
        "[matching\nminimum_confidence = 0.70\n",
    );

    let out_a = smartcommit(
        &repo,
        &["commit", "--messages-file", mf.to_str().unwrap(), "--yes"],
    );
    assert!(!out_a.status.success(), "s17a: must fail on broken TOML");
    let stderr_a = String::from_utf8_lossy(&out_a.stderr);
    assert!(
        !stderr_a.contains("thread 'main' panicked"),
        "s17a: panicked, stderr: {stderr_a}"
    );
    assert!(
        stderr_a.contains("smartcommit.toml"),
        "s17a: error must mention .smartcommit.toml, stderr: {stderr_a}"
    );
    assert_eq!(
        git_output(&repo, &["rev-parse", "HEAD"]),
        original_head,
        "s17a: broken TOML must not result in any commits"
    );

    // Case B — valid TOML but out-of-range value.
    write_file(
        &repo.join(".smartcommit.toml"),
        "[matching]\nminimum_confidence = 1.5\n",
    );

    let out_b = smartcommit(
        &repo,
        &["commit", "--messages-file", mf.to_str().unwrap(), "--yes"],
    );
    assert!(
        !out_b.status.success(),
        "s17b: must fail on out-of-range value"
    );
    let stderr_b = String::from_utf8_lossy(&out_b.stderr);
    assert!(
        !stderr_b.contains("thread 'main' panicked"),
        "s17b: panicked, stderr: {stderr_b}"
    );
    assert_eq!(
        git_output(&repo, &["rev-parse", "HEAD"]),
        original_head,
        "s17b: out-of-range config must not result in any commits"
    );

    fs::remove_dir_all(&repo).ok();
}
