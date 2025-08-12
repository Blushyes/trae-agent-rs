//! Interactive mode state management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use trae_agent_rs_core::Config;

/// State for the interactive session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveState {
    /// Current configuration
    pub config: Config,
    
    /// Session metadata
    pub session_id: String,
    
    /// Current working directory
    pub working_dir: String,
    
    /// Session variables
    pub variables: HashMap<String, String>,
    
    /// Whether the session is active
    pub active: bool,
}

impl InteractiveState {
    /// Create a new interactive state
    pub fn new(config: Config) -> Self {
        Self {
            config,
            session_id: uuid::Uuid::new_v4().to_string(),
            working_dir: std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            variables: HashMap::new(),
            active: true,
        }
    }
    
    /// Set a session variable
    pub fn set_variable(&mut self, key: String, value: String) {
        self.variables.insert(key, value);
    }
    
    /// Get a session variable
    pub fn get_variable(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }
    
    /// Update working directory
    pub fn set_working_dir(&mut self, dir: String) {
        self.working_dir = dir;
    }
    
    /// Deactivate the session
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}
