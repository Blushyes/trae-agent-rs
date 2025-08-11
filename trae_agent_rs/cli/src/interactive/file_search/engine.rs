//! Core file search engine

use super::{
    cache::{CachedFile, FileCache},
    config::SearchConfig,
    fuzzy::{FuzzyMatcher, MatchScore, MatchType},
    git_integration::GitIgnoreFilter,
};
use std::path::PathBuf;
use std::time::Duration;

/// Search result with scoring and highlighting information
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The file information
    pub file: CachedFile,

    /// Match score details
    pub match_score: MatchScore,

    /// Display name with highlighting markers
    pub display_name: String,
}

impl SearchResult {
    /// Create a new search result
    pub fn new(file: CachedFile, match_score: MatchScore) -> Self {
        let display_name = if file.is_directory {
            format!("{}/", file.relative_path)
        } else {
            file.relative_path.clone()
        };

        Self {
            file,
            match_score,
            display_name,
        }
    }

    /// Get the absolute path for insertion
    pub fn get_insertion_text(&self) -> String {
        self.file.path.to_string_lossy().to_string()
    }
}

/// High-performance file search engine
pub struct FileSearchEngine {
    /// File cache for fast lookups
    cache: FileCache,

    /// Fuzzy matcher for scoring
    matcher: FuzzyMatcher,

    /// Git ignore filter
    git_filter: Option<GitIgnoreFilter>,

    /// Search configuration
    config: SearchConfig,

    /// Project root path
    project_path: PathBuf,
}

impl FileSearchEngine {
    /// Create a new search engine
    pub fn new(project_path: PathBuf, config: SearchConfig) -> anyhow::Result<Self> {
        let cache_duration = Duration::from_secs(config.cache_refresh_interval);
        let cache = FileCache::new(project_path.clone(), cache_duration);

        let git_filter = if config.respect_gitignore {
            Some(GitIgnoreFilter::new(&project_path)?)
        } else {
            None
        };

        let matcher = FuzzyMatcher::new(false); // Case insensitive

        let mut engine = Self {
            cache,
            matcher,
            git_filter,
            config,
            project_path,
        };

        // Initial cache population
        engine.refresh()?;

        Ok(engine)
    }

    /// Search for files matching the query
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        // Ensure cache is fresh
        if !self.cache.is_valid() {
            // Note: In a real implementation, we might want to refresh asynchronously
            // For now, we'll work with the existing cache
        }

        let query = query.trim();

        // If query is empty, return all files
        if query.is_empty() {
            return self.get_all_files();
        }

        // Check if query is an absolute path and convert to relative if needed
        let search_query = if query.starts_with('/') {
            // Try to convert absolute path to relative path
            if let Ok(abs_path) = std::path::Path::new(query).canonicalize() {
                if let Ok(rel_path) = abs_path.strip_prefix(&self.project_path) {
                    rel_path.to_string_lossy().to_string()
                } else {
                    // Absolute path outside project, use as-is
                    query.to_string()
                }
            } else {
                // Invalid absolute path, try to make it relative by removing project path prefix
                if let Some(project_str) = self.project_path.to_str() {
                    if query.starts_with(project_str) {
                        let relative = &query[project_str.len()..];
                        relative.trim_start_matches('/').to_string()
                    } else {
                        query.to_string()
                    }
                } else {
                    query.to_string()
                }
            }
        } else {
            query.to_string()
        };

        let mut results = Vec::new();

        // Search through cached files
        for file in self.cache.get_files() {
            // Apply filters
            if !self.should_include_file(file) {
                continue;
            }

            // Try to match the query against file name, relative path, and absolute path
            let name_match = self.matcher.match_string(&search_query, &file.name);
            let relative_path_match = self
                .matcher
                .match_string(&search_query, &file.relative_path);
            let absolute_path_match = self
                .matcher
                .match_string(&search_query, &file.path.to_string_lossy());

            // Use the best of the three matches
            let best_match = [name_match, relative_path_match, absolute_path_match]
                .into_iter()
                .flatten()
                .max_by(|a, b| {
                    a.score
                        .partial_cmp(&b.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

            if let Some(match_score) = best_match {
                if match_score.score >= self.config.min_score_threshold {
                    results.push(SearchResult::new(file.clone(), match_score));
                }
            }
        }

        // Sort results by score and match type
        results.sort_by(|a, b| {
            // First sort by match type (exact > continuous > prefix > word_start > fuzzy)
            let type_cmp = a.match_score.match_type.cmp(&b.match_score.match_type);
            if type_cmp != std::cmp::Ordering::Equal {
                return type_cmp;
            }

            // Then by score (higher is better)
            b.match_score
                .score
                .partial_cmp(&a.match_score.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results
        results.truncate(self.config.max_results);

        results
    }

    /// Get all files without filtering
    pub fn get_all_files(&self) -> Vec<SearchResult> {
        let mut results = Vec::new();

        for file in self.cache.get_files() {
            if !self.should_include_file(file) {
                continue;
            }

            let match_score = MatchScore {
                score: 1.0,
                matched_positions: Vec::new(),
                match_type: MatchType::Exact,
            };

            results.push(SearchResult::new(file.clone(), match_score));
        }

        // Sort: directories first, then files, alphabetically
        results.sort_by(|a, b| {
            let a_is_dir = a.file.is_directory;
            let b_is_dir = b.file.is_directory;

            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.file.name.cmp(&b.file.name),
            }
        });

        // Limit results
        results.truncate(self.config.max_results);

        results
    }

    /// Refresh the file cache
    pub fn refresh(&mut self) -> anyhow::Result<()> {
        let git_filter = self.git_filter.as_ref();

        self.cache.update(|path| {
            // Don't apply filters to the root project directory itself
            if path == self.project_path {
                return true;
            }

            // Apply git ignore filter
            if let Some(filter) = git_filter {
                if filter.should_ignore(path) {
                    return false;
                }
            }

            // Apply hidden file filter (but not to the root directory)
            if !self.config.include_hidden {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.') {
                        return false;
                    }
                }
            }
            true
        })?;

        Ok(())
    }

    /// Check if a file should be included in results
    fn should_include_file(&self, file: &CachedFile) -> bool {
        // Check extension filters
        if !self.config.include_extensions.is_empty() {
            if let Some(ext) = file.path.extension().and_then(|e| e.to_str()) {
                if !self.config.include_extensions.contains(ext) {
                    return false;
                }
            } else {
                // No extension, exclude if we're filtering by extension
                return false;
            }
        }

        // Check exclude extensions
        if let Some(ext) = file.path.extension().and_then(|e| e.to_str()) {
            if self.config.exclude_extensions.contains(ext) {
                return false;
            }
        }

        true
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> super::cache::CacheStats {
        self.cache.stats()
    }
}
