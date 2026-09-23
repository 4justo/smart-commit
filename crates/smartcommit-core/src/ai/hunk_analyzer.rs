#[cfg(feature = "ai")]
use super::config::AIConfig;
#[cfg(feature = "ai")]
use super::provider::{AIProvider, MessageRequest, ProviderError};
#[cfg(feature = "ai")]
use crate::{DiffHunk, DiffLine, FileDiff};
#[cfg(feature = "ai")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "ai")]
use thiserror::Error;

#[cfg(feature = "ai")]
#[derive(Error, Debug)]
pub enum HunkAnalysisError {
    #[error("Provider error: {0}")]
    ProviderError(#[from] ProviderError),

    #[error("No hunks provided for analysis")]
    NoHunks,

    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),

    #[error("Invalid analysis response: {0}")]
    InvalidResponse(String),
}

#[cfg(feature = "ai")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChangeCategory {
    BugFix,
    Feature,
    Refactor,
    Documentation,
    Test,
    Performance,
    Security,
    Style,
    Chore,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HunkAnalysis {
    pub hunk_id: String,
    pub file_path: String,
    pub category: ChangeCategory,
    pub risk_level: RiskLevel,
    pub description: String,
    pub complexity: u8,
    pub related_hunks: Vec<String>,
    pub should_split: bool,
    pub reasoning: String,
}

#[cfg(feature = "ai")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffAnalysis {
    pub hunk_analyses: Vec<HunkAnalysis>,
    pub suggested_groups: Vec<Vec<String>>,
    pub overall_complexity: u8,
    pub overall_risk: RiskLevel,
    pub recommendations: Vec<String>,
}

#[cfg(feature = "ai")]
pub async fn analyze_hunks(
    diffs: &[FileDiff],
    config: &AIConfig,
    provider: &dyn AIProvider,
) -> Result<DiffAnalysis, HunkAnalysisError> {
    let all_hunks: Vec<(&FileDiff, &DiffHunk)> = diffs
        .iter()
        .flat_map(|diff| diff.hunks.iter().map(move |hunk| (diff, hunk)))
        .collect();

    if all_hunks.is_empty() {
        return Err(HunkAnalysisError::NoHunks);
    }

    let hunk_summary = build_hunk_summary(&all_hunks);
    let system_prompt = build_analysis_system_prompt();
    let user_prompt = build_analysis_user_prompt(&hunk_summary);

    let request = MessageRequest {
        system_prompt,
        user_prompt,
        max_tokens: Some(config.generation.max_tokens.unwrap_or(2048)),
        temperature: Some(config.generation.temperature.unwrap_or(0.3)),
    };

    let response = provider.generate_message(&request).await?;

    if response.content.trim().is_empty() {
        return Err(HunkAnalysisError::AnalysisFailed(
            "Empty AI response".to_string(),
        ));
    }

    parse_analysis_response(&response.content, &all_hunks)
}

fn build_hunk_summary(hunks: &[(&FileDiff, &DiffHunk)]) -> String {
    let mut summary = String::new();

    for (i, (diff, hunk)) in hunks.iter().enumerate() {
        summary.push_str(&format!("Hunk {}: {}\n", i, hunk.id));
        summary.push_str(&format!("File: {}\n", diff.path.display()));
        summary.push_str(&format!(
            "Range: @@ -{},{} +{},{} @@\n",
            hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
        ));
        summary.push_str(&format!(
            "Stats: +{} -{} (context: {})\n",
            hunk.stats.additions, hunk.stats.deletions, hunk.stats.context
        ));

        summary.push_str("Content:\n");
        for line in &hunk.lines {
            match line {
                DiffLine::Added(content) => summary.push_str(&format!("+ {}\n", content)),
                DiffLine::Removed(content) => summary.push_str(&format!("- {}\n", content)),
                DiffLine::Context(content) => summary.push_str(&format!("  {}\n", content)),
            }
        }

        summary.push_str("\n");
    }

    summary
}

#[cfg(feature = "ai")]
fn build_analysis_system_prompt() -> String {
    r#"You are an expert code analyst specializing in understanding Git diff hunks.
Your task is to analyze individual code hunks and provide semantic insights.

For each hunk, provide:
1. Category: One of [BugFix, Feature, Refactor, Documentation, Test, Performance, Security, Style, Chore, Other]
2. Risk level: One of [Low, Medium, High, Critical]
3. Description: Brief natural language description of what the hunk does
4. Complexity: 1-10 scale (1 = simple, 10 = very complex)
5. Related hunks: List of hunk IDs that should be grouped with this one
6. Should split: Whether this hunk should be a separate commit (true/false)
7. Reasoning: Brief explanation for your analysis

Then provide:
- Suggested commit groupings (each group as a list of hunk IDs)
- Overall complexity (1-10)
- Overall risk level
- Recommendations for commit strategy

Format your response as JSON with this structure:
{
  "hunk_analyses": [
    {
      "hunk_id": "string",
      "file_path": "string", 
      "category": "string",
      "risk_level": "string",
      "description": "string",
      "complexity": number,
      "related_hunks": ["string"],
      "should_split": boolean,
      "reasoning": "string"
    }
  ],
  "suggested_groups": [["string"]],
  "overall_complexity": number,
  "overall_risk": "string",
  "recommendations": ["string"]
}"#.to_string()
}

