//! Interactive mode command

use anyhow::Result;
use std::path::PathBuf;
use tracing::info;
use crate::interactive::app::run_interactive;

/// Start interactive mode
pub async fn interactive_command(
    config_path: PathBuf,
    trajectory_file: Option<PathBuf>,
    _debug_output: bool,
) -> Result<()> {
    info!("Starting interactive mode");

    println!("⚙️  Config: {}", config_path.display());

    if let Some(trajectory_file) = &trajectory_file {
        println!("📊 Trajectory file: {}", trajectory_file.display());
    }

    // Run the interactive mode (always use rich mode)
    run_interactive().await
}
