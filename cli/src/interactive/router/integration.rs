//! UI framework integration for the router system
//!
//! This module provides UI framework-specific components and utilities for the router system.
//! It bridges the core router functionality with the UI component system.

use super::{Route, RouteId, Router as CoreRouter, RouterConfig};
use iocraft::prelude::*;
use std::collections::HashMap;

/// Type alias for page render functions
/// Each page is a function that takes hooks and returns an element
pub type PageRenderer = Box<dyn Fn(Hooks) -> AnyElement<'static> + Send + Sync>;

/// UI router component properties
#[derive(Props)]
pub struct UIRouterProps {
    /// Router configuration and state
    pub router: CoreRouter,
    /// Map of route IDs to their corresponding page renderers
    pub pages: HashMap<RouteId, PageRenderer>,
    /// Optional fallback page for unknown routes
    pub fallback_page: Option<PageRenderer>,
}

impl Default for UIRouterProps {
    fn default() -> Self {
        // Create a minimal default configuration for testing
        let config = RouterConfig::new();
        let router = CoreRouter::new(config).unwrap_or_else(|_| {
            // Fallback: create a router with a default route
            let config = RouterConfig::new().add_route(Route::new("default", "Default").as_default());
            CoreRouter::new(config).expect("Failed to create default router")
        });

        Self {
            router,
            pages: HashMap::new(),
            fallback_page: None,
        }
    }
}

/// UI router component that renders different pages based on current route
#[component]
pub fn UIRouter(hooks: Hooks, props: &UIRouterProps) -> impl Into<AnyElement<'static>> {
    let current_route_id = props.router.current_route_id();

    // Try to find the page renderer for the current route
    let page_element = if let Some(page_renderer) = props.pages.get(current_route_id) {
        // Render the page for the current route
        page_renderer(hooks)
    } else if let Some(fallback_renderer) = &props.fallback_page {
        // Render fallback page if route not found
        fallback_renderer(hooks)
    } else {
        // Default fallback: show route not found message
        element! {
            View(
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                width: 100pct,
                height: 100pct,
                padding: 2,
            ) {
                Text(
                    content: "Route Not Found",
                    weight: Weight::Bold,
                    color: Color::Red
                )
                Text(
                    content: format!("Unknown route: {}", current_route_id.0)
                )
            }
        }
        .into()
    };

    element! {
        View(
            key: "router-container",
            width: 100pct,
            height: 100pct,
        ) {
            #(page_element)
        }
    }
}

/// Builder for creating UI router configurations with pages
pub struct UIRouterBuilder {
    config: RouterConfig,
    pages: HashMap<RouteId, PageRenderer>,
    fallback_page: Option<PageRenderer>,
}

impl UIRouterBuilder {
    /// Create a new UI router builder
    pub fn new() -> Self {
        Self {
            config: RouterConfig::new(),
            pages: HashMap::new(),
            fallback_page: None,
        }
    }

    /// Add a route with its corresponding page renderer
    pub fn add_route<F>(mut self, route: Route, page_renderer: F) -> Self
    where
        F: Fn(Hooks) -> AnyElement<'static> + Send + Sync + 'static,
    {
        let route_id = route.id.clone();
        self.config = self.config.add_route(route);
        self.pages.insert(route_id, Box::new(page_renderer));
        self
    }

    /// Set a fallback page for unknown routes
    pub fn with_fallback<F>(mut self, fallback_renderer: F) -> Self
    where
        F: Fn(Hooks) -> AnyElement<'static> + Send + Sync + 'static,
    {
        self.fallback_page = Some(Box::new(fallback_renderer));
        self
    }

    /// Set the default route
    pub fn with_default_route(mut self, route_id: RouteId) -> Self {
        self.config = self.config.with_default_route(route_id);
        self
    }

    /// Build the UI router props
    pub fn build(self) -> Result<UIRouterProps, String> {
        let router = CoreRouter::new(self.config)?;

        Ok(UIRouterProps {
            router,
            pages: self.pages,
            fallback_page: self.fallback_page,
        })
    }
}

impl Default for UIRouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_router_builder() {
        let router_props = UIRouterBuilder::new()
            .add_route(Route::new("home", "Home").as_default(), |_hooks| {
                element! { Text(content: "Home Page") }.into()
            })
            .add_route(Route::new("about", "About"), |_hooks| {
                element! { Text(content: "About Page") }.into()
            })
            .build()
            .expect("Failed to build router");

        assert_eq!(router_props.router.current_route_id().0, "home");
        assert_eq!(router_props.pages.len(), 2);
    }

    #[test]
    fn test_default_router_props() {
        let props = UIRouterProps::default();
        assert_eq!(props.router.current_route_id().0, "default");
        assert!(props.pages.is_empty());
        assert!(props.fallback_page.is_none());
    }
}
