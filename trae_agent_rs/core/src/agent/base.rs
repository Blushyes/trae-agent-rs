//! Base agent trait and structures

use crate::config::{AgentConfig, Config};
use crate::error::Result;
use crate::trajectory::TrajectoryRecorder;
use async_trait::async_trait;

use super::execution::AgentExecution;

/// Result type for agent operations
pub type AgentResult<T> = Result<T>;

/// Base trait for all agents
#[async_trait]
pub trait Agent: Send + Sync {
    /// Execute a task
    async fn execute_task(&mut self, task: &str) -> AgentResult<AgentExecution>;
    
    /// Get the agent's configuration
    fn config(&self) -> &AgentConfig;
    
    /// Get the agent's name/type
    fn agent_type(&self) -> &str;
    
    /// Set the trajectory recorder
    fn set_trajectory_recorder(&mut self, recorder: TrajectoryRecorder);
    
    /// Get the trajectory recorder
    fn trajectory_recorder(&self) -> Option<&TrajectoryRecorder>;
}

/// Factory for creating agents
pub struct AgentFactory;

impl AgentFactory {
    /// Create an agent from configuration
    pub async fn create_agent(
        agent_type: &str,
        config: &Config,
        trajectory_recorder: Option<TrajectoryRecorder>,
    ) -> Result<Box<dyn Agent>> {
        match agent_type {
            "trae_agent" => {
                let agent_config = config.get_agent("trae_agent")
                    .ok_or_else(|| crate::error::ConfigError::MissingField {
                        field: "agents.trae_agent".to_string(),
                    })?;
                
                let mut agent = super::trae_agent::TraeAgent::new(
                    agent_config.clone(),
                    config.clone(),
                ).await?;
                
                if let Some(recorder) = trajectory_recorder {
                    agent.set_trajectory_recorder(recorder);
                }
                
                Ok(Box::new(agent))
            }
            _ => Err(crate::error::AgentError::InvalidTask {
                message: format!("Unknown agent type: {}", agent_type),
            }.into()),
        }
    }
}
