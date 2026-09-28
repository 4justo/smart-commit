#[cfg(feature = "ai")]
use super::config::AIConfig;
#[cfg(feature = "ai")]
use super::provider::{AIProvider, MessageRequest, ProviderError};
#[cfg(feature = "ai")]
use crate::{ChangedFile, FileDiff};
#[cfg(feature = "ai")]
use thiserror::Error;

#[cfg(feature = "ai")]
#[derive(Error, Debug)]
pub enum GenerationError {
    #[error("Provider error: {0}")]
    ProviderError(#[from] ProviderError),

    #[error("No diffs provided for generation")]
    NoDiffs,

    #[error("Diff analysis failed: {0}")]
    DiffAnalysisFailed(String),

    #[error("Generated message is empty")]
    EmptyMessage,

    #[error("Message parsing failed: {0}")]
    MessageParsingFailed(String),
}

#[cfg(feature = "ai")]
pub async fn generate_commit_messages(
    diffs: &[FileDiff],
    files: &[ChangedFile],
    config: &AIConfig,
    provider: &dyn AIProvider,
) -> Result<Vec<String>, GenerationError> {
    if diffs.is_empty() {
        return Err(GenerationError::NoDiffs);
    }

    let diff_summary = summarize_diffs(diffs, files);
    let system_prompt = build_system_prompt();
    let user_prompt = build_user_prompt(&diff_summary);

    let request = MessageRequest {
        system_prompt,
        user_prompt,
        max_tokens: config.generation.max_tokens,
        temperature: config.generation.temperature,
    };

    let response = provider.generate_message(&request).await?;

    if response.content.trim().is_empty() {
        return Err(GenerationError::EmptyMessage);
    }

    // Parse the AI response into individual commit messages
    let messages = parse_ai_response(&response.content)?;

    Ok(messages)
}

#[cfg(feature = "ai")]
fn summarize_diffs(diffs: &[FileDiff], files: &[ChangedFile]) -> String {
    let mut summary = String::new();

    for (diff, file) in diffs.iter().zip(files.iter()) {
        summary.push_str(&format!("File: {}\n", file.path.display()));

        match &file.change_kind {
            crate::ChangeKind::Added => summary.push_str("Status: Added\n"),
            crate::ChangeKind::Modified => summary.push_str("Status: Modified\n"),
            crate::ChangeKind::Deleted => summary.push_str("Status: Deleted\n"),
            crate::ChangeKind::Renamed => summary.push_str("Status: Renamed\n"),
        }

        if !diff.hunks.is_empty() {
            summary.push_str("Changes:\n");
            for (hunk_index, hunk) in diff.hunks.iter().enumerate() {
                summary.push_str(&format!("  Hunk #{}\n", hunk_index));
                
                if !hunk.structures.is_empty() {
                    for structure in &hunk.structures {
                        summary.push_str(&format!("    Structure: {} ({})\n", structure.name, structure.kind));
                        summary.push_str(&format!("    Language: {}\n", structure.language));
                    }
                }
                
                for line in &hunk.lines {
                    match line {
                        crate::DiffLine::Added(content) => {
                            summary.push_str(&format!("    + {}\n", content));
                        }
                        crate::DiffLine::Removed(content) => {
                            summary.push_str(&format!("    - {}\n", content));
                        }
                        crate::DiffLine::Context(content) => {
                            summary.push_str(&format!("      {}\n", content));
                        }
                    }
                }
            }
        }

        summary.push_str("\n");
    }

    summary
}

#[cfg(feature = "ai")]
fn build_system_prompt() -> String {
    r#"You are an expert at writing clear, conventional commit messages. 
Your task is to analyze code changes and propose multiple atomic commit messages 
that logically split the changes into meaningful, focused commits.

Follow these guidelines:
1. Use conventional commit format: type(scope): description
2. Types: feat, fix, docs, style, refactor, test, chore, perf, ci, build, revert
3. Keep messages under 72 characters for the first line
4. Use imperative mood ("add" not "added", "fix" not "fixed")
5. Focus on WHAT and WHY, not HOW
6. Each commit should be atomic and focused on one logical change
7. Group related changes together in a single commit
8. Separate unrelated changes into different commits

Return your response as a list of commit messages, one per line."#
        .to_string()
}

#[cfg(feature = "ai")]
fn build_user_prompt(diff_summary: &str) -> String {
    format!(
        "Analyze the following code changes and propose multiple atomic commit messages:\n\n{}",
        diff_summary
    )
}

#[cfg(feature = "ai")]
fn parse_ai_response(response: &str) -> Result<Vec<String>, GenerationError> {
    let messages: Vec<String> = response
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect();

    if messages.is_empty() {
        return Err(GenerationError::MessageParsingFailed(
            "No messages found in response".to_string(),
        ));
    }

    Ok(messages)
}

#[cfg(feature = "ai")]
pub async fn generate_single_commit_message(
    diffs: &[FileDiff],
    files: &[ChangedFile],
    config: &AIConfig,
    provider: &dyn AIProvider,
) -> Result<String, GenerationError> {
    let diff_summary = summarize_diffs(diffs, files);

    let system_prompt = r#"You are an expert at writing clear, conventional commit messages.
Your task is to analyze code changes and write a single, comprehensive commit message
that accurately describes all the changes.

Follow these guidelines:
1. Use conventional commit format: type(scope): description
2. Types: feat, fix, docs, style, refactor, test, chore, perf, ci, build, revert
3. Keep the first line under 72 characters
4. Use imperative mood ("add" not "added", "fix" not "fixed")
5. Focus on WHAT and WHY, not HOW
6. Add a body paragraph if needed to explain complex changes

Return only the commit message, no additional text."#
        .to_string();

    let user_prompt = format!(
        "Analyze the following code changes and write a single commit message:\n\n{}",
        diff_summary
    );

    let request = MessageRequest {
        system_prompt,
        user_prompt,
        max_tokens: config.generation.max_tokens,
        temperature: config.generation.temperature,
    };

    let response = provider.generate_message(&request).await?;

    let message = response.content.trim().to_string();

    if message.is_empty() {
        return Err(GenerationError::EmptyMessage);
    }

    Ok(message)
}
