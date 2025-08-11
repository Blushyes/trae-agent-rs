//! Performance benchmarks for file search system
//!
//! These benchmarks help measure the impact of various optimizations

use super::*;
use std::time::Instant;

#[cfg(test)]
mod benchmarks {
    use super::*;
    use tempfile;

    /// Create a test project with many files for benchmarking
    fn create_large_test_project() -> (tempfile::TempDir, std::path::PathBuf) {
        let temp_dir = tempfile::tempdir().unwrap();
        let project_path = temp_dir.path().to_path_buf();

        // Create a realistic project structure with many files
        let dirs = vec![
            "src/components",
            "src/utils",
            "src/services",
            "tests/unit",
            "tests/integration",
            "docs/api",
            "examples/basic",
            "examples/advanced",
        ];

        for dir in dirs {
            std::fs::create_dir_all(project_path.join(dir)).unwrap();
        }

        // Create many files to simulate a real project
        let files = vec![
            "src/main.rs",
            "src/lib.rs",
            "src/config.rs",
            "src/components/app.rs",
            "src/components/header.rs",
            "src/components/footer.rs",
            "src/utils/helpers.rs",
            "src/utils/validators.rs",
            "src/utils/formatters.rs",
            "src/services/api.rs",
            "src/services/auth.rs",
            "src/services/database.rs",
            "tests/unit/test_main.rs",
            "tests/unit/test_config.rs",
            "tests/unit/test_utils.rs",
            "tests/integration/test_api.rs",
            "tests/integration/test_auth.rs",
            "docs/api/README.md",
            "docs/api/endpoints.md",
            "docs/api/authentication.md",
            "examples/basic/hello.rs",
            "examples/basic/simple.rs",
            "examples/advanced/complex.rs",
            "examples/advanced/performance.rs",
            "Cargo.toml",
            "README.md",
            "LICENSE",
            ".gitignore",
        ];

        for file in files {
            if let Some(parent) = project_path.join(file).parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(project_path.join(file), format!("// Content of {}", file)).unwrap();
        }

        (temp_dir, project_path)
    }

    #[test]
    fn benchmark_search_performance() {
        let (_temp_dir, project_path) = create_large_test_project();
        let config = SearchConfig::default();
        let search_system = FileSearchSystem::new(project_path, config).unwrap();

        // Benchmark regular search
        let start = Instant::now();
        let results = search_system.search("rs");
        let search_duration = start.elapsed();

        println!("Regular search took: {:?}", search_duration);
        println!("Found {} results", results.len());

        // Benchmark search with exclusions
        let exclude_paths = vec!["src/main.rs", "src/lib.rs", "src/config.rs"];
        let start = Instant::now();
        let filtered_results = search_system.search_with_exclusions("rs", &exclude_paths);
        let exclusion_duration = start.elapsed();

        println!("Search with exclusions took: {:?}", exclusion_duration);
        println!("Found {} filtered results", filtered_results.len());

        // Verify exclusions work
        assert!(filtered_results.len() < results.len());
        assert!(!filtered_results
            .iter()
            .any(|r| exclude_paths.contains(&r.file.relative_path.as_str())));

        // Performance should be reasonable (less than 10ms for this small test)
        assert!(search_duration.as_millis() < 50);
        assert!(exclusion_duration.as_millis() < 50);
    }

    #[test]
    fn benchmark_get_all_files_performance() {
        let (_temp_dir, project_path) = create_large_test_project();
        let config = SearchConfig::default();
        let search_system = FileSearchSystem::new(project_path, config).unwrap();

        // Benchmark get all files
        let start = Instant::now();
        let all_files = search_system.get_all_files();
        let all_files_duration = start.elapsed();

        println!("Get all files took: {:?}", all_files_duration);
        println!("Found {} total files", all_files.len());

        // Benchmark get all files with exclusions
        let exclude_paths = vec!["src/main.rs", "README.md", "Cargo.toml"];
        let start = Instant::now();
        let filtered_files = search_system.get_all_files_with_exclusions(&exclude_paths);
        let filtered_duration = start.elapsed();

        println!(
            "Get all files with exclusions took: {:?}",
            filtered_duration
        );
        println!("Found {} filtered files", filtered_files.len());

        // Verify exclusions work
        assert!(filtered_files.len() < all_files.len());

        // Verify excluded files are not in results
        for exclude_path in &exclude_paths {
            assert!(!filtered_files
                .iter()
                .any(|r| r.file.relative_path == *exclude_path));
        }

        // Performance should be reasonable
        assert!(all_files_duration.as_millis() < 50);
        assert!(filtered_duration.as_millis() < 50);
    }

    #[test]
    fn benchmark_file_reference_extraction() {
        // Test input with many file references
        let input = "@src/main.rs check @config.rs and @utils/helper.rs also @tests/unit.rs and @docs/api.md @";

        // Benchmark file reference extraction
        let start = Instant::now();
        for _ in 0..1000 {
            let _refs = extract_existing_file_references(input, input.len());
        }
        let extraction_duration = start.elapsed();

        println!(
            "1000 file reference extractions took: {:?}",
            extraction_duration
        );
        println!("Average per extraction: {:?}", extraction_duration / 1000);

        // Should be very fast (less than 20ms total for 1000 extractions)
        assert!(extraction_duration.as_millis() < 20);
    }
}
