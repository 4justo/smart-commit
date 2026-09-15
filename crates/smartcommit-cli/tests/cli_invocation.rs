//! Basic CLI invocation test for Module 0.
//!
//! Deliberately uses only `std::process::Command` rather than
//! pulling in a test-only crate like `assert_cmd` — Module 0's
//! dependency list should stay minimal, and this check is simple
//! enough not to need one.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn cargo_bin_path() -> std::path::PathBuf {
    // `cargo test` builds the binary into the workspace target dir
    // before running integration tests; CARGO_BIN_EXE_<name> is set
    // by Cargo for exactly this purpose.
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_smartcommit"))
}

fn temp_messages_file(contents: &str, label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "smartcommit-cli-{label}-{}-{n}.txt",
        std::process::id()
    ));
    fs::write(&path, contents).expect("failed to write temporary messages file");
    path
}

fn temp_dir(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "smartcommit-cli-repo-{label}-{}-{n}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("failed to create temp repo dir");
    dir
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
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
    let output = Command::new("git")
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

/// Builds a temp repo with an initial commit and stages three files
/// specifically chosen so the Module 6 matcher assigns the first two
/// confidently and leaves the third unrelated to either message —
/// mirroring modules.md's own Module 7 CLI example (two assigned
/// groups, one unassigned file).
fn repo_with_plannable_staged_changes(label: &str) -> PathBuf {
    let dir = temp_dir(label);
    git(&dir, &["init", "--quiet"]);
    write_file(&dir.join("README.md"), "placeholder\n");
    git(&dir, &["add", "README.md"]);
    git(&dir, &["commit", "--quiet", "-m", "initial commit"]);

    write_file(&dir.join("src/auth/login.rs"), "fn login() {}\n");
    write_file(&dir.join("src/dashboard/widgets.rs"), "fn widgets() {}\n");
    write_file(&dir.join("src/shared/config.rs"), "fn config() {}\n");
    git(&dir, &["add", "-A"]);

    dir
}

#[test]
fn version_flag_prints_version_and_exits_zero() {
    let output = Command::new(cargo_bin_path())
        .arg("--version")
        .output()
        .expect("failed to run smartcommit binary");

    assert!(
        output.status.success(),
        "smartcommit --version should exit 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("smartcommit"),
        "expected version output to mention the binary name, got: {stdout:?}"
    );
}

#[test]
fn help_flag_prints_help_and_exits_zero() {
    let output = Command::new(cargo_bin_path())
        .arg("--help")
        .output()
        .expect("failed to run smartcommit binary");

    assert!(output.status.success(), "smartcommit --help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.to_lowercase().contains("usage"),
        "expected help output to contain a usage line, got: {stdout:?}"
    );
}

#[test]
fn plan_builds_a_commit_plan_from_staged_changes() {
    let repo = repo_with_plannable_staged_changes("plan-ok");
    let messages_file = temp_messages_file(
        "Fix authentication in login module\n\nImprove dashboard widgets\n",
        "plan-ok",
    );

    let output = Command::new(cargo_bin_path())
        .arg("--repo")
        .arg(&repo)
        .args(["plan", "--messages-file"])
        .arg(&messages_file)
        .output()
        .expect("failed to run smartcommit binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "smartcommit plan should exit 0, stdout: {stdout}"
    );

    assert!(stdout.contains("SMARTCOMMIT PLAN"), "stdout: {stdout}");
    assert!(stdout.contains("3 staged files"), "stdout: {stdout}");
    assert!(stdout.contains("2 proposed commits"), "stdout: {stdout}");
    assert!(
        stdout.contains("[1] Fix authentication in login module"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("src/auth/login.rs"), "stdout: {stdout}");
    assert!(
        stdout.contains("[2] Improve dashboard widgets"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("src/dashboard/widgets.rs"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("Confidence:"), "stdout: {stdout}");
    assert!(stdout.contains("Time:"), "stdout: {stdout}");
    assert!(stdout.contains("Unassigned:"), "stdout: {stdout}");
    assert!(stdout.contains("src/shared/config.rs"), "stdout: {stdout}");

    fs::remove_dir_all(&repo).ok();
}

#[test]
fn plan_reports_no_staged_changes() {
    let dir = temp_dir("plan-no-staged");
    git(&dir, &["init", "--quiet"]);
    write_file(&dir.join("README.md"), "placeholder\n");
    git(&dir, &["add", "README.md"]);
    git(&dir, &["commit", "--quiet", "-m", "initial commit"]);
    // Nothing staged after this.

    let messages_file = temp_messages_file("Fix authentication\n", "plan-no-staged");

    let output = Command::new(cargo_bin_path())
        .arg("--repo")
        .arg(&dir)
        .args(["plan", "--messages-file"])
        .arg(&messages_file)
        .output()
        .expect("failed to run smartcommit binary");

    assert!(
        !output.status.success(),
        "smartcommit plan should fail with nothing staged"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No staged changes found."),
        "stdout: {stdout}"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn plan_rejects_empty_messages_file() {
    // Message-file parsing now happens after repository checks
    // (matches ARCHITECTURE.md §34's documented flow order: discover
    // -> inspect state -> read staged changes -> parse messages), so
    // this test needs a real repo with staged changes to reach that
    // point cleanly, rather than relying on `cargo test`'s ambient
    // working directory the way an earlier version of this test did.
    let dir = temp_dir("plan-empty-messages");
    git(&dir, &["init", "--quiet"]);
    write_file(&dir.join("a.txt"), "1\n");
    git(&dir, &["add", "a.txt"]);
    git(&dir, &["commit", "--quiet", "-m", "init"]);
    write_file(&dir.join("a.txt"), "2\n");
    git(&dir, &["add", "a.txt"]);

    let messages_file = temp_messages_file("   \n\t \n", "plan-empty");

    let output = Command::new(cargo_bin_path())
        .arg("--repo")
        .arg(&dir)
        .args(["plan", "--messages-file"])
        .arg(&messages_file)
        .output()
        .expect("failed to run smartcommit binary");

    assert!(!output.status.success(), "smartcommit plan should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("message input is empty"),
        "expected a parse error in stderr, got: {stderr:?}"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn plan_refuses_when_repository_is_blocked_by_a_merge_conflict() {
    let dir = temp_dir("plan-blocked");
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
    // Deliberately don't check the exit status: this merge is
    // expected to conflict.
    let _ = Command::new("git")
        .args(["merge", "feature"])
        .current_dir(&dir)
        .env("GIT_AUTHOR_NAME", "Test User")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test User")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .output();

    let messages_file = temp_messages_file("Resolve conflict\n", "plan-blocked");

    let output = Command::new(cargo_bin_path())
        .arg("--repo")
        .arg(&dir)
        .args(["plan", "--messages-file"])
        .arg(&messages_file)
        .output()
        .expect("failed to run smartcommit binary");

    assert!(
        !output.status.success(),
        "smartcommit plan should refuse on a blocked repo"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not in a safe state") && stderr.contains("merge is in progress"),
        "stderr: {stderr}"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn commit_dry_run_makes_no_git_modification() {
    let repo = repo_with_plannable_staged_changes("commit-dry-run");
    let messages_file = temp_messages_file(
        "Fix authentication in login module\n\nImprove dashboard widgets\n",
        "commit-dry-run",
    );

    let log_before = git_output(&repo, &["log", "--oneline"]);
    let status_before = git_output(&repo, &["status", "--short"]);

    let output = Command::new(cargo_bin_path())
        .arg("--repo")
        .arg(&repo)
        .args(["commit", "--messages-file"])
        .arg(&messages_file)
        .arg("--dry-run")
        .output()
        .expect("failed to run smartcommit binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "dry-run should exit 0, stdout: {stdout}"
    );
    assert!(stdout.contains("SMARTCOMMIT PLAN"), "stdout: {stdout}");
    assert!(
        stdout.contains("Dry run: no changes were made."),
        "stdout: {stdout}"
    );

    let log_after = git_output(&repo, &["log", "--oneline"]);
    let status_after = git_output(&repo, &["status", "--short"]);
    assert_eq!(log_before, log_after, "dry-run must not create any commits");
    assert_eq!(
        status_before, status_after,
        "dry-run must not change staged/unstaged state"
    );

    fs::remove_dir_all(&repo).ok();
}

#[test]
fn commit_with_yes_creates_real_commits() {
    let repo = repo_with_plannable_staged_changes("commit-yes");
    let messages_file = temp_messages_file(
        "Fix authentication in login module\n\nImprove dashboard widgets\n",
        "commit-yes",
    );

    let output = Command::new(cargo_bin_path())
        .arg("--repo")
        .arg(&repo)
        .args(["commit", "--messages-file"])
        .arg(&messages_file)
        .arg("--yes")
        .output()
        .expect("failed to run smartcommit binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "smartcommit commit --yes should exit 0, stdout: {stdout}"
    );
    assert!(
        stdout.contains("All commits created successfully."),
        "stdout: {stdout}"
    );

    let log = git_output(&repo, &["log", "--oneline", "--reverse"]);
    // initial commit + 2 new ones from the plan.
    assert_eq!(log.lines().count(), 3, "log: {log}");
    assert!(log.contains("Fix authentication in login module"));
    assert!(log.contains("Improve dashboard widgets"));

    // The unassigned file must still be staged — never committed.
    let staged = git_output(&repo, &["diff", "--cached", "--name-only"]);
    assert_eq!(staged, "src/shared/config.rs");

    fs::remove_dir_all(&repo).ok();
}

#[test]
fn commit_without_yes_defaults_to_no_on_empty_input() {
    let repo = repo_with_plannable_staged_changes("commit-declined");
    let messages_file = temp_messages_file(
        "Fix authentication in login module\n\nImprove dashboard widgets\n",
        "commit-declined",
    );

    let log_before = git_output(&repo, &["log", "--oneline"]);

    let mut child = Command::new(cargo_bin_path())
        .arg("--repo")
        .arg(&repo)
        .args(["commit", "--messages-file"])
        .arg(&messages_file)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn smartcommit binary");

    // Pressing Enter with no input — "default must be No".
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"\n")
        .expect("failed to write to child stdin");

    let output = child.wait_with_output().expect("failed to wait on child");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(
        output.status.code(),
        Some(2),
        "declining confirmation should exit with code 2, stdout: {stdout}"
    );
    assert!(stdout.contains("Cancelled."), "stdout: {stdout}");

    let log_after = git_output(&repo, &["log", "--oneline"]);
    assert_eq!(
        log_before, log_after,
        "declining must not create any commits"
    );

    fs::remove_dir_all(&repo).ok();
}

#[test]
fn restore_reverts_a_successful_commit_and_restages_its_files() {
    let repo = repo_with_plannable_staged_changes("restore-success");
    let messages_file = temp_messages_file(
        "Fix authentication in login module\n\nImprove dashboard widgets\n",
        "restore-success",
    );

    let original_head = git_output(&repo, &["rev-parse", "HEAD"]);

    let commit_output = Command::new(cargo_bin_path())
        .arg("--repo")
        .arg(&repo)
        .args(["commit", "--messages-file"])
        .arg(&messages_file)
        .arg("--yes")
        .output()
        .expect("failed to run smartcommit commit");
    assert!(commit_output.status.success());

    let restore_output = Command::new(cargo_bin_path())
        .arg("--repo")
        .arg(&repo)
        .args(["restore", "--yes"])
        .output()
        .expect("failed to run smartcommit restore");

    let stdout = String::from_utf8_lossy(&restore_output.stdout);
    assert!(restore_output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("SMARTCOMMIT RESTORE"), "stdout: {stdout}");
    assert!(stdout.contains("Safety: OK"), "stdout: {stdout}");
    assert!(
        stdout.contains(&format!("Restored to {original_head}")),
        "stdout: {stdout}"
    );

    let head_after = git_output(&repo, &["rev-parse", "HEAD"]);
    assert_eq!(head_after, original_head);

    // The two files SmartCommit had committed should be staged
    // again, exactly as before `commit` ever ran — restore never
    // touches the real index, and it was never touched during
    // execution either, so this falls out automatically.
    // src/shared/config.rs was the unassigned file from the plan:
    // it was never committed at all, so it was staged the entire
    // time and remains so — included here for that reason, not
    // because restore re-staged it.
    let staged = git_output(&repo, &["diff", "--cached", "--name-only"]);
    let mut staged_files: Vec<&str> = staged.lines().collect();
    staged_files.sort();
    assert_eq!(
        staged_files,
        vec![
            "src/auth/login.rs",
            "src/dashboard/widgets.rs",
            "src/shared/config.rs"
        ]
    );

    fs::remove_dir_all(&repo).ok();
}

#[test]
fn restore_refuses_when_a_later_commit_exists() {
    let repo = repo_with_plannable_staged_changes("restore-unsafe");
    let messages_file = temp_messages_file(
        "Fix authentication in login module\n\nImprove dashboard widgets\n",
        "restore-unsafe",
    );

    let commit_output = Command::new(cargo_bin_path())
        .arg("--repo")
        .arg(&repo)
        .args(["commit", "--messages-file"])
        .arg(&messages_file)
        .arg("--yes")
        .output()
        .expect("failed to run smartcommit commit");
    assert!(commit_output.status.success());

    let head_before_restore_attempt = git_output(&repo, &["rev-parse", "HEAD"]);

    write_file(&repo.join("unrelated.txt"), "unrelated work\n");
    git(&repo, &["add", "unrelated.txt"]);
    git(
        &repo,
        &["commit", "--quiet", "-m", "an unrelated later commit"],
    );

    let restore_output = Command::new(cargo_bin_path())
        .arg("--repo")
        .arg(&repo)
        .args(["restore", "--yes"])
        .output()
        .expect("failed to run smartcommit restore");

    let stdout = String::from_utf8_lossy(&restore_output.stdout);
    assert!(!restore_output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("Safety: BLOCKED"), "stdout: {stdout}");
    assert!(stdout.contains("git update-ref"), "stdout: {stdout}");

    // HEAD must have advanced past head_before_restore_attempt (the
    // unrelated commit) and must NOT have been reset by the refused
    // restore.
    let head_after = git_output(&repo, &["rev-parse", "HEAD"]);
    assert_ne!(head_after, head_before_restore_attempt);

    fs::remove_dir_all(&repo).ok();
}

#[test]
fn restore_reports_no_operation_when_none_exists() {
    let dir = temp_dir("restore-none");
    git(&dir, &["init", "--quiet"]);
    write_file(&dir.join("a.txt"), "1\n");
    git(&dir, &["add", "a.txt"]);
    git(&dir, &["commit", "--quiet", "-m", "init"]);

    let output = Command::new(cargo_bin_path())
        .arg("--repo")
        .arg(&dir)
        .args(["restore", "--yes"])
        .output()
        .expect("failed to run smartcommit restore");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No SmartCommit operation found to restore."),
        "stdout: {stdout}"
    );

    fs::remove_dir_all(&dir).ok();
}
