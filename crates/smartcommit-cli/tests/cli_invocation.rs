//! Basic CLI invocation test for Module 0.
//!
//! Deliberately uses only `std::process::Command` rather than
//! pulling in a test-only crate like `assert_cmd` — Module 0's
//! dependency list should stay minimal, and this check is simple
//! enough not to need one.

use std::fs;
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

    assert!(output.status.success(), "smartcommit --version should exit 0");
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
    assert!(output.status.success(), "smartcommit plan should exit 0, stdout: {stdout}");

    assert!(stdout.contains("Commit Plan"), "stdout: {stdout}");
    assert!(stdout.contains("[1] Fix authentication in login module"), "stdout: {stdout}");
    assert!(stdout.contains("src/auth/login.rs"), "stdout: {stdout}");
    assert!(stdout.contains("[2] Improve dashboard widgets"), "stdout: {stdout}");
    assert!(stdout.contains("src/dashboard/widgets.rs"), "stdout: {stdout}");
    assert!(stdout.contains("Confidence:"), "stdout: {stdout}");
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

    assert!(!output.status.success(), "smartcommit plan should fail with nothing staged");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No staged changes found."), "stdout: {stdout}");

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn plan_rejects_empty_messages_file() {
    let messages_file = temp_messages_file("   \n\t \n", "plan-empty");

    let output = Command::new(cargo_bin_path())
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
}

