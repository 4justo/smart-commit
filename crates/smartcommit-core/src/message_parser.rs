use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitMessage {
    pub text: String,
}

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
