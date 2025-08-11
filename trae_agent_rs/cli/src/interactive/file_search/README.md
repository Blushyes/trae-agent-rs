# High-Performance File Search System

This is a high-performance, extensible file search system designed for Trae Agent, replacing the original simple file listing functionality.

## Core Features

### 1. Intelligent Fuzzy Search

- **Exact Match**: Exact filename matches get highest priority
- **Continuous Match**: Substring matching, e.g., searching "main" matches "main.rs"
- **Prefix Match**: Files starting with search term, e.g., searching "lib" matches "lib.rs"
- **Word Boundary Match**: Matches at word boundaries, e.g., searching "mr" matches "main.rs" (m=main, r=rs)
- **Fuzzy Match**: Characters can have gaps, e.g., searching "mn" matches "main.rs"

### 2. Absolute Path Support (🆕 Latest Feature)

- **Smart Path Detection**: Automatically detects absolute path queries starting with `/`
- **Path Conversion**: Converts absolute paths to relative paths for project-scoped searching
- **Triple Matching**: Searches against file name, relative path, and absolute path
- **Fallback Handling**: Gracefully handles non-existent absolute paths
- **Cross-Platform**: Works with different path formats and project structures

### 3. Input Parsing System (🆕 Refactored)

- **Modular Design**: Input parsing logic separated into dedicated `input_parser.rs` module
- **Simple Trigger Logic**: Split by spaces, check if last segment starts with `@`
- **Cursor Awareness**: Handles cursor position for real-time search updates
- **Backspace Support**: Re-triggers search when content after `@path` is deleted

### 4. High-Performance Features

- **File Caching**: Smart caching of file lists to avoid repeated scanning
- **Real-time Search**: Results update in real-time during user input, response time <100ms
- **Memory Optimization**: Only cache necessary file information

### 5. Git Integration

- **Auto Ignore**: Follows .gitignore rules
- **Smart Filtering**: Automatically excludes common build artifacts and temporary files
- **Configurable**: Option to enable/disable Git integration

## Architecture Design

### Module Structure

```
file_search/
├── README.md           # This documentation file
├── mod.rs              # Main interface and exports
├── engine.rs           # Core search engine with absolute path support
├── input_parser.rs     # Input parsing logic for @ syntax
├── fuzzy.rs           # Intelligent fuzzy matching algorithm
├── cache.rs           # High-performance file caching system
├── config.rs          # Search configuration management
├── git_integration.rs # Git integration and .gitignore support
├── provider.rs        # Search provider abstraction
└── tests.rs           # Comprehensive test suite
```

### Core Components

#### FileSearchSystem

Main search interface, providing:

- `search(query: &str)` - Search files with intelligent matching
- `get_all_files()` - Get all files in the project
- `refresh()` - Refresh file cache

#### FileSearchEngine

Core search engine with advanced features:

- **Absolute Path Support**: Automatically converts absolute paths to relative paths for searching
- **Triple Matching Strategy**: Matches against file name, relative path, and absolute path
- **Smart Path Conversion**: Handles both existing and non-existing absolute paths
- **Performance Optimized**: Efficient search with configurable result limits

#### InputParser

Handles `@` syntax parsing and query extraction:

- `should_show_file_search(input, cursor_pos)` - Determines when to show search results
- `extract_search_query(input, cursor_pos)` - Extracts search query from user input
- **Simple Logic**: Split by spaces, check if last segment starts with `@`
- **Cursor Aware**: Handles cursor position for real-time updates

#### FuzzyMatcher

Intelligent matching algorithm, supporting:

- Priority ordering of multiple match types
- Detailed match score calculation
- Match position information (for highlighting)

#### FileCache

High-performance caching system:

- Recursive directory scanning
- Smart cache invalidation
- Memory-optimized file information storage

#### GitIgnoreFilter

Git integration features:

- .gitignore file parsing
- Wildcard pattern matching
- Default ignore rules

## Recent Architecture Improvements (🆕)

### Modular Refactoring

The file search system has been refactored for better modularity and maintainability:

**Before**: Input parsing logic scattered in `app.rs`
**After**: Dedicated `input_parser.rs` module with clear responsibilities

### Enhanced Search Capabilities

**Absolute Path Support**:

- Detects absolute path queries (starting with `/`)
- Converts absolute paths to relative paths for project-scoped searching
- Handles both existing and non-existing paths gracefully
- Maintains compatibility with existing relative path searches

**Improved Input Parsing**:

- Simplified trigger logic based on space-separated segments
- Better cursor position handling
- Enhanced backspace scenario support
- Comprehensive test coverage for edge cases

**Enhanced Display System**:

- Search results now show relative paths instead of just file names
- Better context for users to identify the correct file
- Consistent path display across all search scenarios
- Improved readability for nested directory structures

### Code Organization Benefits

1. **Separation of Concerns**: Input parsing separated from application logic
2. **Testability**: Input parsing logic can be tested independently
3. **Maintainability**: Related functionality grouped in logical modules
4. **Extensibility**: Easy to add new input parsing features

## User Experience

### Keyboard Shortcuts

- **↑/↓** or **Ctrl+P/Ctrl+N**: Move selection up/down
- **Tab** or **Enter**: Accept selected file
- **Esc**: Cancel file selection

