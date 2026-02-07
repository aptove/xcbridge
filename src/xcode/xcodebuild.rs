// Copyright 2026 Aptove
// SPDX-License-Identifier: Apache-2.0

//! xcodebuild command wrapper

use crate::error::{Result, XcbridgeError};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Get the installed Xcode version
pub async fn get_xcode_version() -> Result<String> {
    let output = Command::new("xcodebuild")
        .arg("-version")
        .output()
        .await
        .map_err(|_| XcbridgeError::XcodeNotFound)?;

    if !output.status.success() {
        return Err(XcbridgeError::XcodeNotFound);
    }

    let version = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or("Unknown")
        .to_string();

    Ok(version)
}

/// Parameters for a build operation
#[derive(Debug, Clone)]
pub struct BuildParams {
    pub project: Option<String>,
    pub workspace: Option<String>,
    pub scheme: String,
    pub configuration: String,
    pub destination: Option<String>,
    pub derived_data_path: Option<String>,
    pub extra_args: Vec<String>,
}

impl BuildParams {
    /// Convert to xcodebuild arguments
    pub fn to_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        if let Some(project) = &self.project {
            args.push("-project".to_string());
            args.push(project.clone());
        }

        if let Some(workspace) = &self.workspace {
            args.push("-workspace".to_string());
            args.push(workspace.clone());
        }

        args.push("-scheme".to_string());
        args.push(self.scheme.clone());

        args.push("-configuration".to_string());
        args.push(self.configuration.clone());

        if let Some(destination) = &self.destination {
            args.push("-destination".to_string());
            args.push(destination.clone());
        }

        if let Some(derived_data) = &self.derived_data_path {
            args.push("-derivedDataPath".to_string());
            args.push(derived_data.clone());
        }

        args.extend(self.extra_args.clone());

        args
    }
}

/// Parameters for a test operation
#[derive(Debug, Clone)]
pub struct TestParams {
    pub project: Option<String>,
    pub workspace: Option<String>,
    pub scheme: String,
    pub destination: Option<String>,
    pub test_plan: Option<String>,
    pub only_testing: Vec<String>,
    pub skip_testing: Vec<String>,
}

impl TestParams {
    /// Convert to xcodebuild test arguments
    pub fn to_args(&self) -> Vec<String> {
        let mut args = vec!["test".to_string()];

        if let Some(project) = &self.project {
            args.push("-project".to_string());
            args.push(project.clone());
        }

        if let Some(workspace) = &self.workspace {
            args.push("-workspace".to_string());
            args.push(workspace.clone());
        }

        args.push("-scheme".to_string());
        args.push(self.scheme.clone());

        if let Some(destination) = &self.destination {
            args.push("-destination".to_string());
            args.push(destination.clone());
        }

        if let Some(test_plan) = &self.test_plan {
            args.push("-testPlan".to_string());
            args.push(test_plan.clone());
        }

        for test in &self.only_testing {
            args.push("-only-testing".to_string());
            args.push(test.clone());
        }

        for test in &self.skip_testing {
            args.push("-skip-testing".to_string());
            args.push(test.clone());
        }

        args
    }
}

/// Output from a build operation
#[derive(Debug)]
pub struct BuildOutput {
    pub success: bool,
    pub exit_code: i32,
    pub logs: Vec<String>,
    pub build_dir: Option<String>,
}

/// Run xcodebuild with the given arguments, streaming output via callback
pub async fn run_xcodebuild<F>(args: Vec<String>, mut on_line: F) -> Result<BuildOutput>
where
    F: FnMut(String),
{
    let mut cmd = Command::new("xcodebuild");
    cmd.args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    tracing::info!("Running: xcodebuild {}", args.join(" "));

    let mut child = cmd
        .spawn()
        .map_err(|e| XcbridgeError::CommandFailed(format!("Failed to spawn xcodebuild: {}", e)))?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let mut logs = Vec::new();
    let mut build_dir = None;

    loop {
        tokio::select! {
            line = stdout_reader.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        // Check for build directory in output
                        if line.contains("BUILD_DIR = ") {
                            if let Some(dir) = line.split("BUILD_DIR = ").nth(1) {
                                build_dir = Some(dir.trim().to_string());
                            }
                        }
                        on_line(line.clone());
                        logs.push(line);
                    }
                    Ok(None) => break,
                    Err(e) => {
                        tracing::warn!("Error reading stdout: {}", e);
                        break;
                    }
                }
            }
            line = stderr_reader.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        on_line(line.clone());
                        logs.push(line);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        tracing::warn!("Error reading stderr: {}", e);
                    }
                }
            }
        }
    }

    let status = child
        .wait()
        .await
        .map_err(|e| XcbridgeError::CommandFailed(format!("Failed to wait for xcodebuild: {}", e)))?;

    let exit_code = status.code().unwrap_or(-1);

    Ok(BuildOutput {
        success: status.success(),
        exit_code,
        logs,
        build_dir,
    })
}

/// Run a simple xcodebuild command and return output
pub async fn xcodebuild(args: &[&str]) -> Result<String> {
    let output = Command::new("xcodebuild")
        .args(args)
        .output()
        .await
        .map_err(|e| XcbridgeError::CommandFailed(format!("xcodebuild failed: {}", e)))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(XcbridgeError::CommandFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}

/// List available SDKs
pub async fn list_sdks() -> Result<Vec<String>> {
    let output = xcodebuild(&["-showsdks"]).await?;
    let sdks: Vec<String> = output
        .lines()
        .filter(|line| line.contains("-sdk"))
        .filter_map(|line| line.split("-sdk").nth(1))
        .map(|s| s.trim().to_string())
        .collect();
    Ok(sdks)
}
