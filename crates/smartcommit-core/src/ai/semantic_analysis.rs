//! AI-assisted semantic change analysis (Module 17).
//!
//! Combines deterministic analysis with AI semantic understanding to categorize
//! changes while handling disagreements and AI unavailability.

#[cfg(feature = "ai")]
use super::config::AIConfig;
#[cfg(feature = "ai")]
use super::hunk_analyzer::ChangeCategory;
#[cfg(feature = "ai")]
use super::provider::{AIProvider, MessageRequest, ProviderError};
#[cfg(feature = "ai")]
use crate::{ChangedFile, DiffLine, FileDiff};
#[cfg(feature = "ai")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "ai")]
use thiserror::Error;

#[cfg(feature = "ai")]
/// Error types for semantic analysis.
#[derive(Error, Debug)]
pub enum SemanticAnalysisError {
    #[error("Provider error: {0}")]
    ProviderError(#[from] ProviderError),

    #[error("No changes provided for analysis")]
    NoChanges,

    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),

    #[error("Invalid analysis response: {0}")]
    InvalidResponse(String),
}

#[cfg(feature = "ai")]
/// Deterministic change categories based on file patterns and content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeterministicCategory {
    Feature,
    BugFix,
    Refactor,
    Documentation,
    Test,
    Configuration,
    Dependency,
    Performance,
    Security,
    Formatting,
    Unknown,
}

#[cfg(feature = "ai")]
/// Result of deterministic analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeterministicAnalysis {
    pub category: DeterministicCategory,
    pub confidence: f32,
    pub evidence: Vec<String>,
    pub keywords: Vec<String>,
}

/// AI-provided semantic analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AISemanticAnalysis {
    pub category: ChangeCategory,
    pub confidence: f32,
    pub explanation: String,
    pub keywords: Vec<String>,
    pub suggested_intent: String,
}

#[cfg(feature = "ai")]
/// Combined semantic analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticAnalysis {
    pub category: SemanticCategory,
    pub deterministic_analysis: DeterministicAnalysis,
    pub ai_analysis: Option<AISemanticAnalysis>,
    pub combined_result: CombinedResult,
    pub explanation: String,
}

#[cfg(feature = "ai")]
/// Final semantic category after combining analyses.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "ai", derive(Serialize, Deserialize))]
pub enum SemanticCategory {
    Feature,
    BugFix,
    Refactor,
    Documentation,
    Test,
    Configuration,
    Dependency,
    Performance,
    Security,
    Formatting,
    Unknown,
}

#[cfg(feature = "ai")]
/// How the final result was determined.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CombinedResult {
    /// Deterministic and AI agreed
    Agreement,
    /// Deterministic and AI disagreed - AI preferred due to higher confidence
    AIPreferred,
    /// Deterministic and AI disagreed - deterministic preferred due to higher confidence
    DeterministicPreferred,
    /// Only deterministic analysis available
    DeterministicOnly,
    /// Only AI analysis available
    AIOnly,
    /// Both analyses unavailable
    Unknown,
}

#[cfg(feature = "ai")]
/// Analyze changes semantically using both deterministic and AI analysis.
pub async fn analyze_changes_semantically(
    diffs: &[FileDiff],
    files: &[ChangedFile],
    config: &AIConfig,
    provider: Option<&dyn AIProvider>,
) -> Result<Vec<SemanticAnalysis>, SemanticAnalysisError> {
    if diffs.is_empty() {
        return Err(SemanticAnalysisError::NoChanges);
    }

    let mut results = Vec::new();

    for (diff, file) in diffs.iter().zip(files.iter()) {
        // Perform deterministic analysis
        let deterministic = perform_deterministic_analysis(diff, file);

        // Perform AI analysis if provider is available
        let ai_analysis = if let Some(p) = provider {
            match perform_ai_analysis(diff, file, config, p).await {
                Ok(analysis) => Some(analysis),
                Err(e) => {
                    eprintln!(
                        "AI analysis failed for {}: {e}, using deterministic only",
                        file.path.display()
                    );
                    None
                }
            }
        } else {
            None
        };

        // Combine results
        let combined = combine_analyses(&deterministic, ai_analysis.as_ref());

        results.push(SemanticAnalysis {
            category: combined.category.clone(),
            deterministic_analysis: deterministic,
            ai_analysis,
            combined_result: combined.result_type,
            explanation: combined.explanation,
        });
    }

    Ok(results)
}

