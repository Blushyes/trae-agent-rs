// File search provider trait and implementations
//
// This module provides an abstraction layer for file search functionality,
// allowing the UI components to be decoupled from specific search implementations.

use std::path::PathBuf;

/// Represents a file search result with display information
#[derive(Debug, Clone)]
pub struct FileSearchResult {
    /// Display name for the file (relative path, e.g., "src/main.rs", "cli/src/lib.rs")
    pub display_name: String,
    /// Full path to insert when selected
    pub insertion_path: String,
    /// Match score (0.0 to 1.0, higher is better)
    pub score: f64,
    /// Whether this is a directory
    pub is_directory: bool,
}

/// Trait for file search providers
pub trait FileSearchProvider: Send + Sync {
    /// Search for files matching the given query
    /// Returns results sorted by relevance (best matches first)
    fn search(&self, query: &str) -> Vec<FileSearchResult>;

    /// Search for files matching the given query, excluding specified paths
    fn search_with_exclusions(&self, query: &str, exclude_paths: &[&str]) -> Vec<FileSearchResult>;

    /// Get all available files (for empty query or initial display)
    fn get_all_files(&self) -> Vec<FileSearchResult>;

    /// Get all available files excluding specified paths
    fn get_all_files_with_exclusions(&self, exclude_paths: &[&str]) -> Vec<FileSearchResult>;

    /// Refresh the file cache (if applicable)
    fn refresh(&mut self) -> anyhow::Result<()>;
}

/// Default implementation using our FileSearchSystem
pub struct DefaultFileSearchProvider {
    search_system: super::FileSearchSystem,
}

impl DefaultFileSearchProvider {
    pub fn new(project_path: PathBuf) -> anyhow::Result<Self> {
        let config = super::SearchConfig::default();
        let search_system = super::FileSearchSystem::new(project_path, config)?;
        Ok(Self { search_system })
    }

    pub fn with_config(project_path: PathBuf, config: super::SearchConfig) -> anyhow::Result<Self> {
        let search_system = super::FileSearchSystem::new(project_path, config)?;
        Ok(Self { search_system })
    }
}

impl FileSearchProvider for DefaultFileSearchProvider {
    fn search(&self, query: &str) -> Vec<FileSearchResult> {
        self.search_system
            .search(query)
            .into_iter()
            .map(|result| FileSearchResult {
                display_name: if result.file.is_directory {
                    format!("{}/", result.file.relative_path)
                } else {
                    result.file.relative_path.clone()
                },
                insertion_path: result.get_insertion_text(),
                score: result.match_score.score,
                is_directory: result.file.is_directory,
            })
            .collect()
    }

    fn search_with_exclusions(&self, query: &str, exclude_paths: &[&str]) -> Vec<FileSearchResult> {
        self.search_system
            .search_with_exclusions(query, exclude_paths)
            .into_iter()
            .map(|result| FileSearchResult {
                display_name: if result.file.is_directory {
                    format!("{}/", result.file.relative_path)
                } else {
                    result.file.relative_path.clone()
                },
                insertion_path: result.get_insertion_text(),
                score: result.match_score.score,
                is_directory: result.file.is_directory,
            })
            .collect()
    }

    fn get_all_files(&self) -> Vec<FileSearchResult> {
        self.search_system
            .get_all_files()
            .into_iter()
            .map(|result| FileSearchResult {
                display_name: if result.file.is_directory {
                    format!("{}/", result.file.relative_path)
                } else {
                    result.file.relative_path.clone()
                },
                insertion_path: result.get_insertion_text(),
                score: result.match_score.score,
                is_directory: result.file.is_directory,
            })
            .collect()
    }

    fn get_all_files_with_exclusions(&self, exclude_paths: &[&str]) -> Vec<FileSearchResult> {
        self.search_system
            .get_all_files_with_exclusions(exclude_paths)
            .into_iter()
            .map(|result| FileSearchResult {
                display_name: if result.file.is_directory {
                    format!("{}/", result.file.relative_path)
                } else {
                    result.file.relative_path.clone()
                },
                insertion_path: result.get_insertion_text(),
                score: result.match_score.score,
                is_directory: result.file.is_directory,
            })
            .collect()
    }

    fn refresh(&mut self) -> anyhow::Result<()> {
        self.search_system.refresh()
    }
}

/// Mock implementation for testing
#[cfg(test)]
pub struct MockFileSearchProvider {
    files: Vec<FileSearchResult>,
}

#[cfg(test)]
impl MockFileSearchProvider {
    pub fn new(files: Vec<FileSearchResult>) -> Self {
        Self { files }
    }
}

#[cfg(test)]
impl FileSearchProvider for MockFileSearchProvider {
    fn search(&self, query: &str) -> Vec<FileSearchResult> {
        if query.is_empty() {
            return self.files.clone();
        }

        self.files
            .iter()
            .filter(|file| {
                file.display_name
                    .to_lowercase()
                    .contains(&query.to_lowercase())
            })
            .cloned()
            .collect()
    }

    fn search_with_exclusions(&self, query: &str, exclude_paths: &[&str]) -> Vec<FileSearchResult> {
        let base_results = if query.is_empty() {
            self.files.clone()
        } else {
            self.files
                .iter()
                .filter(|file| {
                    file.display_name
                        .to_lowercase()
                        .contains(&query.to_lowercase())
                })
                .cloned()
                .collect()
        };

        // Filter out excluded paths
        base_results
            .into_iter()
            .filter(|file| {
                !exclude_paths.contains(&file.display_name.as_str())
                    && !exclude_paths.contains(&file.insertion_path.as_str())
            })
            .collect()
    }

    fn get_all_files(&self) -> Vec<FileSearchResult> {
        self.files.clone()
    }

    fn get_all_files_with_exclusions(&self, exclude_paths: &[&str]) -> Vec<FileSearchResult> {
        self.files
            .iter()
            .filter(|file| {
                !exclude_paths.contains(&file.display_name.as_str())
                    && !exclude_paths.contains(&file.insertion_path.as_str())
            })
            .cloned()
            .collect()
    }

    fn refresh(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
