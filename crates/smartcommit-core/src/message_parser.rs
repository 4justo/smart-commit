//! Multiline commit message parsing (Module 5).
//!
//! Converts developer-supplied multiline text into individual
//! intended commit messages. Purely deterministic — no AI involved,
//! per modules.md's Module 5 spec.
//!
//! Two input shapes are supported:
//! - **Blank-line-separated blocks**: each block (one or more
//!   consecutive non-blank lines) becomes one message, preserving
//!   internal line breaks — e.g. a subject line plus body lines with
//!   no blank line between them.
//! - **One-message-per-line**: used only when the input has no blank
//!   lines at all, so a plain list of one-liners doesn't need a
//!   trailing blank line per entry.

use std::str::FromStr;

/// One parsed, developer-supplied commit message. Kept intentionally
/// small — just the message text — since planning/matching fields
/// belong to later modules (Module 6 onward), not this one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitMessage {
    pub text: String,
}

/// A parsed list of commit messages extracted from multiline input.
/// The wrapper makes parsing and validation reusable through
/// `FromStr` rather than a bespoke free function.
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
        // Normalize Windows line endings first so the rest of the
        // parser only has to reason about `\n`.
        let normalized = input.replace("\r\n", "\n");
        let normalized = normalized.trim();

        if normalized.is_empty() {
            return Err(ParseCommitMessagesError::EmptyInput);
        }

        let has_blank_lines = normalized.lines().any(|line| line.trim().is_empty());

        let messages = if has_blank_lines {
            parse_blank_line_separated(normalized)
        } else {
            parse_one_per_line(normalized)
        };

        if messages.is_empty() {
            return Err(ParseCommitMessagesError::EmptyInput);
        }

        Ok(CommitMessages(messages))
    }
}

/// Groups lines into messages separated by one or more blank lines.
/// A block's lines are trimmed individually and rejoined with `\n`,
/// so a message can legitimately span multiple lines (e.g. a subject
/// plus body) as long as nothing blank interrupts it.
fn parse_blank_line_separated(normalized: &str) -> Vec<CommitMessage> {
    let mut messages = Vec::new();
    let mut current = Vec::new();

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

    messages
}

/// One message per non-empty line — used only when the input has no
/// blank lines at all, so a plain list of one-liners doesn't need a
/// trailing blank line after each entry.
fn parse_one_per_line(normalized: &str) -> Vec<CommitMessage> {
    normalized
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|text| CommitMessage {
            text: text.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_message_is_constructible() {
        let _ = CommitMessage {
            text: "Fix authentication".to_string(),
        };
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
            "   \n\t \n".parse::<CommitMessages>().unwrap_err(),
            ParseCommitMessagesError::EmptyInput
        );
    }

    #[test]
    fn preserves_a_multiline_message_within_one_blank_line_separated_block() {
        // A message can legitimately span more than one line (e.g. a
        // subject plus body) as long as no blank line interrupts it.
        // This distinguishes "one multi-line message" from "two
        // one-line messages" — both inputs use blank lines as the
        // separator, but only an actual blank line inside the block
        // would split it.
        let parsed: CommitMessages = "Fix authentication\nAcross multiple lines\n\nUpdate README"
            .parse()
            .expect("parser should succeed");

        assert_eq!(parsed.0.len(), 2);
        assert_eq!(
            parsed.0[0].text,
            "Fix authentication\nAcross multiple lines"
        );
        assert_eq!(parsed.0[1].text, "Update README");
    }

    #[test]
    fn ignores_extra_blank_lines_between_messages() {
        let parsed: CommitMessages = "Fix authentication\n\n\n\nUpdate README"
            .parse()
            .expect("parser should succeed");

        assert_eq!(parsed.0.len(), 2);
        assert_eq!(parsed.0[0].text, "Fix authentication");
        assert_eq!(parsed.0[1].text, "Update README");
    }
}
