#[cfg(feature = "ai")]
use super::config::AIConfig;
#[cfg(feature = "ai")]
use super::provider::{AIProvider, MessageRequest, ProviderError};
#[cfg(feature = "ai")]
use crate::{ChangedFile, CommitMessage, FileDiff};
#[cfg(feature = "ai")]
use thiserror::Error;

#[cfg(feature = "ai")]
#[derive(Error, Debug)]
pub enum SemanticMatchError {
    #[error("Provider error: {0}")]
    ProviderError(#[from] ProviderError),

    #[error("No files or messages provided")]
    NoInput,

    #[error("AI response parsing failed: {0}")]
    ResponseParsingFailed(String),

    #[error("Invalid match assignment: {0}")]
    InvalidAssignment(String),
}

#[cfg(feature = "ai")]
#[derive(Debug, Clone)]
pub struct SemanticMatchResult {
    pub assignments: Vec<Vec<usize>>,

    pub unassigned: Vec<usize>,

    pub confidences: Vec<f32>,
}

#[cfg(feature = "ai")]
pub async fn semantic_file_matching(
    messages: &[CommitMessage],
    files: &[ChangedFile],
    diffs: &[FileDiff],
    _config: &AIConfig,
    provider: &dyn AIProvider,
) -> Result<SemanticMatchResult, SemanticMatchError> {
    if messages.is_empty() || files.is_empty() {
        return Err(SemanticMatchError::NoInput);
    }

    let context = build_matching_context(messages, files, diffs);
    let system_prompt = build_semantic_matching_system_prompt();
    let user_prompt = build_semantic_matching_user_prompt(&context);

    let request = MessageRequest {
        system_prompt,
        user_prompt,
        max_tokens: Some(2048),
        temperature: Some(0.3),
    };

    let response = provider.generate_message(&request).await?;

    parse_semantic_response(&response.content, messages.len(), files.len())
}

#[cfg(feature = "ai")]
fn build_matching_context(
    messages: &[CommitMessage],
    files: &[ChangedFile],
    diffs: &[FileDiff],
) -> String {
    let mut context = String::new();

    context.push_str("COMMIT MESSAGES:\n");
    for (i, msg) in messages.iter().enumerate() {
        context.push_str(&format!("{}. {}\n", i + 1, msg.text));
    }

    context.push_str("\nFILES:\n");
    for (i, file) in files.iter().enumerate() {
        context.push_str(&format!("{}. {}\n", i + 1, file.path.display()));

        match &file.change_kind {
            crate::ChangeKind::Added => context.push_str("   Status: Added\n"),
            crate::ChangeKind::Modified => context.push_str("   Status: Modified\n"),
            crate::ChangeKind::Deleted => context.push_str("   Status: Deleted\n"),
            crate::ChangeKind::Renamed => context.push_str("   Status: Renamed\n"),
        }

        if let Some(diff) = diffs.iter().find(|d| d.path == file.path) {
            if !diff.hunks.is_empty() {
                context.push_str("   Sample changes:\n");
                for hunk in diff.hunks.iter().take(2) {
                    for line in hunk.lines.iter().take(5) {
                        match line {
                            crate::DiffLine::Added(content) => {
                                context.push_str(&format!("   + {}\n", content));
                            }
                            crate::DiffLine::Removed(content) => {
                                context.push_str(&format!("   - {}\n", content));
                            }
                            crate::DiffLine::Context(content) => {
                                context.push_str(&format!("     {}\n", content));
                            }
                        }
                    }
                }
            }
        }
    }

    context
}

#[cfg(feature = "ai")]
fn build_semantic_matching_system_prompt() -> String {
    r#"You are an expert at understanding code changes and their semantic relationships.
Your task is to match files to commit messages based on the actual meaning and purpose
of the changes, not just keyword matching.

Consider:
1. The actual purpose of each change (what problem it solves)
2. Logical grouping of related changes
3. The scope and impact of each change
4. Dependencies between files
5. The intent behind each commit message

Return your response in this exact format:
message_index: file_indices (comma-separated)
confidence: score (0.0-1.0)

Example:
1: 1,3,5
confidence: 0.9
2: 2,4
confidence: 0.8
3: 6
confidence: 0.7

For files that don't clearly belong to any message, include them in a final line:
unassigned: file_indices
confidence: 0.3"#
        .to_string()
}

#[cfg(feature = "ai")]
fn build_semantic_matching_user_prompt(context: &str) -> String {
    format!(
        "Match the following files to the commit messages based on semantic understanding:\n\n{}",
        context
    )
}

#[cfg(feature = "ai")]
fn parse_semantic_response(
    response: &str,
    message_count: usize,
    file_count: usize,
) -> Result<SemanticMatchResult, SemanticMatchError> {
    let mut assignments: Vec<Vec<usize>> = vec![Vec::new(); message_count];
    let mut unassigned: Vec<usize> = Vec::new();
    let mut confidences: Vec<f32> = vec![0.0; message_count];

    let mut current_message: Option<usize> = None;

    for line in response.lines() {
        let line = line.trim();

        if line.starts_with("unassigned:") {
            let files_str = line.strip_prefix("unassigned:").unwrap().trim();
            if !files_str.is_empty() {
                unassigned = parse_file_indices(files_str, file_count)?;
            }
        } else if line.contains(':') {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 {
                let key = parts[0].trim();
                let value = parts[1].trim();

                if key == "confidence" {
                    if let Some(msg_idx) = current_message {
                        let confidence = value.parse::<f32>().unwrap_or(0.5);
                        if msg_idx < message_count {
                            confidences[msg_idx] = confidence;
                        }
                    }
                } else if let Ok(msg_idx) = key.parse::<usize>() {
                    if msg_idx > 0 && msg_idx <= message_count {
                        current_message = Some(msg_idx - 1);
                        let file_indices = parse_file_indices(value, file_count)?;
                        if let Some(idx) = current_message {
                            assignments[idx] = file_indices;
                        }
                    }
                }
            }
        }
    }

    Ok(SemanticMatchResult {
        assignments,
        unassigned,
        confidences,
    })
}

fn parse_file_indices(input: &str, max_index: usize) -> Result<Vec<usize>, SemanticMatchError> {
    let mut indices = Vec::new();

    for part in input.split(',') {
        let part = part.trim();
        if let Ok(idx) = part.parse::<usize>() {
            if idx > 0 && idx <= max_index {
                indices.push(idx - 1); // Convert to 0-based
            }
        }
    }

    Ok(indices)
}

#[cfg(feature = "ai")]
pub async fn hybrid_file_matching(
    messages: &[CommitMessage],
    files: &[ChangedFile],
    diffs: &[FileDiff],
    config: &AIConfig,
    provider: &dyn AIProvider,
) -> Result<SemanticMatchResult, SemanticMatchError> {
    match semantic_file_matching(messages, files, diffs, config, provider).await {
        Ok(result) => Ok(result),
        Err(e) => {
            if config.generation.fallback_to_keywords {
                eprintln!(
                    "AI matching failed, falling back to keyword matching: {}",
                    e
                );
                Ok(keyword_matching_fallback(messages, files, diffs))
            } else {
                Err(e)
            }
        }
    }
}

#[cfg(feature = "ai")]
fn keyword_matching_fallback(
    messages: &[CommitMessage],
    files: &[ChangedFile],
    diffs: &[FileDiff],
) -> SemanticMatchResult {
    let scores = crate::match_files_to_messages(messages, files, diffs);

    let message_count = messages.len();
    let mut assignments: Vec<Vec<usize>> = vec![Vec::new(); message_count];
    let mut unassigned: Vec<usize> = Vec::new();
    let mut confidences: Vec<f32> = vec![0.0; message_count];

    for file_idx in 0..files.len() {
        let mut best_message: Option<usize> = None;
        let mut best_score: f32 = 0.0;

        for msg_idx in 0..message_count {
            let score = scores
                .iter()
                .find(|s| s.message_id == msg_idx && s.file_id == file_idx)
                .map(|s| s.score)
                .unwrap_or(0.0);

            if score > best_score {
                best_score = score;
                best_message = Some(msg_idx);
            }
        }

        if let Some(msg_idx) = best_message {
            if best_score > 0.3 {
                // Minimum confidence threshold
                assignments[msg_idx].push(file_idx);
                confidences[msg_idx] = confidences[msg_idx].max(best_score);
            } else {
                unassigned.push(file_idx);
            }
        } else {
            unassigned.push(file_idx);
        }
    }

    SemanticMatchResult {
        assignments,
        unassigned,
        confidences,
    }
}
