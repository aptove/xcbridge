// Copyright 2026 Aptove
// SPDX-License-Identifier: Apache-2.0

//! Status handler

use crate::error::Result;
use crate::models::{DeviceInfo, SimulatorInfo, StatusResponse};
use crate::state::SharedState;
use crate::xcode::{devicectl, simctl};
use axum::{extract::State, Json};

/// GET /status - Health check and status information
pub async fn status(State(state): State<SharedState>) -> Result<Json<StatusResponse>> {
    let simulators = simctl::list_devices()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(SimulatorInfo::from)
        .collect();

    let devices = devicectl::list_devices()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(DeviceInfo::from)
        .collect();

    Ok(Json(StatusResponse {
        healthy: true,
        xcode_version: state.xcode_version.clone(),
        simulators,
        connected_devices: devices,
    }))
}
