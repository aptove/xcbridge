// Copyright 2026 Aptove
// SPDX-License-Identifier: Apache-2.0

//! Error types for xcbridge

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum XcbridgeError {
    #[error("Xcode not found. Please install Xcode and run xcode-select.")]
    XcodeNotFound,

    #[error("Build failed: {0}")]
    BuildFailed(String),

    #[error("Test failed: {0}")]
    TestFailed(String),

    #[error("Simulator not found: {0}")]
    SimulatorNotFound(String),

    #[error("Simulator error: {0}")]
    SimulatorError(String),

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Device error: {0}")]
    DeviceError(String),

    #[error("Path not allowed: {0}")]
    PathNotAllowed(String),

    #[error("Command execution failed: {0}")]
    CommandFailed(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Build not found: {0}")]
    BuildNotFound(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Unauthorized")]
    Unauthorized,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

impl IntoResponse for XcbridgeError {
    fn into_response(self) -> Response {
        let (status, error_type) = match &self {
            XcbridgeError::XcodeNotFound => (StatusCode::SERVICE_UNAVAILABLE, "xcode_not_found"),
            XcbridgeError::BuildFailed(_) => (StatusCode::BAD_REQUEST, "build_failed"),
            XcbridgeError::TestFailed(_) => (StatusCode::BAD_REQUEST, "test_failed"),
            XcbridgeError::SimulatorNotFound(_) => (StatusCode::NOT_FOUND, "simulator_not_found"),
            XcbridgeError::SimulatorError(_) => (StatusCode::BAD_REQUEST, "simulator_error"),
            XcbridgeError::DeviceNotFound(_) => (StatusCode::NOT_FOUND, "device_not_found"),
            XcbridgeError::DeviceError(_) => (StatusCode::BAD_REQUEST, "device_error"),
            XcbridgeError::PathNotAllowed(_) => (StatusCode::FORBIDDEN, "path_not_allowed"),
            XcbridgeError::CommandFailed(_) => (StatusCode::INTERNAL_SERVER_ERROR, "command_failed"),
            XcbridgeError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, "invalid_request"),
            XcbridgeError::BuildNotFound(_) => (StatusCode::NOT_FOUND, "build_not_found"),
            XcbridgeError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
            XcbridgeError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
        };

        let body = Json(ErrorResponse {
            error: error_type.to_string(),
            message: self.to_string(),
        });

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, XcbridgeError>;
