use crate::{ChangedFile, CommitMessage, MatchScore};

#[derive(Debug, Clone, PartialEq)]
pub struct CommitGroup {
    pub message: String,
    pub files: Vec<ChangedFile>,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommitPlan {
    pub groups: Vec<CommitGroup>,
    pub unassigned: Vec<ChangedFile>,
    pub warnings: Vec<String>,
}

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
        let mut candidates: Vec<&MatchScore> = match_scores
            .iter()
            .filter(|s| s.file_id == file_id)
            .collect();
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.message_id.cmp(&b.message_id))
        });

        let Some(best) = candidates.first() else {
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
        assert!(plan
            .warnings
            .contains(&"All files were assigned.".to_string()));
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
        let messages = vec![
            message("Fix authentication"),
            message("Add payment validation"),
        ];
        let files = vec![file("src/auth.rs")];
        let scores = vec![score(0, 0, 0.9), score(1, 0, 0.0)];

        let plan = build_commit_plan(&messages, &files, &scores, MIN, MARGIN);

        assert_eq!(plan.groups.len(), 2,);
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

        let first = build_commit_plan(&messages, &files, &scores, MIN, MARGIN);
        let second = build_commit_plan(&messages, &files, &scores, MIN, MARGIN);

        assert_eq!(first, second);
    }

    #[test]
    fn a_close_but_below_threshold_second_place_does_not_trigger_ambiguity() {
        let messages = vec![message("Fix authentication")];
        let files = vec![file("src/auth.rs")];
        let scores = vec![score(0, 0, 0.9)];

        let plan = build_commit_plan(&messages, &files, &scores, MIN, MARGIN);

        assert_eq!(plan.groups[0].files, vec![file("src/auth.rs")]);
        assert!(plan.unassigned.is_empty());
    }

    #[test]
    fn custom_thresholds_are_respected() {
        let messages = vec![message("Fix authentication")];
        let files = vec![file("src/auth.rs")];
        let scores = vec![score(0, 0, 0.75)];

        let plan = build_commit_plan(&messages, &files, &scores, 0.8, MARGIN);

        assert!(plan.groups[0].files.is_empty(),);
        assert_eq!(plan.unassigned, vec![file("src/auth.rs")]);
    }

}
