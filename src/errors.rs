use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Could not submit document")]
    DocSubmission,
    #[error("Could not submit query")]
    QuerySubmission,
    #[error("Requested ID does not exist")]
    NonExistentId,
    #[error("Could not submit message over internal channel")]
    InternalChannelError,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, err_msg) = match self {
            ApiError::DocSubmission => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::QuerySubmission => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::NonExistentId => (StatusCode::UNPROCESSABLE_ENTITY, self.to_string()),
            ApiError::InternalChannelError => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };
        let body = Json(json!({ "error": err_msg }));

        (status, body).into_response()
    }
}

impl<T> From<tachyonix::SendError<T>> for ApiError {
    fn from(_: tachyonix::SendError<T>) -> Self {
        ApiError::InternalChannelError
    }
}
