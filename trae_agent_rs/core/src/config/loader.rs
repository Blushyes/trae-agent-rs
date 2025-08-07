//! Configuration loader for API providers

use super::{ApiProvider, ApiProviderConfig, ConfigCache};
use crate::error::{ConfigError, Result};
use serde_json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};

/// Configuration loader that handles multiple API providers
pub struct ConfigLoader {
    /// Base directory to search for config files
    config_dir: PathBuf,

    /// Cache for user selections
    cache: ConfigCache,

    /// Cache file path
    cache_path: PathBuf,
}

impl ConfigLoader {
    /// Create a new configuration loader
    pub fn new<P: AsRef<Path>>(config_dir: P) -> Self {
        let config_dir = config_dir.as_ref().to_path_buf();
        let cache_path = ConfigCache::default_cache_path();

        Self {
            config_dir,
            cache: ConfigCache::new(),
            cache_path,
        }
    }

    /// Initialize the loader by loading the cache
    pub async fn init(&mut self) -> Result<()> {
        self.cache = ConfigCache::load(&self.cache_path).await?;
        debug!("Loaded configuration cache from: {}", self.cache_path.display());
        Ok(())
    }

    /// Discover available API provider configurations
    pub async fn discover_configs(&self) -> Result<HashMap<ApiProvider, ApiProviderConfig>> {
        let mut configs = HashMap::new();

        // Check for JSON config files
        let json_configs = self.load_json_configs().await?;
        configs.extend(json_configs);

        // Check for environment variables for providers that don't have JSON configs
        let env_configs = self.load_env_configs(&configs).await?;
        configs.extend(env_configs);

        debug!("Discovered {} provider configurations", configs.len());
        Ok(configs)
    }

    /// Load configurations from JSON files
    async fn load_json_configs(&self) -> Result<HashMap<ApiProvider, ApiProviderConfig>> {
        let mut configs = HashMap::new();

        let providers = [
            ApiProvider::OpenAI,
            ApiProvider::Anthropic,
            ApiProvider::Google,
        ];

        for provider in providers {
            let config_path = self.config_dir.join(provider.config_filename());

            if config_path.exists() {
                match self.load_json_config(&config_path).await {
                    Ok(config) => {
                        info!("Loaded {} configuration from: {}", provider, config_path.display());
                        configs.insert(provider, config);
                    }
                    Err(e) => {
                        warn!("Failed to load {} configuration from {}: {}",
                              provider, config_path.display(), e);
                    }
                }
            }
        }

        Ok(configs)
    }

    /// Load a single JSON configuration file
    async fn load_json_config(&self, path: &Path) -> Result<ApiProviderConfig> {
        let content = fs::read_to_string(path).await?;
        let config: ApiProviderConfig = serde_json::from_str(&content)
            .map_err(|e| ConfigError::InvalidFormat)?;
        Ok(config)
    }

    /// Load configurations from environment variables
    async fn load_env_configs(&self, existing_configs: &HashMap<ApiProvider, ApiProviderConfig>)
        -> Result<HashMap<ApiProvider, ApiProviderConfig>> {
        let mut configs = HashMap::new();

        let providers = [
            ApiProvider::OpenAI,
            ApiProvider::Anthropic,
            ApiProvider::Google,
        ];

        for provider in providers {
            // Skip if we already have a JSON config for this provider
            if existing_configs.contains_key(&provider) {
                continue;
            }

            if let Some(config) = self.load_env_config(&provider) {
                info!("Loaded {} configuration from environment variables", provider);
                configs.insert(provider, config);
            }
        }

        Ok(configs)
    }

    /// Load configuration for a specific provider from environment variables
    fn load_env_config(&self, provider: &ApiProvider) -> Option<ApiProviderConfig> {
        let prefix = provider.env_prefix();

        let base_url = std::env::var(format!("{}_BASE_URL", prefix)).ok();
        let api_key = std::env::var(format!("{}_API_KEY", prefix)).ok();
        let model = std::env::var(format!("{}_MODEL", prefix)).ok();

        // Only create config if at least one environment variable is set
        if base_url.is_some() || api_key.is_some() || model.is_some() {
            Some(ApiProviderConfig {
                base_url,
                api_key,
                model,
                extra: HashMap::new(),
            })
        } else {
            None
        }
    }