#[cfg(feature = "ai")]
/// Perform deterministic analysis based on file patterns and content.
fn perform_deterministic_analysis(diff: &FileDiff, file: &ChangedFile) -> DeterministicAnalysis {
    let mut evidence = Vec::new();
    let mut keywords = Vec::new();
    let mut category = DeterministicCategory::Unknown;
    let mut confidence: f32 = 0.5;

    let path_str = file.path.display().to_string().to_lowercase();

    // File extension analysis
    if path_str.ends_with(".md") || path_str.ends_with(".rst") || path_str.ends_with(".txt") {
        category = DeterministicCategory::Documentation;
        evidence.push("Documentation file extension".to_string());
        confidence = 0.8;
    } else if path_str.ends_with(".toml")
        || path_str.ends_with(".yaml")
        || path_str.ends_with(".yml")
        || path_str.ends_with(".json")
        || path_str.ends_with(".ini")
        || path_str.ends_with(".cfg")
    {
        category = DeterministicCategory::Configuration;
        evidence.push("Configuration file extension".to_string());
        confidence = 0.9;
    } else if path_str.ends_with("cargo.toml")
        || path_str.ends_with("package.json")
        || path_str.ends_with("requirements.txt")
        || path_str.ends_with("gemfile")
    {
        category = DeterministicCategory::Dependency;
        evidence.push("Dependency manifest file".to_string());
        confidence = 0.95;
    } else if path_str.ends_with("_test.rs")
        || path_str.ends_with("_test.py")
        || path_str.ends_with("test_")
        || path_str.contains("/tests/")
    {
        category = DeterministicCategory::Test;
        evidence.push("Test file naming pattern".to_string());
        confidence = 0.9;
    }

    // Directory structure analysis
    if path_str.contains("/docs/") || path_str.contains("/documentation/") {
        category = DeterministicCategory::Documentation;
        evidence.push("Documentation directory".to_string());
        confidence = confidence.max(0.85);
    } else if path_str.contains("/tests/") || path_str.contains("/test/") {
        category = DeterministicCategory::Test;
        evidence.push("Test directory".to_string());
        confidence = confidence.max(0.9);
    } else if path_str.contains("/config/") || path_str.contains("/cfg/") {
        category = DeterministicCategory::Configuration;
        evidence.push("Configuration directory".to_string());
        confidence = confidence.max(0.85);
    }

    // Diff content analysis
    for hunk in &diff.hunks {
        for line in &hunk.lines {
            match line {
                DiffLine::Added(content) => {
                    let content_lower = content.to_lowercase();

                    // Bug fix indicators
                    if content_lower.contains("fix")
                        || content_lower.contains("bug")
                        || content_lower.contains("error")
                        || content_lower.contains("issue")
                    {
                        if category == DeterministicCategory::Unknown {
                            category = DeterministicCategory::BugFix;
                            evidence.push("Bug fix keywords in additions".to_string());
                            confidence = 0.7;
                        }
                        keywords.push("fix".to_string());
                    }

                    // Feature indicators
                    if content_lower.contains("add")
                        || content_lower.contains("new")
                        || content_lower.contains("implement")
                        || content_lower.contains("feature")
                    {
                        if category == DeterministicCategory::Unknown
                            || category == DeterministicCategory::BugFix
                        {
                            category = DeterministicCategory::Feature;
                            evidence.push("Feature keywords in additions".to_string());
                            confidence = 0.65;
                        }
                        keywords.push("feature".to_string());
                    }

                    // Performance indicators
                    if content_lower.contains("optimize")
                        || content_lower.contains("performance")
                        || content_lower.contains("cache")
                        || content_lower.contains("speed")
                    {
                        category = DeterministicCategory::Performance;
                        evidence.push("Performance keywords in additions".to_string());
                        confidence = 0.75;
                        keywords.push("performance".to_string());
                    }

                    // Security indicators
                    if content_lower.contains("security")
                        || content_lower.contains("auth")
                        || content_lower.contains("permission")
                        || content_lower.contains("vulnerability")
                    {
                        category = DeterministicCategory::Security;
                        evidence.push("Security keywords in additions".to_string());
                        confidence = 0.8;
                        keywords.push("security".to_string());
                    }

                    // Refactor indicators
                    if content_lower.contains("refactor")
                        || content_lower.contains("clean")
                        || content_lower.contains("simplify")
                        || content_lower.contains("restructure")
                    {
                        category = DeterministicCategory::Refactor;
                        evidence.push("Refactor keywords in additions".to_string());
                        confidence = 0.7;
                        keywords.push("refactor".to_string());
                    }
                }
                DiffLine::Removed(content) => {
                    let content_lower = content.to_lowercase();

                    // Dependency changes
                    if content_lower.contains("require")
                        || content_lower.contains("import")
                        || content_lower.contains("use")
                    {
                        if category == DeterministicCategory::Unknown {
                            category = DeterministicCategory::Dependency;
                            evidence.push("Import/require changes".to_string());
                            confidence = 0.6;
                        }
                        keywords.push("dependency".to_string());
                    }
                }
                DiffLine::Context(_) => {}
            }
        }
    }

    // Change kind analysis
    match file.change_kind {
        crate::ChangeKind::Added => {
            if category == DeterministicCategory::Unknown {
                category = DeterministicCategory::Feature;
                evidence.push("New file addition".to_string());
                confidence = 0.5;
            }
        }
        crate::ChangeKind::Deleted => {
            if category == DeterministicCategory::Unknown {
                category = DeterministicCategory::Refactor;
                evidence.push("File deletion".to_string());
                confidence = 0.5;
            }
        }
        crate::ChangeKind::Renamed => {
            category = DeterministicCategory::Refactor;
            evidence.push("File rename".to_string());
            confidence = 0.9;
        }
        crate::ChangeKind::Modified => {}
    }

    DeterministicAnalysis {
        category,
        confidence,
        evidence,
        keywords,
    }
}

