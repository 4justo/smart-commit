//! AI provider abstraction supporting multiple backends.

#[cfg(feature = "ai")]
use async_trait::async_trait;
#[cfg(feature = "ai")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "ai")]
use thiserror::Error;

/// Error types for AI provider operations.
#[cfg(feature = "ai")]
#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("API request failed: {0}")]
    RequestFailed(String),
    
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
    
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Unsupported provider: {0}")]
    UnsupportedProvider(String),
}

/// Request for AI message generation.
#[derive(Debug, Clone, Serialize)]
pub struct MessageRequest {
    /// The system prompt/context for the AI.
    pub system_prompt: String,
    
    /// The user prompt containing the diff and context.
    pub user_prompt: String,
    
    /// Maximum tokens for the response.
    pub max_tokens: Option<usize>,
    
    /// Temperature for response randomness (0.0 - 1.0).
    pub temperature: Option<f32>,
}

/// Response from AI message generation.
#[derive(Debug, Clone, Deserialize)]
pub struct MessageResponse {
    /// The generated message text.
    pub content: String,
    
    /// Token usage information if available.
    pub usage: Option<TokenUsage>,
    
    /// Model used for generation.
    pub model: String,
}

/// Token usage information.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// AI provider trait for supporting multiple backends.
#[async_trait]
pub trait AIProvider: Send + Sync {
    /// Generate a response for the given request.
    async fn generate_message(&self, request: &MessageRequest) -> Result<MessageResponse, ProviderError>;
    
    /// Get the provider name for identification.
    fn provider_name(&self) -> &'static str;
    
    /// Check if the provider is properly configured.
    fn is_configured(&self) -> bool;
}

/// Claude (Anthropic) provider implementation.
pub struct ClaudeProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl ClaudeProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AIProvider for ClaudeProvider {
    async fn generate_message(&self, request: &MessageRequest) -> Result<MessageResponse, ProviderError> {
        let url = "https://api.anthropic.com/v1/messages";
        
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": request.max_tokens.unwrap_or(1024),
            "system": request.system_prompt,
            "messages": [
                {
                    "role": "user",
                    "content": request.user_prompt
                }
            ],
            "temperature": request.temperature.unwrap_or(0.7)
        });
        
        let response = self.client
            .post(url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return match status.as_u16() {
                401 => Err(ProviderError::AuthenticationFailed(error_text)),
                429 => Err(ProviderError::RateLimitExceeded(error_text)),
                _ => Err(ProviderError::RequestFailed(format!("{}: {}", status, error_text))),
            };
        }
        
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;
        
        let content = json["content"][0]["text"]
            .as_str()
            .ok_or_else(|| ProviderError::InvalidResponse("Missing content in response".to_string()))?
            .to_string();
        
        let usage = json.get("usage").and_then(|u| {
            Some(TokenUsage {
                prompt_tokens: u["input_tokens"].as_u64()? as u32,
                completion_tokens: u["output_tokens"].as_u64()? as u32,
                total_tokens: (u["input_tokens"].as_u64()? + u["output_tokens"].as_u64()?) as u32,
            })
        });
        
        Ok(MessageResponse {
            content,
            usage,
            model: self.model.clone(),
        })
    }
    
    fn provider_name(&self) -> &'static str {
        "claude"
    }
    
    fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }
}

/// OpenAI provider implementation.
pub struct OpenAIProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAIProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AIProvider for OpenAIProvider {
    async fn generate_message(&self, request: &MessageRequest) -> Result<MessageResponse, ProviderError> {
        let url = "https://api.openai.com/v1/chat/completions";
        
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": request.max_tokens.unwrap_or(1024),
            "messages": [
                {
                    "role": "system",
                    "content": request.system_prompt
                },
                {
                    "role": "user", 
                    "content": request.user_prompt
                }
            ],
            "temperature": request.temperature.unwrap_or(0.7)
        });
        
        let response = self.client
            .post(url)
            .header("authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return match status.as_u16() {
                401 => Err(ProviderError::AuthenticationFailed(error_text)),
                429 => Err(ProviderError::RateLimitExceeded(error_text)),
                _ => Err(ProviderError::RequestFailed(format!("{}: {}", status, error_text))),
            };
        }
        
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;
        
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| ProviderError::InvalidResponse("Missing content in response".to_string()))?
            .to_string();
        
        let usage = json.get("usage").and_then(|u| {
            Some(TokenUsage {
                prompt_tokens: u["prompt_tokens"].as_u64()? as u32,
                completion_tokens: u["completion_tokens"].as_u64()? as u32,
                total_tokens: u["total_tokens"].as_u64()? as u32,
            })
        });
        
        Ok(MessageResponse {
            content,
            usage,
            model: self.model.clone(),
        })
    }
    
    fn provider_name(&self) -> &'static str {
        "openai"
    }
    
    fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }
}

/// Local LLM provider (Ollama) implementation.
pub struct LocalLLMProvider {
    base_url: String,
    model: String,
    client: reqwest::Client,
}

impl LocalLLMProvider {
    pub fn new(base_url: String, model: String) -> Self {
        Self {
            base_url,
            model,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AIProvider for LocalLLMProvider {
    async fn generate_message(&self, request: &MessageRequest) -> Result<MessageResponse, ProviderError> {
        let url = format!("{}/api/generate", self.base_url);
        
        let body = serde_json::json!({
            "model": self.model,
            "prompt": format!("{}\n\n{}", request.system_prompt, request.user_prompt),
            "stream": false,
            "options": {
                "temperature": request.temperature.unwrap_or(0.7),
                "num_predict": request.max_tokens.unwrap_or(1024)
            }
        });
        
        let response = self.client
            .post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProviderError::RequestFailed(format!("{}: {}", status, error_text)));
        }
        
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;
        
        let content = json["response"]
            .as_str()
            .ok_or_else(|| ProviderError::InvalidResponse("Missing response in output".to_string()))?
            .to_string();
        
        Ok(MessageResponse {
            content,
            usage: None, // Ollama doesn't provide token usage
            model: self.model.clone(),
        })
    }
    
    fn provider_name(&self) -> &'static str {
        "local"
    }
    
    fn is_configured(&self) -> bool {
        !self.base_url.is_empty() && !self.model.is_empty()
    }
}