// Copyright 2026 Aptove
// SPDX-License-Identifier: Apache-2.0

//! xcbridge - Xcode Bridge Service for containerized iOS development
//!
//! This service runs on macOS and provides a REST API for Xcode operations,
//! allowing AI agents running in Linux containers to access iOS build tooling.

use axum::{
    http::{header, Method, StatusCode},
    middleware,
    routing::{delete, get, post},
    Router,
};
use clap::Parser;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, Level};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod config;
mod error;
mod handlers;
mod models;
mod state;
mod xcode;

use config::Config;
use state::AppState;

/// API key authentication middleware
async fn auth_middleware(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    request: axum::extract::Request,
    next: middleware::Next,
) -> Result<axum::response::Response, StatusCode> {
    // If no API key is configured, skip authentication
    let Some(expected_key) = &state.config.api_key else {
        return Ok(next.run(request).await);
    };

    // Check for API key in header
    let auth_header = request
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(key) if key == expected_key => Ok(next.run(request).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

fn create_router(state: Arc<AppState>) -> Router {
    // Build routes
    let build_routes = Router::new()
        .route("/", post(handlers::build::start_build))
        .route("/{id}", get(handlers::build::get_build))
        .route("/{id}/logs", get(handlers::build::build_logs))
        .route("/{id}", delete(handlers::build::cancel_build));

    // Test routes
    let test_routes = Router::new()
        .route("/", post(handlers::test::start_test))
        .route("/{id}", get(handlers::test::get_test))
        .route("/{id}/logs", get(handlers::test::test_logs));

    // Simulator routes
    let simulator_routes = Router::new()
        .route("/list", get(handlers::simulator::list))
        .route("/boot", post(handlers::simulator::boot))
        .route("/shutdown", post(handlers::simulator::shutdown))
        .route("/install", post(handlers::simulator::install))
        .route("/launch", post(handlers::simulator::launch))
        .route("/uninstall", post(handlers::simulator::uninstall));

    // Device routes
    let device_routes = Router::new()
        .route("/list", get(handlers::device::list))
        .route("/install", post(handlers::device::install))
        .route("/launch", post(handlers::device::launch))
        .route("/uninstall", post(handlers::device::uninstall));

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE, header::HeaderName::from_static("x-api-key")]);

    // Combine all routes
    Router::new()
        .route("/status", get(handlers::status::status))
        .nest("/build", build_routes)
        .nest("/test", test_routes)
        .nest("/simulator", simulator_routes)
        .nest("/device", device_routes)
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse configuration
    let config = Config::parse();

    // Initialize logging
    let log_level = match config.log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(log_level.to_string())),
        )
        .init();

    // Verify Xcode is available and get version
    let xcode_version = match xcode::xcodebuild::get_xcode_version().await {
        Ok(version) => {
            info!("Xcode version: {}", version);
            version
        }
        Err(e) => {
            tracing::error!("Xcode not found or not working: {}", e);
            tracing::error!("xcbridge requires Xcode to be installed and configured");
            std::process::exit(1);
        }
    };

    // Create application state
    let state = Arc::new(AppState::new(config.clone(), xcode_version));

    // Create router
    let app = create_router(state);

    // Bind to address
    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await?;

    info!("xcbridge listening on {}", addr);
    info!("API documentation available at http://{}/", addr);

    if config.api_key.is_some() {
        info!("API key authentication enabled");
    } else {
        tracing::warn!("No API key configured - authentication disabled");
    }

    // Start server
    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    fn test_config() -> Config {
        Config {
            port: 9090,
            host: "127.0.0.1".to_string(),
            api_key: None,
            log_level: "info".to_string(),
            allowed_paths: vec![],
        }
    }

    #[tokio::test]
    async fn test_status_endpoint() {
        let state = Arc::new(AppState::new(test_config(), "15.0".to_string()));
        let app = create_router(state);

        let response = app
            .oneshot(Request::builder().uri("/status").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_required_when_api_key_set() {
        let mut config = test_config();
        config.api_key = Some("secret-key".to_string());
        let state = Arc::new(AppState::new(config, "15.0".to_string()));
        let app = create_router(state);

        // Request without API key should fail
        let response = app
            .clone()
            .oneshot(Request::builder().uri("/status").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // Request with correct API key should succeed
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/status")
                    .header("X-API-Key", "secret-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