#[cfg(feature = "ai")]
/// Perform AI analysis for semantic understanding.
async fn perform_ai_analysis(
    diff: &FileDiff,
    file: &ChangedFile,
    config: &AIConfig,
    provider: &dyn AIProvider,
) -> Result<AISemanticAnalysis, SemanticAnalysisError> {
    let context = build_semantic_context(diff, file);
    let system_prompt = build_semantic_system_prompt();
    let user_prompt = build_semantic_user_prompt(&context);

    let request = MessageRequest {
        system_prompt,
        user_prompt,
        max_tokens: Some(config.generation.max_tokens.unwrap_or(1024)),
        temperature: Some(config.generation.temperature.unwrap_or(0.3)),
    };

    let response = provider.generate_message(&request).await?;

    if response.content.trim().is_empty() {
        return Err(SemanticAnalysisError::AnalysisFailed(
            "Empty AI response".to_string(),
        ));
    }

    parse_semantic_response(&response.content)
}

#[cfg(feature = "ai")]
/// Build context for AI semantic analysis.
fn build_semantic_context(diff: &FileDiff, file: &ChangedFile) -> String {
    let mut context = String::new();

    context.push_str(&format!("File: {}\n", file.path.display()));
    context.push_str(&format!("Change kind: {:?}\n", file.change_kind));

    if let Some(old_path) = &file.old_path {
        context.push_str(&format!("Previous path: {}\n", old_path.display()));
    }

    context.push_str(&format!(
        "Additions: {}, Deletions: {}\n",
        diff.additions, diff.deletions
    ));
    context.push_str(&format!("Hunks: {}\n", diff.hunks.len()));

    context.push_str("\nDiff content:\n");
    for hunk in &diff.hunks {
        context.push_str(&format!("{}\n", hunk.header));
        for line in &hunk.lines {
            match line {
                DiffLine::Added(content) => context.push_str(&format!("+ {}\n", content)),
                DiffLine::Removed(content) => context.push_str(&format!("- {}\n", content)),
                DiffLine::Context(content) => context.push_str(&format!("  {}\n", content)),
            }
        }
    }

    context
}

