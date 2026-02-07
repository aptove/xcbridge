// Copyright 2026 Aptove
// SPDX-License-Identifier: Apache-2.0

//! simctl command wrapper for iOS Simulator management

use crate::error::{Result, XcbridgeError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::process::Command;

/// Simulator device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Simulator {
    pub udid: String,
    pub name: String,
    pub state: String,
    #[serde(rename = "isAvailable")]
    pub is_available: bool,
    #[serde(rename = "deviceTypeIdentifier", default)]
    pub device_type_identifier: Option<String>,
    #[serde(default)]
    pub data_path: Option<String>,
    #[serde(default)]
    pub log_path: Option<String>,
}

/// Runtime information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runtime {
    #[serde(rename = "bundlePath")]
    pub bundle_path: String,
    #[serde(rename = "buildversion")]
    pub build_version: String,
    pub platform: String,
    #[serde(rename = "runtimeRoot")]
    pub runtime_root: String,
    pub identifier: String,
    pub version: String,
    #[serde(rename = "isInternal")]
    pub is_internal: bool,
    #[serde(rename = "isAvailable")]
    pub is_available: bool,
    pub name: String,
}

/// Output from simctl list -j
#[derive(Debug, Deserialize)]
struct SimctlListOutput {
    devices: HashMap<String, Vec<Simulator>>,
    #[serde(default)]
    runtimes: Vec<Runtime>,
}

/// Run simctl command
async fn simctl(args: &[&str]) -> Result<String> {
    let output = Command::new("xcrun")
        .arg("simctl")
        .args(args)
        .output()
        .await
        .map_err(|e| XcbridgeError::CommandFailed(format!("simctl failed: {}", e)))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(XcbridgeError::SimulatorError(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}

/// List all simulators
pub async fn list_devices() -> Result<Vec<Simulator>> {
    let output = simctl(&["list", "devices", "-j"]).await?;
    let list: SimctlListOutput = serde_json::from_str(&output)
        .map_err(|e| XcbridgeError::Internal(format!("Failed to parse simctl output: {}", e)))?;

    let mut simulators = Vec::new();
    for (_runtime, devices) in list.devices {
        simulators.extend(devices.into_iter().filter(|d| d.is_available));
    }

    Ok(simulators)
}

/// List available runtimes
pub async fn list_runtimes() -> Result<Vec<Runtime>> {
    let output = simctl(&["list", "runtimes", "-j"]).await?;
    let list: SimctlListOutput = serde_json::from_str(&output)
        .map_err(|e| XcbridgeError::Internal(format!("Failed to parse simctl output: {}", e)))?;

    Ok(list.runtimes.into_iter().filter(|r| r.is_available).collect())
}

/// Find a simulator by device type and runtime
pub async fn find_simulator(device_type: &str, runtime: Option<&str>) -> Result<Simulator> {
    let simulators = list_devices().await?;
    
    let matches: Vec<_> = simulators
        .into_iter()
        .filter(|s| s.name.to_lowercase().contains(&device_type.to_lowercase()))
        .filter(|s| {
            if let Some(rt) = runtime {
                s.device_type_identifier
                    .as_ref()
                    .map(|id| id.contains(rt))
                    .unwrap_or(false)
            } else {
                true
            }
        })
        .collect();

    matches
        .into_iter()
        .next()
        .ok_or_else(|| XcbridgeError::SimulatorNotFound(device_type.to_string()))
}

/// Get simulator by UDID
pub async fn get_simulator(udid: &str) -> Result<Simulator> {
    let simulators = list_devices().await?;
    simulators
        .into_iter()
        .find(|s| s.udid == udid)
        .ok_or_else(|| XcbridgeError::SimulatorNotFound(udid.to_string()))
}

/// Get the currently booted simulator (if any)
pub async fn get_booted_simulator() -> Result<Option<Simulator>> {
    let simulators = list_devices().await?;
    Ok(simulators.into_iter().find(|s| s.state == "Booted"))
}

/// Boot a simulator
pub async fn boot(udid: &str) -> Result<()> {
    // Check if already booted
    let sim = get_simulator(udid).await?;
    if sim.state == "Booted" {
        tracing::info!("Simulator {} is already booted", udid);
        return Ok(());
    }

    tracing::info!("Booting simulator {}", udid);
    simctl(&["boot", udid]).await?;

    // Wait for boot to complete
    for _ in 0..30 {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let sim = get_simulator(udid).await?;
        if sim.state == "Booted" {
            tracing::info!("Simulator {} is now booted", udid);
            return Ok(());
        }
    }

    Err(XcbridgeError::SimulatorError(
        "Simulator boot timeout".to_string(),
    ))
}

/// Shutdown a simulator
pub async fn shutdown(udid: &str) -> Result<()> {
    tracing::info!("Shutting down simulator {}", udid);
    simctl(&["shutdown", udid]).await?;
    Ok(())
}

/// Shutdown all simulators
pub async fn shutdown_all() -> Result<()> {
    tracing::info!("Shutting down all simulators");
    simctl(&["shutdown", "all"]).await?;
    Ok(())
}

/// Install an app on a simulator
pub async fn install(udid: &str, app_path: &str) -> Result<()> {
    tracing::info!("Installing {} to simulator {}", app_path, udid);
    simctl(&["install", udid, app_path]).await?;
    Ok(())
}

/// Uninstall an app from a simulator
pub async fn uninstall(udid: &str, bundle_id: &str) -> Result<()> {
    tracing::info!("Uninstalling {} from simulator {}", bundle_id, udid);
    simctl(&["uninstall", udid, bundle_id]).await?;
    Ok(())
}

/// Launch an app on a simulator
pub async fn launch(udid: &str, bundle_id: &str, args: &[String]) -> Result<()> {
    tracing::info!("Launching {} on simulator {}", bundle_id, udid);
    let mut cmd_args = vec!["launch", udid, bundle_id];
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    cmd_args.extend(args_refs);
    simctl(&cmd_args).await?;
    Ok(())
}

/// Terminate an app on a simulator
pub async fn terminate(udid: &str, bundle_id: &str) -> Result<()> {
    tracing::info!("Terminating {} on simulator {}", bundle_id, udid);
    // Ignore errors - app might not be running
    let _ = simctl(&["terminate", udid, bundle_id]).await;
    Ok(())
}

/// Get the app container path
pub async fn get_app_container(udid: &str, bundle_id: &str, container: &str) -> Result<String> {
    let output = simctl(&["get_app_container", udid, bundle_id, container]).await?;
    Ok(output.trim().to_string())
}

/// Open a URL in the simulator
pub async fn open_url(udid: &str, url: &str) -> Result<()> {
    simctl(&["openurl", udid, url]).await?;
    Ok(())
}

/// Take a screenshot
pub async fn screenshot(udid: &str, output_path: &str) -> Result<()> {
    simctl(&["io", udid, "screenshot", output_path]).await?;
    Ok(())
}

/// Record video
pub async fn record_video(udid: &str, output_path: &str) -> Result<tokio::process::Child> {
    let child = Command::new("xcrun")
        .args(["simctl", "io", udid, "recordVideo", output_path])
        .spawn()
        .map_err(|e| XcbridgeError::CommandFailed(format!("Failed to start recording: {}", e)))?;
    Ok(child)
}
