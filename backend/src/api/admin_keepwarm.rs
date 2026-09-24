//! The Admin "Chat case file" box's two routes, both administrator-only.
//!
//! - `GET /api/admin/chat-case-file` — when the last chat question was, by
//!   whom, and until when the case file stays loaded.
//! - `POST /api/admin/chat-case-file/keep-loaded` — send one keep-loaded ping
//!   now and answer with what it did and the refreshed lines.
//!
//! Every value is ready to show; the page composes sentences around them and
//! decides nothing (Standing Rule 12).

use axum::{extract::State, Json};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::services::chat_keepwarm_button::{
    activity, keep_loaded, ActivityDto, KeepLoadedDto, KeepLoadedError,
};
use crate::state::AppState;

/// GET /api/admin/chat-case-file
pub async fn get_chat_case_file(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ActivityDto>, AppError> {
    require_admin(&user)?;
    activity(&state)
        .await
        .map(Json)
        .map_err(|e| into_app_error(e, &user.username, "read the chat case file's activity"))
}

/// POST /api/admin/chat-case-file/keep-loaded
pub async fn post_keep_loaded(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<KeepLoadedDto>, AppError> {
    require_admin(&user)?;
    tracing::info!(by = %user.username, "keep loaded: button pressed");
    keep_loaded(&state)
        .await
        .map(Json)
        .map_err(|e| into_app_error(e, &user.username, "keep the chat case file loaded"))
}

/// One status per failure, each logged with who asked and what failed.
///
/// A provider refusal or outage and a missing key are 503: the server is fine
/// and something it depends on is not. A store or build failure is a 500.
fn into_app_error(e: KeepLoadedError, username: &str, what: &str) -> AppError {
    let message = format!("could not {what}: {e}");
    match e {
        KeepLoadedError::EngineOff
        | KeepLoadedError::Provider(_)
        | KeepLoadedError::CaseFile(_) => {
            tracing::error!(by = username, %message, "keep loaded: not available");
            AppError::ServiceUnavailable { message }
        }
        KeepLoadedError::Request(_) | KeepLoadedError::Store(_) => {
            tracing::error!(by = username, %message, "keep loaded: failed");
            AppError::Internal { message }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    fn status(e: KeepLoadedError) -> u16 {
        into_app_error(e, "roman", "test")
            .into_response()
            .status()
            .as_u16()
    }

    #[test]
    fn each_failure_has_its_own_status() {
        assert_eq!(status(KeepLoadedError::EngineOff), 503);
        assert_eq!(
            status(KeepLoadedError::Provider(
                colossus_chat::ChatTransportError::Status {
                    status: 400,
                    body: "refused".into(),
                }
            )),
            503
        );
        assert_eq!(
            status(KeepLoadedError::Request(
                colossus_chat::RequestError::NothingToWarm
            )),
            500
        );
        // A missing prompt file: the server is fine, its configuration is not.
        let missing = crate::services::chat_question_error::ChatRunError::FileUnreadable {
            what: "the question chat's prompt",
            path: "/templates/prompt.md".into(),
            detail: "No such file".into(),
        };
        assert_eq!(status(KeepLoadedError::CaseFile(missing.into())), 503);
        // A store failure is the server's own: a 500.
        let store = crate::repositories::pipeline_repository::PipelineRepoError::from(
            sqlx::Error::RowNotFound,
        );
        assert_eq!(status(KeepLoadedError::Store(store)), 500);
    }
}
