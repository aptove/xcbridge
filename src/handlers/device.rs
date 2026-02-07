// Copyright 2026 Aptove
// SPDX-License-Identifier: Apache-2.0

//! Device handlers for physical iOS devices

use crate::error::Result;
use crate::models::{
    DeviceInfo, DeviceInstallRequest, DeviceLaunchRequest, DeviceListResponse,
    DeviceUninstallRequest, SuccessResponse,
};
use crate::state::SharedState;
use crate::xcode::devicectl;
use axum::{extract::State, Json};

/// GET /device/list - List all connected physical devices
pub async fn list(State(_state): State<SharedState>) -> Result<Json<DeviceListResponse>> {
    let devices = devicectl::list_devices()
        .await?
        .into_iter()
        .map(DeviceInfo::from)
        .collect();

    Ok(Json(DeviceListResponse { devices }))
}

/// POST /device/install - Install an app on a physical device
pub async fn install(
    State(_state): State<SharedState>,
    Json(req): Json<DeviceInstallRequest>,
) -> Result<Json<SuccessResponse>> {
    devicectl::install(&req.device_id, &req.app_path).await?;

    Ok(Json(SuccessResponse::new(format!(
        "App installed to device {}",
        req.device_id
    ))))
}

/// POST /device/launch - Launch an app on a physical device
pub async fn launch(
    State(_state): State<SharedState>,
    Json(req): Json<DeviceLaunchRequest>,
) -> Result<Json<SuccessResponse>> {
    devicectl::launch(&req.device_id, &req.bundle_id).await?;

    Ok(Json(SuccessResponse::new(format!(
        "App {} launched on device {}",
        req.bundle_id, req.device_id
    ))))
}

/// POST /device/uninstall - Uninstall an app from a physical device
pub async fn uninstall(
    State(_state): State<SharedState>,
    Json(req): Json<DeviceUninstallRequest>,
) -> Result<Json<SuccessResponse>> {
    devicectl::uninstall(&req.device_id, &req.bundle_id).await?;

    Ok(Json(SuccessResponse::new(format!(
        "App {} uninstalled from device {}",
        req.bundle_id, req.device_id
    ))))
}
