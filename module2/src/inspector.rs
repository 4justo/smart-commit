use crate::repo_state::RepositoryState;
use smartcommit_core::{Repository, RepositoryError};
use std::path::Path;
use std::process::Command;

// Errors from Module 2 inspection.
// Keep this narrow: only repository discovery and Git status failures.
#[derive(Debug, thiserror::Error)]
pub enum InspectorError {
    #[error(transparent)]
    Repository(#[from] RepositoryError),

    #[error("failed to run git status: {0}")]
    GitUnavailable(String),

    #[error("git status returned a failure status")]
    StatusFailed,
}

// Inspect the current repository state by asking Git for porcelain v2 output.
// This builds the Module 2 model without mutating the repository.
pub fn inspect_repository_state(
    path: impl AsRef<Path>,
) -> Result<RepositoryState, InspectorError> {
    // Module 1 handles repo discovery and path validation.
    let repo = Repository::discover(path)?;

    let mut state = RepositoryState::default();

    // Ask Git for branch metadata plus tracked/untracked file state in one call.
    let output = Command::new("git")
        .arg("-C")
        .arg(&repo.root)
        .args(["status", "--porcelain=v2", "--branch"])
        .output()
        .map_err(|e| InspectorError::GitUnavailable(e.to_string()))?;

    // A non-zero exit code means Git could not produce a trustworthy status.
    if !output.status.success() {
        return Err(InspectorError::StatusFailed);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        // The branch line tells us whether HEAD is attached or detached.
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            if rest == "(detached)" {
                state.detached_head = true;
            } else {
                state.branch = Some(rest.to_string());
            }
            continue;
        }

        // The oid line gives the current HEAD commit when Git knows it.
        if let Some(rest) = line.strip_prefix("# branch.oid ") {
            if rest != "(initial)" {
                state.head = Some(rest.to_string());
            }
            continue;
        }

        // Record type `1` is a normal tracked file entry.
        if let Some(rest) = line.strip_prefix("1 ") {
            let mut parts = rest.split_whitespace();
            if let Some(xy) = parts.next() {
                let mut chars = xy.chars();
                let index = chars.next().unwrap_or('.');
                let worktree = chars.next().unwrap_or('.');

                if index != '.' {
                    state.has_staged_changes = true;
                }
                if worktree != '.' {
                    state.has_unstaged_changes = true;
                }
            }
            continue;
        }

        // Record type `2` is a renamed or copied tracked file.
        if let Some(rest) = line.strip_prefix("2 ") {
            let mut parts = rest.split_whitespace();
            if let Some(xy) = parts.next() {
                let mut chars = xy.chars();
                let index = chars.next().unwrap_or('.');
                let worktree = chars.next().unwrap_or('.');

                if index != '.' {
                    state.has_staged_changes = true;
                }
                if worktree != '.' {
                    state.has_unstaged_changes = true;
                }
            }
            continue;
        }

        // `?` means Git sees an untracked file in the working tree.
        if line.starts_with("? ") {
            state.has_untracked_files = true;
            continue;
        }

        // `u` means Git has an unresolved conflict for this path.
        if line.starts_with("u ") {
            state.has_conflicts = true;
            state.has_staged_changes = true;
        }
    }

    // These special files are reliable markers for in-progress operations.
    state.merge_in_progress = repo.git_dir.join("MERGE_HEAD").exists();
    state.cherry_pick_in_progress = repo.git_dir.join("CHERRY_PICK_HEAD").exists();

    // Rebase uses either `rebase-apply` or `rebase-merge`, depending on how
    // Git started the operation.
    state.rebase_in_progress = repo.git_dir.join("rebase-apply").exists()
        || repo.git_dir.join("rebase-merge").exists();

    Ok(state)
}
