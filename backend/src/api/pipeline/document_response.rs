//! Document response with computed fields for frontend consumption.
//!
//! The frontend must not contain business logic — all access control,
//! status grouping, and visibility decisions are computed here and
//! included in the API response.

use serde::Serialize;

use crate::auth::AuthUser;
use crate::repositories::pipeline_repository::DocumentRecord;

/// Document response with computed fields.
///
/// Wraps a `DocumentRecord` (from the DB) and adds fields that the
/// frontend needs for display and access control, so the frontend
/// never compares status strings or checks user roles.
#[derive(Debug, Serialize)]
pub struct DocumentResponse {
    // All fields from DocumentRecord are flattened into the response
    #[serde(flatten)]
    pub document: DocumentRecord,

    /// Tabs the current user can see for this document.
    pub visible_tabs: Vec<String>,

    /// Whether the current user can view/interact with this document.
    pub can_view: bool,

    /// Display grouping: "published", "processing", "in_review", "uploaded".
    pub status_group: String,
}

/// Build a DocumentResponse from a DocumentRecord and user context.
pub fn enrich_document(doc: DocumentRecord, user: &AuthUser) -> DocumentResponse {
    let is_admin = user.is_admin();
    let is_assigned = doc.assigned_reviewer.as_deref() == Some(&user.username);

    let visible_tabs = compute_visible_tabs(&doc.status, is_admin, is_assigned);
    let can_view = compute_can_view(&doc.status, is_admin);
    let status_group = compute_status_group(&doc.status);

    DocumentResponse {
        document: doc,
        visible_tabs,
        can_view,
        status_group,
    }
}

/// Compute which tabs a user can see for a document in its current state.
///
/// ## Rust Learning: Centralized access control
///
/// This function is the single source of truth for tab visibility.
/// The frontend reads `visible_tabs` from the response and renders
/// only those tabs — no role checks or status comparisons needed.
fn compute_visible_tabs(status: &str, is_admin: bool, is_assigned_reviewer: bool) -> Vec<String> {
    let mut tabs = vec!["document".to_string()];
    let is_published = status == "PUBLISHED";

    if is_published || is_admin {
        tabs.push("content".to_string());
    }
    if is_admin {
        tabs.push("processing".to_string());
    }
    if is_admin || is_assigned_reviewer {
        tabs.push("review".to_string());
    }
    if is_published || is_admin {
        tabs.push("people".to_string());
    }
    tabs
}

/// Whether the current user can view/interact with this document.
fn compute_can_view(status: &str, is_admin: bool) -> bool {
    is_admin || status == "PUBLISHED"
}

/// Map pipeline status to a display group for frontend filtering/sorting.
fn compute_status_group(status: &str) -> String {
    match status {
        "PUBLISHED" => "published",
        "UPLOADED" => "uploaded",
        "VERIFIED" | "REVIEWED" => "in_review",
        _ => "processing",
    }
    .to_string()
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_visible_tabs_admin_sees_all() {
        let tabs = compute_visible_tabs("UPLOADED", true, false);
        assert_eq!(tabs.len(), 5);
        assert!(tabs.contains(&"document".to_string()));
        assert!(tabs.contains(&"content".to_string()));
        assert!(tabs.contains(&"processing".to_string()));
        assert!(tabs.contains(&"review".to_string()));
        assert!(tabs.contains(&"people".to_string()));
    }

    #[test]
    fn compute_visible_tabs_user_published() {
        let tabs = compute_visible_tabs("PUBLISHED", false, false);
        assert!(tabs.contains(&"document".to_string()));
        assert!(tabs.contains(&"content".to_string()));
        assert!(tabs.contains(&"people".to_string()));
        assert!(!tabs.contains(&"processing".to_string()));
        assert!(!tabs.contains(&"review".to_string()));
    }

    #[test]
    fn compute_visible_tabs_reviewer() {
        let tabs = compute_visible_tabs("VERIFIED", false, true);
        assert!(tabs.contains(&"document".to_string()));
        assert!(tabs.contains(&"review".to_string()));
        assert!(!tabs.contains(&"processing".to_string()));
        assert!(!tabs.contains(&"content".to_string()));
        assert!(!tabs.contains(&"people".to_string()));
    }

    #[test]
    fn compute_visible_tabs_user_processing() {
        let tabs = compute_visible_tabs("EXTRACTED", false, false);
        assert_eq!(tabs, vec!["document"]);
    }

    #[test]
    fn compute_status_group_values() {
        assert_eq!(compute_status_group("PUBLISHED"), "published");
        assert_eq!(compute_status_group("UPLOADED"), "uploaded");
        assert_eq!(compute_status_group("VERIFIED"), "in_review");
        assert_eq!(compute_status_group("REVIEWED"), "in_review");
        assert_eq!(compute_status_group("EXTRACTED"), "processing");
        assert_eq!(compute_status_group("INGESTED"), "processing");
        assert_eq!(compute_status_group("INDEXED"), "processing");
    }

    #[test]
    fn compute_can_view_admin() {
        assert!(compute_can_view("UPLOADED", true));
        assert!(compute_can_view("EXTRACTED", true));
    }

    #[test]
    fn compute_can_view_user() {
        assert!(compute_can_view("PUBLISHED", false));
        assert!(!compute_can_view("UPLOADED", false));
        assert!(!compute_can_view("VERIFIED", false));
    }
}
