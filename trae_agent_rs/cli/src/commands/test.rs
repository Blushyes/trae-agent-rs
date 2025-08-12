//! Test command for basic functionality

use anyhow::Result;
use tracing::info;

/// Test basic functionality
pub async fn test_command() -> Result<()> {
    info!("Testing basic functionality");

    println!("🧪 Running Trae Agent Tests");

    // Test 1: Configuration loading
    println!("📋 Test 1: Configuration system");
    match test_config().await {
        Ok(_) => println!("   ✅ Configuration system works"),
        Err(e) => println!("   ❌ Configuration system failed: {}", e),
    }

    // Test 2: Tool system
    println!("🛠️  Test 2: Tool system");
    match test_tools().await {
        Ok(_) => println!("   ✅ Tool system works"),
        Err(e) => println!("   ❌ Tool system failed: {}", e),
    }

    // Test 3: LLM client (mock)
    println!("🤖 Test 3: LLM client");
    match test_llm().await {
        Ok(_) => println!("   ✅ LLM client interface works"),
        Err(e) => println!("   ❌ LLM client failed: {}", e),
    }

    println!("\n🎉 Basic tests completed!");

    Ok(())
}

async fn test_config() -> Result<()> {
    use trae_agent_rs_core::Config;

    // Test default configuration
    let config = Config::default();

    // Verify we have at least one agent
    if config.agents.is_empty() {
        return Err(anyhow::anyhow!("No agents in default config"));
    }

    // Verify we have at least one model
    if config.models.is_empty() {
        return Err(anyhow::anyhow!("No models in default config"));
    }

    // Verify we have at least one provider
    if config.model_providers.is_empty() {
        return Err(anyhow::anyhow!("No providers in default config"));
    }

    Ok(())
}

async fn test_tools() -> Result<()> {
    use trae_agent_rs_core::tools::ToolRegistry;

    let registry = ToolRegistry::default();
    let tools = registry.list_tools();

    if tools.is_empty() {
        return Err(anyhow::anyhow!("No tools registered"));
    }

    println!("   Available tools: {}", tools.join(", "));

    Ok(())
}

async fn test_llm() -> Result<()> {
    // Just test that we can create the types
    use trae_agent_rs_core::llm::{LlmMessage, MessageRole};

    let message = LlmMessage::system("Test system message");

    if message.role != MessageRole::System {
        return Err(anyhow::anyhow!("Message role mismatch"));
    }

    if message.get_text().is_none() {
        return Err(anyhow::anyhow!("Message text is None"));
    }

    Ok(())
}
