//! Everything a question-chat request can fail with — one variant per
//! operationally distinct state, each with its own HTTP status (Standing Rule 1).

use uuid::Uuid;

use crate::error::AppError;
use crate::repositories::pipeline_repository::PipelineRepoError;

/// A question-chat failure BEFORE the reply starts streaming. (Once the stream is
/// open, failures travel as a `failed` event instead — the status line is sent.)
#[derive(Debug, thiserror::Error)]
pub enum ChatRunError {
    #[error("a message needs some text")]
    BlankText,
    #[error("practice question {0} not found")]
    QuestionNotFound(Uuid),
    #[error("{owner}'s thread is read-only for you — only its owner writes in it")]
    NotOwner { owner: String },
    #[error("this thread has reached its limit of {max} replies")]
    CapReached { max: u32 },
    #[error("the question chat is not configured on this server: no ANTHROPIC_API_KEY")]
    EngineOff,
    #[error("the chat model cannot be used: {0}")]
    ModelUnusable(String),
    #[error("{what} could not be read from {path}: {detail}")]
    FileUnreadable {
        what: &'static str,
        path: String,
        detail: String,
    },
    #[error(
        "the package is about {estimate} tokens and leaves less than the configured \
         headroom inside {model}'s {limit}-token context — refused before any call"
    )]
    ContextTooLarge {
        estimate: usize,
        limit: usize,
        model: String,
    },
    #[error("{operation} failed for question {question_id}: {source}")]
    Store {
        operation: &'static str,
        question_id: Uuid,
        #[source]
        source: PipelineRepoError,
    },
}

/// A closure that tags a repository failure with the operation that failed.
pub(crate) fn store(
    operation: &'static str,
    question_id: Uuid,
) -> impl FnOnce(PipelineRepoError) -> ChatRunError {
    move |source| ChatRunError::Store {
        operation,
        question_id,
        source,
    }
}

impl ChatRunError {
    /// The status each failure answers with, logged with the question and user.
    pub fn into_app_error(self, question_id: Uuid, username: &str) -> AppError {
        let message = self.to_string();
        match self {
            Self::BlankText => AppError::BadRequest {
                message,
                details: serde_json::Value::Null,
            },
            Self::QuestionNotFound(_) => AppError::NotFound { message },
            Self::NotOwner { .. } => {
                tracing::warn!(%question_id, by = username, %message, "question chat: write to another's thread refused");
                AppError::Forbidden { message }
            }
            Self::CapReached { max } => AppError::Conflict {
                message,
                details: serde_json::json!({ "max_turns": max }),
            },
            Self::ContextTooLarge { .. } => {
                tracing::error!(%question_id, by = username, %message, "question chat: package too large");
                AppError::UnprocessableEntity {
                    message,
                    details: serde_json::Value::Null,
                }
            }
            Self::EngineOff | Self::ModelUnusable(_) | Self::FileUnreadable { .. } => {
                tracing::error!(%question_id, by = username, %message, "question chat: not available");
                AppError::ServiceUnavailable { message }
            }
            Self::Store { .. } => {
                tracing::error!(%question_id, by = username, %message, "question chat: store failure");
                AppError::Internal {
                    message: "the discussion could not be completed".to_string(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    fn status(e: ChatRunError) -> u16 {
        e.into_app_error(Uuid::nil(), "docmarie")
            .into_response()
            .status()
            .as_u16()
    }

    /// Every failure has its own status — never collapsed into one 500.
    #[test]
    fn each_failure_maps_to_its_own_status() {
        assert_eq!(status(ChatRunError::BlankText), 400);
        assert_eq!(status(ChatRunError::QuestionNotFound(Uuid::nil())), 404);
        assert_eq!(
            status(ChatRunError::NotOwner {
                owner: "Chuck".into()
            }),
            403
        );
        assert_eq!(status(ChatRunError::CapReached { max: 2 }), 409);
        assert_eq!(
            status(ChatRunError::ContextTooLarge {
                estimate: 1,
                limit: 1,
                model: "m".into()
            }),
            422
        );
        assert_eq!(status(ChatRunError::EngineOff), 503);
        assert_eq!(status(ChatRunError::ModelUnusable("x".into())), 503);
        assert_eq!(
            status(ChatRunError::Store {
                operation: "x",
                question_id: Uuid::nil(),
                source: PipelineRepoError::from(sqlx::Error::RowNotFound),
            }),
            500
        );
    }
}
