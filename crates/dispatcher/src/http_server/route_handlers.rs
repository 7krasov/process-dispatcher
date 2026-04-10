use crate::dispatcher::ReportFinishError;
use crate::http_server::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use shared::ProcessFinishReport;
use std::sync::Arc;
use uuid::Uuid;

pub async fn obtain_new_process_handler(
    State(state): State<Arc<AppState>>,
    Path(supervisor_id): Path<Uuid>,
) -> impl IntoResponse {
    let res = state.dispatcher.assign_process(supervisor_id).await;

    if res.is_err() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "message": format!("Failed to obtain new process: {}", res.unwrap_err())
            })),
        );
    }
    let assigned_process_option = res.unwrap();
    if assigned_process_option.is_none() {
        return (
            StatusCode::NO_CONTENT,
            Json(serde_json::json!({
                "message": "No available processes to assign"
            })),
        );
    }

    (
        StatusCode::OK,
        Json(serde_json::json!(assigned_process_option.unwrap())),
    )
}

pub async fn report_process_finish_handler(
    State(state): State<Arc<AppState>>,
    Path(process_id): Path<Uuid>,
    Json(report): Json<ProcessFinishReport>,
) -> impl IntoResponse {
    match state
        .dispatcher
        .report_process_finish(process_id, &report.result)
        .await
    {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({ "message": "ok" })),
        ),
        Err(ReportFinishError::InvalidResult(value)) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "message": format!("invalid result value '{}'", value)
            })),
        ),
        Err(ReportFinishError::NotFound(id)) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "message": format!("process {} not found", id)
            })),
        ),
        Err(ReportFinishError::Db(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "message": format!("Failed to report process finish: {}", e)
            })),
        ),
    }
}
