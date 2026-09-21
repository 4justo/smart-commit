//! AI integration for SmartCommit (Module 15+).
//!
//! Provides AI-powered commit message generation, semantic file-to-commit
//! matching, hunk-level diff analysis, and semantic change analysis using
//! multiple provider backends (Claude, OpenAI, local LLMs).

#[cfg(feature = "ai")]
pub mod config;
#[cfg(feature = "ai")]
pub mod factory;
#[cfg(feature = "ai")]
pub mod hunk_analyzer;
#[cfg(feature = "ai")]
pub mod message_generator;
#[cfg(feature = "ai")]
pub mod provider;
#[cfg(feature = "ai")]
pub mod semantic_analysis;
#[cfg(feature = "ai")]
pub mod semantic_matcher;

#[cfg(feature = "ai")]
pub use config::{
    load_ai_config_from_env, validate_ai_config, AIConfig, ModelSelection, ProviderType,
};
#[cfg(feature = "ai")]
pub use factory::{create_provider, create_provider_with_fallback};
#[cfg(feature = "ai")]
pub use hunk_analyzer::{
    analyze_hunks, ChangeCategory, DiffAnalysis, HunkAnalysis, HunkAnalysisError, RiskLevel,
};
#[cfg(feature = "ai")]
pub use message_generator::{generate_commit_messages, GenerationError};
#[cfg(feature = "ai")]
pub use provider::{AIProvider, MessageRequest, MessageResponse, ProviderError};
#[cfg(feature = "ai")]
pub use semantic_analysis::{
    analyze_changes_semantically, AISemanticAnalysis, CombinedResult, DeterministicAnalysis,
    DeterministicCategory, SemanticAnalysis, SemanticAnalysisError, SemanticCategory,
};
#[cfg(feature = "ai")]
pub use semantic_matcher::{semantic_file_matching, SemanticMatchError};
