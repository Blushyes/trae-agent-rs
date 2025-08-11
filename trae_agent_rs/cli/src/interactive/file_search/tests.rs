//! Tests for the file search system

#[cfg(test)]
mod tests {
    use super::super::{FileSearchSystem, SearchConfig};
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_project() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_path_buf();

        println!("Creating test project at: {:?}", project_path);

        // Create test files
        fs::write(project_path.join("main.rs"), "fn main() {}").unwrap();
        fs::write(project_path.join("lib.rs"), "pub mod test;").unwrap();
        fs::write(project_path.join("config.toml"), "[package]").unwrap();
        fs::write(project_path.join("README.md"), "# Test Project").unwrap();

        // Create subdirectory
        fs::create_dir(project_path.join("src")).unwrap();
        fs::write(project_path.join("src/module.rs"), "pub fn test() {}").unwrap();
        fs::write(project_path.join("src/utils.rs"), "pub fn helper() {}").unwrap();

        // Verify files were created
        println!("Files in test project:");
        if let Ok(entries) = fs::read_dir(&project_path) {
            for entry in entries.flatten() {
                println!("  - {:?}", entry.file_name());
            }
        }

        (temp_dir, project_path)
    }

    #[test]
    fn test_search_system_creation() {
        let (_temp_dir, project_path) = create_test_project();
        let config = SearchConfig::default().with_gitignore(false); // Disable git filtering for test

        let search_system = FileSearchSystem::new(project_path, config);
        assert!(search_system.is_ok());

        // Debug: print what files were found
        if let Ok(system) = search_system {
            let all_files = system.get_all_files();
            println!("Found {} files:", all_files.len());
            for result in &all_files {
                println!("  - {} ({})", result.file.name, result.file.relative_path);
            }

            // Also test search functionality
            let search_results = system.search("main");
            println!("Search for 'main' found {} results:", search_results.len());
            for result in &search_results {
                println!(
                    "  - {} (score: {:.2})",
                    result.file.name, result.match_score.score
                );
            }
        }
    }

    #[test]
    fn test_get_all_files() {
        let (_temp_dir, project_path) = create_test_project();
        let config = SearchConfig::default();
        let search_system = FileSearchSystem::new(project_path, config).unwrap();

        let results = search_system.get_all_files();
        assert!(!results.is_empty());

        // Should find our test files
        let file_names: Vec<String> = results.iter().map(|r| r.file.name.clone()).collect();

        assert!(file_names.contains(&"main.rs".to_string()));
        assert!(file_names.contains(&"lib.rs".to_string()));
        assert!(file_names.contains(&"README.md".to_string()));
    }

    #[test]
    fn test_search_functionality() {
        let (_temp_dir, project_path) = create_test_project();
        let config = SearchConfig::default();
        let search_system = FileSearchSystem::new(project_path, config).unwrap();

        // Test exact match
        let results = search_system.search("main.rs");
        assert!(!results.is_empty());
        assert_eq!(results[0].file.name, "main.rs");

        // Test fuzzy match
        let results = search_system.search("mr");
        assert!(!results.is_empty());
        // Should find main.rs
        let found_main = results.iter().any(|r| r.file.name == "main.rs");
        assert!(found_main);

        // Test prefix match
        let results = search_system.search("main");
        assert!(!results.is_empty());
        assert_eq!(results[0].file.name, "main.rs");
    }

    #[test]
    fn test_empty_query() {
        let (_temp_dir, project_path) = create_test_project();
        let config = SearchConfig::default().with_gitignore(false);
        let search_system = FileSearchSystem::new(project_path, config).unwrap();

        let results = search_system.search("");
        assert!(!results.is_empty());
        // Empty query should return all files
        assert!(results.len() >= 4); // At least our test files
    }

    #[test]
    fn test_absolute_path_insertion() {
        let (_temp_dir, project_path) = create_test_project();
        let config = SearchConfig::default().with_gitignore(false);
        let search_system = FileSearchSystem::new(project_path.clone(), config).unwrap();

        let results = search_system.search("main.rs");
        assert!(!results.is_empty());

        let main_result = &results[0];
        assert_eq!(main_result.file.name, "main.rs");

        // Check that insertion text is absolute path
        let insertion_text = main_result.get_insertion_text();
        assert!(insertion_text.starts_with("/"));
        assert!(insertion_text.ends_with("main.rs"));
        assert!(insertion_text.contains(&project_path.to_string_lossy().to_string()));

        println!("Insertion text: {}", insertion_text);
    }
}
