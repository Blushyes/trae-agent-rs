# Trae Agent Rust

**语言**: [English](README.md) | [中文](README_zh.md)

[Trae Agent](https://github.com/bytedance/trae-agent) 的高性能 Rust 实现 - 基于大语言模型的软件工程任务智能代理。

![demo](./images/demo.gif)

## 🚀 项目状态

这是一个简单的 AI Coding Agent，具备先进的 UI 和工具能力。作为原始 [Trae Agent](https://github.com/bytedance/trae-agent) Python 实现的高性能移植版本，该版本在保持与工具规范完全兼容的同时，添加了 Rust 特有的性能优化和增强的 UI 功能。

**当前状态：**

- ✅ **核心架构**：模块化设计，独立的核心库和 CLI
- ✅ **配置系统**：基于 JSON/环境变量的灵活配置，具备类型安全
- ✅ **工具系统**：包含 7 个内置工具的综合工具生态系统
- ✅ **CLI 接口**：功能完整的命令行界面，支持多种执行模式
- ✅ **交互模式**：基于 iocraft 的丰富终端 UI，支持实时更新
- ✅ **LLM 集成**：完整的 Anthropic、OpenAI 和 Google API 集成
  - ✅ **OpenAI 集成**：完整的 OpenAI API 集成
  - ⏳ **Anthropic 集成**：即将到来
  - ⏳ **Google 集成**：即将到来
- ✅ **轨迹记录**：详细的执行日志和分析
- ✅ **补丁生成**：自动化代码变更跟踪和差异生成
- ✅ **文件搜索系统**：高性能模糊文件搜索，集成 Git
- ✅ **代理逻辑**：完整的推理循环，包含工具执行和上下文管理

**高级功能：**

- 🎨 **丰富的终端 UI**：基于 iocraft 的美观界面，支持动画和实时状态
- 🔍 **智能文件搜索**：支持 `@` 语法的模糊匹配，快速文件引用
- 📝 **输入历史**：持久化命令历史，支持键盘导航
- 🔧 **MCP 集成**：模型上下文协议支持，可接入外部工具提供者
- 📊 **实时状态**：动态状态行，显示进度跟踪和令牌使用情况
- 🎯 **上下文感知**：项目感知代理，智能路径解析

## 🏗️ 架构

项目采用清晰的模块化架构，关注点分离明确：

### 核心库 (`core/`)

- **`agent/`**：代理逻辑、执行引擎和系统提示
- **`config/`**：配置管理，支持 JSON/环境变量
- **`llm/`**：LLM 客户端抽象和提供商实现
- **`tools/`**：工具系统，包含 7 个内置工具和可扩展架构
- **`trajectory/`**：执行跟踪和分析
- **`output/`**：输出抽象层，实现清洁架构

### CLI 应用 (`cli/`)

- **`commands/`**：CLI 命令实现（run、interactive、tools、test）
- **`interactive/`**：丰富的终端 UI，集成 iocraft
  - **`components/`**：UI 组件（输入、状态、logo）
  - **`file_search/`**：高性能文件搜索系统
  - **`animation/`**：UI 动画和缓动函数
- **`output/`**：CLI 特定的输出处理器
- **`tools/`**：CLI 工具集成

## 🚀 快速开始

### 前置要求

- **Rust 1.70+**：最新稳定版 Rust 工具链
- **API 密钥**：您首选的 LLM 提供商（Anthropic、OpenAI 或 Google）

### 安装

```bash
# 克隆仓库
git clone https://github.com/your-org/trae-agent-rs
cd trae-agent-rs

# 构建项目（优化版本）
cargo build --release

# 全局安装 CLI
cargo install --path cli

# 或直接从源码运行
cargo run --bin trae
```

### ⚙️ 配置

Trae Agent 支持多种配置方法，提供最大灵活性：

#### 方法 1：JSON 配置文件（推荐）

在项目目录中创建特定提供商的 JSON 文件：

```bash
# Anthropic Claude（推荐）
echo '{
  "api_key": "your_anthropic_api_key",
  "model": "claude-3-5-sonnet-20241022"
}' > anthropic.json

# OpenAI GPT
echo '{
  "api_key": "your_openai_api_key",
  "model": "gpt-4o"
}' > openai.json

# Google Gemini
echo '{
  "api_key": "your_google_api_key",
  "model": "gemini-1.5-pro"
}' > google.json
```

#### 方法 2：环境变量

```bash
# Anthropic
export ANTHROPIC_API_KEY="your_anthropic_api_key"
export ANTHROPIC_MODEL="claude-3-5-sonnet-20241022"

# OpenAI
export OPENAI_API_KEY="your_openai_api_key"
export OPENAI_MODEL="gpt-4o"

# Google
export GOOGLE_API_KEY="your_google_api_key"
export GOOGLE_MODEL="gemini-1.5-pro"
```

### 🎯 使用方法

#### 安装

```bash
cargo install --git https://github.com/Blushyes/trae-agent-rs --bin trae-rs
trae-rs
```

#### 交互模式（推荐）

交互模式提供最佳体验，具备实时 UI 和文件搜索：

```bash
# 启动交互模式，丰富 UI
trae-rs interactive

# 或简单地（默认为交互模式）
trae-rs

# 带调试输出
trae-rs interactive --debug
```

**交互功能：**

- 🔍 **文件搜索**：输入 `@path/to/file` 搜索和引用文件
- ⬆️⬇️ **历史导航**：使用方向键导航命令历史
- 🎨 **实时 UI**：美观的终端界面，支持进度跟踪
- ⚡ **即时反馈**：实时状态更新和令牌使用情况

#### 单任务执行

适用于自动化工作流和 CI/CD 集成：

```bash
# 执行单个任务
trae-rs run "修复 main.rs 中的 bug"

# 指定提供商和模型
trae-rs run "创建一个 hello world 程序" --provider anthropic --model claude-3-5-sonnet-20241022

# 记录轨迹用于分析
trae-rs run "优化数据库查询" --trajectory-file analysis.json

# 生成代码变更的补丁文件
trae-rs run "修复认证 bug" --must-patch --patch-path fix.patch

# 指定工作目录
trae-rs run "添加单元测试" --working-dir /path/to/project
```

#### 工具管理

```bash
# 列出所有可用工具
trae-rs tools

# 测试基本功能
trae-rs test
```

## 🛠️ 开发

### 项目结构

```
trae_agent_rs/
├── core/                          # 核心库 (trae-agent-core)
│   ├── src/
│   │   ├── agent/                # 代理逻辑和执行引擎
│   │   │   ├── base.rs           # 代理特征和接口
│   │   │   ├── execution.rs      # 执行结果结构
│   │   │   ├── prompt.rs         # 系统提示和上下文
│   │   │   └── trae_agent.rs     # 主要代理实现
│   │   ├── config/               # 配置管理
│   │   │   ├── api_config.rs     # API 提供商配置
│   │   │   ├── config.rs         # 主配置结构
│   │   │   ├── loader.rs         # 配置加载逻辑
│   │   │   ├── model_config.rs   # 模型特定配置
│   │   │   └── provider_config.rs # 提供商配置
│   │   ├── llm/                  # LLM 客户端抽象
│   │   │   ├── client.rs         # LLM 客户端特征
│   │   │   ├── message.rs        # 消息结构
│   │   │   └── providers/        # 提供商实现
│   │   │       ├── anthropic.rs  # Anthropic Claude 客户端
│   │   │       └── openai.rs     # OpenAI GPT 客户端
│   │   ├── tools/                # 工具系统
│   │   │   ├── builtin/          # 内置工具
│   │   │   │   ├── bash.rs       # Shell 命令执行
│   │   │   │   ├── edit.rs       # 文件编辑工具
│   │   │   │   ├── json_edit.rs  # JSON 操作
│   │   │   │   ├── thinking.rs   # 顺序思考
│   │   │   │   ├── task_done.rs  # 任务完成
│   │   │   │   ├── ckg.rs        # 代码知识图谱
│   │   │   │   └── mcp.rs        # 模型上下文协议
│   │   │   ├── base.rs           # 工具特征和接口
│   │   │   ├── executor.rs       # 工具执行引擎
│   │   │   └── registry.rs       # 工具注册表
│   │   ├── trajectory/           # 执行跟踪
│   │   ├── output/               # 输出抽象层
│   │   └── error.rs              # 错误处理
├── cli/                          # CLI 应用 (trae-agent-cli)
│   ├── src/
│   │   ├── commands/             # CLI 命令实现
│   │   │   ├── run.rs            # 单任务执行
│   │   │   ├── interactive.rs    # 交互模式
│   │   │   ├── tools.rs          # 工具列表
│   │   │   └── test.rs           # 测试命令
│   │   ├── interactive/          # 丰富终端 UI
│   │   │   ├── app.rs            # 主应用组件
│   │   │   ├── components/       # UI 组件
│   │   │   │   ├── input_section.rs    # 增强输入，支持文件搜索
│   │   │   │   ├── status_line.rs      # 动态状态显示
│   │   │   │   └── logo.rs             # TRAE ASCII 艺术
│   │   │   ├── file_search/      # 高性能文件搜索
│   │   │   │   ├── engine.rs     # 核心搜索引擎
│   │   │   │   ├── fuzzy.rs      # 模糊匹配算法
│   │   │   │   ├── cache.rs      # 文件缓存系统
│   │   │   │   ├── git_integration.rs # Git 忽略支持
│   │   │   │   └── input_parser.rs    # @ 语法解析
│   │   │   ├── input_history.rs  # 命令历史管理
│   │   │   ├── animation.rs      # UI 动画
│   │   │   └── task_executor.rs  # 代理任务执行
│   │   ├── output/               # CLI 输出处理器
│   │   └── main.rs               # CLI 入口点
└── examples/                     # 示例配置
```

### 构建与测试

```bash
# 构建所有组件
cargo build

# 优化构建
cargo build --release

# 运行综合测试
cargo test

# 带调试日志运行
RUST_LOG=debug cargo run -- interactive

# 运行特定测试
cargo test --package trae-agent-core
cargo test --package trae-agent-cli

# 检查代码格式
cargo fmt --check

# 运行 clippy 检查
cargo clippy -- -D warnings
```

### 🤝 贡献

我们欢迎贡献！以下是开始的方法：

1. **Fork 仓库**并克隆您的 fork
2. **创建功能分支**：`git checkout -b feature/amazing-feature`
3. **按照编码标准进行更改**
4. **为新功能添加测试**
5. **运行测试套件**：`cargo test`
6. **检查格式**：`cargo fmt`
7. **运行检查**：`cargo clippy`
8. **提交拉取请求**，并提供清晰的描述

#### 开发指南

- **代码风格**：遵循 Rust 约定，使用 `cargo fmt`
- **测试**：为新功能添加单元测试
- **文档**：更新 README 和代码注释
- **错误处理**：使用适当的错误类型和处理
- **性能**：考虑更改的性能影响

#### 添加新工具

要添加新工具：

1. 在 `core/src/tools/builtin/` 中创建新文件
2. 实现 `Tool` 特征
3. 在 `builtin/mod.rs` 中将工具添加到注册表
4. 添加测试和文档

## 📊 性能

Trae Agent Rust 专为高性能设计：

- **启动时间**：< 100ms 冷启动
- **内存使用**：< 50MB 基线内存
- **文件搜索**：< 10ms 搜索 10k+ 文件，支持模糊匹配
- **UI 响应性**：60fps 动画，< 16ms 帧时间
- **并发操作**：非阻塞异步架构

## 🔧 高级功能

### 文件搜索系统

`@` 语法支持强大的文件引用：

```bash
# 搜索和引用文件
"修复 @src/main.rs 中的 bug"

# 多文件引用
"比较 @src/lib.rs 和 @tests/integration.rs"

# 目录引用
"在 @tests/ 目录中添加测试"
```

**搜索功能：**

- **模糊匹配**：智能评分，支持多种匹配类型
- **Git 集成**：遵循 `.gitignore` 模式
- **性能**：缓存机制，搜索时间低于 10ms
- **绝对路径支持**：处理相对和绝对路径

### 输入历史

持久化命令历史，智能导航：

- **持久存储**：历史保存到 `input_history.txt`
- **键盘导航**：↑/↓ 箭头键浏览历史
- **重复预防**：避免连续重复条目
- **性能优化**：延迟保存机制，保证响应性

### 实时状态

动态状态行显示：

- **当前操作**：代理当前正在执行的操作
- **经过时间**：当前操作运行的时长
- **令牌使用**：实时令牌消耗跟踪
- **进度指示器**：带动画的视觉反馈

## 📄 许可证

根据以下任一许可证授权

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

由您选择。

## 🙏 致谢

- **[Trae Agent](https://github.com/bytedance/trae-agent)**：字节跳动的原始 Python 实现，为这个 Rust 移植版本提供了宝贵的参考和灵感
- **iocraft**：提供美观的终端 UI 框架
- **Anthropic**：提供 Claude API 和出色的工具调用支持
- **OpenAI**：提供 GPT 模型和 API
- **Google**：提供 Gemini 模型
- **Rust 社区**：提供令人惊叹的生态系统和工具

---

**用 ❤️ 在 Rust 中构建** | **由 LLMs 驱动** | **为开发者设计**