#[cfg(feature = "ai")]
/// Build system prompt for semantic analysis.
fn build_semantic_system_prompt() -> String {
    r#"You are an expert code analyst specializing in understanding the semantic meaning of changes.
Your task is to analyze code changes and categorize them semantically.

Categories:
- Feature: New functionality or capabilities
- BugFix: Fixing a bug or error
- Refactor: Code restructuring without changing behavior
- Documentation: Documentation changes
- Test: Test code changes
- Configuration: Configuration file changes
- Dependency: Dependency/requirement changes
- Performance: Performance optimizations
- Security: Security-related changes
- Formatting: Code formatting/style changes
- Other: Changes that don't fit other categories

For each change, provide:
1. Category: One of the categories above
2. Confidence: 0.0-1.0 scale
3. Explanation: Brief explanation of your reasoning
4. Keywords: List of relevant keywords
5. Suggested intent: What the change is trying to accomplish

Format your response as JSON:
{
  "category": "string",
  "confidence": number,
  "explanation": "string",
  "keywords": ["string"],
  "suggested_intent": "string"
}"#
    .to_string()
}

#[cfg(feature = "ai")]
/// Build user prompt with semantic context.
fn build_semantic_user_prompt(context: &str) -> String {
    format!(
        "Analyze the following code change semantically:\n\n{}",
        context
    )
}

#[cfg(feature = "ai")]
/// Parse AI semantic analysis response.
fn parse_semantic_response(response: &str) -> Result<AISemanticAnalysis, SemanticAnalysisError> {
    let json_start = response.find('{').ok_or_else(|| {
        SemanticAnalysisError::InvalidResponse("No JSON found in response".to_string())
    })?;
    let json_end = response.rfind('}').ok_or_else(|| {
        SemanticAnalysisError::InvalidResponse("No JSON closing brace found".to_string())
    })?;
    let json_str = &response[json_start..=json_end];

    let value: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| SemanticAnalysisError::InvalidResponse(format!("JSON parse error: {}", e)))?;

    let category_str = value["category"].as_str().unwrap_or("Other");
    let category = parse_change_category(category_str);

    Ok(AISemanticAnalysis {
        category,
        confidence: value["confidence"].as_f64().unwrap_or(0.5) as f32,
        explanation: value["explanation"].as_str().unwrap_or("").to_string(),
        keywords: value["keywords"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        suggested_intent: value["suggested_intent"].as_str().unwrap_or("").to_string(),
    })
}

#[cfg(feature = "ai")]
/// Parse change category string to enum.
fn parse_change_category(s: &str) -> ChangeCategory {
    match s.to_lowercase().as_str() {
        "feature" => ChangeCategory::Feature,
        "bugfix" => ChangeCategory::BugFix,
        "refactor" => ChangeCategory::Refactor,
        "documentation" => ChangeCategory::Documentation,
        "test" => ChangeCategory::Test,
        "configuration" => ChangeCategory::Style, // Map to Style as closest
        "dependency" => ChangeCategory::Chore,    // Map to Chore
        "performance" => ChangeCategory::Performance,
        "security" => ChangeCategory::BugFix, // Security issues are often bug fixes
        "formatting" => ChangeCategory::Style,
        "other" => ChangeCategory::Other("Other".to_string()),
        _ => ChangeCategory::Other(s.to_string()),
    }
}

