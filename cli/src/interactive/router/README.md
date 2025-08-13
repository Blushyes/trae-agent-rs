# Router Module

This module provides a flexible routing system for interactive applications. It defines core types and functionality for managing navigation state and route configuration, designed for use with modern UI frameworks.

## Overview

The router system consists of three main components:

1. **Route Definition** (`route.rs`) - Defines individual routes with metadata
2. **Router State Management** (`router.rs`) - Manages navigation state and history
3. **Router Configuration** (`router.rs`) - Configures available routes and settings

## Core Types

### `Route`

Represents a single route with metadata:

```rust
use trae_agent_rs_core::interactive::router::Route;

let route = Route::new("home", "Home Page")
    .with_description("The main landing page")
    .as_default()
    .with_metadata("icon", "🏠");
```

### `RouterState`

Manages the current navigation state:

```rust
use trae_agent_rs_core::interactive::router::{RouterState, RouteId};

let mut state = RouterState::new(RouteId::from("home"));

// Navigate to a new route
state.navigate_to(RouteId::from("about"));

// Go back to previous route
if state.can_go_back() {
    state.go_back();
}
```

### `RouterConfig`

Configures the router with available routes:

```rust
use trae_agent_rs_core::interactive::router::{RouterConfig, Route};

let config = RouterConfig::new()
    .add_route(Route::new("home", "Home").as_default())
    .add_route(Route::new("about", "About"))
    .with_max_history(100);
```

### `Router`

The main router that combines configuration and state:

```rust
use trae_agent_rs_core::interactive::router::{Router, RouterConfig, Route};

let config = RouterConfig::new()
    .add_route(Route::new("home", "Home").as_default());

let mut router = Router::new(config)?;

// Navigate to different routes
router.navigate_to("about".into())?;

// Get current route information
if let Some(current_route) = router.current_route() {
    println!("Current route: {}", current_route.name);
}
```

## Features

- **Route Management**: Define routes with IDs, names, descriptions, and metadata
- **Navigation History**: Automatic history tracking with configurable limits
- **Default Routes**: Support for default route selection
- **Route Validation**: Ensures navigation only to existing routes
- **Flexible Configuration**: Builder pattern for easy router setup

## Usage in UI Components

This router module is designed for modern UI framework applications. It provides UI-specific components for easy integration:

```rust
use crate::interactive::router::{UIRouter, UIRouterBuilder, Route};

let router_props = UIRouterBuilder::new()
    .add_route(
        Route::new("main", "Main").as_default(),
        |_hooks| element! { MainPage() }.into()
    )
    .build()?;
```

## Error Handling

The router system provides clear error messages for common issues:

- Route not found during navigation
- No routes configured
- Invalid initial route configuration

All operations that can fail return `Result<T, String>` with descriptive error messages.

## Thread Safety

All router types are designed to be thread-safe when needed:

- `Route` and `RouterConfig` implement `Clone` for easy sharing
- `Router` can be wrapped in `Arc<Mutex<>>` for shared mutable access
- Navigation operations are atomic and consistent

## Extension Points

The router system is designed to be extensible:

- Add custom metadata to routes for application-specific needs
- Implement custom navigation logic by extending `RouterState`
- Create specialized router configurations for different use cases

## UI Framework Integration

This module provides seamless integration with modern UI frameworks:

### UI Components

The `integration` module provides:

- `UIRouter`: UI component for rendering pages based on routes
- `UIRouterProps`: Component properties containing router state and page renderers
- `UIRouterBuilder`: Builder pattern for creating router configurations
- `PageRenderer`: Type alias for page render functions

### Example Usage

```rust
use crate::interactive::router::{
    UIRouter, UIRouterBuilder, Route
};

// Create router with pages
let router_props = UIRouterBuilder::new()
    .add_route(
        Route::new("home", "Home").as_default(),
        |_hooks| element! { Text(content: "Home Page") }.into()
    )
    .add_route(
        Route::new("settings", "Settings"),
        |_hooks| element! { Text(content: "Settings Page") }.into()
    )
    .build()?;

// Use in UI component
element! {
    UIRouter(
        router: router_props.router,
        pages: router_props.pages,
        fallback_page: router_props.fallback_page
    )
}
```

## Reusable Design

This router implementation is designed as a reusable routing solution. The modular design allows it to be extracted and used in other applications.

## Standalone Core Components

The core routing functionality (`Route`, `Router`, `RouterState`, `RouterConfig`) can be extracted and used independently with other UI frameworks or even in non-UI contexts.
