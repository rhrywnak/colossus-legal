//! Grounding-status invariants for the review panel.
//!
//! Owns the [`GROUNDED_STATUSES`] whitelist that defines which
//! grounding-status values are "auto-approve eligible" and the three
//! review-panel counts that filter on it. Keeping the const and every
//! query that binds it into SQL in one file makes the SQL behaviour
//! and the whitelist semantics impossible to drift apart.

use sqlx::PgPool;

/// Grounding statuses treated as "grounded" by auto-approve and the related
/// review-panel counts.
///
/// - `exact` / `normalized`: verbatim quote matched PDF text.
/// - `derived`: schema-declared `Derived` grounding mode — verified by
///   provenance (e.g., v2 Harm), not PDF search.
/// - `unverified`: schema-declared `None` grounding mode — grounding was
///   never required for this entity type.
/// - `manual`: user manually supplied page/quote via edit.
///
/// `bulk_approve`, `count_pending`, and `count_ungrounded_pending` all bind
/// this const into their SQL via `= ANY(...)`, so this is the single source
/// of truth — add a new status here and every query picks it up.
///
/// ## v5.1: `derived_invalid` is intentionally NOT in this list
///
/// `derived_invalid` is the v5.1 §5.4 status for derived-mode items that
/// failed provenance validation (no provenance array, dangling reference,
/// or null `item_data`). Excluding it from `GROUNDED_STATUSES` is the
/// load-bearing decision: invalid items must NOT auto-approve, must NOT
/// be counted as grounded in the review-panel summary, and must remain
/// visible in `count_ungrounded_pending` so the operator sees the
/// failure. The diagnostic reason lives in `extraction_items.verification_reason`
/// and surfaces in the Review tab UI.
/// `grounding_status` written when a HUMAN supplied the page.
///
/// Named rather than repeated as a literal because two other modules now have
/// to recognise it: `edit_item` writes it, and both verify paths refuse to
/// overwrite it. A machine must not silently overrule a person who has already
/// read the page (2026-08-17).
pub const GROUNDING_STATUS_MANUAL: &str = "manual";

pub(crate) const GROUNDED_STATUSES: &[&str] = &[
    "exact",
    "normalized",
    "derived",
    "unverified",
    GROUNDING_STATUS_MANUAL,
];

/// The ids of every item a human has grounded by hand.
///
/// ## Rust Learning: returning a `HashSet` rather than testing inside the loop
///
/// The caller checks membership once per snippet. A `Vec::contains` would be a
/// linear scan each time — fine at these sizes, but the set states the intent:
/// this is a membership question, not a list.
///
/// Pure, so the rule "a manual grounding is never recomputed" is testable
/// without a database.
/// Whether this item's row must be left exactly as it is, and why.
///
/// Returns `true` when a human has already grounded the item. The matcher's own
/// answer is logged beside the human's either way, so agreement and
/// disagreement are both visible in the record — a silent skip would bury the
/// interesting case, which is the one where they differ.
pub fn preserve_manual_grounding(
    items: &[crate::repositories::pipeline_repository::ExtractionItemRecord],
    manual: &std::collections::HashSet<i32>,
    item_id: i32,
    matcher_page: Option<i32>,
) -> bool {
    if !manual.contains(&item_id) {
        return false;
    }
    tracing::info!(
        item_id,
        manual_page = ?manual_page_of(items, item_id),
        matcher_page = ?matcher_page,
        "verify: manual grounding kept; the matcher's page is recorded for comparison only"
    );
    true
}

pub fn manually_grounded_ids(
    items: &[crate::repositories::pipeline_repository::ExtractionItemRecord],
) -> std::collections::HashSet<i32> {
    let ids: std::collections::HashSet<i32> = items
        .iter()
        .filter(|it| it.grounding_status.as_deref() == Some(GROUNDING_STATUS_MANUAL))
        .map(|it| it.id)
        .collect();
    if !ids.is_empty() {
        tracing::info!(
            manual_items = ids.len(),
            "verify: preserving manually-grounded items; their pages are not recomputed"
        );
    }
    ids
}

