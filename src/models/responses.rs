// Copyright 2026 Aptove
// SPDX-License-Identifier: Apache-2.0

//! Response models for xcbridge API

use crate::xcode::devicectl::Device;
use crate::xcode::simctl::Simulator;
use serde::Serialize;

/// Health check and status response
#[derive(Debug, Serialize)]
pub struct StatusResponse {
    /// Service is healthy
    pub healthy: bool,
    /// Xcode version
    pub xcode_version: String,
    /// Available simulators
    pub simulators: Vec<SimulatorInfo>,
    /// Connected physical devices
    pub connected_devices: Vec<DeviceInfo>,
}

/// Simplified simulator info for status response
#[derive(Debug, Serialize)]
pub struct SimulatorInfo {
    pub udid: String,
    pub name: String,
    pub state: String,
}

impl From<Simulator> for SimulatorInfo {
    fn from(sim: Simulator) -> Self {
        Self {
            udid: sim.udid,
            name: sim.name,
            state: sim.state,
        }
    }
}

/// Simplified device info for status response
#[derive(Debug, Serialize)]
pub struct DeviceInfo {
    pub udid: String,
    pub name: String,
    pub os_version: String,
    pub connection_type: String,
}

impl From<Device> for DeviceInfo {
    fn from(device: Device) -> Self {
        Self {
            udid: device.udid,
            name: device.name,
            os_version: device.os_version,
            connection_type: device.connection_type,
        }
    }
}

/// Response when a build is started
#[derive(Debug, Serialize)]
pub struct BuildStartedResponse {
    /// Unique build identifier
    pub build_id: String,
    /// Build status
    pub status: String,
    /// URL to stream logs
    pub logs_url: String,
}

/// Response for build status query
#[derive(Debug, Serialize)]
pub struct BuildStatusResponse {
    /// Build identifier
    pub build_id: String,
    /// Current status: "running", "success", "failed", "cancelled"
    pub status: String,
    /// Exit code (if completed)
    pub exit_code: Option<i32>,
    /// Build artifacts (if successful)
    pub artifacts: Option<Vec<String>>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Build logs
    pub logs: Vec<String>,
}

/// Response when a simulator is booted
#[derive(Debug, Serialize)]
pub struct SimulatorBootResponse {
    /// Simulator UDID
    pub udid: String,
    /// Simulator name
    pub name: String,
    /// Current state
    pub status: String,
}

/// Response for simulator list
#[derive(Debug, Serialize)]
pub struct SimulatorListResponse {
    pub simulators: Vec<SimulatorInfo>,
}

/// Response for simple success operations
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

impl SuccessResponse {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
        }
    }
}

/// Response for device list
#[derive(Debug, Serialize)]
pub struct DeviceListResponse {
    pub devices: Vec<DeviceInfo>,
}

/// Test result response
#[derive(Debug, Serialize)]
pub struct TestResultResponse {
    /// Test run identifier
    pub test_id: String,
    /// Current status
    pub status: String,
    /// Number of passed tests
    pub passed: Option<u32>,
    /// Number of failed tests
    pub failed: Option<u32>,
    /// Number of skipped tests
    pub skipped: Option<u32>,
    /// Total duration in seconds
    pub duration: Option<f64>,
    /// Test failures
    pub failures: Vec<TestFailure>,
    /// Test logs
    pub logs: Vec<String>,
}

/// Individual test failure
#[derive(Debug, Serialize)]
pub struct TestFailure {
    /// Test name
    pub test_name: String,
    /// Failure message
    pub message: String,
    /// File location
    pub file: Option<String>,
    /// Line number
    pub line: Option<u32>,
}
