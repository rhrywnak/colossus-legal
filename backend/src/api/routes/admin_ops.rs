//! Admin operational routes: evidence import, QA-entry admin, audit health,
//! status, and the nested pipeline admin router.

use axum::{
    routing::{get, post, put},
    Router,
};

use crate::api::{
    admin_ai_jobs, admin_audit_health, admin_evidence, admin_keepwarm, admin_qa, admin_status,
    pipeline,
};
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
        .route(
            "/admin/chat-case-file",
            get(admin_keepwarm::get_chat_case_file),
        )
        .route(
            "/admin/chat-case-file/keep-loaded",
            post(admin_keepwarm::post_keep_loaded),
        )
        .route("/admin/ai-jobs", get(admin_ai_jobs::get_ai_jobs))
        .route("/admin/ai-jobs/:job", put(admin_ai_jobs::put_ai_job))
        .route(
            "/admin/ai-jobs/:job/switch-back",
            post(admin_ai_jobs::post_switch_back),
        )
        .nest("/admin/pipeline", pipeline::router())
}