    /// Select a configuration, handling multiple options
    pub async fn select_config(&mut self, configs: HashMap<ApiProvider, ApiProviderConfig>)
        -> Result<(ApiProvider, ApiProviderConfig)> {

        if configs.is_empty() {
            return Err(ConfigError::NoConfigFound.into());
        }

        // If only one config, use it
        if configs.len() == 1 {
            let (provider, config) = configs.into_iter().next().unwrap();
            info!("Using single available configuration: {}", provider);
            return Ok((provider, config));
        }

        // Check cache for previous selection
        if let Some(cached_provider) = self.cache.get_selected_provider() {
            if !self.cache.is_expired() {
                if let Ok(provider) = cached_provider.parse::<ApiProvider>() {
                    if let Some(config) = configs.get(&provider) {
                        info!("Using cached provider selection: {}", provider);
                        return Ok((provider, config.clone()));
                    }
                }
            }
        }

        // Multiple configs available, need user selection
        self.prompt_user_selection(configs).await
    }

    /// Prompt user to select from multiple configurations
    async fn prompt_user_selection(&mut self, configs: HashMap<ApiProvider, ApiProviderConfig>)
        -> Result<(ApiProvider, ApiProviderConfig)> {

        println!("Multiple API provider configurations found:");

        let providers: Vec<_> = configs.keys().collect();
        for (i, provider) in providers.iter().enumerate() {
            let config = configs.get(provider).unwrap();
            let source = if config.api_key.is_some() { "configured" } else { "env vars" };
            println!("  {}. {} ({})", i + 1, provider, source);
        }

        println!("Please select a provider (1-{}): ", providers.len());

        // Read user input
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)
            .map_err(|_| ConfigError::InvalidValue {
                field: "user_selection".to_string(),
                value: "failed to read input".to_string(),
            })?;

        let selection: usize = input.trim().parse()
            .map_err(|_| ConfigError::InvalidValue {
                field: "user_selection".to_string(),
                value: input.trim().to_string(),
            })?;

        if selection == 0 || selection > providers.len() {
            return Err(ConfigError::InvalidValue {
                field: "user_selection".to_string(),
                value: selection.to_string(),
            }.into());
        }

        let selected_provider = providers[selection - 1].clone();
        let selected_config = configs.get(&selected_provider).unwrap().clone();

        // Cache the selection
        self.cache.set_selected_provider(selected_provider.to_string());
        if let Err(e) = self.cache.save(&self.cache_path).await {
            warn!("Failed to save configuration cache: {}", e);
        }