/// The page a human recorded for `item_id`, if this item is one of theirs.
///
/// Used only to put both numbers side by side in the log line that fires when
/// verify declines to overwrite a manual grounding: the operator can then see
/// at a glance whether the matcher agrees with the reviewer or contradicts
/// them, which is the interesting case and the one a silent skip would bury.
pub fn manual_page_of(
    items: &[crate::repositories::pipeline_repository::ExtractionItemRecord],
    item_id: i32,
) -> Option<i32> {
    items
        .iter()
        .find(|it| it.id == item_id)
        .and_then(|it| it.grounded_page)
}

/// Return an owned `Vec<String>` copy of [`GROUNDED_STATUSES`] for sqlx
/// binding. sqlx's Postgres `TEXT[]` encoding wants an owned vector of
/// owned strings — hence the allocation per call. Cheap: ≤ 10 entries.
///
/// Visibility note: `pub(super)` so the sibling [`super::review_actions`]
/// can bind the same whitelist into `bulk_approve` without re-deriving
/// the list. The helper stays here next to the const it copies, so a
/// future entry added to `GROUNDED_STATUSES` flows through to every
/// query automatically.
pub(super) fn grounded_statuses_vec() -> Vec<String> {
    GROUNDED_STATUSES.iter().map(|s| s.to_string()).collect()
}

/// Count ungrounded pending items for a document.
///
/// These are items with review_status = 'pending' whose grounding_status is
/// outside the auto-approve set — i.e., items the "grounded" bulk_approve
/// filter will skip (NULL, `not_found`, `missing_quote`, etc.).
/// Complement of GROUNDED_STATUSES.
pub async fn count_ungrounded_pending(
    pool: &PgPool,
    document_id: &str,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM extraction_items
         WHERE document_id = $1
           AND LOWER(review_status) = 'pending'
           AND (grounding_status IS NULL OR grounding_status <> ALL($2))",
    )
    .bind(document_id)
    .bind(grounded_statuses_vec())
    .fetch_one(pool)
    .await
}

/// Count every extraction_items row for the document whose grounding_status
/// is NOT in [`GROUNDED_STATUSES`] (or is NULL). Unlike
/// `count_ungrounded_pending`, this is NOT gated by `review_status='pending'`
/// — it's a document-wide tally used by Ingest to populate
/// `documents.entities_flagged` for the Processing-tab grounding stat.
pub async fn count_flagged_items_for_document(
    pool: &PgPool,
    document_id: &str,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM extraction_items
         WHERE document_id = $1
           AND (grounding_status IS NULL OR grounding_status <> ALL($2))",
    )
    .bind(document_id)
    .bind(grounded_statuses_vec())
    .fetch_one(pool)
    .await
}

