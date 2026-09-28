use crate::{ChangedFile, CommitMessage, DiffLine, FileDiff};
use std::collections::BTreeSet;
use std::path::Path;

const FILENAME_WEIGHT: f32 = 0.5;
const DIRECTORY_WEIGHT: f32 = 0.3;
const DIFF_WEIGHT: f32 = 0.15;
const EXTENSION_WEIGHT: f32 = 0.1;

const STOPWORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "from", "has", "have", "if",
    "in", "into", "is", "it", "its", "of", "on", "or", "that", "the", "this", "to", "was", "were",
    "will", "with",
];

#[derive(Debug, Clone, PartialEq)]
pub struct MatchScore {
    pub message_id: usize,
    pub file_id: usize,
    pub score: f32,
    pub reasons: Vec<String>,
}

pub fn match_files_to_messages(
    messages: &[CommitMessage],
    files: &[ChangedFile],
    diffs: &[FileDiff],
) -> Vec<MatchScore> {
    let file_evidence: Vec<FileEvidence> = files
        .iter()
        .map(|f| FileEvidence::build(f, diffs))
        .collect();

    let mut results = Vec::with_capacity(messages.len() * files.len());
    for (message_id, message) in messages.iter().enumerate() {
        let message_tokens = tokenize(&message.text);
        for (file_id, evidence) in file_evidence.iter().enumerate() {
            let (score, reasons) = score_pair(&message_tokens, evidence);
            results.push(MatchScore {
                message_id,
                file_id,
                score,
                reasons,
            });
        }
    }
    results
}

struct FileEvidence {
    filename_tokens: BTreeSet<String>,
    directory_tokens: BTreeSet<String>,
    diff_tokens: BTreeSet<String>,
    extension: Option<String>,
}

impl FileEvidence {
    fn build(file: &ChangedFile, diffs: &[FileDiff]) -> Self {
        let path = Path::new(&file.path);

        let filename_tokens = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(tokenize)
            .unwrap_or_default();

        let directory_tokens = path
            .parent()
            .map(|p| tokenize(&p.to_string_lossy()))
            .unwrap_or_default();

        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase);

        let diff_tokens = diffs
            .iter()
            .find(|d| d.path == file.path)
            .map(diff_added_line_tokens)
            .unwrap_or_default();

        FileEvidence {
            filename_tokens,
            directory_tokens,
            diff_tokens,
            extension,
        }
    }
}

fn diff_added_line_tokens(diff: &FileDiff) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    for hunk in &diff.hunks {
        for line in &hunk.lines {
            if let DiffLine::Added(text) = line {
                tokens.extend(tokenize(text));
            }
        }
    }
    tokens
}

fn score_pair(message_tokens: &BTreeSet<String>, evidence: &FileEvidence) -> (f32, Vec<String>) {
    let mut score = 0.0f32;
    let mut reasons = Vec::new();

    for token in message_tokens {
        if let Some(matched) = best_related_token(token, &evidence.filename_tokens) {
            score += FILENAME_WEIGHT;
            reasons.push(format!("matched filename token \"{matched}\""));
        } else if let Some(matched) = best_related_token(token, &evidence.directory_tokens) {
            score += DIRECTORY_WEIGHT;
            reasons.push(format!("matched directory token \"{matched}\""));
        }

        if let Some(matched) = best_related_token(token, &evidence.diff_tokens) {
            score += DIFF_WEIGHT;
            reasons.push(format!("matched diff token \"{matched}\""));
        }

        if let Some(ext) = &evidence.extension {
            if tokens_related(token, ext) {
                score += EXTENSION_WEIGHT;
                reasons.push(format!("matched file extension \"{ext}\""));
            }
        }
    }

    (score.min(1.0), reasons)
}

fn tokens_related(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    a.len() >= 3 && b.len() >= 3 && (a.starts_with(b) || b.starts_with(a))
}

