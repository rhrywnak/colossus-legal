use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    BadRequest {
        message: String,
        details: serde_json::Value,
    },
    NotFound {
        message: String,
    },
    Internal {
        message: String,
    },
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
    message: String,
    details: serde_json::Value,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::BadRequest { message, details } => {
                let body = ErrorBody {
                    error: "validation_error".to_string(),
                    message,
                    details,
                };
                (StatusCode::BAD_REQUEST, Json(body)).into_response()
            }
            AppError::NotFound { message } => {
                let body = ErrorBody {
                    error: "not_found".to_string(),
                    message,
                    details: json!({}),
                };
                (StatusCode::NOT_FOUND, Json(body)).into_response()
            }
            AppError::Internal { message } => {
                let body = ErrorBody {
                    error: "internal_error".to_string(),
                    message,
                    details: json!({}),
                };
                (StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response()
            }
        }
    }
}
