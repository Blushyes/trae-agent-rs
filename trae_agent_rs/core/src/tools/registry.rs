//! Tool registry for managing available tools

use crate::tools::{Tool, ToolExecutor};
use std::collections::HashMap;

/// Registry for managing tool creation and registration
pub struct ToolRegistry {
    factories: HashMap<String, Box<dyn ToolFactory>>,
}

/// Factory trait for creating tools
pub trait ToolFactory: Send + Sync {
    /// Create a new instance of the tool
    fn create(&self) -> Box<dyn Tool>;
    
    /// Get the name of the tool this factory creates
    fn tool_name(&self) -> &str;
    
    /// Get the description of the tool this factory creates
    fn tool_description(&self) -> &str;
}

impl ToolRegistry {
    /// Create a new tool registry
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }
    
    /// Register a tool factory
    pub fn register_factory(&mut self, factory: Box<dyn ToolFactory>) {
        self.factories.insert(factory.tool_name().to_string(), factory);
    }
    
    /// Create a tool by name
    pub fn create_tool(&self, name: &str) -> Option<Box<dyn Tool>> {
        self.factories.get(name).map(|factory| factory.create())
    }
    
    /// List all available tool names
    pub fn list_tools(&self) -> Vec<&str> {
        self.factories.keys().map(|s| s.as_str()).collect()
    }
    
    /// Get tool information
    pub fn get_tool_info(&self, name: &str) -> Option<(&str, &str)> {
        self.factories.get(name).map(|factory| {
            (factory.tool_name(), factory.tool_description())
        })
    }
    
    /// Create a tool executor with the specified tools
    pub fn create_executor(&self, tool_names: &[String]) -> ToolExecutor {
        let mut executor = ToolExecutor::new();
        
        for name in tool_names {
            if let Some(tool) = self.create_tool(name) {
                executor.register_tool(tool);
            }
        }
        
        executor
    }
    
    /// Create a tool executor with all available tools
    pub fn create_executor_with_all(&self) -> ToolExecutor {
        let mut executor = ToolExecutor::new();
        
        for factory in self.factories.values() {
            executor.register_tool(factory.create());
        }
        
        executor
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        let mut registry = Self::new();

        // Register built-in tools
        registry.register_factory(Box::new(crate::tools::builtin::BashToolFactory));
        registry.register_factory(Box::new(crate::tools::builtin::EditToolFactory));
        registry.register_factory(Box::new(crate::tools::builtin::ThinkingToolFactory));
        registry.register_factory(Box::new(crate::tools::builtin::TaskDoneToolFactory));
        registry.register_factory(Box::new(crate::tools::builtin::JsonEditToolFactory));
        registry.register_factory(Box::new(crate::tools::builtin::CkgToolFactory));
        registry.register_factory(Box::new(crate::tools::builtin::McpToolFactory));

        registry
    }
}