fn best_related_token<'a>(token: &str, set: &'a BTreeSet<String>) -> Option<&'a str> {
    set.iter()
        .find(|candidate| tokens_related(token, candidate))
        .map(String::as_str)
}

fn tokenize(text: &str) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();

    for (i, &c) in chars.iter().enumerate() {
        if c.is_alphanumeric() {
            let is_camel_boundary = i > 0 && c.is_uppercase() && chars[i - 1].is_lowercase();
            if is_camel_boundary {
                push_token(&mut current, &mut tokens);
            }
            current.push(c);
        } else {
            push_token(&mut current, &mut tokens);
        }
    }
    push_token(&mut current, &mut tokens);

    tokens
}

fn push_token(current: &mut String, tokens: &mut BTreeSet<String>) {
    if current.is_empty() {
        return;
    }
    let normalized = normalize(current);
    if is_meaningful(&normalized) {
        tokens.insert(normalized);
    }
    current.clear();
}

fn normalize(token: &str) -> String {
    let lower = token.to_lowercase();
    // Minimal plural stemming: "logins" -> "login". Skipped for
    // words ending "ss" (e.g. "class") so it doesn't mangle them,
    // and for words of length <= 3 where stripping a trailing `s`
    // would leave almost nothing meaningful.
    if lower.len() > 3 && lower.ends_with('s') && !lower.ends_with("ss") {
        lower[..lower.len() - 1].to_string()
    } else {
        lower
    }
}

