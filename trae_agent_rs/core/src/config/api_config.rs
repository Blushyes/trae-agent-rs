//! API provider configuration structures for different protocols

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a specific API provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiProviderConfig {
    /// Base URL for the API
    pub base_url: Option<String>,
    
    /// API key for authentication
    pub api_key: Option<String>,
    
    /// Default model to use with this provider
    pub model: Option<String>,
    
    /// Additional provider-specific settings
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl ApiProviderConfig {
    /// Create a new API provider configuration
    pub fn new() -> Self {
        Self {
            base_url: None,
            api_key: None,
            model: None,
            extra: HashMap::new(),
        }
    }
    
    /// Set the base URL
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = Some(base_url);
        self
    }
    
    /// Set the API key
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }
    
    /// Set the default model
    pub fn with_model(mut self, model: String) -> Self {
        self.model = Some(model);
        self
    }
    
    /// Add extra configuration
    pub fn with_extra(mut self, key: String, value: serde_json::Value) -> Self {
        self.extra.insert(key, value);
        self
    }
}

impl Default for ApiProviderConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// OpenAI-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConfig {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub organization: Option<String>,
    pub project: Option<String>,
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            base_url: Some("https://api.openai.com/v1".to_string()),
            api_key: None,
            model: Some("gpt-4".to_string()),
            organization: None,
            project: None,
        }
    }
}

impl From<OpenAIConfig> for ApiProviderConfig {
    fn from(config: OpenAIConfig) -> Self {
        let mut extra = HashMap::new();
        if let Some(org) = config.organization {
            extra.insert("organization".to_string(), serde_json::Value::String(org));
        }
        if let Some(project) = config.project {
            extra.insert("project".to_string(), serde_json::Value::String(project));
        }
        
        ApiProviderConfig {
            base_url: config.base_url,
            api_key: config.api_key,
            model: config.model,
            extra,
        }
    }
}

/// Anthropic-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub version: Option<String>,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            base_url: Some("https://api.anthropic.com".to_string()),
            api_key: None,
            model: Some("claude-3-5-sonnet-20241022".to_string()),
            version: Some("2023-06-01".to_string()),
        }
    }
}

impl From<AnthropicConfig> for ApiProviderConfig {
    fn from(config: AnthropicConfig) -> Self {
        let mut extra = HashMap::new();
        if let Some(version) = config.version {
            extra.insert("version".to_string(), serde_json::Value::String(version));
        }
        
        ApiProviderConfig {
            base_url: config.base_url,
            api_key: config.api_key,
            model: config.model,
            extra,
        }
    }
}

/// Google/Gemini-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleConfig {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub project_id: Option<String>,
}

impl Default for GoogleConfig {
    fn default() -> Self {
        Self {
            base_url: Some("https://generativelanguage.googleapis.com".to_string()),
            api_key: None,
            model: Some("gemini-pro".to_string()),
            project_id: None,
        }
    }
}

impl From<GoogleConfig> for ApiProviderConfig {
    fn from(config: GoogleConfig) -> Self {
        let mut extra = HashMap::new();
        if let Some(project_id) = config.project_id {
            extra.insert("project_id".to_string(), serde_json::Value::String(project_id));
        }
        
        ApiProviderConfig {
            base_url: config.base_url,
            api_key: config.api_key,
            model: config.model,
            extra,
        }
    }
}

/// Supported API providers
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ApiProvider {
    OpenAI,
    Anthropic,
    Google,
    Custom(String),
}

impl ApiProvider {
    /// Get the provider name as a string
    pub fn as_str(&self) -> &str {
        match self {
            ApiProvider::OpenAI => "openai",
            ApiProvider::Anthropic => "anthropic", 
            ApiProvider::Google => "google",
            ApiProvider::Custom(name) => name,
        }
    }
    
    /// Get the default configuration file name for this provider
    pub fn config_filename(&self) -> String {
        format!("{}.json", self.as_str())
    }
    
    /// Get environment variable prefix for this provider
    pub fn env_prefix(&self) -> String {
        match self {
            ApiProvider::OpenAI => "OPENAI".to_string(),
            ApiProvider::Anthropic => "ANTHROPIC".to_string(),
            ApiProvider::Google => "GOOGLE".to_string(),
            ApiProvider::Custom(name) => name.to_uppercase(),
        }
    }
}

impl std::fmt::Display for ApiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for ApiProvider {
    type Err = ();
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(ApiProvider::OpenAI),
            "anthropic" => Ok(ApiProvider::Anthropic),
            "google" | "gemini" => Ok(ApiProvider::Google),
            _ => Ok(ApiProvider::Custom(s.to_string())),
        }
    }
}
