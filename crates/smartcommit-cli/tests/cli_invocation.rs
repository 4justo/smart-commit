//! Basic CLI invocation test for Module 0.
//!
//! Deliberately uses only `std::process::Command` rather than
//! pulling in a test-only crate like `assert_cmd` — Module 0's
//! dependency list should stay minimal, and this check is simple
//! enough not to need one.

use std::process::Command;

fn cargo_bin_path() -> std::path::PathBuf {
    // `cargo test` builds the binary into the workspace target dir
    // before running integration tests; CARGO_BIN_EXE_<name> is set
    // by Cargo for exactly this purpose.
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_smartcommit"))
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
