//! AI integration for SmartCommit (Module 15+).
//!
//! Provides AI-powered commit message generation, semantic file-to-commit
//! matching, and hunk-level diff analysis using multiple provider backends
//! (Claude, OpenAI, local LLMs).

#[cfg(feature = "ai")]
pub mod provider;
#[cfg(feature = "ai")]
pub mod message_generator;
#[cfg(feature = "ai")]
pub mod semantic_matcher;
#[cfg(feature = "ai")]
pub mod config;
#[cfg(feature = "ai")]
pub mod factory;
#[cfg(feature = "ai")]
pub mod hunk_analyzer;

#[cfg(feature = "ai")]
pub use provider::{AIProvider, ProviderError, MessageRequest, MessageResponse};
#[cfg(feature = "ai")]
pub use message_generator::{generate_commit_messages, GenerationError};
#[cfg(feature = "ai")]
pub use semantic_matcher::{semantic_file_matching, SemanticMatchError};
#[cfg(feature = "ai")]
pub use config::{AIConfig, ModelSelection, ProviderType, load_ai_config_from_env, validate_ai_config};
#[cfg(feature = "ai")]
pub use factory::{create_provider, create_provider_with_fallback};
#[cfg(feature = "ai")]
pub use hunk_analyzer::{analyze_hunks, HunkAnalysisError, HunkAnalysis, DiffAnalysis, ChangeCategory, RiskLevel};