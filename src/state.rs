// Copyright 2026 Aptove
// SPDX-License-Identifier: Apache-2.0

//! Application state for xcbridge

use crate::config::Config;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Status of a build
#[derive(Debug, Clone)]
pub enum BuildStatus {
    Running {
        logs: Vec<String>,
    },
    Success {
        logs: Vec<String>,
        artifacts: Vec<String>,
    },
    Failed {
        logs: Vec<String>,
        error: String,
        exit_code: Option<i32>,
    },
    Cancelled,
}

impl BuildStatus {
    pub fn is_complete(&self) -> bool {
        matches!(
            self,
            BuildStatus::Success { .. } | BuildStatus::Failed { .. } | BuildStatus::Cancelled
        )
    }

    pub fn logs(&self) -> &[String] {
        match self {
            BuildStatus::Running { logs } => logs,
            BuildStatus::Success { logs, .. } => logs,
            BuildStatus::Failed { logs, .. } => logs,
            BuildStatus::Cancelled => &[],
        }
    }
}

/// Shared application state
pub struct AppState {
    pub config: Config,
    pub builds: RwLock<HashMap<String, BuildStatus>>,
    pub xcode_version: String,
}

impl AppState {
    pub fn new(config: Config, xcode_version: String) -> Self {
        Self {
            config,
            builds: RwLock::new(HashMap::new()),
            xcode_version,
        }
    }

    /// Create a new build entry
    pub async fn create_build(&self, build_id: &str) {
        let mut builds = self.builds.write().await;
        builds.insert(
            build_id.to_string(),
            BuildStatus::Running { logs: Vec::new() },
        );
    }

    /// Append a log line to a build
    pub async fn append_build_log(&self, build_id: &str, line: String) {
        let mut builds = self.builds.write().await;
        if let Some(BuildStatus::Running { logs }) = builds.get_mut(build_id) {
            logs.push(line);
        }
    }

    /// Mark a build as successful
    pub async fn complete_build(&self, build_id: &str, artifacts: Vec<String>) {
        let mut builds = self.builds.write().await;
        if let Some(status) = builds.get_mut(build_id) {
            if let BuildStatus::Running { logs } = status {
                *status = BuildStatus::Success {
                    logs: std::mem::take(logs),
                    artifacts,
                };
            }
        }
    }

    /// Mark a build as failed
    pub async fn fail_build(&self, build_id: &str, error: String, exit_code: Option<i32>) {
        let mut builds = self.builds.write().await;
        if let Some(status) = builds.get_mut(build_id) {
            if let BuildStatus::Running { logs } = status {
                *status = BuildStatus::Failed {
                    logs: std::mem::take(logs),
                    error,
                    exit_code,
                };
            }
        }
    }

    /// Get build status
    pub async fn get_build(&self, build_id: &str) -> Option<BuildStatus> {
        let builds = self.builds.read().await;
        builds.get(build_id).cloned()
    }

    /// Cancel a build
    pub async fn cancel_build(&self, build_id: &str) -> bool {
        let mut builds = self.builds.write().await;
        if let Some(status) = builds.get_mut(build_id) {
            if matches!(status, BuildStatus::Running { .. }) {
                *status = BuildStatus::Cancelled;
                return true;
            }
        }
        false
    }

    /// Clean up old completed builds (call periodically)
    pub async fn cleanup_old_builds(&self, max_completed: usize) {
        let mut builds = self.builds.write().await;
        let completed: Vec<_> = builds
            .iter()
            .filter(|(_, status)| status.is_complete())
            .map(|(id, _)| id.clone())
            .collect();

        let remove_count = completed.len().saturating_sub(max_completed);
        if remove_count > 0 {
            for id in completed.into_iter().take(remove_count) {
                builds.remove(&id);
            }
        }
    }
}

pub type SharedState = Arc<AppState>;
