// Copyright 2026 Aptove
// SPDX-License-Identifier: Apache-2.0

//! Simulator handlers

use crate::error::{Result, XcbridgeError};
use crate::models::{
    SimulatorBootRequest, SimulatorBootResponse, SimulatorInstallRequest, SimulatorLaunchRequest,
    SimulatorListResponse, SimulatorShutdownRequest, SimulatorUninstallRequest, SimulatorInfo,
    SuccessResponse,
};
use crate::state::SharedState;
use crate::xcode::simctl;
use axum::{extract::State, Json};

/// GET /simulator/list - List all available simulators
pub async fn list(State(_state): State<SharedState>) -> Result<Json<SimulatorListResponse>> {
    let simulators = simctl::list_devices()
        .await?
        .into_iter()
        .map(SimulatorInfo::from)
        .collect();

    Ok(Json(SimulatorListResponse { simulators }))
}

/// POST /simulator/boot - Boot a simulator
pub async fn boot(
    State(_state): State<SharedState>,
    Json(req): Json<SimulatorBootRequest>,
) -> Result<Json<SimulatorBootResponse>> {
    // Find the simulator
    let simulator = if let Some(udid) = req.udid {
        simctl::get_simulator(&udid).await?
    } else if let Some(device_type) = req.device_type {
        simctl::find_simulator(&device_type, req.runtime.as_deref()).await?
    } else {
        return Err(XcbridgeError::InvalidRequest(
            "Either udid or device_type must be specified".into(),
        ));
    };

    // Boot the simulator
    simctl::boot(&simulator.udid).await?;

    // Get updated status
    let booted = simctl::get_simulator(&simulator.udid).await?;

    Ok(Json(SimulatorBootResponse {
        udid: booted.udid,
        name: booted.name,
        status: booted.state,
    }))
}

/// POST /simulator/shutdown - Shutdown a simulator
pub async fn shutdown(
    State(_state): State<SharedState>,
    Json(req): Json<SimulatorShutdownRequest>,
) -> Result<Json<SuccessResponse>> {
    if req.all {
        simctl::shutdown_all().await?;
        Ok(Json(SuccessResponse::new("All simulators shut down")))
    } else if let Some(udid) = req.udid {
        simctl::shutdown(&udid).await?;
        Ok(Json(SuccessResponse::new(format!(
            "Simulator {} shut down",
            udid
        ))))
    } else {
        Err(XcbridgeError::InvalidRequest(
            "Either udid or all=true must be specified".into(),
        ))
    }
}

/// POST /simulator/install - Install an app on a simulator
pub async fn install(
    State(_state): State<SharedState>,
    Json(req): Json<SimulatorInstallRequest>,
) -> Result<Json<SuccessResponse>> {
    // Get the target simulator
    let udid = if let Some(udid) = req.udid {
        udid
    } else {
        // Use the currently booted simulator
        simctl::get_booted_simulator()
            .await?
            .ok_or_else(|| {
                XcbridgeError::SimulatorError("No simulator is currently booted".into())
            })?
            .udid
    };

    // Install the app
    simctl::install(&udid, &req.app_path).await?;

    Ok(Json(SuccessResponse::new(format!(
        "App installed to simulator {}",
        udid
    ))))
}

/// POST /simulator/launch - Launch an app on a simulator
pub async fn launch(
    State(_state): State<SharedState>,
    Json(req): Json<SimulatorLaunchRequest>,
) -> Result<Json<SuccessResponse>> {
    // Get the target simulator
    let udid = if let Some(udid) = req.udid {
        udid
    } else {
        // Use the currently booted simulator
        simctl::get_booted_simulator()
            .await?
            .ok_or_else(|| {
                XcbridgeError::SimulatorError("No simulator is currently booted".into())
            })?
            .udid
    };

    // Launch the app
    simctl::launch(&udid, &req.bundle_id, &req.arguments).await?;

    Ok(Json(SuccessResponse::new(format!(
        "App {} launched on simulator {}",
        req.bundle_id, udid
    ))))
}

/// POST /simulator/uninstall - Uninstall an app from a simulator
pub async fn uninstall(
    State(_state): State<SharedState>,
    Json(req): Json<SimulatorUninstallRequest>,
) -> Result<Json<SuccessResponse>> {
    // Get the target simulator
    let udid = if let Some(udid) = req.udid {
        udid
    } else {
        // Use the currently booted simulator
        simctl::get_booted_simulator()
            .await?
            .ok_or_else(|| {
                XcbridgeError::SimulatorError("No simulator is currently booted".into())
            })?
            .udid
    };

    // Uninstall the app
    simctl::uninstall(&udid, &req.bundle_id).await?;

    Ok(Json(SuccessResponse::new(format!(
        "App {} uninstalled from simulator {}",
        req.bundle_id, udid
    ))))
}
