//! Agent core logic and execution engine

pub mod base;
pub mod execution;
pub mod prompt;
pub mod trae_agent;

pub use base::{Agent, AgentResult};
pub use execution::AgentExecution;
pub use prompt::{TRAE_AGENT_SYSTEM_PROMPT, build_system_prompt_with_context, build_user_message};
pub use trae_agent::TraeAgent;
