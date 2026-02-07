// Copyright 2026 Aptove
// SPDX-License-Identifier: Apache-2.0

//! Build handler

use crate::error::{Result, XcbridgeError};
use crate::models::{BuildRequest, BuildStartedResponse, BuildStatusResponse};
use crate::state::{BuildStatus, SharedState};
use crate::xcode::xcodebuild::{self, BuildParams};
use axum::{
    extract::{Path, State},
    response::sse::{Event, Sse},
    Json,
};
use futures::stream::Stream;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

/// POST /build - Start a new build
pub async fn start_build(
    State(state): State<SharedState>,
    Json(req): Json<BuildRequest>,
) -> Result<Json<BuildStartedResponse>> {
    // Validate project/workspace path
    let project_path = req
        .project
        .as_ref()
        .or(req.workspace.as_ref())
        .ok_or_else(|| {
            XcbridgeError::InvalidRequest("Either project or workspace must be specified".into())
        })?;

    let path = PathBuf::from(project_path);
    if !state.config.is_path_allowed(&path) {
        return Err(XcbridgeError::PathNotAllowed(project_path.clone()));
    }

    // Generate build ID
    let build_id = Uuid::new_v4().to_string();
    
    // Create build entry
    state.create_build(&build_id).await;

    // Convert request to build params
    let params = BuildParams {
        project: req.project,
        workspace: req.workspace,
        scheme: req.scheme,
        configuration: req.configuration,
        destination: req.destination,
        derived_data_path: req.derived_data_path,
        extra_args: req.extra_args,
    };

    // Spawn build task
    let state_clone = Arc::clone(&state);
    let build_id_clone = build_id.clone();
    tokio::spawn(async move {
        run_build(state_clone, build_id_clone, params).await;
    });

    Ok(Json(BuildStartedResponse {
        build_id: build_id.clone(),
        status: "running".to_string(),
        logs_url: format!("/build/{}/logs", build_id),
    }))
}

/// Run the actual build
async fn run_build(state: SharedState, build_id: String, params: BuildParams) {
    let state_clone = Arc::clone(&state);
    let build_id_clone = build_id.clone();

    let (tx, mut rx) = mpsc::channel::<String>(100);

    // Spawn log collector
    let state_for_logs = Arc::clone(&state);
    let build_id_for_logs = build_id.clone();
    tokio::spawn(async move {
        while let Some(line) = rx.recv().await {
            state_for_logs
                .append_build_log(&build_id_for_logs, line)
                .await;
        }
    });

    // Run xcodebuild
    let result = xcodebuild::run_xcodebuild(params.to_args(), move |line| {
        let _ = tx.try_send(line);
    })
    .await;

    match result {
        Ok(output) => {
            if output.success {
                let artifacts = output
                    .build_dir
                    .map(|d| vec![d])
                    .unwrap_or_default();
                state_clone.complete_build(&build_id_clone, artifacts).await;
            } else {
                let error = output
                    .logs
                    .iter()
                    .rev()
                    .find(|l| l.contains("error:"))
                    .cloned()
                    .unwrap_or_else(|| "Build failed".to_string());
                state_clone
                    .fail_build(&build_id_clone, error, Some(output.exit_code))
                    .await;
            }
        }
        Err(e) => {
            state_clone
                .fail_build(&build_id_clone, e.to_string(), None)
                .await;
        }
    }
}

/// GET /build/:id - Get build status
pub async fn get_build(
    State(state): State<SharedState>,
    Path(build_id): Path<String>,
) -> Result<Json<BuildStatusResponse>> {
    let build = state
        .get_build(&build_id)
        .await
        .ok_or_else(|| XcbridgeError::BuildNotFound(build_id.clone()))?;

    let (status, exit_code, artifacts, error, logs) = match build {
        BuildStatus::Running { logs } => ("running", None, None, None, logs),
        BuildStatus::Success { logs, artifacts } => {
            ("success", Some(0), Some(artifacts), None, logs)
        }
        BuildStatus::Failed {
            logs,
            error,
            exit_code,
        } => ("failed", exit_code, None, Some(error), logs),
        BuildStatus::Cancelled => ("cancelled", None, None, None, vec![]),
    };

    Ok(Json(BuildStatusResponse {
        build_id,
        status: status.to_string(),
        exit_code,
        artifacts,
        error,
        logs,
    }))
}

/// GET /build/:id/logs - Stream build logs via SSE
pub async fn build_logs(
    State(state): State<SharedState>,
    Path(build_id): Path<String>,
) -> Result<Sse<impl Stream<Item = std::result::Result<Event, Infallible>>>> {
    // Verify build exists
    if state.get_build(&build_id).await.is_none() {
        return Err(XcbridgeError::BuildNotFound(build_id));
    }

    let stream = async_stream::stream! {
        let mut last_index = 0;
        
        loop {
            if let Some(build) = state.get_build(&build_id).await {
                let logs = build.logs();
                
                // Send new log lines
                for line in logs.iter().skip(last_index) {
                    yield Ok(Event::default().data(line.clone()));
                }
                last_index = logs.len();

                // Check if build is complete
                if build.is_complete() {
                    let status = match &build {
                        BuildStatus::Success { .. } => "success",
                        BuildStatus::Failed { .. } => "failed",
                        BuildStatus::Cancelled => "cancelled",
                        _ => "unknown",
                    };
                    yield Ok(Event::default().event("complete").data(status));
                    break;
                }
            } else {
                break;
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    };

    Ok(Sse::new(stream))
}

/// DELETE /build/:id - Cancel a build
pub async fn cancel_build(
    State(state): State<SharedState>,
    Path(build_id): Path<String>,
) -> Result<Json<BuildStatusResponse>> {
    let cancelled = state.cancel_build(&build_id).await;
    
    if !cancelled {
        return Err(XcbridgeError::BuildNotFound(build_id));
    }

    Ok(Json(BuildStatusResponse {
        build_id,
        status: "cancelled".to_string(),
        exit_code: None,
        artifacts: None,
        error: None,
        logs: vec![],
    }))
}
