// Copyright 2026 Aptove
// SPDX-License-Identifier: Apache-2.0

//! Request models for xcbridge API

use serde::Deserialize;

fn default_configuration() -> String {
    "Debug".to_string()
}

/// Request to start a build
#[derive(Debug, Deserialize)]
pub struct BuildRequest {
    /// Path to .xcodeproj file
    pub project: Option<String>,
    /// Path to .xcworkspace file
    pub workspace: Option<String>,
    /// Build scheme
    pub scheme: String,
    /// Build configuration (Debug, Release)
    #[serde(default = "default_configuration")]
    pub configuration: String,
    /// Build destination (e.g., "platform=iOS Simulator,name=iPhone 15 Pro")
    pub destination: Option<String>,
    /// Custom derived data path
    pub derived_data_path: Option<String>,
    /// Additional xcodebuild arguments
    #[serde(default)]
    pub extra_args: Vec<String>,
}

/// Request to start tests
#[derive(Debug, Deserialize)]
pub struct TestRequest {
    /// Path to .xcodeproj file
    pub project: Option<String>,
    /// Path to .xcworkspace file
    pub workspace: Option<String>,
    /// Test scheme
    pub scheme: String,
    /// Test destination
    pub destination: Option<String>,
    /// Test plan to use
    pub test_plan: Option<String>,
    /// Only run these tests
    #[serde(default)]
    pub only_testing: Vec<String>,
    /// Skip these tests
    #[serde(default)]
    pub skip_testing: Vec<String>,
}

/// Request to boot a simulator
#[derive(Debug, Deserialize)]
pub struct SimulatorBootRequest {
    /// Device type name (e.g., "iPhone 15 Pro")
    pub device_type: Option<String>,
    /// Specific simulator UDID
    pub udid: Option<String>,
    /// Runtime (e.g., "iOS 17.0")
    pub runtime: Option<String>,
}

/// Request to shut down a simulator
#[derive(Debug, Deserialize)]
pub struct SimulatorShutdownRequest {
    /// Simulator UDID (or "all" for all simulators)
    pub udid: Option<String>,
    /// Shut down all simulators
    #[serde(default)]
    pub all: bool,
}

/// Request to install an app on a simulator
#[derive(Debug, Deserialize)]
pub struct SimulatorInstallRequest {
    /// Path to .app bundle
    pub app_path: String,
    /// Simulator UDID (uses booted if not specified)
    pub udid: Option<String>,
}

/// Request to launch an app on a simulator
#[derive(Debug, Deserialize)]
pub struct SimulatorLaunchRequest {
    /// App bundle identifier
    pub bundle_id: String,
    /// Simulator UDID (uses booted if not specified)
    pub udid: Option<String>,
    /// Launch arguments
    #[serde(default)]
    pub arguments: Vec<String>,
}

/// Request to uninstall an app from a simulator
#[derive(Debug, Deserialize)]
pub struct SimulatorUninstallRequest {
    /// App bundle identifier
    pub bundle_id: String,
    /// Simulator UDID (uses booted if not specified)
    pub udid: Option<String>,
}

/// Request to install an app on a physical device
#[derive(Debug, Deserialize)]
pub struct DeviceInstallRequest {
    /// Path to .app or .ipa bundle
    pub app_path: String,
    /// Device UDID
    pub device_id: String,
}

/// Request to launch an app on a physical device
#[derive(Debug, Deserialize)]
pub struct DeviceLaunchRequest {
    /// App bundle identifier
    pub bundle_id: String,
    /// Device UDID
    pub device_id: String,
}

/// Request to uninstall an app from a physical device
#[derive(Debug, Deserialize)]
pub struct DeviceUninstallRequest {
    /// App bundle identifier
    pub bundle_id: String,
    /// Device UDID
    pub device_id: String,
}
