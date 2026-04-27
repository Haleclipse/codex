//! Translation module for agent reasoning content.
//!
//! This module provides:
//! - `TranslationConfig` - Configuration for translation settings
//! - `ReasoningTranslator` - Barrier mechanism to ensure
//!   translation results appear immediately after original content
//! - `TranslationClient` - HTTP client for translation APIs
//! - `ProviderId` - Supported LLM provider identifiers

mod client;
mod config;
mod error;
mod orchestrator;
mod provider;

pub(crate) use config::TranslationConfig;
pub(crate) use orchestrator::ReasoningTranslator;
pub(crate) use provider::ProviderId;
