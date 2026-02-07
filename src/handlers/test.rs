// Copyright 2026 Aptove
// SPDX-License-Identifier: Apache-2.0

//! Test handler

use crate::error::{Result, XcbridgeError};
use crate::models::{BuildStartedResponse, TestRequest, TestResultResponse};
use crate::state::{BuildStatus, SharedState};
use crate::xcode::xcodebuild::{self, TestParams};
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

/// POST /test - Start a test run
pub async fn start_test(
    State(state): State<SharedState>,
    Json(req): Json<TestRequest>,
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

    // Generate test ID
    let test_id = Uuid::new_v4().to_string();
    
    // Create test entry (reusing build state)
    state.create_build(&test_id).await;

    // Convert request to test params
    let params = TestParams {
        project: req.project,
        workspace: req.workspace,
        scheme: req.scheme,
        destination: req.destination,
        test_plan: req.test_plan,
        only_testing: req.only_testing,
        skip_testing: req.skip_testing,
    };

    // Spawn test task
    let state_clone = Arc::clone(&state);
    let test_id_clone = test_id.clone();
    tokio::spawn(async move {
        run_test(state_clone, test_id_clone, params).await;
    });

    Ok(Json(BuildStartedResponse {
        build_id: test_id.clone(),
        status: "running".to_string(),
        logs_url: format!("/test/{}/logs", test_id),
    }))
}

/// Run the actual test
async fn run_test(state: SharedState, test_id: String, params: TestParams) {
    let state_clone = Arc::clone(&state);
    let test_id_clone = test_id.clone();

    let (tx, mut rx) = mpsc::channel::<String>(100);

    // Spawn log collector
    let state_for_logs = Arc::clone(&state);
    let test_id_for_logs = test_id.clone();
    tokio::spawn(async move {
        while let Some(line) = rx.recv().await {
            state_for_logs
                .append_build_log(&test_id_for_logs, line)
                .await;
        }
    });

    // Run xcodebuild test
    let result = xcodebuild::run_xcodebuild(params.to_args(), move |line| {
        let _ = tx.try_send(line);
    })
    .await;

    match result {
        Ok(output) => {
            if output.success {
                state_clone.complete_build(&test_id_clone, vec![]).await;
            } else {
                let error = output
                    .logs
                    .iter()
                    .rev()
                    .find(|l| l.contains("** TEST FAILED **") || l.contains("error:"))
                    .cloned()
                    .unwrap_or_else(|| "Tests failed".to_string());
                state_clone
                    .fail_build(&test_id_clone, error, Some(output.exit_code))
                    .await;
            }
        }
        Err(e) => {
            state_clone
                .fail_build(&test_id_clone, e.to_string(), None)
                .await;
        }
    }
}

/// GET /test/:id - Get test status
pub async fn get_test(
    State(state): State<SharedState>,
    Path(test_id): Path<String>,
) -> Result<Json<TestResultResponse>> {
    let test = state
        .get_build(&test_id)
        .await
        .ok_or_else(|| XcbridgeError::BuildNotFound(test_id.clone()))?;

    let (status, logs) = match &test {
        BuildStatus::Running { logs } => ("running", logs.clone()),
        BuildStatus::Success { logs, .. } => ("success", logs.clone()),
        BuildStatus::Failed { logs, .. } => ("failed", logs.clone()),
        BuildStatus::Cancelled => ("cancelled", vec![]),
    };

    // Parse test results from logs (basic parsing)
    let (passed, failed, skipped) = parse_test_counts(&logs);

    Ok(Json(TestResultResponse {
        test_id,
        status: status.to_string(),
        passed: Some(passed),
        failed: Some(failed),
        skipped: Some(skipped),
        duration: None, // TODO: Parse from logs
        failures: vec![], // TODO: Parse failures from logs
        logs,
    }))
}

/// Parse test counts from xcodebuild output
fn parse_test_counts(logs: &[String]) -> (u32, u32, u32) {
    let passed = 0u32;
    let failed = 0u32;
    let skipped = 0u32;

    for line in logs {
        if line.contains("Test Suite") && line.contains("passed") {
            // Parse: "Test Suite 'All tests' passed at ..."
            // This is a simplistic approach
        }
        if line.contains("Executed") && line.contains("tests") {
            // Parse: "Executed 10 tests, with 2 failures (0 unexpected) in 1.234 (1.456) seconds"
            if let Some(nums) = parse_test_summary(line) {
                return nums;
            }
        }
    }

    (passed, failed, skipped)
}

fn parse_test_summary(line: &str) -> Option<(u32, u32, u32)> {
    // "Executed 10 tests, with 2 failures (0 unexpected) in 1.234 seconds"
    let parts: Vec<&str> = line.split_whitespace().collect();
    
    let executed_idx = parts.iter().position(|&p| p == "Executed")?;
    let total: u32 = parts.get(executed_idx + 1)?.parse().ok()?;
    
    let failures_idx = parts.iter().position(|&p| p == "failures" || p == "failure")?;
    let failed: u32 = parts.get(failures_idx - 1)?.parse().ok()?;
    
    let passed = total.saturating_sub(failed);
    
    Some((passed, failed, 0))
}

/// GET /test/:id/logs - Stream test logs via SSE
pub async fn test_logs(
    State(state): State<SharedState>,
    Path(test_id): Path<String>,
) -> Result<Sse<impl Stream<Item = std::result::Result<Event, Infallible>>>> {
    // Verify test exists
    if state.get_build(&test_id).await.is_none() {
        return Err(XcbridgeError::BuildNotFound(test_id));
    }

    let stream = async_stream::stream! {
        let mut last_index = 0;
        
        loop {
            if let Some(test) = state.get_build(&test_id).await {
                let logs = test.logs();
                
                // Send new log lines
                for line in logs.iter().skip(last_index) {
                    yield Ok(Event::default().data(line.clone()));
                }
                last_index = logs.len();

                // Check if test is complete
                if test.is_complete() {
                    let status = match &test {
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
