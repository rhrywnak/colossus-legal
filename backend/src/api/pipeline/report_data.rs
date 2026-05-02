//! Quality-report data shape and pure construction logic.
//!
//! This module owns the `DocumentQualityReport` struct that the JSON
//! endpoint returns and the HTML endpoint reads when building its
//! sections. Lifting the construction into a pure function (`build_report`)
//! lets both endpoints share one source of truth and lets the
//! construction logic be unit-tested without a database.
//!
//! Audit reference: AUDIT_PIPELINE_CONFIG_GAPS.md Gap 14.
//!
//! ## Empty-document case
//!
//! A document that exists but has no extraction runs yet is a valid
//! input to `build_report`: the result has `passes: []`,
//! `entity_breakdown: []`, `relationship_breakdown: []`, and totals
//! of 0. The HTML handler renders a friendly "No extraction runs
//! yet" message instead of empty tables; the JSON handler returns
//! the empty-but-shaped report. **The handler must NOT 404 on this
//! case** — the document exists; nothing has run on it.
//!
//! ## Per-pass parse-failure handling
//!
//! Each `PerPassRunMetadata` row carries a `parse_error: Option<String>`
//! field set by the repo helper when a row's `processing_config`
//! JSONB couldn't be deserialised into `ResolvedConfig`. The builder
//! folds those errors into the report's `limitations` field — so the
//! report still builds and the operator sees an explicit "we
//! couldn't read the audit body for Pass-N" notice rather than a
//! silent omission. Per Roman's Step 1 directive: no silent fails;
//! degrade explicitly with a recorded reason.

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::repositories::pipeline_repository::{
    ExtractionItemRecord, PerPassRunMetadata, RelationshipTypeCount,
};

/// The full quality-report payload for one document.
///
/// Returned as JSON by the new `/report.json` endpoint and consumed
/// by the existing HTML `/report` endpoint to render its sections.
/// Field set is fixed (i.e. additive-only changes from here forward
/// — the JSON shape is part of the API surface).
#[derive(Debug, Serialize)]
pub struct DocumentQualityReport {
    pub document_id: String,
    pub document_label: String,
    pub document_type: String,
    pub status: String,

    pub total_entity_count: i64,
    pub total_relationship_count: i64,

    pub entity_breakdown: Vec<EntityTypeCount>,
    pub relationship_breakdown: Vec<RelationshipTypeCount>,

    pub verification: VerificationRates,

    /// Per-pass run metadata. Empty when no extraction has run.
    pub passes: Vec<PerPassRunMetadata>,

    /// Configuration fingerprints derived from Pass-1 (or the only
    /// pass that ran). Pass-2-specific fingerprints are on the
    /// per-pass entries inside `passes`.
    pub fingerprints: ConfigFingerprints,

