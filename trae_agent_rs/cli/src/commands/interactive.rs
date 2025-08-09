//! Interactive mode command

use anyhow::Result;
use std::path::PathBuf;
use tracing::{info, debug};
use crate::interactive::app::run_rich_interactive;
use trae_agent_core::Config;

/// Start interactive mode
pub async fn interactive_command(
    config_dir: PathBuf,
    trajectory_file: Option<PathBuf>,
    _debug_output: bool,
) -> Result<()> {
    info!("Starting interactive mode");

    println!("⚙️  Config directory: {}", config_dir.display());

    if let Some(trajectory_file) = &trajectory_file {
        println!("📊 Trajectory file: {}", trajectory_file.display());
    }

    // Load configuration using API-based system
    let config = match Config::from_api_configs(&config_dir).await {
        Ok(config) => {
            debug!("📋 Loaded API-based configuration from: {}", config_dir.display());
            config
        }
        Err(e) => {
            debug!("⚠️  Failed to load configuration from {}: {}", config_dir.display(), e);
            debug!("📋 Using default configuration");
            Config::default()
        }
    };

    // Get current working directory
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let project_path = current_dir.canonicalize().unwrap_or(current_dir);

    debug!("📁 Project path: {}", project_path.display());

    // Run the interactive mode (always use rich mode)
    run_rich_interactive(config, project_path).await
}
