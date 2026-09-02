//! `smartcommit-core` — the Git engine and planning logic for
//! SmartCommit, independent of any CLI or UI.
//!
//! This crate must remain usable without AI/an external LLM, and
//! must not depend on `smartcommit-cli`.

mod repository;

pub use repository::{Repository, RepositoryError, RepositoryInfo};

/// Placeholder for a single changed file (path + change kind).
/// Fields land in Module 3 — Staged Change Discovery.
pub struct ChangedFile;

/// Placeholder for one parsed, developer-supplied commit message.
/// Fields land in Module 5 — Multiline Commit Message Parser.
pub struct CommitMessage;

/// Placeholder for a proposed set of commits (groups + unassigned
/// files). Fields land in Module 7 — Commit Planner.
pub struct CommitPlan;

#[cfg(test)]
mod tests {
    // Compile-time smoke test: the remaining placeholder types exist
    // and are constructible. RepositoryInfo is no longer a
    // placeholder as of Module 1 — see repository.rs for its tests.
    use super::*;

    #[test]
    fn placeholder_types_are_constructible() {
        let _ = ChangedFile;
        let _ = CommitMessage;
        let _ = CommitPlan;
    }
}