/// Macro to help implement tool factories
#[macro_export]
macro_rules! impl_tool_factory {
    ($factory:ident, $tool:ident, $name:expr, $description:expr) => {
        pub struct $factory;
        
        impl $crate::tools::ToolFactory for $factory {
            fn create(&self) -> Box<dyn $crate::tools::Tool> {
                Box::new($tool::new())
            }
            
            fn tool_name(&self) -> &str {
                $name
            }
            
            fn tool_description(&self) -> &str {
                $description
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::tools::registry::ToolRegistry;

    #[test]
    fn test_default_registry_has_all_tools() {
        let registry = ToolRegistry::default();
        let tools = registry.list_tools();
        
        // Expected tools based on Python version
        let expected_tools = vec![
            "bash",
            "str_replace_based_edit_tool", 
            "sequentialthinking",
            "task_done",
            "json_edit_tool",
            "ckg_tool",
            "mcp_tool",
        ];
        
        println!("Available tools: {:?}", tools);
        
        // Check that all expected tools are registered
        for expected_tool in &expected_tools {
            assert!(
                tools.contains(expected_tool),
                "Tool '{}' is not registered in the default registry",
                expected_tool
            );
        }
        
        // Check that we have the expected number of tools
        assert_eq!(
            tools.len(),
            expected_tools.len(),
            "Expected {} tools, but found {}. Tools: {:?}",
            expected_tools.len(),
            tools.len(),
            tools
        );
    }

    #[test]
    fn test_tool_creation() {
        let registry = ToolRegistry::default();
        
        // Test creating each tool
        let tools_to_test = vec![
            "bash",
            "str_replace_based_edit_tool",
            "sequentialthinking", 
            "task_done",
            "json_edit_tool",
            "ckg_tool",
            "mcp_tool",
        ];
        
        for tool_name in tools_to_test {
            let tool = registry.create_tool(tool_name);
            assert!(
                tool.is_some(),
                "Failed to create tool '{}'",
                tool_name
            );
            
            let tool = tool.unwrap();
            assert_eq!(
                tool.name(),
                tool_name,
                "Tool name mismatch for '{}'",
                tool_name
            );
            
            // Verify tool has a description
            assert!(
                !tool.description().is_empty(),
                "Tool '{}' has empty description",
                tool_name
            );
            
            // Verify tool has parameters schema
            let schema = tool.parameters_schema();
            assert!(
                schema.is_object(),
                "Tool '{}' parameters schema is not an object",
                tool_name
            );
        }
    }

    #[test]
    fn test_tool_info() {
        let registry = ToolRegistry::default();
        
        for tool_name in registry.list_tools() {
            let info = registry.get_tool_info(tool_name);
            assert!(
                info.is_some(),
                "Failed to get info for tool '{}'",
                tool_name
            );
            
            let (name, description) = info.unwrap();
            assert_eq!(name, tool_name);
            assert!(!description.is_empty());
        }
    }

    #[test]
    fn test_executor_creation() {
        let registry = ToolRegistry::default();
        
        // Test creating executor with specific tools
        let tool_names = vec!["bash".to_string(), "str_replace_based_edit_tool".to_string()];
        let _executor = registry.create_executor(&tool_names);
        
        // The executor should have the requested tools
        // Note: We can't easily test this without exposing internal state
        // This test mainly ensures the method doesn't panic
        
        // Test creating executor with all tools
        let _all_executor = registry.create_executor_with_all();
    }

    #[test]
    fn test_tool_examples() {
        let registry = ToolRegistry::default();
        
        for tool_name in registry.list_tools() {
            let tool = registry.create_tool(tool_name).unwrap();
            let examples = tool.examples();
            
            // Each tool should have at least one example
            assert!(
                !examples.is_empty(),
                "Tool '{}' has no examples",
                tool_name
            );
            
            // Verify example structure
            for (i, example) in examples.iter().enumerate() {
                assert!(
                    !example.description.is_empty(),
                    "Tool '{}' example {} has empty description",
                    tool_name,
                    i
                );
                
                assert!(
                    example.parameters.is_object(),
                    "Tool '{}' example {} parameters is not an object",
                    tool_name,
                    i
                );
                
                assert!(
                    !example.expected_result.is_empty(),
                    "Tool '{}' example {} has empty expected result",
                    tool_name,
                    i
                );
            }
        }
    }

    #[test]
    fn test_tool_parameter_schemas() {
        let registry = ToolRegistry::default();
        
        for tool_name in registry.list_tools() {
            let tool = registry.create_tool(tool_name).unwrap();
            let schema = tool.parameters_schema();
            
            // Schema should be an object
            assert!(
                schema.is_object(),
                "Tool '{}' schema is not an object",
                tool_name
            );
            
            let schema_obj = schema.as_object().unwrap();
            
            // Should have type property
            if let Some(type_val) = schema_obj.get("type") {
                assert_eq!(
                    type_val.as_str(),
                    Some("object"),
                    "Tool '{}' schema type is not 'object'",
                    tool_name
                );
            }
            
            // Should have properties
            if let Some(properties) = schema_obj.get("properties") {
                assert!(
                    properties.is_object(),
                    "Tool '{}' schema properties is not an object",
                    tool_name
                );
                
                let props = properties.as_object().unwrap();
                assert!(
                    !props.is_empty(),
                    "Tool '{}' has no properties in schema",
                    tool_name
                );
            }
        }
    }
}
