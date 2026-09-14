//! Commit planning (Module 7).
//!
//! Turns parsed commit messages, staged files, and Module 6's match
//! scores into a proposed, non-destructive `CommitPlan`. This module
//! never touches Git — it only groups.
//!
//! # Configuration (Module 13)
//!
//! The two key thresholds that control assignment behaviour —
//! `min_score` and `ambiguity_margin` — are now parameters to
//! [`build_commit_plan`] rather than hardcoded constants. The CLI
//! reads them from [`SmartCommitConfig`](crate::SmartCommitConfig)
//! (loaded via [`load_config`](crate::load_config)) and passes them
//! through, so a project's `.smartcommit.toml` can tune the matcher
//! without recompiling.

use crate::{ChangedFile, CommitMessage, MatchScore};

/// One proposed commit: a message paired with the files the planner
/// assigned to it.
#[derive(Debug, Clone, PartialEq)]
pub struct CommitGroup {
    pub message: String,
    /// Empty when no staged file matched this message with enough
    /// confidence — the group is still reported (per modules.md
    /// rule 6), not dropped, so the developer can see which of
    /// their messages had nothing staged for it.
    pub files: Vec<ChangedFile>,
    /// Mean match score of this group's files, in `[0.0, 1.0]`.
    /// `0.0` for an empty group.
    pub confidence: f32,
}

/// The complete proposed plan: one group per input message, in the
/// same order the developer supplied them, plus every file the
/// planner didn't feel confident assigning anywhere.
#[derive(Debug, Clone, PartialEq)]
pub struct CommitPlan {
    pub groups: Vec<CommitGroup>,
    /// Files with no message scoring high enough, or whose top two
    /// candidate messages were too close to call.
    pub unassigned: Vec<ChangedFile>,
    /// Human-readable validation findings per modules.md's
    /// Validation section: messages with no matching files, files
    /// with no matching message, files matched almost equally to
    /// multiple messages, and (positively) whether every file ended
    /// up assigned. Always in a stable order for the same input.
    pub warnings: Vec<String>,
}

