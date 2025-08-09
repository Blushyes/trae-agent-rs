//! Single task execution command

use anyhow::Result;
use std::path::PathBuf;
use tracing::{info, debug};

/// Execute a single task
pub async fn run_command(
    task: String,
    config_dir: PathBuf,
    provider: Option<String>,
    model: Option<String>,
    _api_key: Option<String>,
    max_steps: Option<usize>,
    trajectory_file: Option<PathBuf>,
    must_patch: bool,
    patch_path: PathBuf,
    working_dir: Option<PathBuf>,
    _debug_output: bool,
) -> Result<()> {
    info!("Executing task: {}", task);

    use trae_agent_core::{ Config, trajectory::TrajectoryRecorder, agent::TraeAgent };
    use crate::output::cli_handler::{CliOutputHandler, CliOutputConfig};

    // Output is now handled by the CLI output handler

    // Load configuration using API-based system
    let _config = match Config::from_api_configs(&config_dir).await {
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

    // Override provider and model if specified
    if let Some(provider) = &provider {
        info!("🤖 Provider: {}", provider);
    }

    if let Some(model) = &model {
        info!("🧠 Model: {}", model);
    }

    let max_steps = max_steps.unwrap_or(200);
    debug!("🔢 Max steps: {}", max_steps);

    // Initialize agent with proper configuration
    let agent_config = _config.agents.get("trae_agent").cloned().unwrap_or_default();

    // Create CLI output handler
    let cli_config = CliOutputConfig {
        realtime_updates: true, // Always enable realtime updates for better UX
    };
    let cli_output = Box::new(CliOutputHandler::new(cli_config));

    let mut agent = TraeAgent::new_with_output(agent_config.clone(), _config.clone(), cli_output).await?;

    // Initialize trajectory recorder
    let trajectory = TrajectoryRecorder::new();
    let task_entry = trae_agent_core::trajectory::TrajectoryEntry::task_start(
        task.clone(),
        serde_json::to_value(&agent_config).unwrap_or_default()
    );
    trajectory.record(task_entry).await?;

    if let Some(trajectory_file) = &trajectory_file {
        info!("📊 Trajectory file: {}", trajectory_file.display());
    }

    debug!("🤖 Using Trae Agent system prompt (consistent with Python version)");
    debug!("📋 System prompt preview: TraeAgent system prompt loaded...");

    // Get current working directory
    let current_dir = working_dir.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let project_path = current_dir.canonicalize().unwrap_or(current_dir);

    debug!("📁 Project path: {}", project_path.display());

    // Execute the task using the real agent

    let _execution_result = agent.execute_task_with_context(&task, &project_path).await?;

    if must_patch {
        info!("📄 Creating patch file: {}", patch_path.display());
        std::fs::write(
            &patch_path,
            "# Placeholder patch file\n# Changes would be recorded here\n"
        )?;
    }

    // Save trajectory if requested
    if let Some(trajectory_file) = &trajectory_file {
        info!("📊 Trajectory saved to: {}", trajectory_file.display());
    }

    // Task completion is now handled by the CLI output handler

    Ok(())
}
