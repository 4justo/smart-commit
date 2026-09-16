//! Factory for creating AI provider instances.

use super::provider::{AIProvider, ClaudeProvider, OpenAIProvider, LocalLLMProvider};
use super::config::{AIConfig, ProviderType};
use std::sync::Arc;

/// Create an AI provider instance based on configuration.
pub fn create_provider(config: &AIConfig) -> Result<Arc<dyn AIProvider>, String> {
    match config.provider {
        ProviderType::Claude => {
            let api_key = config.api.claude_api_key.as_ref()
                .ok_or("Claude API key not configured")?;
            let model = config.model.claude.as_ref()
                .ok_or("Claude model not specified")?;
            
            Ok(Arc::new(ClaudeProvider::new(api_key.clone(), model.clone())))
        }
        ProviderType::OpenAI => {
            let api_key = config.api.openai_api_key.as_ref()
                .ok_or("OpenAI API key not configured")?;
            let model = config.model.openai.as_ref()
                .ok_or("OpenAI model not specified")?;
            
            Ok(Arc::new(OpenAIProvider::new(api_key.clone(), model.clone())))
        }
        ProviderType::Local => {
            let base_url = config.api.local_base_url.as_ref()
                .ok_or("Local LLM base URL not configured")?;
            let model = config.model.local.as_ref()
                .ok_or("Local model not specified")?;
            
            Ok(Arc::new(LocalLLMProvider::new(base_url.clone(), model.clone())))
        }
    }
}

/// Create provider with fallback to alternative if first fails.
pub fn create_provider_with_fallback(config: &AIConfig) -> Arc<dyn AIProvider> {
    // Try primary provider first
    if let Ok(provider) = create_provider(config) {
        return provider;
    }
    
    // Fallback chain: Claude -> OpenAI -> Local
    let fallback_configs = vec![
        (ProviderType::Claude, "claude"),
        (ProviderType::OpenAI, "openai"), 
        (ProviderType::Local, "local"),
    ];
    
    for (provider_type, name) in fallback_configs {
        if provider_type != config.provider {
            let mut fallback_config = config.clone();
            fallback_config.provider = provider_type;
            
            if let Ok(provider) = create_provider(&fallback_config) {
                eprintln!("Falling back to {} provider", name);
                return provider;
            }
        }
    }
    
    // If all fail, return a dummy provider that will error appropriately
    Arc::new(UnconfiguredProvider)
}

/// Dummy provider for when no AI is configured.
struct UnconfiguredProvider;

#[async_trait::async_trait]
impl AIProvider for UnconfiguredProvider {
    async fn generate_message(
        &self, 
        _request: &super::provider::MessageRequest
    ) -> Result<super::provider::MessageResponse, super::provider::ProviderError> {
        Err(super::provider::ProviderError::ConfigError(
            "No AI provider configured. Set API keys in environment or config file.".to_string()
        ))
    }
    
    fn provider_name(&self) -> &'static str {
        "unconfigured"
    }
    
    fn is_configured(&self) -> bool {
        false
    }
}