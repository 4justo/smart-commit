#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RepositoryState {
    pub branch: Option<String>,
    pub head: Option<String>,
    pub detached_head: bool,
    pub merge_in_progress: bool,
    pub rebase_in_progress: bool,
    pub cherry_pick_in_progress: bool,
    pub has_conflicts: bool,
    pub has_staged_changes: bool,
    pub has_unstaged_changes: bool,
    pub has_untracked_files: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositorySafety {
    Safe,
    Warning,
    Blocked,
}

impl RepositoryState {
    // Summarize whether SmartCommit should proceed on this repo state.
    // This is intentionally conservative: anything risky becomes Warning
    // or Blocked rather than guessing.
    pub fn safety(&self) -> RepositorySafety {
        // In-progress Git operations mean the repository is not safe to touch.
        if self.merge_in_progress
            || self.rebase_in_progress
            || self.cherry_pick_in_progress
            || self.has_conflicts
        {
            return RepositorySafety::Blocked;
        }

        // A repo is only "Safe" when there is staged work and no unstable
        // side conditions that could confuse later commit planning.
        if !self.has_staged_changes || self.has_unstaged_changes || self.has_untracked_files {
            return RepositorySafety::Warning;
        }

        RepositorySafety::Safe
    }
}