#[cfg(feature = "ai")]
fn build_analysis_user_prompt(hunk_summary: &str) -> String {
    format!(
        "Analyze the following Git diff hunks and provide semantic analysis:\n\n{}",
        hunk_summary
    )
}

#[cfg(feature = "ai")]
fn parse_analysis_response(
    response: &str,
    _hunks: &[(&FileDiff, &DiffHunk)],
) -> Result<DiffAnalysis, HunkAnalysisError> {
    let json_start = response.find('{').ok_or_else(|| {
        HunkAnalysisError::InvalidResponse("No JSON found in response".to_string())
    })?;
    let json_end = response.rfind('}').ok_or_else(|| {
        HunkAnalysisError::InvalidResponse("No JSON closing brace found".to_string())
    })?;
    let json_str = &response[json_start..=json_end];

    let analysis: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| HunkAnalysisError::InvalidResponse(format!("JSON parse error: {}", e)))?;

    // Parse hunk analyses
    let hunk_analyses: Vec<HunkAnalysis> = analysis["hunk_analyses"]
        .as_array()
        .ok_or_else(|| {
            HunkAnalysisError::InvalidResponse("Missing hunk_analyses array".to_string())
        })?
        .iter()
        .map(|v| -> Result<HunkAnalysis, HunkAnalysisError> {
            let category_str = v["category"].as_str().unwrap_or("Other");
            let category = parse_category(category_str);
            let risk_str = v["risk_level"].as_str().unwrap_or("Medium");
            let risk_level = parse_risk_level(risk_str);

            Ok(HunkAnalysis {
                hunk_id: v["hunk_id"].as_str().unwrap_or("").to_string(),
                file_path: v["file_path"].as_str().unwrap_or("").to_string(),
                category,
                risk_level,
                description: v["description"].as_str().unwrap_or("").to_string(),
                complexity: v["complexity"].as_u64().unwrap_or(5) as u8,
                related_hunks: v["related_hunks"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|s| s.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                should_split: v["should_split"].as_bool().unwrap_or(false),
                reasoning: v["reasoning"].as_str().unwrap_or("").to_string(),
            })
        })
        .collect::<Result<Vec<_>, HunkAnalysisError>>()?;

    // Parse suggested groups
    let suggested_groups: Vec<Vec<String>> = analysis["suggested_groups"]
        .as_array()
        .ok_or_else(|| {
            HunkAnalysisError::InvalidResponse("Missing suggested_groups array".to_string())
        })?
        .iter()
        .map(|v| {
            v.as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|s| s.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect();

    let overall_complexity = analysis["overall_complexity"].as_u64().unwrap_or(5) as u8;
    let overall_risk_str = analysis["overall_risk"].as_str().unwrap_or("Medium");
    let overall_risk = parse_risk_level(overall_risk_str);

    let recommendations: Vec<String> = analysis["recommendations"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Ok(DiffAnalysis {
        hunk_analyses,
        suggested_groups,
        overall_complexity,
        overall_risk,
        recommendations,
    })
}

#[cfg(feature = "ai")]
fn parse_category(s: &str) -> ChangeCategory {
    match s.to_lowercase().as_str() {
        "bugfix" => ChangeCategory::BugFix,
        "feature" => ChangeCategory::Feature,
        "refactor" => ChangeCategory::Refactor,
        "documentation" => ChangeCategory::Documentation,
        "test" => ChangeCategory::Test,
        "performance" => ChangeCategory::Performance,
        "security" => ChangeCategory::Security,
        "style" => ChangeCategory::Style,
        "chore" => ChangeCategory::Chore,
        other => ChangeCategory::Other(other.to_string()),
    }
}

#[cfg(feature = "ai")]
fn parse_risk_level(s: &str) -> RiskLevel {
    match s.to_lowercase().as_str() {
        "low" => RiskLevel::Low,
        "medium" => RiskLevel::Medium,
        "high" => RiskLevel::High,
        "critical" => RiskLevel::Critical,
        _ => RiskLevel::Medium,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_category() {
        assert_eq!(parse_category("bugfix"), ChangeCategory::BugFix);
        assert_eq!(parse_category("BUGFIX"), ChangeCategory::BugFix);
        assert_eq!(parse_category("feature"), ChangeCategory::Feature);
        assert_eq!(
            parse_category("custom"),
            ChangeCategory::Other("custom".to_string())
        );
    }

    #[test]
    fn test_parse_risk_level() {
        assert_eq!(parse_risk_level("low"), RiskLevel::Low);
        assert_eq!(parse_risk_level("HIGH"), RiskLevel::High);
        assert_eq!(parse_risk_level("unknown"), RiskLevel::Medium);
    }
}
