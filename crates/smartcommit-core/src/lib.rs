//! `smartcommit-core` — the Git engine and planning logic for
//! SmartCommit, independent of any CLI or UI.
//!
//! This crate must remain usable without AI/an external LLM, and
//! must not depend on `smartcommit-cli`.

use std::str::FromStr;
mod diff;
mod repository;
mod repository_state;
mod staged_changes;

pub use diff::{get_staged_diffs, DiffError, DiffHunk, DiffLine, FileDiff};
pub use repository::{Repository, RepositoryError, RepositoryInfo};
pub use repository_state::{
    inspect_repository_state, InspectorError, RepositorySafety, RepositoryState,
};
pub use staged_changes::{get_staged_changes, ChangeKind, ChangedFile, StagedChangesError};

/// One parsed, developer-supplied commit message.
/// Module 5 keeps this intentionally small: just the message text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitMessage {
    pub text: String,
}

/// A parsed list of commit messages extracted from multiline input.
/// The wrapper makes parsing and validation reusable through `FromStr`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitMessages(pub Vec<CommitMessage>);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseCommitMessagesError {
    #[error("message input is empty")]
    EmptyInput,
}

impl FromStr for CommitMessages {
    type Err = ParseCommitMessagesError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        // Normalize Windows line endings first so the rest of the parser only
        // has to reason about `\n`.
        let normalized = input.replace("\r\n", "\n");
        let normalized = normalized.trim();

        if normalized.is_empty() {
            return Err(ParseCommitMessagesError::EmptyInput);
        }

        let mut messages = Vec::new();
        let mut current = Vec::new();
        let has_blank_lines = normalized.lines().any(|line| line.trim().is_empty());

        // Blank lines separate messages; non-blank lines are collected into
        // the current message block and trimmed before storage.
        for line in normalized.lines() {
            if line.trim().is_empty() {
                if !current.is_empty() {
                    messages.push(CommitMessage {
                        text: current.join("\n"),
                    });
                    current.clear();
                }
                continue;
            }

            current.push(line.trim().to_string());
        }

        if !current.is_empty() {
            messages.push(CommitMessage {
                text: current.join("\n"),
            });
        }

        if messages.is_empty() {
            return Err(ParseCommitMessagesError::EmptyInput);
        }

        if !has_blank_lines {
            messages = normalized
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(|text| CommitMessage {
                    text: text.to_string(),
                })
                .collect();
        }

        Ok(CommitMessages(messages))
    }
}


/// Placeholder for a proposed set of commits (groups + unassigned
/// files). Fields land in Module 7 — Commit Planner.
pub struct CommitPlan;

#[cfg(test)]
mod tests {
    // Keep a small smoke test here so the public Module 5 types stay
    // constructible as the parser evolves.
    use super::*;

    #[test]
    fn placeholder_types_are_constructible() {
        let _ = CommitMessage {
            text: "Fix authentication".to_string(),
        };
        let _ = CommitPlan;
    }

    #[test]
    fn parses_blank_line_separated_messages() {
        let parsed: CommitMessages = "Fix authentication\n\nImprove dashboard UI\n\nUpdate README"
            .parse()
            .expect("parser should succeed");

        assert_eq!(
            parsed.0,
            vec![
                CommitMessage {
                    text: "Fix authentication".to_string(),
                },
                CommitMessage {
                    text: "Improve dashboard UI".to_string(),
                },
                CommitMessage {
                    text: "Update README".to_string(),
                },
            ]
        );
    }

    #[test]
    fn parses_crlf_input_and_trims_spaces() {
        let parsed: CommitMessages = "  Fix authentication  \r\n\r\n  Update README  "
            .parse()
            .expect("parser should succeed");

        assert_eq!(
            parsed.0,
            vec![
                CommitMessage {
                    text: "Fix authentication".to_string(),
                },
                CommitMessage {
                    text: "Update README".to_string(),
                },
            ]
        );
    }

    #[test]
    fn parses_one_message_per_line() {
        let parsed: CommitMessages = "Fix authentication\nImprove dashboard UI\nUpdate README"
            .parse()
            .expect("parser should succeed");

        assert_eq!(parsed.0.len(), 3);
        assert_eq!(parsed.0[0].text, "Fix authentication");
        assert_eq!(parsed.0[1].text, "Improve dashboard UI");
        assert_eq!(parsed.0[2].text, "Update README");
    }

    #[test]
    fn rejects_empty_input() {
        assert_eq!(
            "".parse::<CommitMessages>().unwrap_err(),
            ParseCommitMessagesError::EmptyInput
        );
        assert_eq!(
            "   \n\t ".parse::<CommitMessages>().unwrap_err(),
            ParseCommitMessagesError::EmptyInput
        );
    }
}
