//! File cache for high-performance search

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Cached file information
#[derive(Debug, Clone)]
pub struct CachedFile {
    /// Full path to the file
    pub path: PathBuf,

    /// Relative path from project root
    pub relative_path: String,

    /// File name only
    pub name: String,

    /// Whether this is a directory
    pub is_directory: bool,

    /// File size in bytes (for files only)
    pub size: Option<u64>,

    /// Last modified time
    pub modified: Option<std::time::SystemTime>,

    /// Cached lowercase name for faster searching
    pub name_lowercase: String,

    /// Cached lowercase relative path for faster searching
    pub relative_path_lowercase: String,
}

/// High-performance file cache
pub struct FileCache {
    /// Cached files indexed by relative path
    files: HashMap<String, CachedFile>,

    /// Last cache update time
    last_update: Instant,

    /// Cache validity duration
    cache_duration: Duration,

    /// Project root path
    project_path: PathBuf,
}

impl FileCache {
    /// Create a new file cache
    pub fn new(project_path: PathBuf, cache_duration: Duration) -> Self {
        Self {
            files: HashMap::new(),
            last_update: Instant::now() - cache_duration, // Force initial update
            cache_duration,
            project_path,
        }
    }

    /// Check if cache is valid
    pub fn is_valid(&self) -> bool {
        self.last_update.elapsed() < self.cache_duration
    }

    /// Get all cached files
    pub fn get_files(&self) -> Vec<&CachedFile> {
        self.files.values().collect()
    }

    /// Get files matching a predicate
    pub fn get_files_filtered<F>(&self, predicate: F) -> Vec<&CachedFile>
    where
        F: Fn(&CachedFile) -> bool,
    {
        self.files.values().filter(|f| predicate(f)).collect()
    }

    /// Update the cache with fresh file information
    pub fn update<F>(&mut self, should_include: F) -> anyhow::Result<()>
    where
        F: Fn(&Path) -> bool,
    {
        self.files.clear();
        self.scan_directory(&self.project_path.clone(), "", &should_include)?;
        self.last_update = Instant::now();
        Ok(())
    }

    /// Recursively scan directory and populate cache
    fn scan_directory<F>(
        &mut self,
        dir_path: &Path,
        relative_prefix: &str,
        should_include: &F,
    ) -> anyhow::Result<()>
    where
        F: Fn(&Path) -> bool,
    {
        if !should_include(dir_path) {
            return Ok(());
        }

        let entries = fs::read_dir(dir_path)
            .map_err(|e| anyhow::anyhow!("Failed to read directory {:?}: {}", dir_path, e))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if !should_include(&path) {
                continue;
            }

            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let relative_path = if relative_prefix.is_empty() {
                file_name.clone()
            } else {
                format!("{}/{}", relative_prefix, file_name)
            };

            let metadata = entry.metadata().ok();
            let is_directory = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);

            let cached_file = CachedFile {
                path: path.clone(),
                relative_path: relative_path.clone(),
                name: file_name.clone(),
                is_directory,
                size: if is_directory {
                    None
                } else {
                    metadata.as_ref().map(|m| m.len())
                },
                modified: metadata.and_then(|m| m.modified().ok()),
                name_lowercase: file_name.to_lowercase(),
                relative_path_lowercase: relative_path.to_lowercase(),
            };

            self.files.insert(relative_path.clone(), cached_file);

            // Recursively scan subdirectories
            if is_directory {
                self.scan_directory(&path, &relative_path, should_include)?;
            }
        }

        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let (files, directories) = self.files.values().fold((0, 0), |(f, d), file| {
            if file.is_directory {
                (f, d + 1)
            } else {
                (f + 1, d)
            }
        });

        CacheStats {
            total_files: files,
            total_directories: directories,
            cache_age: self.last_update.elapsed(),
            is_valid: self.is_valid(),
        }
    }
}

/// Cache statistics
#[derive(Debug)]
pub struct CacheStats {
    pub total_files: usize,
    pub total_directories: usize,
    pub cache_age: Duration,
    pub is_valid: bool,
}
