//! Route definitions and utilities
//!
//! This module defines the core route types and functionality
//! for the routing system.

use std::collections::HashMap;

/// Unique identifier for a route
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RouteId(pub String);

impl RouteId {
    /// Create a new route ID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl From<&str> for RouteId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl From<String> for RouteId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

/// Route definition containing metadata and configuration
#[derive(Debug, Clone)]
pub struct Route {
    /// Unique identifier for this route
    pub id: RouteId,
    /// Human-readable name for this route
    pub name: String,
    /// Optional description of what this route displays
    pub description: Option<String>,
    /// Whether this route is the default route
    pub is_default: bool,
    /// Additional metadata for the route
    pub metadata: HashMap<String, String>,
}

impl Route {
    /// Create a new route with the given ID and name
    pub fn new(id: impl Into<RouteId>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            is_default: false,
            metadata: HashMap::new(),
        }
    }

    /// Set the description for this route
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Mark this route as the default route
    pub fn as_default(mut self) -> Self {
        self.is_default = true;
        self
    }

    /// Add metadata to this route
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}
