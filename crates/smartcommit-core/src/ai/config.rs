//! AI configuration for SmartCommit.

use serde::{Deserialize, Serialize};

/// AI configuration options.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AIConfig {
    /// Which AI provider to use.
    pub provider: ProviderType,
    
    /// Model selection for the chosen provider.
    pub model: ModelSelection,
    
    /// API configuration (keys, endpoints, etc.).
    pub api: ApiConfig,
    
    /// Generation behavior settings.
    pub generation: GenerationConfig,
}

/// Supported AI providers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Claude,
    OpenAI,
    Local,
}

/// Model selection per provider.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelSelection {
    /// Claude model identifier (e.g., "claude-3-sonnet-20240229").
    pub claude: Option<String>,
    
    /// OpenAI model identifier (e.g., "gpt-4-turbo-preview").
    pub openai: Option<String>,
    
    /// Local LLM model name (e.g., "llama2", "mistral").
    pub local: Option<String>,
}

/// API configuration keys and endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiConfig {
    /// Anthropic API key for Claude.
    pub claude_api_key: Option<String>,
    
    /// OpenAI API key.
    pub openai_api_key: Option<String>,
    
    /// Base URL for local LLM (e.g., "http://localhost:11434").
    pub local_base_url: Option<String>,
}

/// Generation behavior settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GenerationConfig {
    /// Maximum tokens for AI responses.
    pub max_tokens: Option<usize>,
    
    /// Temperature for response randomness (0.0 - 1.0).
    pub temperature: Option<f32>,
    
    /// Whether to use AI for message generation.
    pub enable_message_generation: bool,
    
    /// Whether to use AI for semantic file matching.
    pub enable_semantic_matching: bool,
    
    /// Fallback to keyword matching if AI fails.
    pub fallback_to_keywords: bool,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            provider: ProviderType::Claude,
            model: ModelSelection {
                claude: Some("claude-3-sonnet-20240229".to_string()),
                openai: Some("gpt-4-turbo-preview".to_string()),
                local: Some("llama2".to_string()),
            },
            api: ApiConfig {
                claude_api_key: None,
                openai_api_key: None,
                local_base_url: Some("http://localhost:11434".to_string()),
            },
            generation: GenerationConfig {
                max_tokens: Some(1024),
                temperature: Some(0.7),
                enable_message_generation: true,
                enable_semantic_matching: true,
                fallback_to_keywords: true,
            },
        }
    }
}

/// Load AI configuration from environment variables.
pub fn load_ai_config_from_env() -> AIConfig {
    let mut config = AIConfig::default();
    
    // Override with environment variables if present
    if let Ok(claude_key) = std::env::var("ANTHROPIC_API_KEY") {
        config.api.claude_api_key = Some(claude_key);
    }
    
    if let Ok(openai_key) = std::env::var("OPENAI_API_KEY") {
        config.api.openai_api_key = Some(openai_key);
    }
    
    if let Ok(local_url) = std::env::var("OLLAMA_BASE_URL") {
        config.api.local_base_url = Some(local_url);
    }
    
    // Provider selection from env
    if let Ok(provider) = std::env::var("SMC_AI_PROVIDER") {
        config.provider = match provider.to_lowercase().as_str() {
            "claude" => ProviderType::Claude,
            "openai" => ProviderType::OpenAI,
            "local" => ProviderType::Local,
            _ => config.provider, // keep default if invalid
        };
    }
    
    config
}

/// Validate that the AI configuration is usable.
pub fn validate_ai_config(config: &AIConfig) -> Result<(), String> {
    match config.provider {
        ProviderType::Claude => {
            if config.api.claude_api_key.as_ref().map_or(true, |k| k.is_empty()) {
                return Err("Claude provider requires ANTHROPIC_API_KEY to be set".to_string());
            }
        }
        ProviderType::OpenAI => {
            if config.api.openai_api_key.as_ref().map_or(true, |k| k.is_empty()) {
                return Err("OpenAI provider requires OPENAI_API_KEY to be set".to_string());
            }
        }
        ProviderType::Local => {
            if config.api.local_base_url.as_ref().map_or(true, |u| u.is_empty()) {
                return Err("Local provider requires OLLAMA_BASE_URL to be set".to_string());
            }
        }
    }
    
    Ok(())
}

/// Get the model identifier for the current provider.
pub fn get_model_for_provider(config: &AIConfig) -> Result<String, String> {
    match config.provider {
        ProviderType::Claude => config.model.claude.clone()
            .ok_or_else(|| "Claude model not specified".to_string()),
        ProviderType::OpenAI => config.model.openai.clone()
            .ok_or_else(|| "OpenAI model not specified".to_string()),
        ProviderType::Local => config.model.local.clone()
            .ok_or_else(|| "Local model not specified".to_string()),
    }
}