#[cfg(feature = "ai")]
/// Combined analysis result.
struct CombinedAnalysis {
    category: SemanticCategory,
    result_type: CombinedResult,
    explanation: String,
}

#[cfg(feature = "ai")]
/// Combine deterministic and AI analyses.
fn combine_analyses(
    deterministic: &DeterministicAnalysis,
    ai: Option<&AISemanticAnalysis>,
) -> CombinedAnalysis {
    match ai {
        Some(ai_analysis) => {
            let det_category = map_deterministic_to_semantic(&deterministic.category);
            let ai_category = map_ai_to_semantic(&ai_analysis.category);

            if det_category == ai_category {
                // Agreement
                CombinedAnalysis {
                    category: det_category,
                    result_type: CombinedResult::Agreement,
                    explanation: format!(
                        "Deterministic and AI agree on {}. Deterministic confidence: {:.2}, AI confidence: {:.2}. Evidence: {}. AI explanation: {}",
                        format!("{:?}", det_category),
                        deterministic.confidence,
                        ai_analysis.confidence,
                        deterministic.evidence.join(", "),
                        ai_analysis.explanation
                    ),
                }
            } else {
                // Disagreement - choose based on confidence
                if ai_analysis.confidence > deterministic.confidence {
                    CombinedAnalysis {
                        category: ai_category,
                        result_type: CombinedResult::AIPreferred,
                        explanation: format!(
                            "Disagreement between deterministic ({:?}, {:.2}) and AI ({:?}, {:.2}). AI preferred due to higher confidence. Deterministic evidence: {}. AI explanation: {}",
                            det_category, deterministic.confidence,
                            ai_category, ai_analysis.confidence,
                            deterministic.evidence.join(", "),
                            ai_analysis.explanation
                        ),
                    }
                } else {
                    CombinedAnalysis {
                        category: det_category,
                        result_type: CombinedResult::DeterministicPreferred,
                        explanation: format!(
                            "Disagreement between deterministic ({:?}, {:.2}) and AI ({:?}, {:.2}). Deterministic preferred due to higher confidence. Deterministic evidence: {}. AI explanation: {}",
                            det_category, deterministic.confidence,
                            ai_category, ai_analysis.confidence,
                            deterministic.evidence.join(", "),
                            ai_analysis.explanation
                        ),
                    }
                }
            }
        }
        None => {
            // Only deterministic available
            let category = map_deterministic_to_semantic(&deterministic.category);
            CombinedAnalysis {
                category,
                result_type: CombinedResult::DeterministicOnly,
                explanation: format!(
                    "Deterministic analysis only (AI unavailable). Category: {:?}, confidence: {:.2}. Evidence: {}",
                    deterministic.category,
                    deterministic.confidence,
                    deterministic.evidence.join(", ")
                ),
            }
        }
    }
}

#[cfg(feature = "ai")]
/// Map deterministic category to semantic category.
fn map_deterministic_to_semantic(det: &DeterministicCategory) -> SemanticCategory {
    match det {
        DeterministicCategory::Feature => SemanticCategory::Feature,
        DeterministicCategory::BugFix => SemanticCategory::BugFix,
        DeterministicCategory::Refactor => SemanticCategory::Refactor,
        DeterministicCategory::Documentation => SemanticCategory::Documentation,
        DeterministicCategory::Test => SemanticCategory::Test,
        DeterministicCategory::Configuration => SemanticCategory::Configuration,
        DeterministicCategory::Dependency => SemanticCategory::Dependency,
        DeterministicCategory::Performance => SemanticCategory::Performance,
        DeterministicCategory::Security => SemanticCategory::Security,
        DeterministicCategory::Formatting => SemanticCategory::Formatting,
        DeterministicCategory::Unknown => SemanticCategory::Unknown,
    }
}

