# High-Performance File Search System

This is a high-performance, extensible file search system designed for Trae Agent, replacing the original simple file listing functionality.

## Core Features

### 1. Intelligent Fuzzy Search

- **Exact Match**: Exact filename matches get highest priority
- **Continuous Match**: Substring matching, e.g., searching "main" matches "main.rs"
- **Prefix Match**: Files starting with search term, e.g., searching "lib" matches "lib.rs"
- **Word Boundary Match**: Matches at word boundaries, e.g., searching "mr" matches "main.rs" (m=main, r=rs)
- **Fuzzy Match**: Characters can have gaps, e.g., searching "mn" matches "main.rs"

### 2. High-Performance Features

- **File Caching**: Smart caching of file lists to avoid repeated scanning
- **Real-time Search**: Results update in real-time during user input, response time <100ms
- **Memory Optimization**: Only cache necessary file information

### 3. Git Integration

- **Auto Ignore**: Follows .gitignore rules
- **Smart Filtering**: Automatically excludes common build artifacts and temporary files
- **Configurable**: Option to enable/disable Git integration

## Architecture Design

### Module Structure

```
file_search/
├── mod.rs              # Main interface
├── engine.rs           # Core search engine
├── fuzzy.rs           # Fuzzy matching algorithm
├── cache.rs           # File caching system
├── git_integration.rs # Git integration
├── config.rs          # Configuration management
└── tests.rs           # Test suite
```

### Core Components

#### FileSearchSystem

Main search interface, providing:

- `search(query: &str)` - Search files
- `get_all_files()` - Get all files
- `refresh()` - Refresh cache

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

## User Experience

### Keyboard Shortcuts

- **↑/↓** or **Ctrl+P/Ctrl+N**: Move selection up/down
- **Tab** or **Enter**: Accept selected file
- **Esc**: Cancel file selection

### Search Experience

1. Enter `@` to show all files
2. Type search terms after `@` for real-time filtering
3. Results automatically sorted by match quality
4. Support for mixed directory and file display
5. **Insert absolute path when accepting files**, ensuring accurate path references

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

## Usage Example

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

This search system provides Trae Agent with powerful and flexible file discovery capabilities, greatly improving user productivity.
