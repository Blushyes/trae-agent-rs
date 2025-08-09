# Trae Agent Rust

A Rust implementation of Trae Agent - an LLM-based agent for software engineering tasks.

## 🚧 Project Status

This is a **proof-of-concept** implementation that demonstrates the core architecture and functionality of Trae Agent in Rust.

**Current Status:**

- ✅ **Core Architecture**: Modular design with separate core library and CLI
- ✅ **Configuration System**: YAML-based configuration with type safety
- ✅ **Tool System**: Pluggable tool architecture with built-in tools
- ✅ **CLI Interface**: Command-line interface with multiple modes
- ✅ **Interactive Mode**: Basic terminal-based interaction
- ✅ **Trajectory Recording**: Execution logging and analysis
- ✅ **Patch Generation**: Code change tracking
- 🚧 **LLM Integration**: Simplified mock implementation (needs real API integration)
- 🚧 **Rich UI**: Basic terminal UI (iocraft integration planned)
- 🚧 **Agent Logic**: Simplified execution flow (needs full reasoning loop)

**Next Steps:**

1. Implement actual LLM API integration (Anthropic, OpenAI)
2. Add real tool implementations (bash, file editing, etc.)
3. Enhance interactive UI with iocraft
4. Implement sophisticated agent reasoning
5. Add comprehensive test coverage

## Features

- 🤖 **AI-Powered Agent**: Intelligent agent capable of complex software engineering tasks
- 🛠️ **Rich Tool System**: Built-in tools for bash execution, file editing, and more
- 🎨 **Beautiful CLI**: Modern terminal UI powered by iocraft
- ⚡ **High Performance**: Built with Rust for speed and reliability
- 🔧 **Configurable**: Flexible configuration system supporting multiple LLM providers
- 📊 **Trajectory Recording**: Detailed execution tracking for debugging and analysis
- 🔄 **Interactive Mode**: Real-time conversation interface inspired by gemini-cli

## Architecture

The project is organized into two main components:

- **`core/`**: Core library containing agent logic, tools, and LLM abstractions
- **`cli/`**: Command-line interface with beautiful terminal UI

## Quick Start

### Prerequisites

- Rust 1.70+
- An API key for your preferred LLM provider (Anthropic, OpenAI, etc.)

### Installation

```bash
# Clone the repository
git clone <repository-url>
cd trae-agent-rs

# Build the project
cargo build --release

# Install the CLI
cargo install --path cli
```

### Configuration

Trae Agent uses a flexible API-based configuration system. Create provider-specific JSON files:

```bash
# For Anthropic
echo '{"api_key": "your_anthropic_api_key", "model": "claude-3-5-sonnet-20241022"}' > anthropic.json

# For OpenAI
echo '{"api_key": "your_openai_api_key", "model": "gpt-4"}' > openai.json

# For Google
echo '{"api_key": "your_google_api_key", "model": "gemini-pro"}' > google.json
```

Or use environment variables:

```bash
export ANTHROPIC_API_KEY="your_anthropic_api_key"
export ANTHROPIC_MODEL="claude-3-5-sonnet-20241022"
```

### Usage

#### Single Task Execution

```bash
# Execute a single task
trae run "Fix the bug in main.rs"

# With trajectory recording
trae run "Create a hello world program" --trajectory-file trajectory.json

# With patch generation
trae run "Fix the authentication bug" --must-patch --patch-path fix.patch

# With custom configuration
trae run "Optimize the database queries" --config my_config.yaml
```

#### Interactive Mode

```bash
# Start interactive mode
trae interactive

# With rich UI (when implemented)
trae interactive --console-type rich

# Or simply
trae
```

#### List Available Tools

```bash
trae tools
```

#### Run Tests

```bash
# Test basic functionality
trae test
```

## Development

### Project Structure

```
trae_agent_rs/
├── core/                   # Core library
│   ├── src/
│   │   ├── agent/         # Agent logic
│   │   ├── config/        # Configuration system
│   │   ├── llm/           # LLM client abstractions
│   │   ├── tools/         # Tool system
│   │   ├── trajectory/    # Execution tracking
│   │   └── error.rs       # Error handling
├── cli/                   # CLI application
│   ├── src/
│   │   ├── commands/      # CLI commands
│   │   ├── ui/            # iocraft UI components
│   │   └── interactive/   # Interactive mode
└── examples/              # Examples and documentation
```

### Building

```bash
# Build all components
cargo build

# Build with optimizations
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run
```

### Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