    /// Schema-attribution caveats and per-pass parse failures the
    /// builder couldn't fully answer. Empty Vec when nothing was
    /// missing or malformed. See module doc-comment for the no-silent-
    /// fails rationale.
    pub limitations: Vec<String>,

    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct EntityTypeCount {
    pub entity_type: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct VerificationRates {
    pub total: i64,
    pub exact: i64,
    pub normalized: i64,
    pub not_found: i64,
    pub pending: i64,
}

/// Top-level configuration fingerprints. Sourced from the Pass-1
/// row's `processing_config` (Pass-1 is authoritative for these
/// scalar fields — same profile / global rules / schema apply to
/// both passes; per-pass `template_file` and `template_hash` differ
/// and live on each `PerPassRunMetadata` instead).
#[derive(Debug, Serialize)]
pub struct ConfigFingerprints {
    pub profile_name: Option<String>,
    pub profile_hash: Option<String>,
    pub pass1_template_file: Option<String>,
    pub pass1_template_hash: Option<String>,
    pub pass2_template_file: Option<String>,
    pub pass2_template_hash: Option<String>,
    pub global_rules_file: Option<String>,
    pub global_rules_hash: Option<String>,
    pub schema_file: Option<String>,
}

/// Aggregate the per-source data into a `DocumentQualityReport`.
///
/// Pure function: takes only data, returns only data. All side
/// effects (DB queries, HTTP) happen at the handler layer above this.
/// That's what makes the empty-document and parse-failure cases
/// unit-testable without a DB or HTTP fixture.
///
/// `verification` counts come from the `items` slice — the rule is
/// `grounding_status` mapped to a known bucket, with
/// `pending` as the catch-all (matches the existing HTML handler's
/// behaviour at `report.rs:56-65`).
///
/// `limitations` accumulates one entry per `pass.parse_error` so the
/// report records *which* pass had a malformed snapshot.
///
/// 8 parameters trips clippy::too_many_arguments. Each carries a
/// distinct, orthogonal concern (document scalars, item slice, count,
/// breakdown, passes); grouping them into a struct just to satisfy
/// the lint would shift the verbosity to the call site without
/// making it clearer. Same pattern as `insert_extraction_run` in
/// `extraction.rs:112`.
#[allow(clippy::too_many_arguments)]
pub fn build_report(
    document_id: String,
    document_label: String,
    document_type: String,
    status: String,
    items: &[ExtractionItemRecord],
    relationship_count: i64,
    relationship_breakdown: Vec<RelationshipTypeCount>,
    passes: Vec<PerPassRunMetadata>,
) -> DocumentQualityReport {
    // Verification rates — same buckets as the existing HTML report.
    let mut exact: i64 = 0;
    let mut normalized: i64 = 0;
    let mut not_found: i64 = 0;
    let mut pending: i64 = 0;
    for item in items {
        match item.grounding_status.as_deref() {
            Some("exact") => exact += 1,
            Some("normalized") => normalized += 1,
            Some("not_found") => not_found += 1,
            _ => pending += 1,
        }
    }

    // Entity breakdown — group items by entity_type, count per group.
    // We iterate the slice once to build a HashMap then sort the result
    // so the JSON shape is stable across runs (same input → same order).
    use std::collections::HashMap;
    let mut by_type: HashMap<&str, i64> = HashMap::new();
    for item in items {
        *by_type.entry(item.entity_type.as_str()).or_insert(0) += 1;
    }
    let mut entity_breakdown: Vec<EntityTypeCount> = by_type
        .into_iter()
        .map(|(k, v)| EntityTypeCount {
            entity_type: k.to_string(),
            count: v,
        })
        .collect();
    // Sort: count desc, then entity_type asc (stable alphabetical
    // tiebreak). Mirrors `get_relationship_breakdown_by_type`'s SQL
    // sort so JSON consumers can rely on a single ordering convention
    // across both breakdown shapes.
    entity_breakdown.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.entity_type.cmp(&b.entity_type))
    });

    // Limitations — one entry per pass that failed to deserialise its
    // processing_config snapshot. Explicit, per-pass, named — never
    // silent. See module doc-comment for the no-silent-fails rationale.
    let mut limitations: Vec<String> = Vec::new();
    for pass in &passes {
        if let Some(err) = &pass.parse_error {
            limitations.push(format!(
                "Pass-{} processing_config could not be parsed: {err}",
                pass.pass_number
            ));
        }
    }

    // Top-level fingerprints — pulled from the Pass-1 row when present
    // (Pass-1 is authoritative for the document-level scalars). When
    // only Pass-2 exists (uncommon — Pass-2 strictly depends on
    // Pass-1), fall back to it. When neither exists (no extraction
    // yet), every fingerprint is None.
    let pass1 = passes.iter().find(|p| p.pass_number == 1);
    let pass2 = passes.iter().find(|p| p.pass_number == 2);
    let primary = pass1.or(pass2);

    let fingerprints = ConfigFingerprints {
        profile_name: primary.and_then(|p| p.profile_name.clone()),
        profile_hash: primary.and_then(|p| p.profile_hash.clone()),
        // Pass-1 template (authoritative). When only Pass-2 exists,
        // the Pass-1 template fields are None — operator can see the
        // Pass-2 template on the per-pass entry.
        pass1_template_file: pass1.and_then(|p| p.template_file.clone()),
        pass1_template_hash: pass1.and_then(|p| p.template_hash.clone()),
        // Pass-2 template — straight from the Pass-2 row's snapshot
        // (Instruction B's effective_pass=2 swap recorded the Pass-2
        // template_file there).
        pass2_template_file: pass2.and_then(|p| p.template_file.clone()),
        pass2_template_hash: pass2.and_then(|p| p.template_hash.clone()),
        global_rules_file: primary.and_then(|p| p.global_rules_file.clone()),
        global_rules_hash: primary.and_then(|p| p.global_rules_hash.clone()),
        schema_file: primary.and_then(|p| p.schema_file.clone()),
    };

    DocumentQualityReport {
        document_id,
        document_label,
        document_type,
        status,
        total_entity_count: items.len() as i64,
        total_relationship_count: relationship_count,
        entity_breakdown,
        relationship_breakdown,
        verification: VerificationRates {
            total: items.len() as i64,
            exact,
            normalized,
            not_found,
            pending,
        },
        passes,
        fingerprints,
        limitations,
        generated_at: Utc::now(),
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_pass(pass_number: i32, with_parse_error: bool) -> PerPassRunMetadata {
        PerPassRunMetadata {
            pass_number,
            model_name: format!("model-p{pass_number}"),
            input_tokens: Some(100),
            output_tokens: Some(200),
            cost_usd: Some("0.0327".into()),
            status: "COMPLETED".into(),
            started_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            effective_pass: Some(pass_number as u8),
            profile_name: Some("complaint".into()),
            profile_hash: Some(format!("p{pass_number}_profile_hash")),
            template_file: Some(format!("pass{pass_number}_template.md")),
            template_hash: Some(format!("p{pass_number}_template_hash")),
            system_prompt_file: Some("legal_extraction_system.md".into()),
            system_prompt_hash: Some(format!("p{pass_number}_sys_hash")),
            global_rules_file: Some("global_rules_v4.md".into()),
            global_rules_hash: Some(format!("p{pass_number}_rules_hash")),
            schema_file: Some("complaint_v4.yaml".into()),
            pass2_cross_doc_entity_count: 0,
            pass2_source_document_count: 0,
            pass2_source_document_ids: Vec::new(),
            parse_error: if with_parse_error {
                Some("simulated parse failure".into())
            } else {
                None
            },
        }
    }

    fn item(entity_type: &str, grounding: Option<&str>) -> ExtractionItemRecord {
        ExtractionItemRecord {
            id: 0,
            run_id: 0,
            document_id: "doc-x".into(),
            entity_type: entity_type.into(),
            item_data: serde_json::json!({}),
            verbatim_quote: None,
            grounding_status: grounding.map(str::to_string),
            grounded_page: None,
            review_status: "pending".into(),
            reviewed_by: None,
            reviewed_at: None,
            review_notes: None,
            graph_status: "pending".into(),
            neo4j_node_id: None,
            resolved_entity_type: None,
        }
    }

    /// Empty-document case: no extraction has run. Builder must
    /// produce a valid report with empty Vecs and zero totals.
    /// Per Roman's Step 1 directive: no 404 — the document exists.
    #[test]
    fn build_report_empty_document_yields_valid_empty_report() {
        let report = build_report(
            "doc-x".into(),
            "Doc X".into(),
            "complaint".into(),
            "NEW".into(),
            &[],
            0,
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(report.total_entity_count, 0);
        assert_eq!(report.total_relationship_count, 0);
        assert!(report.entity_breakdown.is_empty());
        assert!(report.relationship_breakdown.is_empty());
        assert!(report.passes.is_empty());
        assert!(report.limitations.is_empty());
        // Fingerprints all None when no pass exists.
        assert!(report.fingerprints.profile_name.is_none());
        assert!(report.fingerprints.profile_hash.is_none());
        assert!(report.fingerprints.pass1_template_file.is_none());
        assert!(report.fingerprints.pass2_template_file.is_none());
    }

    #[test]
    fn build_report_both_passes_populates_pass1_and_pass2_fingerprints() {
        let passes = vec![empty_pass(1, false), empty_pass(2, false)];
        let report = build_report(
            "doc-x".into(),
            "Doc X".into(),
            "complaint".into(),
            "PUBLISHED".into(),
            &[],
            0,
            Vec::new(),
            passes,
        );
        assert_eq!(report.passes.len(), 2);
        // profile_name / profile_hash come from Pass-1 (authoritative).
        assert_eq!(report.fingerprints.profile_name.as_deref(), Some("complaint"));
        assert_eq!(
            report.fingerprints.profile_hash.as_deref(),
            Some("p1_profile_hash")
        );
        // Per-pass template fingerprints split correctly.
        assert_eq!(
            report.fingerprints.pass1_template_file.as_deref(),
            Some("pass1_template.md")
        );
        assert_eq!(
            report.fingerprints.pass2_template_file.as_deref(),
            Some("pass2_template.md")
        );
        assert_eq!(
            report.fingerprints.pass1_template_hash.as_deref(),
            Some("p1_template_hash")
        );
        assert_eq!(
            report.fingerprints.pass2_template_hash.as_deref(),
            Some("p2_template_hash")
        );
        // No parse errors → no limitations entries.
        assert!(report.limitations.is_empty());
    }

    #[test]
    fn build_report_pass1_only_pass2_template_is_none() {
        let passes = vec![empty_pass(1, false)];
        let report = build_report(
            "doc-x".into(),
            "Doc X".into(),
            "complaint".into(),
            "PUBLISHED".into(),
            &[],
            0,
            Vec::new(),
            passes,
        );
        assert_eq!(report.passes.len(), 1);
        assert_eq!(
            report.fingerprints.pass1_template_file.as_deref(),
            Some("pass1_template.md")
        );
        // Pass-2 absent → Pass-2 template fields are None.
        assert!(report.fingerprints.pass2_template_file.is_none());
        assert!(report.fingerprints.pass2_template_hash.is_none());
    }

    /// Roman's Step 1 directive B: per-pass parse failure must
    /// degrade gracefully. Pass-1 OK, Pass-2 parse-failed → report
    /// builds, Pass-2's fingerprints are None, limitations contains
    /// an explicit named entry.
    #[test]
    fn build_report_records_pass2_parse_failure_in_limitations() {
        // The empty_pass helper sets all fingerprint fields to Some
        // even when with_parse_error=true (matches the production
        // helper's "everything stays Some until I overwrite it"
        // shape). For this test we override pass-2 to mirror the
        // real degraded-row state: parse_error set, fingerprints all
        // None.
        let mut bad_pass2 = empty_pass(2, true);
        bad_pass2.profile_name = None;
        bad_pass2.profile_hash = None;
        bad_pass2.template_file = None;
        bad_pass2.template_hash = None;
        bad_pass2.global_rules_file = None;
        bad_pass2.global_rules_hash = None;
        bad_pass2.schema_file = None;
        bad_pass2.system_prompt_file = None;
        bad_pass2.system_prompt_hash = None;
        bad_pass2.effective_pass = None;
        let passes = vec![empty_pass(1, false), bad_pass2];

        let report = build_report(
            "doc-x".into(),
            "Doc X".into(),
            "complaint".into(),
            "PUBLISHED".into(),
            &[],
            0,
            Vec::new(),
            passes,
        );
        // Report still built (no panic, no early return).
        assert_eq!(report.passes.len(), 2);
        // Pass-1 fingerprints survive.
        assert_eq!(
            report.fingerprints.pass1_template_file.as_deref(),
            Some("pass1_template.md")
        );
        // Pass-2 template is None — the row degraded.
        assert!(report.fingerprints.pass2_template_file.is_none());
        // Limitations has exactly one entry naming Pass-2.
        assert_eq!(report.limitations.len(), 1);
        assert!(
            report.limitations[0].contains("Pass-2"),
            "limitations entry must name the pass; got: {}",
            report.limitations[0]
        );
        assert!(
            report.limitations[0].contains("simulated parse failure"),
            "limitations entry must surface the underlying error message"
        );
    }

    #[test]
    fn build_report_entity_breakdown_sorted_count_desc_then_alpha_asc() {
        let items = vec![
            item("Party", Some("exact")),
            item("Party", Some("exact")),
            item("Party", Some("normalized")),
            item("Evidence", Some("exact")),
            item("LegalCount", None), // pending
            item("Citation", None),   // pending
            item("Citation", None),
        ];
        let report = build_report(
            "doc-x".into(),
            "Doc X".into(),
            "complaint".into(),
            "PUBLISHED".into(),
            &items,
            0,
            Vec::new(),
            Vec::new(),
        );
        // Counts: Party=3, Citation=2, Evidence=1, LegalCount=1.
        // Sort: count desc → Party, Citation, then ties on 1
        //       broken alphabetically → Evidence, LegalCount.
        let names: Vec<&str> = report
            .entity_breakdown
            .iter()
            .map(|e| e.entity_type.as_str())
            .collect();
        assert_eq!(names, vec!["Party", "Citation", "Evidence", "LegalCount"]);
    }

    #[test]
    fn build_report_verification_rates_count_correctly() {
        let items = vec![
            item("Party", Some("exact")),
            item("Party", Some("exact")),
            item("Party", Some("normalized")),
            item("Party", Some("not_found")),
            item("Party", None), // pending (None grounding_status)
            item("Party", Some("unknown_bucket")), // also pending (catch-all)
        ];
        let report = build_report(
            "doc-x".into(),
            "Doc X".into(),
            "complaint".into(),
            "PUBLISHED".into(),
            &items,
            0,
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(report.verification.total, 6);
        assert_eq!(report.verification.exact, 2);
        assert_eq!(report.verification.normalized, 1);
        assert_eq!(report.verification.not_found, 1);
        assert_eq!(report.verification.pending, 2);
    }

    /// JSON round-trip: the report serialises to JSON and the JSON
    /// has the documented top-level field names. Pins the API shape.
    #[test]
    fn build_report_serialises_to_documented_json_shape() {
        let report = build_report(
            "doc-x".into(),
            "Doc X".into(),
            "complaint".into(),
            "PUBLISHED".into(),
            &[],
            0,
            Vec::new(),
            Vec::new(),
        );
        let json = serde_json::to_value(&report).expect("must serialise");
        for key in [
            "document_id",
            "document_label",
            "document_type",
            "status",
            "total_entity_count",
            "total_relationship_count",
            "entity_breakdown",
            "relationship_breakdown",
            "verification",
            "passes",
            "fingerprints",
            "limitations",
            "generated_at",
        ] {
            assert!(json.get(key).is_some(), "JSON missing top-level key: {key}");
        }
        for key in [
            "profile_name",
            "profile_hash",
            "pass1_template_file",
            "pass1_template_hash",
            "pass2_template_file",
            "pass2_template_hash",
            "global_rules_file",
            "global_rules_hash",
            "schema_file",
        ] {
            assert!(
                json["fingerprints"].get(key).is_some(),
                "fingerprints JSON missing key: {key}"
            );
        }
    }
}