        info!("Selected provider: {}", selected_provider);
        Ok((selected_provider, selected_config))
    }

    /// Load configuration with automatic discovery and selection
    pub async fn load_config(&mut self) -> Result<(ApiProvider, ApiProviderConfig)> {
        self.init().await?;
        let configs = self.discover_configs().await?;
        self.select_config(configs).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_load_json_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("openai.json");

        let config_content = r#"{
            "base_url": "https://api.openai.com/v1",
            "api_key": "test-key",
            "model": "gpt-4"
        }"#;

        fs::write(&config_path, config_content).await.unwrap();

        let loader = ConfigLoader::new(temp_dir.path());
        let config = loader.load_json_config(&config_path).await.unwrap();

        assert_eq!(config.base_url, Some("https://api.openai.com/v1".to_string()));
        assert_eq!(config.api_key, Some("test-key".to_string()));
        assert_eq!(config.model, Some("gpt-4".to_string()));
    }

    #[test]
    fn test_load_env_config() {
        std::env::set_var("OPENAI_API_KEY", "test-env-key");
        std::env::set_var("OPENAI_MODEL", "gpt-3.5-turbo");

        let loader = ConfigLoader::new(".");
        let config = loader.load_env_config(&ApiProvider::OpenAI).unwrap();

        assert_eq!(config.api_key, Some("test-env-key".to_string()));
        assert_eq!(config.model, Some("gpt-3.5-turbo".to_string()));

        // Clean up
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("OPENAI_MODEL");
    }
}


    #[tokio::test]
    async fn test_discover_configs_prefers_json_over_env() {
        // Prepare: set env vars but also provide JSON to ensure JSON wins
        std::env::set_var("OPENAI_API_KEY", "env-key");
        std::env::set_var("OPENAI_MODEL", "env-model");

        let temp_dir = tempfile::tempdir().unwrap();
        let openai_json = temp_dir.path().join("openai.json");
        let content = r#"{
            "base_url": "https://api.openai.com/v1",
            "api_key": "json-key",
            "model": "gpt-4"
        }"#;
        fs::write(&openai_json, content).await.unwrap();

        let loader = ConfigLoader::new(temp_dir.path());
        let configs = loader.discover_configs().await.unwrap();

        let openai = configs.get(&ApiProvider::OpenAI).expect("openai config missing");
        assert_eq!(openai.api_key.as_deref(), Some("json-key"));
        assert_eq!(openai.model.as_deref(), Some("gpt-4"));

        // Clean env
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("OPENAI_MODEL");
    }

    #[tokio::test]
    async fn test_select_config_single_and_cache() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut loader = ConfigLoader::new(temp_dir.path());

        // Single config should be selected directly
        let mut single = HashMap::new();
        single.insert(
            ApiProvider::OpenAI,
            ApiProviderConfig { base_url: Some("https://api.openai.com/v1".into()), api_key: Some("k".into()), model: Some("gpt-4".into()), extra: HashMap::new() }
        );
        let (prov, _cfg) = loader.select_config(single).await.unwrap();
        assert!(matches!(prov, ApiProvider::OpenAI));

        // Multiple with cache
        let mut many = HashMap::new();
        many.insert(
            ApiProvider::OpenAI,
            ApiProviderConfig { base_url: None, api_key: Some("k1".into()), model: Some("m1".into()), extra: HashMap::new() }
        );
        many.insert(
            ApiProvider::Anthropic,
            ApiProviderConfig { base_url: None, api_key: Some("k2".into()), model: Some("m2".into()), extra: HashMap::new() }
        );

        // Inject cache to avoid interactive prompt
        loader.cache.set_selected_provider("openai".into());
        let (prov2, _cfg2) = loader.select_config(many).await.unwrap();
        assert!(matches!(prov2, ApiProvider::OpenAI));
    }

    #[tokio::test]
    async fn test_load_config_uses_cache_file() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Write openai.json
        let openai_json = temp_dir.path().join("openai.json");
        let content = r#"{
            "base_url": "https://api.openai.com/v1",
            "api_key": "json-key",
            "model": "gpt-4"
        }"#;
        fs::write(&openai_json, content).await.unwrap();

        // Prepare cache file with fresh timestamp
        let cache_path = temp_dir.path().join("cache.json");
        let mut cache = ConfigCache::new();
        cache.set_selected_provider("openai".into());
        cache.save(&cache_path).await.unwrap();

        // Loader with overridden cache path
        let mut loader = ConfigLoader::new(temp_dir.path());
        loader.cache_path = cache_path; // same module, allowed in tests

        let (prov, cfg) = loader.load_config().await.unwrap();
        assert!(matches!(prov, ApiProvider::OpenAI));
        assert_eq!(cfg.api_key.as_deref(), Some("json-key"));
    }


    #[tokio::test]
    async fn test_load_config_no_config_returns_error() {
        // Ensure env does not provide configs
        for k in [
            "OPENAI_API_KEY","OPENAI_BASE_URL","OPENAI_MODEL",
            "ANTHROPIC_API_KEY","ANTHROPIC_BASE_URL","ANTHROPIC_MODEL",
            "GOOGLE_API_KEY","GOOGLE_BASE_URL","GOOGLE_MODEL"
        ] { let _ = std::env::remove_var(k); }

        let temp_dir = tempfile::tempdir().unwrap();
        let mut loader = ConfigLoader::new(temp_dir.path());
        // Avoid using any real cache path
        loader.cache_path = temp_dir.path().join("cache.json");

        let err = loader.load_config().await.err().expect("expected error");
        let msg = format!("{}", err);
        assert!(msg.contains("No configuration"), "unexpected error: {}", msg);
    }
