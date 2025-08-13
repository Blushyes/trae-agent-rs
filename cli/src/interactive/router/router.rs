//! Router implementation for managing navigation state
//!
//! This module provides the core router functionality including
//! state management, navigation, and route resolution.

use super::route::{Route, RouteId};
use std::collections::HashMap;

/// Current state of the router
#[derive(Debug, Clone)]
pub struct RouterState {
    /// Currently active route ID
    pub current_route: RouteId,
    /// Navigation history (most recent first)
    pub history: Vec<RouteId>,
    /// Maximum number of history entries to keep
    pub max_history: usize,
}

impl RouterState {
    /// Create a new router state with the given initial route
    pub fn new(initial_route: RouteId) -> Self {
        Self {
            current_route: initial_route,
            history: Vec::new(),
            max_history: 50, // Default history limit
        }
    }

    /// Navigate to a new route
    pub fn navigate_to(&mut self, route_id: RouteId) {
        // Add current route to history if it's different
        if self.current_route != route_id {
            self.history.insert(0, self.current_route.clone());
            
            // Trim history if it exceeds max size
            if self.history.len() > self.max_history {
                self.history.truncate(self.max_history);
            }
        }
        
        self.current_route = route_id;
    }

    /// Go back to the previous route in history
    pub fn go_back(&mut self) -> bool {
        if let Some(previous_route) = self.history.first().cloned() {
            self.history.remove(0);
            self.current_route = previous_route;
            true
        } else {
            false
        }
    }

    /// Check if we can go back
    pub fn can_go_back(&self) -> bool {
        !self.history.is_empty()
    }

    /// Get the current route ID
    pub fn current_route(&self) -> &RouteId {
        &self.current_route
    }

    /// Get the navigation history
    pub fn history(&self) -> &[RouteId] {
        &self.history
    }
}

/// Configuration for the router
#[derive(Debug, Clone)]
pub struct RouterConfig {
    /// All available routes
    pub routes: HashMap<RouteId, Route>,
    /// Default route to use when no route is specified
    pub default_route: Option<RouteId>,
    /// Whether to enable navigation history
    pub enable_history: bool,
    /// Maximum number of history entries
    pub max_history: usize,
}

impl RouterConfig {
    /// Create a new router configuration
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
            default_route: None,
            enable_history: true,
            max_history: 50,
        }
    }

    /// Add a route to the configuration
    pub fn add_route(mut self, route: Route) -> Self {
        let route_id = route.id.clone();
        
        // Set as default if this is marked as default and no default exists
        if route.is_default && self.default_route.is_none() {
            self.default_route = Some(route_id.clone());
        }
        
        self.routes.insert(route_id, route);
        self
    }

    /// Set the default route
    pub fn with_default_route(mut self, route_id: RouteId) -> Self {
        self.default_route = Some(route_id);
        self
    }

    /// Disable navigation history
    pub fn without_history(mut self) -> Self {
        self.enable_history = false;
        self
    }

    /// Set maximum history size
    pub fn with_max_history(mut self, max_history: usize) -> Self {
        self.max_history = max_history;
        self
    }

    /// Get a route by ID
    pub fn get_route(&self, route_id: &RouteId) -> Option<&Route> {
        self.routes.get(route_id)
    }

    /// Get the default route ID
    pub fn default_route(&self) -> Option<&RouteId> {
        self.default_route.as_ref()
    }

    /// Get all routes
    pub fn routes(&self) -> &HashMap<RouteId, Route> {
        &self.routes
    }
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Main router struct that manages navigation state and configuration
#[derive(Debug, Clone)]
pub struct Router {
    /// Router configuration
    config: RouterConfig,
    /// Current router state
    state: RouterState,
}

impl Router {
    /// Create a new router with the given configuration
    pub fn new(config: RouterConfig) -> Result<Self, String> {
        // Determine initial route
        let initial_route = if let Some(default_route) = config.default_route() {
            default_route.clone()
        } else if let Some((route_id, _)) = config.routes().iter().next() {
            route_id.clone()
        } else {
            return Err("No routes configured".to_string());
        };

        // Validate that the initial route exists
        if !config.routes().contains_key(&initial_route) {
            return Err(format!("Initial route '{}' not found in configuration", initial_route.0));
        }

        let mut state = RouterState::new(initial_route);
        if config.enable_history {
            state.max_history = config.max_history;
        } else {
            state.max_history = 0;
        }

        Ok(Self { config, state })
    }

    /// Get the current router state
    pub fn state(&self) -> &RouterState {
        &self.state
    }

    /// Get the router configuration
    pub fn config(&self) -> &RouterConfig {
        &self.config
    }

    /// Navigate to a route by ID
    pub fn navigate_to(&mut self, route_id: RouteId) -> Result<(), String> {
        // Validate that the route exists
        if !self.config.routes().contains_key(&route_id) {
            return Err(format!("Route '{}' not found", route_id.0));
        }

        self.state.navigate_to(route_id);
        Ok(())
    }

    /// Go back to the previous route
    pub fn go_back(&mut self) -> bool {
        if self.config.enable_history {
            self.state.go_back()
        } else {
            false
        }
    }

    /// Check if we can go back
    pub fn can_go_back(&self) -> bool {
        self.config.enable_history && self.state.can_go_back()
    }

    /// Get the current route
    pub fn current_route(&self) -> Option<&Route> {
        self.config.get_route(self.state.current_route())
    }

    /// Get the current route ID
    pub fn current_route_id(&self) -> &RouteId {
        self.state.current_route()
    }
}
