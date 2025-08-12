//! Tools listing command

use anyhow::Result;
use tracing::info;

/// Show available tools
pub async fn tools_command() -> Result<()> {
    info!("Listing available tools");

    use trae_agent_rs_core::tools::ToolRegistry;

    println!("🛠️  Available Tools\n");

    let registry = ToolRegistry::default();
    let tool_names = registry.list_tools();

    for name in tool_names {
        if let Some((tool_name, description)) = registry.get_tool_info(name) {
            println!("📦 {}", tool_name);
            // Show first line of description only for brevity
            let first_line = description.lines().next().unwrap_or(description);
            println!("   {}\n", first_line);
        }
    }

    println!("💡 Use these tools in your tasks to accomplish complex workflows!");
    println!("📋 All tools follow the exact same specifications as the Python version of Trae Agent.");

    Ok(())
}
