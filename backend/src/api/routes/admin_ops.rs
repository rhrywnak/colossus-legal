//! Admin operational routes: evidence import, QA-entry admin, audit health,
//! status, and the nested pipeline admin router.

use axum::{
    routing::{get, post},
    Router,
};

use crate::api::{admin_audit_health, admin_evidence, admin_qa, admin_status, pipeline};
use crate::state::AppState;

/// Admin operational routes: evidence import, QA-entry admin, audit health,
/// status, and the nested pipeline admin router.
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/admin/evidence", post(admin_evidence::import_evidence))
        .route(
            "/admin/qa-entries",
            get(admin_qa::list_all_entries).delete(admin_qa::bulk_delete_entries),
        )
        .route("/admin/audit/health", get(admin_audit_health::audit_health))
        .route("/admin/status", get(admin_status::get_status))
        .nest("/admin/pipeline", pipeline::router())
}