/// Builds a `CommitPlan` from parsed messages, staged files, and
/// their match scores (from `match_files_to_messages`). Does not
/// touch Git in any way — purely an in-memory grouping decision.
///
/// Each file is assigned to at most one message (never split across
/// commits in the MVP — hunk-level splitting is explicitly out of
/// scope per modules.md). A file is assigned to its highest-scoring
/// message only if that score clears `min_score` *and* isn't within
/// `ambiguity_margin` of its second-best candidate; otherwise it
/// goes to `unassigned`.
///
/// # Parameters
///
/// - `messages` — the ordered list of intended commit messages, as
///   parsed from the messages file.
/// - `files` — the staged files returned by `get_staged_changes`.
/// - `match_scores` — the full score matrix from
///   `match_files_to_messages` (every message × file pair).
/// - `min_score` — minimum score for a file to be assigned at all.
///   Corresponds to `matching.minimum_confidence` in config.
///   Typically 0.3. Files below this go to `unassigned`.
/// - `ambiguity_margin` — if the top two candidates are within this
///   margin of each other (and both are real evidence), the file is
///   left unassigned rather than arbitrarily picking one. Corresponds
///   to `matching.ambiguity_margin` in config. Typically 0.1.
pub fn build_commit_plan(
    messages: &[CommitMessage],
    files: &[ChangedFile],
    match_scores: &[MatchScore],
    min_score: f32,
    ambiguity_margin: f32,
) -> CommitPlan {
    let mut assigned: Vec<Option<usize>> = vec![None; files.len()];
    let mut unassigned_reasons: Vec<String> = Vec::new();

    for (file_id, file) in files.iter().enumerate() {
        // Sort this file's candidate messages best-score-first, with
        // message_id as a tiebreaker so ties resolve the same way on
        // every run.
        let mut candidates: Vec<&MatchScore> =
            match_scores.iter().filter(|s| s.file_id == file_id).collect();
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.message_id.cmp(&b.message_id))
        });

        let Some(best) = candidates.first() else {
            // No scores at all for this file (e.g. messages was
            // empty) — nothing to assign it to.
            unassigned_reasons.push(format!(
                "\"{}\" has no candidate messages to match against.",
                file.path.display()
            ));
            continue;
        };

        if best.score < min_score {
            unassigned_reasons.push(format!(
                "\"{}\" has no matching message (best match was \"{}\" at {:.0}%).",
                file.path.display(),
                messages[best.message_id].text,
                best.score * 100.0
            ));
            continue;
        }

        if let Some(second) = candidates.get(1) {
            if second.score > 0.0 && (best.score - second.score) < ambiguity_margin {
                unassigned_reasons.push(format!(
                    "\"{}\" matched almost equally to multiple messages (\"{}\" {:.0}% vs \"{}\" {:.0}%).",
                    file.path.display(),
                    messages[best.message_id].text,
                    best.score * 100.0,
                    messages[second.message_id].text,
                    second.score * 100.0,
                ));
                continue;
            }
        }

        assigned[file_id] = Some(best.message_id);
    }

    let mut groups: Vec<CommitGroup> = messages
        .iter()
        .map(|m| CommitGroup {
            message: m.text.clone(),
            files: Vec::new(),
            confidence: 0.0,
        })
        .collect();
    let mut group_scores: Vec<Vec<f32>> = vec![Vec::new(); messages.len()];

    for (file_id, assignment) in assigned.iter().enumerate() {
        if let Some(message_id) = assignment {
            groups[*message_id].files.push(files[file_id].clone());
            if let Some(s) = match_scores
                .iter()
                .find(|s| s.file_id == file_id && s.message_id == *message_id)
            {
                group_scores[*message_id].push(s.score);
            }
        }
    }

    for (group, scores) in groups.iter_mut().zip(group_scores.iter()) {
        if !scores.is_empty() {
            let mean = scores.iter().sum::<f32>() / scores.len() as f32;
            group.confidence = mean.min(1.0);
        }
    }

    let unassigned: Vec<ChangedFile> = assigned
        .iter()
        .enumerate()
        .filter(|(_, a)| a.is_none())
        .map(|(file_id, _)| files[file_id].clone())
        .collect();

    let mut warnings = Vec::new();
    for group in &groups {
        if group.files.is_empty() {
            warnings.push(format!(
                "Message \"{}\" has no matching files.",
                group.message
            ));
        }
    }
    warnings.extend(unassigned_reasons);
    if !files.is_empty() && unassigned.is_empty() {
        warnings.push("All files were assigned.".to_string());
    }

    CommitPlan {
        groups,
        unassigned,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChangeKind;
    use std::path::PathBuf;

    // Default thresholds used in every test — mirror the built-in defaults
    // so existing behaviour is preserved exactly.
    const MIN: f32 = 0.3;
    const MARGIN: f32 = 0.1;

    fn message(text: &str) -> CommitMessage {
        CommitMessage {
            text: text.to_string(),
        }
    }

    fn file(path: &str) -> ChangedFile {
        ChangedFile {
            path: PathBuf::from(path),
            change_kind: ChangeKind::Modified,
            old_path: None,
        }
    }

    fn score(message_id: usize, file_id: usize, value: f32) -> MatchScore {
        MatchScore {
            message_id,
            file_id,
            score: value,
            reasons: vec![],
        }
    }

    #[test]
    fn successful_one_to_one_grouping() {
        let messages = vec![message("Fix authentication"), message("Improve dashboard")];
        let files = vec![file("src/auth.rs"), file("src/dashboard.rs")];
        let scores = vec![
            score(0, 0, 0.9),
            score(0, 1, 0.0),
            score(1, 0, 0.0),
            score(1, 1, 0.85),
        ];

        let plan = build_commit_plan(&messages, &files, &scores, MIN, MARGIN);

        assert_eq!(plan.groups.len(), 2);
        assert_eq!(plan.groups[0].message, "Fix authentication");
        assert_eq!(plan.groups[0].files, vec![file("src/auth.rs")]);
        assert!((plan.groups[0].confidence - 0.9).abs() < 1e-6);
        assert_eq!(plan.groups[1].message, "Improve dashboard");
        assert_eq!(plan.groups[1].files, vec![file("src/dashboard.rs")]);
        assert!(plan.unassigned.is_empty());
        assert!(plan.warnings.contains(&"All files were assigned.".to_string()));
    }

    #[test]
    fn one_message_to_several_files() {
        let messages = vec![message("Fix authentication")];
        let files = vec![file("src/auth/login.rs"), file("src/auth/jwt.rs")];
        let scores = vec![score(0, 0, 0.8), score(0, 1, 0.75)];

        let plan = build_commit_plan(&messages, &files, &scores, MIN, MARGIN);

        assert_eq!(plan.groups.len(), 1);
        assert_eq!(
            plan.groups[0].files,
            vec![file("src/auth/login.rs"), file("src/auth/jwt.rs")]
        );
        let expected_confidence = (0.8 + 0.75) / 2.0;
        assert!((plan.groups[0].confidence - expected_confidence).abs() < 1e-6);
    }

    #[test]
    fn reports_an_empty_group_with_a_warning() {
        let messages = vec![message("Fix authentication"), message("Add payment validation")];
        let files = vec![file("src/auth.rs")];
        // Only message 0 has any relevant file; message 1 matches nothing.
        let scores = vec![score(0, 0, 0.9), score(1, 0, 0.0)];

        let plan = build_commit_plan(&messages, &files, &scores, MIN, MARGIN);

        assert_eq!(plan.groups.len(), 2, "empty group must still be reported, not dropped");
        assert!(plan.groups[1].files.is_empty());
        assert_eq!(plan.groups[1].confidence, 0.0);
        assert!(plan
            .warnings
            .iter()
            .any(|w| w.contains("Add payment validation") && w.contains("no matching files")));
    }

    #[test]
    fn unassigned_file_with_no_matching_message() {
        let messages = vec![message("Fix authentication")];
        let files = vec![file("src/utils.rs")];
        // Score well below the minimum_confidence threshold.
        let scores = vec![score(0, 0, 0.05)];

        let plan = build_commit_plan(&messages, &files, &scores, MIN, MARGIN);

        assert!(plan.groups[0].files.is_empty());
        assert_eq!(plan.unassigned, vec![file("src/utils.rs")]);
        assert!(plan
            .warnings
            .iter()
            .any(|w| w.contains("src/utils.rs") && w.contains("no matching message")));
    }

    #[test]
    fn ambiguous_file_is_left_unassigned_with_a_warning() {
        let messages = vec![message("Fix authentication"), message("Improve dashboard")];
        let files = vec![file("src/shared/utils.rs")];
        // Both candidates clear the minimum threshold, and are within
        // the ambiguity margin (0.1) of each other.
        let scores = vec![score(0, 0, 0.45), score(1, 0, 0.42)];

        let plan = build_commit_plan(&messages, &files, &scores, MIN, MARGIN);

        assert!(plan.groups[0].files.is_empty());
        assert!(plan.groups[1].files.is_empty());
        assert_eq!(plan.unassigned, vec![file("src/shared/utils.rs")]);
        assert!(plan
            .warnings
            .iter()
            .any(|w| w.contains("matched almost equally")));
    }

    #[test]
    fn produces_deterministic_output_across_repeated_runs() {
        let messages = vec![message("Fix authentication"), message("Improve dashboard")];
        let files = vec![
            file("src/auth.rs"),
            file("src/dashboard.rs"),
            file("src/shared/utils.rs"),
        ];
        let scores = vec![
            score(0, 0, 0.9),
            score(0, 1, 0.0),
            score(0, 2, 0.45),
            score(1, 0, 0.0),
            score(1, 1, 0.85),
            score(1, 2, 0.42),
        ];

        let first  = build_commit_plan(&messages, &files, &scores, MIN, MARGIN);
        let second = build_commit_plan(&messages, &files, &scores, MIN, MARGIN);

        assert_eq!(first, second);
    }

    #[test]
    fn a_close_but_below_threshold_second_place_does_not_trigger_ambiguity() {
        // Second-best is 0.0 (no real evidence at all) — should assign
        // confidently, not treat as ambiguous. Covers the
        // `second.score > 0.0` guard.
        let messages = vec![message("Fix authentication")];
        let files = vec![file("src/auth.rs")];
        let scores = vec![score(0, 0, 0.9)];

        let plan = build_commit_plan(&messages, &files, &scores, MIN, MARGIN);

        assert_eq!(plan.groups[0].files, vec![file("src/auth.rs")]);
        assert!(plan.unassigned.is_empty());
    }

    #[test]
    fn custom_thresholds_are_respected() {
        // With a raised minimum_confidence of 0.8, a file scoring 0.75
        // must be left unassigned even though it would have been assigned
        // at the default 0.3 threshold.
        let messages = vec![message("Fix authentication")];
        let files = vec![file("src/auth.rs")];
        let scores = vec![score(0, 0, 0.75)];

        let plan = build_commit_plan(&messages, &files, &scores, 0.8, MARGIN);

        assert!(plan.groups[0].files.is_empty(), "score 0.75 must be below raised threshold 0.8");
        assert_eq!(plan.unassigned, vec![file("src/auth.rs")]);
    }
}
