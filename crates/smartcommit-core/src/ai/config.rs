#[cfg(feature = "ai")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "ai")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AIConfig {
    pub provider: ProviderType,

    pub model: ModelSelection,

    pub api: ApiConfig,

    pub generation: GenerationConfig,
}

#[cfg(feature = "ai")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Claude,
    OpenAI,
    Local,
}

#[cfg(feature = "ai")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelSelection {
    pub claude: Option<String>,

    pub openai: Option<String>,

    pub local: Option<String>,
}

#[cfg(feature = "ai")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiConfig {
    pub claude_api_key: Option<String>,

    pub openai_api_key: Option<String>,

    pub local_base_url: Option<String>,
}

#[cfg(feature = "ai")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GenerationConfig {
    pub max_tokens: Option<usize>,

    pub temperature: Option<f32>,

    pub enable_message_generation: bool,

    pub enable_semantic_matching: bool,

    pub fallback_to_keywords: bool,
}

#[cfg(feature = "ai")]
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

#[cfg(feature = "ai")]
pub fn load_ai_config_from_env() -> AIConfig {
    let mut config = AIConfig::default();

    if let Ok(claude_key) = std::env::var("ANTHROPIC_API_KEY") {
        config.api.claude_api_key = Some(claude_key);
    }

    if let Ok(openai_key) = std::env::var("OPENAI_API_KEY") {
        config.api.openai_api_key = Some(openai_key);
    }

    if let Ok(local_url) = std::env::var("OLLAMA_BASE_URL") {
        config.api.local_base_url = Some(local_url);
    }

    if let Ok(provider) = std::env::var("SMC_AI_PROVIDER") {
        config.provider = match provider.to_lowercase().as_str() {
            "claude" => ProviderType::Claude,
            "openai" => ProviderType::OpenAI,
            "local" => ProviderType::Local,
            _ => config.provider,
        };
    }

    config
}

#[cfg(feature = "ai")]
pub fn validate_ai_config(config: &AIConfig) -> Result<(), String> {
    match config.provider {
        ProviderType::Claude => {
            if config
                .api
                .claude_api_key
                .as_ref()
                .map_or(true, |k| k.is_empty())
            {
                return Err("Claude provider requires ANTHROPIC_API_KEY to be set".to_string());
            }
        }
        ProviderType::OpenAI => {
            if config
                .api
                .openai_api_key
                .as_ref()
                .map_or(true, |k| k.is_empty())
            {
                return Err("OpenAI provider requires OPENAI_API_KEY to be set".to_string());
            }
        }
        ProviderType::Local => {
            if config
                .api
                .local_base_url
                .as_ref()
                .map_or(true, |u| u.is_empty())
            {
                return Err("Local provider requires OLLAMA_BASE_URL to be set".to_string());
            }
        }
    }

    Ok(())
}

#[cfg(feature = "ai")]
pub fn get_model_for_provider(config: &AIConfig) -> Result<String, String> {
    match config.provider {
        ProviderType::Claude => config
            .model
            .claude
            .clone()
            .ok_or_else(|| "Claude model not specified".to_string()),
        ProviderType::OpenAI => config
            .model
            .openai
            .clone()
            .ok_or_else(|| "OpenAI model not specified".to_string()),
        ProviderType::Local => config
            .model
            .local
            .clone()
            .ok_or_else(|| "Local model not specified".to_string()),
    }
}
