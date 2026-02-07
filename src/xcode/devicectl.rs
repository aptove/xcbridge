// Copyright 2026 Aptove
// SPDX-License-Identifier: Apache-2.0

//! devicectl command wrapper for physical iOS device management

use crate::error::{Result, XcbridgeError};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

/// Physical device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub udid: String,
    pub name: String,
    #[serde(rename = "osVersion", default)]
    pub os_version: String,
    #[serde(rename = "connectionType", default)]
    pub connection_type: String,
    #[serde(default)]
    pub platform: String,
    #[serde(rename = "modelName", default)]
    pub model_name: String,
}

/// devicectl list output structure
#[derive(Debug, Deserialize)]
struct DeviceCtlOutput {
    result: DeviceCtlResult,
}

#[derive(Debug, Deserialize)]
struct DeviceCtlResult {
    devices: Vec<DeviceCtlDevice>,
}

#[derive(Debug, Deserialize)]
struct DeviceCtlDevice {
    #[serde(rename = "hardwareProperties")]
    hardware_properties: Option<HardwareProperties>,
    #[serde(rename = "deviceProperties")]
    device_properties: Option<DeviceProperties>,
    #[serde(rename = "connectionProperties")]
    connection_properties: Option<ConnectionProperties>,
    identifier: String,
}

#[derive(Debug, Deserialize)]
struct HardwareProperties {
    #[serde(rename = "udid")]
    udid: Option<String>,
    platform: Option<String>,
    #[serde(rename = "deviceType")]
    device_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeviceProperties {
    name: Option<String>,
    #[serde(rename = "osVersionNumber")]
    os_version_number: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConnectionProperties {
    #[serde(rename = "transportType")]
    transport_type: Option<String>,
}

/// Run devicectl command
async fn devicectl(args: &[&str]) -> Result<String> {
    let output = Command::new("xcrun")
        .arg("devicectl")
        .args(args)
        .output()
        .await
        .map_err(|e| XcbridgeError::CommandFailed(format!("devicectl failed: {}", e)))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // devicectl might not be available on older Xcode versions
        if stderr.contains("unable to locate") {
            return Err(XcbridgeError::CommandFailed(
                "devicectl not available. Requires Xcode 15+".to_string(),
            ));
        }
        Err(XcbridgeError::DeviceError(stderr.to_string()))
    }
}

/// List all connected physical devices
pub async fn list_devices() -> Result<Vec<Device>> {
    let output = devicectl(&["list", "devices", "--json-output", "-"]).await?;
    
    let parsed: DeviceCtlOutput = serde_json::from_str(&output)
        .map_err(|e| XcbridgeError::Internal(format!("Failed to parse devicectl output: {}", e)))?;

    let devices = parsed
        .result
        .devices
        .into_iter()
        .map(|d| {
            let hw = d.hardware_properties.unwrap_or(HardwareProperties {
                udid: None,
                platform: None,
                device_type: None,
            });
            let dp = d.device_properties.unwrap_or(DeviceProperties {
                name: None,
                os_version_number: None,
            });
            let cp = d.connection_properties.unwrap_or(ConnectionProperties {
                transport_type: None,
            });

            Device {
                udid: hw.udid.unwrap_or(d.identifier),
                name: dp.name.unwrap_or_else(|| "Unknown".to_string()),
                os_version: dp.os_version_number.unwrap_or_else(|| "Unknown".to_string()),
                connection_type: cp.transport_type.unwrap_or_else(|| "Unknown".to_string()),
                platform: hw.platform.unwrap_or_else(|| "iOS".to_string()),
                model_name: hw.device_type.unwrap_or_else(|| "Unknown".to_string()),
            }
        })
        .collect();

    Ok(devices)
}

/// Get a specific device by UDID
pub async fn get_device(udid: &str) -> Result<Device> {
    let devices = list_devices().await?;
    devices
        .into_iter()
        .find(|d| d.udid == udid)
        .ok_or_else(|| XcbridgeError::DeviceNotFound(udid.to_string()))
}

/// Install an app on a physical device
pub async fn install(device_id: &str, app_path: &str) -> Result<()> {
    tracing::info!("Installing {} to device {}", app_path, device_id);
    devicectl(&["device", "install", "app", "--device", device_id, app_path]).await?;
    Ok(())
}

/// Launch an app on a physical device
pub async fn launch(device_id: &str, bundle_id: &str) -> Result<()> {
    tracing::info!("Launching {} on device {}", bundle_id, device_id);
    devicectl(&["device", "process", "launch", "--device", device_id, bundle_id]).await?;
    Ok(())
}

/// Uninstall an app from a physical device
pub async fn uninstall(device_id: &str, bundle_id: &str) -> Result<()> {
    tracing::info!("Uninstalling {} from device {}", bundle_id, device_id);
    devicectl(&["device", "uninstall", "app", "--device", device_id, bundle_id]).await?;
    Ok(())
}

/// Copy files from device
pub async fn copy_from_device(device_id: &str, source: &str, destination: &str) -> Result<()> {
    devicectl(&[
        "device",
        "copy",
        "from",
        "--device",
        device_id,
        source,
        destination,
    ])
    .await?;
    Ok(())
}

/// Copy files to device
pub async fn copy_to_device(device_id: &str, source: &str, destination: &str) -> Result<()> {
    devicectl(&[
        "device",
        "copy",
        "to",
        "--device",
        device_id,
        source,
        destination,
    ])
    .await?;
    Ok(())
}
