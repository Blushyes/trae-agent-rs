//! # Trae Agent Core
//!
//! Core library for Trae Agent - an LLM-based agent for software engineering tasks.
//!
//! This library provides the fundamental building blocks for creating AI agents
//! that can interact with codebases, execute tools, and perform complex software
//! engineering tasks.

// Core modules
pub mod config;
pub mod error;
pub mod agent;
pub mod output;
pub mod trajectory;
pub mod llm;
pub mod tools;

// Re-export commonly used types
pub use config::{Config, AgentConfig, ModelConfig, ProviderConfig, ConfigLoader, ApiProvider, ApiProviderConfig};
pub use agent::TraeAgent;
pub use trajectory::TrajectoryRecorder;

/// Current version of the trae-agent-core library
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize tracing for the library
pub fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

/// Initialize tracing with a specific debug mode
pub fn init_tracing_with_debug(debug: bool) {
    let filter = if debug {
        "debug"
    } else {
        "info"
    };

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(filter))
        .init();
}
