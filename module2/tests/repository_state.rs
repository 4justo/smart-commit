use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use module2::inspector::inspect_repository_state;
use module2::repo_state::RepositorySafety;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_dir(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "module2-state-test-{label}-{}-{n}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("failed to create temp dir");
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

    let status = Command::new("git")
        .args(["merge", "feature"])
        .current_dir(&dir)
        .status()
        .expect("failed to run merge");
    assert!(!status.success(), "merge should conflict");

    let state = inspect_repository_state(&dir).expect("inspection should succeed");
    assert!(state.merge_in_progress);
    assert!(state.has_conflicts);
    assert_eq!(state.safety(), RepositorySafety::Blocked);

    fs::remove_dir_all(&dir).ok();
}
