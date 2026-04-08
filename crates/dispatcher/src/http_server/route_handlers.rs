use crate::http_server::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use std::sync::Arc;
use uuid::Uuid;

pub async fn assign_process_handler(
    State(state): State<Arc<AppState>>,
    Path(supervisor_id): Path<Uuid>,
) -> impl IntoResponse {
    let res = state.dispatcher.assign_process(supervisor_id).await;

    if res.is_err() {
        // I want to return a json string here
        return (
            //500
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "message": format!("Failed to assign process: {}", res.unwrap_err())
            })),
        );
    }
    let assigned_process_option = res.unwrap();
    if assigned_process_option.is_none() {
        return (
            //204
            axum::http::StatusCode::NO_CONTENT,
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
