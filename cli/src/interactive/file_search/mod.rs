//! High-performance file search system
//!
//! This module provides a comprehensive file search system with the following features:
//! - Fuzzy matching with intelligent scoring
//! - Git integration (respects .gitignore)
//! - Real-time search with <100ms response time
//! - Extensible architecture for future enhancements

pub mod cache;
pub mod config;
pub mod engine;
pub mod fuzzy;
pub mod git_integration;
pub mod input_parser;
pub mod provider;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod benchmarks;

// Export the main interfaces
pub use config::SearchConfig;
pub use engine::{FileSearchEngine, SearchResult};
pub use input_parser::{
    extract_existing_file_references, extract_search_query, should_show_file_search,
};
pub use provider::{DefaultFileSearchProvider, FileSearchProvider, FileSearchResult};

/// Main search interface
pub struct FileSearchSystem {
    engine: FileSearchEngine,
    config: SearchConfig,
}

impl FileSearchSystem {
    /// Create a new file search system
    pub fn new(project_path: std::path::PathBuf, config: SearchConfig) -> anyhow::Result<Self> {
        let engine = FileSearchEngine::new(project_path, config.clone())?;
        Ok(Self { engine, config })
    }

    /// Search for files matching the query
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        self.engine.search(query)
    }

    /// Search for files matching the query, excluding specified paths
    pub fn search_with_exclusions(&self, query: &str, exclude_paths: &[&str]) -> Vec<SearchResult> {
        self.engine.search_with_exclusions(query, exclude_paths)
    }

    /// Refresh the file cache
    pub fn refresh(&mut self) -> anyhow::Result<()> {
        self.engine.refresh()
    }

    /// Get all files (for @ without query)
    pub fn get_all_files(&self) -> Vec<SearchResult> {
        self.engine.get_all_files()
    }

    /// Get all files excluding specified paths
    pub fn get_all_files_with_exclusions(&self, exclude_paths: &[&str]) -> Vec<SearchResult> {
        self.engine.get_all_files_with_exclusions(exclude_paths)
    }
}
