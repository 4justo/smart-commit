//! AI integration for SmartCommit (Module 15+).
//!
//! Provides AI-powered commit message generation and semantic file-to-commit
//! matching using multiple provider backends (Claude, OpenAI, local LLMs).

pub mod provider;
pub mod message_generator;
pub mod semantic_matcher;
pub mod config;
pub mod factory;

pub use provider::{AIProvider, ProviderError, MessageRequest, MessageResponse};
pub use message_generator::{generate_commit_messages, GenerationError};
pub use semantic_matcher::{semantic_file_matching, SemanticMatchError};
pub use config::{AIConfig, ModelSelection, ProviderType, load_ai_config_from_env, validate_ai_config};
pub use factory::{create_provider, create_provider_with_fallback};