#[cfg(feature = "ai")]
/// Map AI category to semantic category.
fn map_ai_to_semantic(ai: &ChangeCategory) -> SemanticCategory {
    match ai {
        ChangeCategory::BugFix => SemanticCategory::BugFix,
        ChangeCategory::Feature => SemanticCategory::Feature,
        ChangeCategory::Refactor => SemanticCategory::Refactor,
        ChangeCategory::Documentation => SemanticCategory::Documentation,
        ChangeCategory::Test => SemanticCategory::Test,
        ChangeCategory::Performance => SemanticCategory::Performance,
        ChangeCategory::Security => SemanticCategory::Security,
        ChangeCategory::Style => SemanticCategory::Formatting,
        ChangeCategory::Chore => SemanticCategory::Configuration,
        ChangeCategory::Other(_s) => SemanticCategory::Unknown,
    }
}

#[cfg(all(feature = "ai", test))]
mod tests {
    use super::*;

    #[test]
    fn test_map_deterministic_to_semantic() {
        assert_eq!(
            map_deterministic_to_semantic(&DeterministicCategory::Feature),
            SemanticCategory::Feature
        );
        assert_eq!(
            map_deterministic_to_semantic(&DeterministicCategory::BugFix),
            SemanticCategory::BugFix
        );
        assert_eq!(
            map_deterministic_to_semantic(&DeterministicCategory::Unknown),
            SemanticCategory::Unknown
        );
    }

    #[test]
    fn test_map_ai_to_semantic() {
        assert_eq!(
            map_ai_to_semantic(&ChangeCategory::Feature),
            SemanticCategory::Feature
        );
        assert_eq!(
            map_ai_to_semantic(&ChangeCategory::BugFix),
            SemanticCategory::BugFix
        );
        assert_eq!(
            map_ai_to_semantic(&ChangeCategory::Style),
            SemanticCategory::Formatting
        );
    }

    #[test]
    fn test_combined_analysis_agreement() {
        let det = DeterministicAnalysis {
            category: DeterministicCategory::Feature,
            confidence: 0.8,
            evidence: vec!["Feature keywords".to_string()],
            keywords: vec!["add".to_string()],
        };

        let ai = AISemanticAnalysis {
            category: ChangeCategory::Feature,
            confidence: 0.9,
            explanation: "New feature added".to_string(),
            keywords: vec!["implement".to_string()],
            suggested_intent: "Add new functionality".to_string(),
        };

        let combined = combine_analyses(&det, Some(&ai));
        assert_eq!(combined.result_type, CombinedResult::Agreement);
        assert_eq!(combined.category, SemanticCategory::Feature);
    }

    #[test]
    fn test_combined_analysis_disagreement_ai_preferred() {
        let det = DeterministicAnalysis {
            category: DeterministicCategory::Feature,
            confidence: 0.6,
            evidence: vec!["File addition".to_string()],
            keywords: vec!["new".to_string()],
        };

        let ai = AISemanticAnalysis {
            category: ChangeCategory::BugFix,
            confidence: 0.8,
            explanation: "Actually fixing a bug".to_string(),
            keywords: vec!["fix".to_string()],
            suggested_intent: "Fix critical bug".to_string(),
        };

        let combined = combine_analyses(&det, Some(&ai));
        assert_eq!(combined.result_type, CombinedResult::AIPreferred);
        assert_eq!(combined.category, SemanticCategory::BugFix);
    }

    #[test]
    fn test_combined_analysis_deterministic_only() {
        let det = DeterministicAnalysis {
            category: DeterministicCategory::Feature,
            confidence: 0.7,
            evidence: vec!["Feature indicators".to_string()],
            keywords: vec!["feature".to_string()],
        };

        let combined = combine_analyses(&det, None);
        assert_eq!(combined.result_type, CombinedResult::DeterministicOnly);
        assert_eq!(combined.category, SemanticCategory::Feature);
    }
}
