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
    Unauthorized {
        message: String,
    },
    Forbidden {
        message: String,
    },
    Conflict {
        message: String,
        details: serde_json::Value,
    },
    Internal {
        message: String,
    },
}

/// Convert colossus-auth's AuthError into our AppError.
///
/// AuthError with user=None means unauthenticated (401).
/// AuthError with user=Some means insufficient permissions (403).
impl From<colossus_auth::AuthError> for AppError {
    fn from(err: colossus_auth::AuthError) -> Self {
        if err.user.is_none() {
            AppError::Unauthorized {
                message: err.message,
            }
        } else {
            AppError::Forbidden {
                message: err.message,
            }
        }
    }
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
            AppError::Unauthorized { message } => {
                let body = ErrorBody {
                    error: "unauthorized".to_string(),
                    message,
                    details: json!({}),
                };
                (StatusCode::UNAUTHORIZED, Json(body)).into_response()
            }
            AppError::Forbidden { message } => {
                let body = ErrorBody {
                    error: "forbidden".to_string(),
                    message,
                    details: json!({}),
                };
                (StatusCode::FORBIDDEN, Json(body)).into_response()
            }
            AppError::Conflict { message, details } => {
                let body = ErrorBody {
                    error: "conflict".to_string(),
                    message,
                    details,
                };
                (StatusCode::CONFLICT, Json(body)).into_response()
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
