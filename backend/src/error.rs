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
    /// The server understands the request but a required dependency is
    /// not currently available — e.g. an optional config variable is
    /// unset on a deployment that needs it, or an upstream service is
    /// reachable but reports `unavailable` for the operation.
    ///
    /// Maps to HTTP 503 Service Unavailable. Distinct from `Internal`
    /// (500): the operator can correct a `ServiceUnavailable` by
    /// fixing configuration or starting the missing dependency, while
    /// `Internal` signals a bug or a transient error inside the server
    /// itself.
    ServiceUnavailable {
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

/// Convert a [`ProcessingProfileLoadError`] into an HTTP-500 [`AppError`].
///
/// All variants (FileNotFound, IoError, ParseError) represent operator-
/// visible misconfiguration the upload handler should surface immediately
/// per silent-fallback audit defect #2.1. The Display impl on the source
/// error carries the human-readable detail (path, underlying source);
/// that string becomes the `message` field of the JSON 500 body.
///
/// One caller — `schema_file_for_document_type` — uses pattern-matching
/// instead of this `From` impl so it can apply a different policy to
/// the `FileNotFound` variant (see defect #2.3 deferral). Every other
/// caller uses `?` and goes through this conversion.
impl From<crate::pipeline::config::ProcessingProfileLoadError> for AppError {
    fn from(err: crate::pipeline::config::ProcessingProfileLoadError) -> Self {
        AppError::Internal {
            message: err.to_string(),
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
            AppError::ServiceUnavailable { message } => {
                let body = ErrorBody {
                    error: "service_unavailable".to_string(),
                    message,
                    details: json!({}),
                };
                (StatusCode::SERVICE_UNAVAILABLE, Json(body)).into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    /// Pins the IntoResponse mapping for the new `ServiceUnavailable`
    /// variant: HTTP 503 + `error: "service_unavailable"` body.
    ///
    /// This is the only `AppError` variant with a dedicated IntoResponse
    /// test — the existing variants (`Internal`, `NotFound`, `Forbidden`,
    /// etc.) have no equivalent test, so adding one for every variant
    /// would be substantial scope creep on the response-mapping
    /// boilerplate. We test the new variant only, both to pin the
    /// new contract and to establish the pattern for future variants:
    /// each new `AppError` arm gets a one-shot test asserting its
    /// status code and the `error` slug it emits.
    #[tokio::test]
    async fn service_unavailable_maps_to_503_with_error_slug() {
        let err = AppError::ServiceUnavailable {
            message: "Restate ingress not configured".to_string(),
        };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body_bytes = to_bytes(response.into_body(), 4096)
            .await
            .expect("body must be small and readable");
        let body_json: serde_json::Value =
            serde_json::from_slice(&body_bytes).expect("body must be valid JSON");

        // Pin the slug — frontend / log-scraping tooling switches on
        // this string, so a typo would silently break operator
        // workflows.
        assert_eq!(body_json["error"], serde_json::json!("service_unavailable"));
        assert_eq!(
            body_json["message"],
            serde_json::json!("Restate ingress not configured")
        );
    }
}
