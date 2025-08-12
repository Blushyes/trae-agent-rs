//! Main configuration structure and loading logic

use crate::error::{ConfigError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use super::{
    AgentConfig, ApiProvider, ApiProviderConfig, ConfigLoader, ModelConfig, ProviderConfig,
};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Agent configurations
    pub agents: HashMap<String, AgentConfig>,

    /// Model provider configurations
    pub model_providers: HashMap<String, ProviderConfig>,

    /// Model configurations
    pub models: HashMap<String, ModelConfig>,
}

impl Config {
    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate that all model references exist
        for (agent_name, agent_config) in &self.agents {
            if !self.models.contains_key(&agent_config.model) {
                return Err(ConfigError::InvalidValue {
                    field: format!("agents.{}.model", agent_name),
                    value: agent_config.model.clone(),
                }
                .into());
            }
        }

        // Validate that all model provider references exist
        for (model_name, model_config) in &self.models {
            if !self
                .model_providers
                .contains_key(&model_config.model_provider)
            {
                return Err(ConfigError::InvalidValue {
                    field: format!("models.{}.model_provider", model_name),
                    value: model_config.model_provider.clone(),
                }
                .into());
            }
        }

        Ok(())
    }

    /// Get agent configuration by name
    pub fn get_agent(&self, name: &str) -> Option<&AgentConfig> {
        self.agents.get(name)
    }

    /// Get model configuration by name
    pub fn get_model(&self, name: &str) -> Option<&ModelConfig> {
        self.models.get(name)
    }

    /// Get provider configuration by name
    pub fn get_provider(&self, name: &str) -> Option<&ProviderConfig> {
        self.model_providers.get(name)
    }

    /// Get the default agent configuration (first one found)
    pub fn get_default_agent(&self) -> Option<(&String, &AgentConfig)> {
        self.agents.iter().next()
    }

    /// Load configuration using the new API-based configuration system
    pub async fn from_api_configs<P: AsRef<Path>>(config_dir: P) -> Result<Self> {
        let mut loader = ConfigLoader::new(config_dir);
        let (provider, api_config) = loader.load_config().await?;

        // Convert API configuration to the existing Config structure
        Self::from_api_provider_config(provider, api_config)
    }

    /// Create configuration from a single API provider config
    pub fn from_api_provider_config(
        provider: ApiProvider,
        api_config: ApiProviderConfig,
    ) -> Result<Self> {
        let mut agents = HashMap::new();
        let mut models = HashMap::new();
        let mut model_providers = HashMap::new();

        // Create provider config
        let provider_config = ProviderConfig {
            provider: provider.to_string(),
            api_key: api_config.api_key.clone(),
            base_url: api_config.base_url.clone(),
        };

        model_providers.insert(provider.to_string(), provider_config);

        // Create model config
        let model_name = format!("{}_model", provider.as_str());
        let model_config = ModelConfig {
            model_provider: provider.to_string(),
            model: api_config
                .model
                .unwrap_or_else(|| Self::default_model_for_provider(&provider)),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            top_p: Some(1.0),
            top_k: None,
            max_retries: Some(3),
            parallel_tool_calls: Some(true),
            stop_sequences: None,
        };

        models.insert(model_name.clone(), model_config);

        // Create agent config
        let agent_config = AgentConfig {
            model: model_name,
            max_steps: 200,
            enable_lakeview: true,
            tools: vec![
                "bash".to_string(),
                "str_replace_based_edit_tool".to_string(),
                "sequentialthinking".to_string(),
                "task_done".to_string(),
            ],
            output_mode: crate::config::agent_config::OutputMode::Normal,
            system_prompt: None,
        };

        agents.insert("trae_agent".to_string(), agent_config);

        Ok(Self {
            agents,
            model_providers,
            models,
        })
    }

    /// Get default model name for a provider
    fn default_model_for_provider(provider: &ApiProvider) -> String {
        match provider {
            ApiProvider::OpenAI => "gpt-4".to_string(),
            ApiProvider::Anthropic => "claude-3-5-sonnet-20241022".to_string(),
            ApiProvider::Google => "gemini-pro".to_string(),
            ApiProvider::Custom(_) => "default".to_string(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut agents = HashMap::new();
        let mut models = HashMap::new();
        let mut model_providers = HashMap::new();

        // Default provider
        model_providers.insert(
            "anthropic".to_string(),
            ProviderConfig {
                provider: "anthropic".to_string(),
                api_key: None,
                base_url: None,
            },
        );

        // Default model
        models.insert(
            "default_model".to_string(),
            ModelConfig {
                model_provider: "anthropic".to_string(),
                model: "claude-3-5-sonnet-20241022".to_string(),
                max_tokens: Some(4096),
                temperature: Some(0.5),
                top_p: Some(1.0),
                top_k: None,
                max_retries: Some(3),
                parallel_tool_calls: Some(true),
                stop_sequences: None,
            },
        );

        // Default agent
        agents.insert(
            "trae_agent".to_string(),
            AgentConfig {
                model: "default_model".to_string(),
                max_steps: 200,
                enable_lakeview: true,
                tools: vec![
                    "bash".to_string(),
                    "str_replace_based_edit_tool".to_string(),
                    "sequentialthinking".to_string(),
                    "task_done".to_string(),
                ],
                output_mode: crate::config::agent_config::OutputMode::Normal,
                system_prompt: None,
            },
        );

        Self {
            agents,
            model_providers,
            models,
        }
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_from_api_configs_builds_full_config() {
        let temp_dir = tempdir().unwrap();
        // Provide only one provider via JSON; should auto-select
        let openai_json = temp_dir.path().join("openai.json");
        let content = r#"{
            "base_url": "https://api.openai.com/v1",
            "api_key": "json-key",
            "model": "gpt-4"
        }"#;
        tokio::fs::write(&openai_json, content).await.unwrap();

        let cfg = Config::from_api_configs(temp_dir.path()).await.unwrap();

        // validate core structure
        assert!(cfg.get_provider("openai").is_some());
        let default_agent = cfg.get_default_agent().unwrap().1;
        let model_cfg = cfg.get_model(&default_agent.model).unwrap();
        assert_eq!(model_cfg.model_provider, "openai");
        assert_eq!(model_cfg.model, "gpt-4");

        // validation should pass
        assert!(cfg.validate().is_ok());
    }
}
