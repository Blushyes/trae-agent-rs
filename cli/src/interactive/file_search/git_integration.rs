//! Git integration for file search

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Git ignore filter that respects .gitignore rules
pub struct GitIgnoreFilter {
    /// Patterns from .gitignore files
    ignore_patterns: Vec<GitIgnorePattern>,
    
    /// Cached ignored paths for performance
    ignored_paths: HashSet<PathBuf>,
}

/// A single gitignore pattern
#[derive(Debug, Clone)]
struct GitIgnorePattern {
    pattern: String,
    is_directory: bool,
    is_negation: bool,
}

impl GitIgnoreFilter {
    /// Create a new git ignore filter
    pub fn new(project_path: &Path) -> anyhow::Result<Self> {
        let mut filter = Self {
            ignore_patterns: Vec::new(),
            ignored_paths: HashSet::new(),
        };
        
        filter.load_gitignore_files(project_path)?;
        Ok(filter)
    }
    
    /// Check if a path should be ignored
    pub fn should_ignore(&self, path: &Path) -> bool {
        // Always ignore .git directory
        if path.file_name().and_then(|n| n.to_str()) == Some(".git") {
            return true;
        }
        
        // Check cached ignored paths first
        if self.ignored_paths.contains(path) {
            return true;
        }
        
        // Check against patterns
        for pattern in &self.ignore_patterns {
            if self.matches_pattern(pattern, path) {
                return !pattern.is_negation;
            }
        }
        
        false
    }
    
    /// Load .gitignore files from the project
    fn load_gitignore_files(&mut self, project_path: &Path) -> anyhow::Result<()> {
        // Load root .gitignore
        let gitignore_path = project_path.join(".gitignore");
        if gitignore_path.exists() {
            self.load_gitignore_file(&gitignore_path)?;
        }
        
        // Add common ignore patterns
        self.add_default_patterns();
        
        Ok(())
    }
    
    /// Load a single .gitignore file
    fn load_gitignore_file(&mut self, gitignore_path: &Path) -> anyhow::Result<()> {
        let content = fs::read_to_string(gitignore_path)?;
        
        for line in content.lines() {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            let is_negation = line.starts_with('!');
            let pattern = if is_negation { &line[1..] } else { line };
            let is_directory = pattern.ends_with('/');
            let pattern = if is_directory { 
                pattern.trim_end_matches('/').to_string() 
            } else { 
                pattern.to_string() 
            };
            
            self.ignore_patterns.push(GitIgnorePattern {
                pattern,
                is_directory,
                is_negation,
            });
        }
        
        Ok(())
    }
    
    /// Add default ignore patterns
    fn add_default_patterns(&mut self) {
        let default_patterns = vec![
            // Version control
            ".git/",
            ".svn/",
            ".hg/",
            
            // Build outputs
            "target/",
            "build/",
            "dist/",
            "out/",
            "bin/",
            "obj/",
            
            // Dependencies
            "node_modules/",
            "vendor/",
            ".cargo/",
            
            // IDE files
            ".vscode/",
            ".idea/",
            "*.swp",
            "*.swo",
            "*~",
            
            // OS files
            ".DS_Store",
            "Thumbs.db",
            "desktop.ini",
        ];
        
        for pattern in default_patterns {
            let is_directory = pattern.ends_with('/');
            let pattern = if is_directory { 
                pattern.trim_end_matches('/').to_string() 
            } else { 
                pattern.to_string() 
            };
            
            self.ignore_patterns.push(GitIgnorePattern {
                pattern,
                is_directory,
                is_negation: false,
            });
        }
    }
    
    /// Check if a path matches a gitignore pattern
    fn matches_pattern(&self, pattern: &GitIgnorePattern, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        
        // Simple pattern matching (can be enhanced with glob patterns later)
        if pattern.pattern.contains('*') {
            // Basic wildcard support
            self.matches_wildcard(&pattern.pattern, &path_str) || 
            self.matches_wildcard(&pattern.pattern, file_name)
        } else {
            // Exact match
            path_str.contains(&pattern.pattern) || 
            file_name == pattern.pattern ||
            path_str.ends_with(&format!("/{}", pattern.pattern))
        }
    }
    
    /// Simple wildcard matching
    fn matches_wildcard(&self, pattern: &str, text: &str) -> bool {
        if !pattern.contains('*') {
            return text == pattern;
        }
        
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.is_empty() {
            return true;
        }
        
        let mut text_pos = 0;
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }
            
            if i == 0 {
                // First part must match at the beginning
                if !text[text_pos..].starts_with(part) {
                    return false;
                }
                text_pos += part.len();
            } else if i == parts.len() - 1 {
                // Last part must match at the end
                return text[text_pos..].ends_with(part);
            } else {
                // Middle parts can match anywhere
                if let Some(pos) = text[text_pos..].find(part) {
                    text_pos += pos + part.len();
                } else {
                    return false;
                }
            }
        }
        
        true
    }
}

impl Default for GitIgnoreFilter {
    fn default() -> Self {
        Self {
            ignore_patterns: Vec::new(),
            ignored_paths: HashSet::new(),
        }
    }
}
