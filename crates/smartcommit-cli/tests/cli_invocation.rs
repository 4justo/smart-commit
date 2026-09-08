//! Basic CLI invocation test for Module 0.
//!
//! Deliberately uses only `std::process::Command` rather than
//! pulling in a test-only crate like `assert_cmd` — Module 0's
//! dependency list should stay minimal, and this check is simple
//! enough not to need one.

use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::process::Command;
use std::path::PathBuf;

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
fn plan_reads_and_prints_messages_file() {
    let messages_file = temp_messages_file(
        "Fix authentication\n\nImprove dashboard UI\n",
        "plan-ok",
    );

    let output = Command::new(cargo_bin_path())
        .args(["plan", "--messages-file"])
        .arg(&messages_file)
        .output()
        .expect("failed to run smartcommit binary");

    assert!(output.status.success(), "smartcommit plan should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Parsed 2 commit message(s):"),
        "expected plan output to report the parsed message count, got: {stdout:?}"
    );
    assert!(
        stdout.contains("1: Fix authentication"),
        "expected plan output to include the first message, got: {stdout:?}"
    );
    assert!(
        stdout.contains("2: Improve dashboard UI"),
        "expected plan output to include the second message, got: {stdout:?}"
    );
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