fn is_meaningful(token: &str) -> bool {
    token.len() >= 2 && !STOPWORDS.contains(&token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChangeKind, DiffHunk};
    use std::path::PathBuf;

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

    fn diff_with_added_lines(path: &str, added_lines: &[&str]) -> FileDiff {
        let lines: Vec<DiffLine> = added_lines
            .iter()
            .map(|l| DiffLine::Added(l.to_string()))
            .collect();
        FileDiff {
            path: PathBuf::from(path),
            hunks: vec![DiffHunk {
                id: format!("{path}#0"),
                file_path: PathBuf::from(path),
                old_start: 1,
                old_count: 1,
                new_start: 1,
                new_count: 1,
                header: "@@ -1,1 +1,1 @@".to_string(),
                lines,
                raw_content: String::new(),
                stats: Default::default(),
                structures: vec![],
            }],
            additions: added_lines.len(),
            deletions: 0,
            is_binary: false,
        }
    }

    fn score_for(results: &[MatchScore], message_id: usize, file_id: usize) -> &MatchScore {
        results
            .iter()
            .find(|r| r.message_id == message_id && r.file_id == file_id)
            .expect("expected a MatchScore for that message/file pair")
    }

    #[test]
    fn obvious_auth_match_scores_highly_with_explained_reasons() {
        let messages = vec![message("Fix login authentication")];
        let files = vec![file("src/auth/login.rs")];
        let diffs = vec![diff_with_added_lines(
            "src/auth/login.rs",
            &["fn validate_login_token() {"],
        )];

        let results = match_files_to_messages(&messages, &files, &diffs);
        let m = score_for(&results, 0, 0);

        assert!(
            m.score > 0.5,
            "expected a strongly-matched auth file to score highly, got {}",
            m.score
        );
        assert!(
            m.reasons
                .iter()
                .any(|r| r.contains("filename token \"login\"")),
            "reasons was {:?}",
            m.reasons
        );
        assert!(
            m.reasons
                .iter()
                .any(|r| r.contains("directory token \"auth\"")),
            "reasons was {:?}",
            m.reasons
        );
        assert!(
            m.reasons.iter().any(|r| r.contains("diff token \"login\"")),
            "reasons was {:?}",
            m.reasons
        );
    }

    #[test]
    fn obvious_docs_match_scores_via_filename_token() {
        let messages = vec![message("Update README docs")];
        let files = vec![file("README.md")];
        let diffs = vec![];

        let results = match_files_to_messages(&messages, &files, &diffs);
        let m = score_for(&results, 0, 0);

        assert!(
            m.score > 0.0,
            "expected README.md to match a message literally mentioning \"README\""
        );
        assert!(m
            .reasons
            .iter()
            .any(|r| r.contains("filename token \"readme\"")));
    }

    #[test]
    fn ambiguous_shared_utility_scores_comparably_for_two_messages() {
        // A shared utils.rs whose diff touches on both an auth
        // change and a dashboard change — neither message should
        // dominate; that's what makes it a planner-level ambiguity
        // case rather than an obvious match either way.
        let messages = vec![message("Fix authentication"), message("Improve dashboard")];
        let files = vec![file("src/utils.rs")];
        let diffs = vec![diff_with_added_lines(
            "src/utils.rs",
            &[
                "fn format_for_login_and_dashboard(",
                "used by auth and dashboard code",
            ],
        )];

        let results = match_files_to_messages(&messages, &files, &diffs);
        let auth_score = score_for(&results, 0, 0).score;
        let dashboard_score = score_for(&results, 1, 0).score;

        assert!(auth_score > 0.0 && dashboard_score > 0.0);
        assert!(
            (auth_score - dashboard_score).abs() < 0.1,
            "expected comparable (ambiguous) scores, got auth={auth_score} dashboard={dashboard_score}"
        );
    }

    #[test]
    fn no_relevant_message_scores_zero_with_no_reasons() {
        let messages = vec![message("Update payment validation")];
        let files = vec![file("src/dashboard/widgets.rs")];
        let diffs = vec![diff_with_added_lines(
            "src/dashboard/widgets.rs",
            &["fn render_widget_grid() {"],
        )];

        let results = match_files_to_messages(&messages, &files, &diffs);
        let m = score_for(&results, 0, 0);

        assert_eq!(m.score, 0.0);
        assert!(m.reasons.is_empty());
    }

    #[test]
    fn multiple_relevant_messages_each_score_independently() {
        let messages = vec![
            message("Fix authentication"),
            message("Improve session handling"),
            message("Update README"),
        ];
        let files = vec![file("src/auth/session.rs")];
        let diffs = vec![];

        let results = match_files_to_messages(&messages, &files, &diffs);

        assert!(
            score_for(&results, 0, 0).score > 0.0,
            "auth message should match via directory token"
        );
        assert!(
            score_for(&results, 1, 0).score > 0.0,
            "session message should match via filename token"
        );
        assert_eq!(
            score_for(&results, 2, 0).score,
            0.0,
            "an unrelated README message should not match src/auth/session.rs"
        );
    }

    #[test]
    fn results_are_deterministic_across_repeated_runs() {
        let messages = vec![message("Fix authentication"), message("Improve dashboard")];
        let files = vec![file("src/auth/login.rs"), file("src/dashboard/widgets.rs")];
        let diffs = vec![
            diff_with_added_lines("src/auth/login.rs", &["validate login token"]),
            diff_with_added_lines("src/dashboard/widgets.rs", &["render widget grid"]),
        ];

        let first = match_files_to_messages(&messages, &files, &diffs);
        let second = match_files_to_messages(&messages, &files, &diffs);

        assert_eq!(
            first, second,
            "identical input must produce identical output, reasons included"
        );
    }

    #[test]
    fn tokenizes_camel_case_and_snake_case_and_kebab_case() {
        assert_eq!(
            tokenize("fooBarBaz"),
            BTreeSet::from(["foo".to_string(), "bar".to_string(), "baz".to_string()])
        );
        assert_eq!(
            tokenize("foo_bar_baz"),
            BTreeSet::from(["foo".to_string(), "bar".to_string(), "baz".to_string()])
        );
        assert_eq!(
            tokenize("foo-bar-baz"),
            BTreeSet::from(["foo".to_string(), "bar".to_string(), "baz".to_string()])
        );
    }
}