/// Count remaining pending items that are actionable in the pipeline.
///
/// Only counts items whose grounding_status is in the auto-approve set
/// (see [`GROUNDED_STATUSES`]). Ungrounded pending items (NULL, `not_found`,
/// `missing_quote`) are intentionally excluded — they don't block the
/// Ingest button from appearing.
pub async fn count_pending(pool: &PgPool, document_id: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM extraction_items
         WHERE document_id = $1
           AND LOWER(review_status) = 'pending'
           AND grounding_status = ANY($2)",
    )
    .bind(document_id)
    .bind(grounded_statuses_vec())
    .fetch_one(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::pipeline_repository::ExtractionItemRecord;

    // GROUNDED_STATUSES is bound into bulk_approve / count_pending /
    // count_ungrounded_pending via `= ANY($n)`, so asserting membership
    // here is equivalent to asserting the SQL behavior — no SQL-literal
    // sync risk.

    #[test]
    fn test_grounded_statuses_membership() {
        // Routing table: status string → expected_in_set. Each row pins
        // a specific design decision documented in the source tests:
        //
        // - "derived"        → in: v2 Harm entities have grounding_mode=
        //                     derived → grounding_status='derived'. Must
        //                     auto-approve. Was the bug that stranded
        //                     8 Harms on DEV.
        // - "unverified"     → in: schema-mode=none entities get
        //                     'unverified'. Schema said grounding wasn't
        //                     required — must auto-approve.
        // - "manual"         → in: edit_item sets 'manual' when the user
        //                     supplies a page; manually grounded items
        //                     must auto-approve.
        // - "exact"          → in: regression baseline (verbatim hit).
        // - "normalized"     → in: regression baseline (normalized hit).
        // - "not_found"      → out: LLM quote not in PDF — extraction
        //                     failure, user must review.
        // - "missing_quote"  → out: LLM didn't supply a verbatim quote
        //                     at all — user must review.
        // - "derived_invalid"→ out (v5.1 §5.4): items that failed
        //                     provenance validation (missing/empty
        //                     array, dangling reference, or null
        //                     item_data) MUST NOT auto-approve. Roman's
        //                     Q1A explicitly excluded them.
        let cases = [
            ("derived", true),
            ("unverified", true),
            ("manual", true),
            ("exact", true),
            ("normalized", true),
            ("not_found", false),
            ("missing_quote", false),
            ("derived_invalid", false),
        ];
        for (status, expected_in_set) in cases {
            assert_eq!(
                GROUNDED_STATUSES.contains(&status),
                expected_in_set,
                "GROUNDED_STATUSES.contains({status:?}) should be {expected_in_set}"
            );
        }
    }

    #[test]
    fn grounded_statuses_vec_roundtrips_const() {
        // Sanity: the Vec binding helper must expose every const entry.
        let v = grounded_statuses_vec();
        assert_eq!(v.len(), GROUNDED_STATUSES.len());
        for s in GROUNDED_STATUSES {
            assert!(v.iter().any(|x| x == s));
        }
    }

    fn item(
        id: i32,
        grounding_status: Option<&str>,
        grounded_page: Option<i32>,
    ) -> ExtractionItemRecord {
        ExtractionItemRecord {
            id,
            run_id: 1,
            document_id: "doc-x".to_string(),
            entity_type: "Evidence".to_string(),
            item_data: serde_json::json!({}),
            verbatim_quote: Some("a quote".to_string()),
            grounding_status: grounding_status.map(|s| s.to_string()),
            grounded_page,
            review_status: "PENDING".to_string(),
            reviewed_by: None,
            reviewed_at: None,
            review_notes: None,
            graph_status: "pending".to_string(),
            neo4j_node_id: None,
            resolved_entity_type: None,
        }
    }

    #[test]
    fn only_manual_items_are_protected_from_recomputation() {
        let items = vec![
            item(1, Some("manual"), Some(10)),
            item(2, Some("exact"), Some(4)),
            item(3, Some("not_found"), None),
            item(4, None, None),
            item(5, Some("manual"), Some(23)),
        ];
        let protected = manually_grounded_ids(&items);
        assert_eq!(protected.len(), 2);
        assert!(protected.contains(&1) && protected.contains(&5));
        // An ungrounded item is exactly what re-verify is FOR — it must not be
        // protected, or the fix would never reach the items it was built for.
        assert!(!protected.contains(&3));
        assert!(!protected.contains(&2));
        assert!(!protected.contains(&4));
    }

    #[test]
    fn manual_page_is_reported_for_the_log_comparison() {
        let items = vec![
            item(1, Some("manual"), Some(10)),
            item(2, Some("exact"), Some(4)),
        ];
        assert_eq!(manual_page_of(&items, 1), Some(10));
        // An id that is not in the list is None, not a panic: the log line must
        // never be the thing that takes verify down.
        assert_eq!(manual_page_of(&items, 99), None);
    }

    #[test]
    fn a_manual_grounding_is_preserved_and_a_machine_one_is_not() {
        let items = vec![
            item(1, Some("manual"), Some(10)),
            item(2, Some("not_found"), None),
        ];
        let manual = manually_grounded_ids(&items);

        // The human's row is left alone even though the matcher now has an
        // answer for it — that is the whole point.
        assert!(preserve_manual_grounding(&items, &manual, 1, Some(11)));
        // And the ungrounded row, which is what re-verify exists for, is not.
        assert!(!preserve_manual_grounding(&items, &manual, 2, Some(4)));
        // An id nobody knows is not protected either — protection is a positive
        // fact about a row, never a default.
        assert!(!preserve_manual_grounding(&items, &manual, 99, None));
    }
}
