#[cfg(feature = "ai")]
use super::provider::{AIProvider, MessageRequest, ProviderError};
#[cfg(feature = "ai")]
use crate::diff::StructureKind;
#[cfg(feature = "ai")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "ai")]
use thiserror::Error;

#[cfg(feature = "ai")]
#[derive(Error, Debug)]
pub enum StructureAnalysisError {
    #[error("Provider error: {0}")]
    ProviderError(#[from] ProviderError),

    #[error("No code provided for analysis")]
    NoCode,

    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),

    #[error("Invalid analysis response: {0}")]
    InvalidResponse(String),
}

#[cfg(feature = "ai")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureAnalysis {
    pub structure_type: StructureKind,
    pub confidence: f32,
    pub explanation: String,
    pub is_ambiguous: bool,
    pub suggested_name: Option<String>,
}

#[cfg(feature = "ai")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureAnalysisRequest {
    pub code: String,
    pub language: String,
    pub line_range: (usize, usize),
    pub context: String,
}

#[cfg(feature = "ai")]
pub async fn analyze_structure_ambiguity(
    request: &StructureAnalysisRequest,
    provider: &dyn AIProvider,
) -> Result<StructureAnalysis, StructureAnalysisError> {
    let user_prompt = build_structure_analysis_prompt(request);

    let message_request = MessageRequest {
        system_prompt: "You are an expert code structure analyzer. Respond only with valid JSON.".to_string(),
        user_prompt,
        max_tokens: Some(500),
        temperature: Some(0.3),
    };

    let response = provider.generate_message(&message_request).await?;

    parse_structure_analysis(&response.content)
}

#[cfg(feature = "ai")]
fn build_structure_analysis_prompt(request: &StructureAnalysisRequest) -> String {
    format!(
        r#"Analyze the following code snippet and identify the structure type.

Language: {}
Line range: {}-{}

Code:
```
{}
```

Context:
```
{}
```

Respond in JSON format with the following structure:
{{
    "structure_type": "function|method|class|struct|enum|interface|module|import|export|trait|unknown",
    "confidence": 0.0-1.0,
    "explanation": "Brief explanation of why this structure was identified",
    "is_ambiguous": true/false,
    "suggested_name": "Name of the structure if identifiable, or null"
}}

Focus on:
1. Identifying the primary structure type
2. Providing a confidence score based on clarity
3. Explaining any ambiguity in the code
4. Suggesting a name if the structure has one"#,
        request.language,
        request.line_range.0,
        request.line_range.1,
        request.code,
        request.context
    )
}

#[cfg(feature = "ai")]
fn parse_structure_analysis(response: &str) -> Result<StructureAnalysis, StructureAnalysisError> {
    let json_str = response
        .trim()
        .trim_start_matches("```json")
        .trim_end_matches("```")
        .trim();

    let parsed: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| StructureAnalysisError::InvalidResponse(format!("JSON parse error: {}", e)))?;

    let structure_type_str = parsed["structure_type"]
        .as_str()
        .ok_or_else(|| StructureAnalysisError::InvalidResponse("Missing structure_type".to_string()))?;

    let structure_type = match structure_type_str {
        "function" => StructureKind::Function,
        "method" => StructureKind::Method,
        "class" => StructureKind::Class,
        "struct" => StructureKind::Struct,
        "enum" => StructureKind::Enum,
        "interface" => StructureKind::Interface,
        "module" => StructureKind::Module,
        "import" => StructureKind::Import,
        "export" => StructureKind::Export,
        "trait" => StructureKind::Trait,
        _ => StructureKind::Unknown,
    };

    let confidence = parsed["confidence"]
        .as_f64()
        .ok_or_else(|| StructureAnalysisError::InvalidResponse("Missing or invalid confidence".to_string()))?
        as f32;

    let explanation = parsed["explanation"]
        .as_str()
        .ok_or_else(|| StructureAnalysisError::InvalidResponse("Missing explanation".to_string()))?
        .to_string();

    let is_ambiguous = parsed["is_ambiguous"]
        .as_bool()
        .unwrap_or(false);

    let suggested_name = parsed["suggested_name"]
        .as_str()
        .map(|s| s.to_string());

    Ok(StructureAnalysis {
        structure_type,
        confidence,
        explanation,
        is_ambiguous,
        suggested_name,
    })
}

#[cfg(feature = "ai")]
pub fn should_use_ai_analysis(structures: &[crate::diff::StructureInfo], hunk_line_count: usize) -> bool {
    structures.is_empty() && hunk_line_count > 0
}
