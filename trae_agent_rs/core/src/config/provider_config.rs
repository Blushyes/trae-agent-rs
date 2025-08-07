//! Model provider configuration structures

use serde::{Deserialize, Serialize};

/// Configuration for a model provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider name (e.g., "anthropic", "openai")
    pub provider: String,
    
    /// API key for authentication
    pub api_key: Option<String>,
    
    /// Custom base URL for the API
    pub base_url: Option<String>,
}

impl ProviderConfig {
    /// Get the API key, checking environment variables if not set in config
    pub fn get_api_key(&self) -> Option<String> {
        if let Some(key) = &self.api_key {
            return Some(key.clone());
        }
        
        // Check common environment variable patterns
        let env_vars = match self.provider.as_str() {
            "anthropic" => vec!["ANTHROPIC_API_KEY", "CLAUDE_API_KEY"],
            "openai" => vec!["OPENAI_API_KEY"],
            "google" => vec!["GOOGLE_API_KEY", "GEMINI_API_KEY"],
            _ => vec![],
        };
        
        for var in env_vars {
            if let Ok(key) = std::env::var(var) {
                return Some(key);
            }
        }
        
        None
    }
    
    /// Get the base URL for the provider
    pub fn get_base_url(&self) -> String {
        if let Some(url) = &self.base_url {
            return url.clone();
        }
        
        // Default base URLs for known providers
        match self.provider.as_str() {
            "anthropic" => "https://api.anthropic.com".to_string(),
            "openai" => "https://api.openai.com".to_string(),
            "google" => "https://generativelanguage.googleapis.com".to_string(),
            _ => "".to_string(),
        }
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            provider: "anthropic".to_string(),
            api_key: None,
            base_url: None,
        }
    }
}
