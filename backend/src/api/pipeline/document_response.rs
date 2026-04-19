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
    #[serde(flatten)]
    pub document: DocumentRecord,

    /// Tabs the current user can see for this document.
    pub visible_tabs: Vec<&'static str>,

    /// Whether the current user can view/interact with this document.
    pub can_view: bool,

    /// Display grouping: "new", "processing", "completed", "failed", "cancelled".
    pub status_group: &'static str,
}

/// Build a DocumentResponse from a DocumentRecord and user context.
pub fn enrich_document(doc: DocumentRecord, user: &AuthUser) -> DocumentResponse {
    let is_admin = user.is_admin();

    let visible_tabs = compute_visible_tabs(&doc.status, is_admin);
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
fn compute_visible_tabs(status: &str, _is_admin: bool) -> Vec<&'static str> {
    match status {
        "NEW" | "UPLOADED" => vec!["document", "processing"],
        "PROCESSING" => vec!["document", "processing"],
        "EXTRACTED" | "VERIFIED" => vec!["document", "content", "processing"],
        "INDEXED" | "COMPLETED" | "PUBLISHED" => {
            vec!["document", "content", "processing", "review", "people"]
        }
        "FAILED" | "CANCELLED" => vec!["document", "processing"],
        _ => vec!["document", "processing"],
    }
}

/// Whether the current user can view/interact with this document.
fn compute_can_view(status: &str, is_admin: bool) -> bool {
    if is_admin {
        return true;
    }
    matches!(
        status,
        "EXTRACTED" | "VERIFIED" | "INDEXED" | "COMPLETED" | "PUBLISHED"
    )
}

/// Map pipeline status to a display group for frontend filtering/sorting.
pub fn compute_status_group(status: &str) -> &'static str {
    match status {
        "NEW" | "UPLOADED" => "new",
        "PROCESSING" => "processing",
        "EXTRACTED" | "VERIFIED" | "INDEXED" | "COMPLETED" | "PUBLISHED" => "completed",
        "FAILED" => "failed",
        "CANCELLED" => "cancelled",
        _ => "unknown",
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // --- status_group ---

    #[test]
    fn test_status_group_new() {
        assert_eq!(compute_status_group("NEW"), "new");
    }

    #[test]
    fn test_status_group_uploaded() {
        assert_eq!(compute_status_group("UPLOADED"), "new");
    }

    #[test]
    fn test_status_group_processing() {
        assert_eq!(compute_status_group("PROCESSING"), "processing");
    }

    #[test]
    fn test_status_group_extracted() {
        assert_eq!(compute_status_group("EXTRACTED"), "completed");
    }

    #[test]
    fn test_status_group_verified() {
        assert_eq!(compute_status_group("VERIFIED"), "completed");
    }

    #[test]
    fn test_status_group_indexed() {
        assert_eq!(compute_status_group("INDEXED"), "completed");
    }

    #[test]
    fn test_status_group_completed() {
        assert_eq!(compute_status_group("COMPLETED"), "completed");
    }

    #[test]
    fn test_status_group_published() {
        assert_eq!(compute_status_group("PUBLISHED"), "completed");
    }

    #[test]
    fn test_status_group_failed() {
        assert_eq!(compute_status_group("FAILED"), "failed");
    }

    #[test]
    fn test_status_group_cancelled() {
        assert_eq!(compute_status_group("CANCELLED"), "cancelled");
    }

    #[test]
    fn test_status_group_unknown() {
        assert_eq!(compute_status_group("GARBAGE"), "unknown");
    }

    // --- visible_tabs ---

    #[test]
    fn test_visible_tabs_new() {
        let tabs = compute_visible_tabs("NEW", false);
        assert_eq!(tabs, vec!["document", "processing"]);
    }

    #[test]
    fn test_visible_tabs_uploaded() {
        let tabs = compute_visible_tabs("UPLOADED", false);
        assert_eq!(tabs, vec!["document", "processing"]);
    }

    #[test]
    fn test_visible_tabs_processing() {
        let tabs = compute_visible_tabs("PROCESSING", false);
        assert_eq!(tabs, vec!["document", "processing"]);
    }

    #[test]
    fn test_visible_tabs_extracted() {
        let tabs = compute_visible_tabs("EXTRACTED", false);
        assert_eq!(tabs, vec!["document", "content", "processing"]);
    }

    #[test]
    fn test_visible_tabs_indexed() {
        let tabs = compute_visible_tabs("INDEXED", false);
        assert_eq!(tabs, vec!["document", "content", "processing", "review", "people"]);
    }

    #[test]
    fn test_visible_tabs_published() {
        let tabs = compute_visible_tabs("PUBLISHED", false);
        assert_eq!(tabs, vec!["document", "content", "processing", "review", "people"]);
    }

    #[test]
    fn test_visible_tabs_failed() {
        let tabs = compute_visible_tabs("FAILED", false);
        assert_eq!(tabs, vec!["document", "processing"]);
    }

    #[test]
    fn test_visible_tabs_cancelled() {
        let tabs = compute_visible_tabs("CANCELLED", false);
        assert_eq!(tabs, vec!["document", "processing"]);
    }

    // --- can_view ---

    #[test]
    fn test_can_view_published_non_admin() {
        assert!(compute_can_view("PUBLISHED", false));
    }

    #[test]
    fn test_can_view_indexed_non_admin() {
        assert!(compute_can_view("INDEXED", false));
    }

    #[test]
    fn test_can_view_extracted_non_admin() {
        assert!(compute_can_view("EXTRACTED", false));
    }

    #[test]
    fn test_can_view_new_non_admin_rejected() {
        assert!(!compute_can_view("NEW", false));
    }

    #[test]
    fn test_can_view_processing_non_admin_rejected() {
        assert!(!compute_can_view("PROCESSING", false));
    }

    #[test]
    fn test_can_view_admin_always() {
        assert!(compute_can_view("NEW", true));
        assert!(compute_can_view("PROCESSING", true));
        assert!(compute_can_view("EXTRACTED", true));
        assert!(compute_can_view("INDEXED", true));
        assert!(compute_can_view("PUBLISHED", true));
        assert!(compute_can_view("FAILED", true));
        assert!(compute_can_view("CANCELLED", true));
    }
}
