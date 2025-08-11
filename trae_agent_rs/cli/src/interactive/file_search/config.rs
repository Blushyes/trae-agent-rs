//! Search configuration and settings

use std::collections::HashSet;

/// Configuration for file search behavior
#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// Maximum directory depth to search
    pub max_depth: usize,
    
    /// Maximum number of results to return
    pub max_results: usize,
    
    /// File extensions to include (empty means all)
    pub include_extensions: HashSet<String>,
    
    /// File extensions to exclude
    pub exclude_extensions: HashSet<String>,
    
    /// Whether to respect .gitignore files
    pub respect_gitignore: bool,
    
    /// Whether to include hidden files (starting with .)
    pub include_hidden: bool,
    
    /// Minimum score threshold for fuzzy matching
    pub min_score_threshold: f64,
    
    /// Whether to cache file listings for performance
    pub enable_caching: bool,
    
    /// Cache refresh interval in seconds
    pub cache_refresh_interval: u64,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            max_results: 50,
            include_extensions: HashSet::new(),
            exclude_extensions: {
                let mut set = HashSet::new();
                // Common binary/generated file extensions to exclude
                set.insert("exe".to_string());
                set.insert("dll".to_string());
                set.insert("so".to_string());
                set.insert("dylib".to_string());
                set.insert("a".to_string());
                set.insert("o".to_string());
                set.insert("obj".to_string());
                set.insert("bin".to_string());
                set.insert("class".to_string());
                set.insert("jar".to_string());
                set.insert("war".to_string());
                set.insert("pyc".to_string());
                set.insert("pyo".to_string());
                set.insert("pyd".to_string());
                set
            },
            respect_gitignore: true,
            include_hidden: false,
            min_score_threshold: 0.1,
            enable_caching: true,
            cache_refresh_interval: 5, // 5 seconds
        }
    }
}

impl SearchConfig {
    /// Create a new search config with custom settings
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set maximum search depth
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }
    
    /// Set maximum results
    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }
    
    /// Include specific file extensions
    pub fn with_extensions(mut self, extensions: Vec<String>) -> Self {
        self.include_extensions = extensions.into_iter().collect();
        self
    }
    
    /// Exclude specific file extensions
    pub fn exclude_extensions(mut self, extensions: Vec<String>) -> Self {
        self.exclude_extensions = extensions.into_iter().collect();
        self
    }
    
    /// Enable or disable gitignore respect
    pub fn with_gitignore(mut self, respect: bool) -> Self {
        self.respect_gitignore = respect;
        self
    }
    
    /// Include or exclude hidden files
    pub fn with_hidden_files(mut self, include: bool) -> Self {
        self.include_hidden = include;
        self
    }
    
    /// Set minimum score threshold
    pub fn with_min_score(mut self, threshold: f64) -> Self {
        self.min_score_threshold = threshold;
        self
    }
}
