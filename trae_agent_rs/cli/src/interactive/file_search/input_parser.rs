//! Input parsing logic for file search functionality

/// Check if input should trigger file search list
/// 
/// Uses simple logic: split by spaces, if last segment starts with @, trigger search
pub fn should_show_file_search(input: &str, cursor_pos: usize) -> bool {
    if input.is_empty() {
        return false;
    }

    let safe_cursor_pos = cursor_pos.min(input.len());
    
    // Get text before cursor
    let before_cursor = &input[..safe_cursor_pos];
    
    // Split by spaces and get the last segment
    let last_segment = before_cursor.split(' ').last().unwrap_or("");
    
    // Check if last segment starts with @ (including just @)
    last_segment.starts_with('@')
}

/// Extract search query from input value after the last @
/// 
/// Returns the content after @ in the last space-separated segment
pub fn extract_search_query(value: &str, cursor_pos: usize) -> Option<String> {
    if value.is_empty() {
        return None;
    }

    let safe_cursor_pos = cursor_pos.min(value.len());
    
    // Get text before cursor
    let before_cursor = &value[..safe_cursor_pos];
    
    // Split by spaces and get the last segment
    let last_segment = before_cursor.split(' ').last().unwrap_or("");
    
    // Check if last segment starts with @
    if last_segment.starts_with('@') {
        if last_segment.len() > 1 {
            Some(last_segment[1..].to_string()) // Remove @ prefix
        } else {
            Some(String::new()) // Just @ returns empty query
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_show_file_search() {
        // Should show when last segment starts with @ and has content
        assert!(should_show_file_search("@main", 5));
        assert!(should_show_file_search("@src/", 5));
        assert!(should_show_file_search("@src/main.rs", 12));
        assert!(should_show_file_search("@src/main.rs", 5)); // cursor in middle is OK now

        // Should not show when @ is not in the last segment (separated by space)
        assert!(!should_show_file_search("@src/main.rs hello", 13));
        assert!(!should_show_file_search("@src/main.rs hello", 18));

        // Should show when cursor is at end of path
        assert!(should_show_file_search("@/Users/pan/projects/file.txt", 30));
        
        // Should show when @ is in the last segment after spaces
        assert!(should_show_file_search("hello @main", 11));
        assert!(should_show_file_search("hello world @src/file", 22));
        
        // Should show even for just @ (triggers search with empty query)
        assert!(should_show_file_search("@", 1));
        assert!(should_show_file_search("hello @", 7));

        // Test the specific scenario: path without trailing content
        assert!(should_show_file_search(
            "@/Users/pan/projects/trae-agent-rs/trae_agent_rs/core/src/agent",
            66
        ));

        // Test backspace scenario: after deleting " 哈哈哈"
        let _original = "@/Users/pan/projects/trae-agent-rs/trae_agent_rs/core/src/agent 哈哈哈";
        let after_delete = "@/Users/pan/projects/trae-agent-rs/trae_agent_rs/core/src/agent";
        assert!(should_show_file_search(after_delete, after_delete.len()));

        // Test extract_search_query for the same scenario
        assert_eq!(
            extract_search_query(after_delete, after_delete.len()),
            Some("/Users/pan/projects/trae-agent-rs/trae_agent_rs/core/src/agent".to_string())
        );
    }

    #[test]
    fn test_backspace_scenario() {
        // Simulate the exact backspace scenario
        let _original = "@/Users/pan/projects/trae-agent-rs/trae_agent_rs/core/src/agent 哈哈哈";
        let after_delete = "@/Users/pan/projects/trae-agent-rs/trae_agent_rs/core/src/agent";

        // Test that should_show_file_search returns true when cursor is at end after deletion
        assert!(should_show_file_search(after_delete, after_delete.len()));

        // Test that extract_search_query works correctly
        let query = extract_search_query(after_delete, after_delete.len());
        assert!(query.is_some());
        assert_eq!(
            query.unwrap(),
            "/Users/pan/projects/trae-agent-rs/trae_agent_rs/core/src/agent"
        );

        // With new simple logic, cursor position in @path segment doesn't matter
        // So these should return true (cursor in @path segment)
        assert!(should_show_file_search(
            after_delete,
            after_delete.len() - 1
        ));
        assert!(should_show_file_search(
            after_delete,
            after_delete.len() - 5
        ));
    }

    #[test]
    fn test_extract_search_query() {
        // Test basic extraction from last segment
        assert_eq!(extract_search_query("@main", 5), Some("main".to_string()));
        assert_eq!(extract_search_query("@src/", 5), Some("src/".to_string()));

        // Test with cursor in middle - now extracts full segment
        assert_eq!(
            extract_search_query("@main.rs", 5),
            Some("main".to_string())
        );
        assert_eq!(
            extract_search_query("@main.rs", 8),
            Some("main.rs".to_string())
        );

        // Test with spaces - only last segment matters
        assert_eq!(
            extract_search_query("hello @main", 11), 
            Some("main".to_string())
        );
        assert_eq!(
            extract_search_query("@old hello @new", 15), 
            Some("new".to_string())
        );

        // Test no @ symbol
        assert_eq!(extract_search_query("hello", 5), None);

        // Test @ without content - should return empty string
        assert_eq!(extract_search_query("@", 1), Some(String::new()));
        assert_eq!(extract_search_query("hello @", 7), Some(String::new()));

        // Test @ not in last segment
        assert_eq!(extract_search_query("@file hello", 11), None);
    }

    #[test]
    fn test_absolute_path_support() {
        // Test that absolute paths are properly handled
        let abs_path =
            "/Users/pan/projects/trae-agent-rs/trae_agent_rs/cli/src/interactive/file_search/";

        // Should trigger search for absolute paths
        assert!(should_show_file_search(
            &format!("@{}", abs_path),
            abs_path.len() + 1
        ));

        // Should extract absolute path correctly
        assert_eq!(
            extract_search_query(&format!("@{}", abs_path), abs_path.len() + 1),
            Some(abs_path.to_string())
        );

        // Test mixed case: relative and absolute paths
        assert!(should_show_file_search("hello @/absolute/path", 21));
        assert_eq!(
            extract_search_query("hello @/absolute/path", 21),
            Some("/absolute/path".to_string())
        );
    }
}