### Search Experience

1. **Trigger Search**: Enter `@` to show all files
2. **Real-time Filtering**: Type search terms after `@` for instant results
3. **Smart Sorting**: Results automatically sorted by match quality and type
4. **Relative Path Display**: Search results show relative paths (e.g., `cli/src/main.rs`) for better context
5. **Mixed Content Support**: Use file references within larger text input
6. **Absolute Path Support**: Use absolute paths like `@/Users/pan/projects/file.txt`
7. **Path Conversion**: Absolute paths automatically converted to relative paths
8. **Backspace Handling**: Deleting content after `@path` re-triggers search

### Advanced Usage Patterns

**Basic File Search:**

```
@main                    → Search for files containing "main"
                          Results: cli/src/main.rs, core/src/main.rs
@src/lib                 → Search in src directory for "lib"
                          Results: cli/src/lib.rs, core/src/lib.rs
@.rs                     → Search for Rust files
                          Results: cli/src/main.rs, core/src/agent.rs, ...
```

**Absolute Path Search:**

```
@/Users/pan/projects/trae-agent-rs/trae_agent_rs/cli/src/
→ Automatically converts to: cli/src/
→ Results: cli/src/main.rs, cli/src/lib.rs, cli/src/interactive/
```

**Mixed Content:**

```
Please check @config.rs and @src/main.rs for the implementation
→ File references resolved to: core/src/config.rs, cli/src/main.rs
→ Both paths will be made clickable in the final message
```

**Interactive Scenarios:**

```
User types: @/absolute/path/file.txt content
→ Search hidden (has content after path)

User deletes " content":
→ Search re-appears showing: relative/path/file.txt
→ User can select from relative path results
```

## Performance Features

### Caching Strategy

- Default cache validity: 5 seconds
- Automatic cache invalidation detection
- On-demand refresh mechanism

### Search Optimization

- Maximum result limit: 50 items
- Minimum score threshold: 0.1
- Intelligent sorting algorithm

### Memory Management

- Cache only necessary file metadata
- Lazy loading strategy
- Automatic garbage collection

## Configuration Options

```rust
SearchConfig::default()
    .with_max_results(100)           // Maximum results
    .with_gitignore(true)            // Enable Git integration
    .with_hidden_files(false)        // Exclude hidden files
    .with_min_score(0.1)             // Minimum match score
    .exclude_extensions(vec![        // Exclude file types
        "exe".to_string(),
        "dll".to_string(),
    ])
```

## Extensibility

### Reserved Interfaces

System designed with future extensions in mind:

- **Semantic Search**: Search based on file content
- **Usage History**: Recently used files priority
- **Custom Sorting**: User-defined sorting rules
- **Search History**: Search records and suggestions

### Plugin Architecture

- Modular design for easy feature addition
- Clear interface separation
- Configuration-driven behavior

## Test Coverage

Includes comprehensive test suite:

- Unit Tests: Independent testing of each component
- Integration Tests: Complete search flow testing
- Performance Tests: Large file collection performance verification

## Usage Examples

### Basic Search System Usage

```rust
// Create search system
let config = SearchConfig::default();
let search_system = FileSearchSystem::new(project_path, config)?;

// Search files
let results = search_system.search("main");
for result in results {
    println!("{} (score: {:.2})",
        result.file.name,
        result.match_score.score
    );
}
```

### Input Parsing Usage

```rust
use crate::interactive::file_search::{should_show_file_search, extract_search_query};

// Check if input should trigger search
let input = "@src/main.rs";
let cursor_pos = input.len();

if should_show_file_search(input, cursor_pos) {
    if let Some(query) = extract_search_query(input, cursor_pos) {
        println!("Search query: {}", query); // Output: "src/main.rs"
    }
}

// Handle absolute paths
let abs_input = "@/Users/pan/projects/trae-agent-rs/cli/src/main.rs";
if should_show_file_search(abs_input, abs_input.len()) {
    if let Some(query) = extract_search_query(abs_input, abs_input.len()) {
        // query will be the absolute path, engine will convert to relative
        println!("Absolute query: {}", query);
    }
}
```

### Integration with UI Components

```rust
// In input_section.rs
use crate::interactive::file_search::{should_show_file_search, extract_search_query};

// Check if we should show file search
let should_show = should_show_file_search(&input_value, cursor_position);

if should_show {
    if let Some(query) = extract_search_query(&input_value, cursor_position) {
        // Trigger search with the extracted query
        let results = search_system.search(&query);
        // Display results in UI...
    }
}
```

## Testing

### Running Tests

```bash
# Run all file search tests
cargo test file_search

# Run specific test modules
cargo test input_parser::tests
cargo test engine::tests
cargo test fuzzy::tests

# Run with output
cargo test file_search -- --nocapture
```

### Test Coverage

- **Input Parser Tests**: 4 test functions covering all input scenarios
- **Engine Tests**: Search functionality and absolute path conversion
- **Fuzzy Matcher Tests**: All match types and scoring
- **Integration Tests**: End-to-end search workflows

This search system provides Trae Agent with powerful and flexible file discovery capabilities, greatly improving user productivity through intelligent search and seamless absolute path support.
