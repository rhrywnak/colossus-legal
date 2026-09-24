//! `GET /api/admin/pipeline/last-run` — Admin → Data → Last document run
//! (CC_TASK_MODEL_JOBS_PANEL_v1, board 5). Administrator-only; every value
//! arrives ready to show.

use axum::{extract::State, Json};

use crate::auth::{require_admin, AuthUser};
use crate::dto::last_run::LastRunDto;
use crate::error::AppError;
use crate::services::last_run::last_run;
use crate::state::AppState;

/// GET /last-run
pub async fn last_run_handler(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<LastRunDto>, AppError> {
    require_admin(&user)?;
    last_run(&state).await.map(Json).map_err(|e| {
        let message = format!("could not read the last document run: {e}");
        tracing::error!(by = %user.username, %message, "last run: read failed");
        AppError::Internal { message }
    })
}
