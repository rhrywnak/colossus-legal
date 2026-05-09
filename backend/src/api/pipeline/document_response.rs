//! Document response with computed fields for frontend consumption.
//!
//! The frontend must not contain business logic — all access control,
//! status grouping, and visibility decisions are computed here and
//! included in the API response.

use serde::Serialize;

use crate::auth::AuthUser;
use crate::models::document_status::{
    STATUS_CANCELLED, STATUS_COMPLETED, STATUS_EXTRACTED, STATUS_FAILED, STATUS_INDEXED,
    STATUS_INGESTED, STATUS_NEW, STATUS_PROCESSING, STATUS_PUBLISHED, STATUS_UPLOADED,
    STATUS_VERIFIED,
};
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
        STATUS_NEW | STATUS_UPLOADED => vec!["document", "processing"],
        STATUS_PROCESSING => vec!["document", "processing"],
        STATUS_EXTRACTED | STATUS_VERIFIED => vec!["document", "content", "processing"],
        STATUS_INGESTED | STATUS_INDEXED | STATUS_COMPLETED | STATUS_PUBLISHED => {
            vec!["document", "content", "processing", "review", "people"]
        }
        STATUS_FAILED | STATUS_CANCELLED => vec!["document", "processing"],
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
        STATUS_EXTRACTED
            | STATUS_VERIFIED
            | STATUS_INGESTED
            | STATUS_INDEXED
            | STATUS_COMPLETED
            | STATUS_PUBLISHED
    )
}

/// Map pipeline status to a display group for frontend filtering/sorting.
///
/// Only `COMPLETED` and `PUBLISHED` qualify as "completed" — every
/// earlier status the pipeline writes mid-run (`EXTRACTED`, `VERIFIED`,
/// `INGESTED`, `INDEXED`) stays in the `"processing"` bucket. The
/// frontend polls the documents list every 3s while `status_group ==
/// "processing"` (`DocumentWorkspaceTabs.tsx`), so classifying a
/// mid-pipeline status as terminal stops the polling interval before
/// the later steps (Index, Completeness) can be observed. This caused
/// the "Index never updates" UI bug: Ingest's `status = INGESTED` write
/// flipped the group to `"completed"` while Index + Completeness were
/// still queued.
pub fn compute_status_group(status: &str) -> &'static str {
    match status {
        STATUS_NEW | STATUS_UPLOADED => "new",
        STATUS_PROCESSING | STATUS_EXTRACTED | STATUS_VERIFIED | STATUS_INGESTED
        | STATUS_INDEXED => "processing",
        STATUS_COMPLETED | STATUS_PUBLISHED => "completed",
        STATUS_FAILED => "failed",
        STATUS_CANCELLED => "cancelled",
        _ => "unknown",
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // --- status_group ---

    #[test]
    fn test_status_group_routing_table() {
        // Routing table: status string → display group. Mid-pipeline
        // statuses (EXTRACTED/VERIFIED/INGESTED/INDEXED) MUST stay
        // in "processing" — collapsing them to "completed" stops the
        // 3s frontend poll before Index + Completeness can finish
        // (the original "Index never updates" UI bug).
        let cases = [
            ("NEW", "new"),
            ("UPLOADED", "new"),
            ("PROCESSING", "processing"),
            ("EXTRACTED", "processing"),  // mid-pipeline: keep polling
            ("VERIFIED", "processing"),   // mid-pipeline: keep polling
            ("INGESTED", "processing"),   // mid-pipeline: keep polling
            ("INDEXED", "processing"),    // mid-pipeline: keep polling
            ("COMPLETED", "completed"),
            ("PUBLISHED", "completed"),
            ("FAILED", "failed"),
            ("CANCELLED", "cancelled"),
            ("GARBAGE", "unknown"),       // fallback
        ];
        for (input, expected) in cases {
            assert_eq!(
                compute_status_group(input),
                expected,
                "compute_status_group({input:?}) should be {expected:?}"
            );
        }
    }

    // --- visible_tabs ---

    #[test]
    fn test_visible_tabs_per_status() {
        // Routing table: status string → tab list (non-admin). The frontend
        // reads this; renaming any tab here breaks the UI silently.
        let cases: &[(&str, &[&str])] = &[
            ("NEW",        &["document", "processing"]),
            ("UPLOADED",   &["document", "processing"]),
            ("PROCESSING", &["document", "processing"]),
            ("EXTRACTED",  &["document", "content", "processing"]),
            ("INGESTED",   &["document", "content", "processing", "review", "people"]),
            ("INDEXED",    &["document", "content", "processing", "review", "people"]),
            ("PUBLISHED",  &["document", "content", "processing", "review", "people"]),
            ("FAILED",     &["document", "processing"]),
            ("CANCELLED",  &["document", "processing"]),
        ];
        for (input, expected) in cases {
            let tabs = compute_visible_tabs(input, false);
            assert_eq!(
                tabs.as_slice(),
                *expected,
                "compute_visible_tabs({input:?}, false) should be {expected:?}"
            );
        }
    }

    // --- can_view ---

    #[test]
    fn test_can_view_published_non_admin() {
        assert!(compute_can_view("PUBLISHED", false));
    }

    #[test]
    fn test_can_view_ingested_non_admin() {
        assert!(compute_can_view("INGESTED", false));
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
        assert!(compute_can_view("INGESTED", true));
        assert!(compute_can_view("INDEXED", true));
        assert!(compute_can_view("PUBLISHED", true));
        assert!(compute_can_view("FAILED", true));
        assert!(compute_can_view("CANCELLED", true));
    }
}
