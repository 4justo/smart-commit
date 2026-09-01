//! `smartcommit-core` — the Git engine and planning logic for
//! SmartCommit, independent of any CLI or UI.
//!
//! This crate must remain usable without AI/an external LLM, and
//! must not depend on `smartcommit-cli`.
//!
//! # Module 0 status
//! Workspace scaffolding only. No Git behavior is implemented yet —
//! these are placeholder shared types, deliberately left unfilled
//! until the module that actually needs their fields defines them
//! (per AI_DEVELOPMENT_CONTEXT.md guidance: don't speculate on
//! fields ahead of the module that requires them).

/// Placeholder for repository metadata (root path, `.git` location,
/// bare-repo status, etc.). Fields land in Module 1 — Repository
/// Detection.
pub struct RepositoryInfo;

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
    // Compile-time smoke test: these placeholder types exist and
    // are constructible. Real behavior is tested module-by-module
    // as fields and logic are added.
    use super::*;

    #[test]
    fn placeholder_types_are_constructible() {
        let _ = RepositoryInfo;
        let _ = ChangedFile;
        let _ = CommitMessage;
        let _ = CommitPlan;
    }
}
