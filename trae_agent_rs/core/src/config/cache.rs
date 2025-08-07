//! Configuration cache for storing user preferences

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use crate::error::Result;

/// Configuration cache to store user's provider selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigCache {
    /// Last selected provider
    pub selected_provider: Option<String>,
    
    /// Timestamp of last selection
    pub last_updated: Option<u64>,
    
    /// Cache version for compatibility
    pub version: u32,
}

impl ConfigCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            selected_provider: None,
            last_updated: None,
            version: 1,
        }
    }
    
    /// Get the default cache file path
    pub fn default_cache_path() -> PathBuf {
        let mut path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."));
        path.push("trae-agent");
        path.push("cache.json");
        path
    }
    
    /// Load cache from file
    pub async fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        
        if !path.exists() {
            return Ok(Self::new());
        }
        
        let content = fs::read_to_string(path).await?;
        let cache: ConfigCache = serde_json::from_str(&content)
            .unwrap_or_else(|_| Self::new());
            
        Ok(cache)
    }
    
    /// Save cache to file
    pub async fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content).await?;
        
        Ok(())
    }
    
    /// Update the selected provider
    pub fn set_selected_provider(&mut self, provider: String) {
        self.selected_provider = Some(provider);
        self.last_updated = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );
    }
    
    /// Get the selected provider
    pub fn get_selected_provider(&self) -> Option<&String> {
        self.selected_provider.as_ref()
    }
    
    /// Check if the cache is expired (older than 30 days)
    pub fn is_expired(&self) -> bool {
        if let Some(last_updated) = self.last_updated {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            let thirty_days = 30 * 24 * 60 * 60; // 30 days in seconds
            now - last_updated > thirty_days
        } else {
            true
        }
    }
    
    /// Clear the cache
    pub fn clear(&mut self) {
        self.selected_provider = None;
        self.last_updated = None;
    }
}

impl Default for ConfigCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_cache_save_load() {
        let temp_dir = tempdir().unwrap();
        let cache_path = temp_dir.path().join("test_cache.json");
        
        let mut cache = ConfigCache::new();
        cache.set_selected_provider("openai".to_string());
        
        // Save cache
        cache.save(&cache_path).await.unwrap();
        
        // Load cache
        let loaded_cache = ConfigCache::load(&cache_path).await.unwrap();
        
        assert_eq!(loaded_cache.selected_provider, Some("openai".to_string()));
        assert!(loaded_cache.last_updated.is_some());
    }
    
    #[tokio::test]
    async fn test_cache_load_nonexistent() {
        let temp_dir = tempdir().unwrap();
        let cache_path = temp_dir.path().join("nonexistent.json");
        
        let cache = ConfigCache::load(&cache_path).await.unwrap();
        
        assert_eq!(cache.selected_provider, None);
        assert_eq!(cache.last_updated, None);
    }
    
    #[test]
    fn test_cache_expiry() {
        let mut cache = ConfigCache::new();
        
        // New cache should be expired
        assert!(cache.is_expired());
        
        // Set provider (this sets current timestamp)
        cache.set_selected_provider("anthropic".to_string());
        
        // Should not be expired now
        assert!(!cache.is_expired());
        
        // Manually set old timestamp
        cache.last_updated = Some(0); // Unix epoch
        
        // Should be expired now
        assert!(cache.is_expired());
    }
}
