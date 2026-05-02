//! GET /api/admin/pipeline/documents/:id/report (HTML)
//! GET /api/admin/pipeline/documents/:id/report.json (JSON)
//!
//! Two endpoints sharing one data path:
//!   - The HTML endpoint generates a self-contained HTML page for
//!     human review in a browser (entity tables, verification cards,
//!     relationship table, plus the Instruction-E additions:
//!     configuration fingerprints, per-pass run metadata,
//!     relationship breakdown by type, limitations).
//!   - The JSON endpoint returns the same `DocumentQualityReport`
//!     struct as JSON for programmatic verification (jq queries,
//!     scripts, automated quality dashboards).
//!
//! Audit reference: AUDIT_PIPELINE_CONFIG_GAPS.md Gap 14.

use std::collections::HashMap;
use std::fmt::Write;

use axum::{extract::Path, extract::State, response::Html, Json};

use crate::api::pipeline::report_data::{
    build_report, ConfigFingerprints, DocumentQualityReport,
};
use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::pipeline_repository::{
    self, ExtractionItemRecord, PerPassRunMetadata, RelationshipTypeCount,
};
use crate::state::AppState;

// ── Constants ──────────────────────────────────────────────────────

/// Number of leading hex characters of a SHA-256 hash to display.
///
/// 8 chars are enough for human eyeballing (matches the Configuration
/// Panel's display from commit `ae9976a`); the full hash is one hover
/// away via the rendered `title` attribute.
const HASH_DISPLAY_LEN: usize = 8;

// ── Handlers ───────────────────────────────────────────────────────

/// GET /api/admin/pipeline/documents/:id/report — HTML page.
///
/// Renders the same `DocumentQualityReport` the JSON endpoint
/// returns, plus the legacy entity tables / relationships table /
/// summary cards. Empty-document case (no extraction has run yet)
/// renders an explanatory message; never 404s on a real document.
pub async fn report_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> Result<Html<String>, AppError> {
    require_admin(&user)?;

    let bundle = fetch_report_bundle(&state, &doc_id).await?;
    let report = build_report_from_bundle(&bundle);

    Ok(Html(render_html(&bundle, &report)))
}

/// GET /api/admin/pipeline/documents/:id/report.json — JSON payload.
///
/// Returns the full `DocumentQualityReport` for programmatic
/// consumption (`curl ... | jq`). Empty-document case returns a
/// valid (deserialisable) report with empty Vecs and `passes: []` —
/// **never 404 on a real document that simply hasn't been processed
/// yet**, per Roman's Step 1 directive.
///
/// Authentication mirrors the HTML handler: `require_admin`. No
/// content-negotiation; the sibling-path convention matches the rest
/// of `/api/admin/pipeline/...`.
pub async fn report_json_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> Result<Json<DocumentQualityReport>, AppError> {
    require_admin(&user)?;

    let bundle = fetch_report_bundle(&state, &doc_id).await?;
    let report = build_report_from_bundle(&bundle);

    Ok(Json(report))
}

// ── Shared data path ───────────────────────────────────────────────

/// Everything a report rendering needs, in one struct so the JSON
/// and HTML handlers fetch the same data the same way.
///
/// Fetched once per request via `fetch_report_bundle`. The HTML
/// renderer uses every field; the JSON renderer uses only what
/// `build_report` needs (`items`, `relationships`, `passes`,
/// `relationship_breakdown`, plus the document scalars). The wasted
/// work for JSON is minimal — a few hundred bytes of relationships
/// list — and not worth a dedicated JSON-only fetch path.
struct ReportBundle {
    document: pipeline_repository::DocumentRecord,
    items: Vec<ExtractionItemRecord>,
    relationships: Vec<pipeline_repository::ExtractionRelationshipRecord>,
    relationship_breakdown: Vec<RelationshipTypeCount>,
    passes: Vec<PerPassRunMetadata>,
}

async fn fetch_report_bundle(
    state: &AppState,
    doc_id: &str,
) -> Result<ReportBundle, AppError> {
    let document = pipeline_repository::get_document(&state.pipeline_pool, doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    let items = pipeline_repository::get_all_items(&state.pipeline_pool, doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?;
    let relationships = pipeline_repository::get_all_relationships(&state.pipeline_pool, doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?;
    let relationship_breakdown =
        pipeline_repository::get_relationship_breakdown_by_type(&state.pipeline_pool, doc_id)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("DB error: {e}"),
            })?;
    let passes = pipeline_repository::get_extraction_runs_with_processing_config(
        &state.pipeline_pool,
        doc_id,
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("DB error: {e}"),
    })?;

    Ok(ReportBundle {
        document,
        items,
        relationships,
        relationship_breakdown,
        passes,
    })
}

fn build_report_from_bundle(bundle: &ReportBundle) -> DocumentQualityReport {
    build_report(
        bundle.document.id.clone(),
        bundle.document.title.clone(),
        bundle.document.document_type.clone(),
        bundle.document.status.clone(),
        &bundle.items,
        bundle.relationships.len() as i64,
        bundle.relationship_breakdown.clone(),
        bundle.passes.clone(),
    )
}

// ── HTML rendering ─────────────────────────────────────────────────

fn render_html(bundle: &ReportBundle, report: &DocumentQualityReport) -> String {
    let mut html = String::with_capacity(48_000);
    write_header(&mut html, bundle, report);

    if report.passes.is_empty() {
        // Empty-document case: friendly message instead of a wall of
        // empty tables. The handler still returned 200 with a valid
        // HTML page (no 404 — the document exists).
        html.push_str(
            r#"<div style="padding: 2rem; text-align: center; color: #666; \
border: 1px solid #ddd; border-radius: 8px; margin: 20px 0;">
<h2>No extraction runs yet</h2>
<p>This document exists in the pipeline but has not been processed. \
Click <strong>Process Document</strong> on the Configuration Panel to start extraction.</p>
</div>"#,
        );
        // Always close the body — the page is structurally complete.
        html.push_str("</body></html>");
        return html;
    }

    // Configuration Fingerprints (new — Gap 11/4 audit surface).
    write_fingerprints_section(&mut html, &report.fingerprints);

    // Per-Pass Run Metadata (new — replaces the prior single-run
    // conflation flagged at audit §7).
    write_per_pass_section(&mut html, &report.passes);

    // Entity-by-type tables (existing).
    let item_labels: HashMap<i32, String> =
        bundle.items.iter().map(|i| (i.id, item_label(i))).collect();
    let mut grouped: Vec<(String, Vec<&ExtractionItemRecord>)> = Vec::new();
    let mut current_type = String::new();
    for item in &bundle.items {
        if item.entity_type != current_type {
            current_type = item.entity_type.clone();
            grouped.push((current_type.clone(), Vec::new()));
        }
        grouped.last_mut().expect("just pushed").1.push(item);
    }
    for (entity_type, group) in &grouped {
        write_entity_table(&mut html, entity_type, group);
    }

    // Relationship Breakdown by Type (new — high-level view above
    // the existing flat table).
    write_relationship_breakdown_section(&mut html, &report.relationship_breakdown);

    // Existing flat relationships table.
    write_relationships_table(&mut html, &bundle.relationships, &item_labels);

    // Limitations (new — explicit-not-silent gaps).
    if !report.limitations.is_empty() {
        write_limitations_section(&mut html, &report.limitations);
    }

    html.push_str("</body></html>");
    html
}

/// Extract a human-readable label from an item's JSON data.
fn item_label(item: &ExtractionItemRecord) -> String {
    item.item_data["label"]
        .as_str()
        .or_else(|| item.item_data["properties"]["party_name"].as_str())
        .or_else(|| item.item_data["properties"]["count_name"].as_str())
        .or_else(|| item.item_data["properties"]["claim_text"].as_str())
        .unwrap_or("—")
        .chars()
        .take(80)
        .collect()
}

/// Escape HTML special characters.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Render a hash for inline display: leading [`HASH_DISPLAY_LEN`]
/// characters wrapped in a `<span>` whose `title` attribute carries
/// the full value for hover. Mirrors the Configuration Panel's
/// truncate-with-tooltip pattern from commit `ae9976a`.
///
/// Returns `"(no hash)"` when the input is empty — explicit, not a
/// silent empty span.
fn render_hash(hash: &str) -> String {
    if hash.is_empty() {
        return "(no hash)".to_string();
    }
    let display = if hash.len() > HASH_DISPLAY_LEN {
        &hash[..HASH_DISPLAY_LEN]
    } else {
        hash
    };
    format!(
        "<span class=\"hash\" title=\"{full}\">{display}</span>",
        full = esc(hash),
        display = esc(display),
    )
}

fn render_optional_hash(hash: Option<&str>) -> String {
    match hash {
        Some(h) => render_hash(h),
        None => "—".to_string(),
    }
}

fn write_header(html: &mut String, bundle: &ReportBundle, report: &DocumentQualityReport) {
    let _ = write!(
        html,
        r#"<!DOCTYPE html>
<html><head><meta charset="utf-8">
<title>Extraction Report: {title}</title>
<style>
body {{ font-family: -apple-system, BlinkMacSystemFont, sans-serif; max-width: 1200px; margin: 0 auto; padding: 20px; }}
table {{ border-collapse: collapse; width: 100%; margin: 20px 0; }}
th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; vertical-align: top; }}
th {{ background: #f5f5f5; }}
.exact {{ background: #d4edda; }}
.normalized {{ background: #fff3cd; }}
.not_found {{ background: #f8d7da; }}
.pending {{ background: #e2e3e5; }}
.summary {{ display: flex; gap: 20px; flex-wrap: wrap; margin: 20px 0; }}
.summary-card {{ padding: 15px; border-radius: 8px; border: 1px solid #ddd; min-width: 120px; }}
.quote {{ font-style: italic; font-size: 0.9em; max-width: 400px; }}
h1 {{ margin-bottom: 5px; }}
.meta {{ color: #666; margin-bottom: 20px; }}
.hash {{ font-family: ui-monospace, Menlo, monospace; padding: 0 0.25rem; background: #f1f5f9; border-radius: 3px; cursor: help; }}
.fingerprints, .per-pass, .relbreakdown, .limitations {{ margin: 24px 0; padding: 16px; border: 1px solid #ddd; border-radius: 8px; }}
.fingerprints h2, .per-pass h2, .relbreakdown h2, .limitations h2 {{ margin-top: 0; }}
.limitations {{ background: #fff3cd; border-color: #facc15; }}
.pass-block {{ border: 1px solid #e2e8f0; border-radius: 6px; padding: 12px; margin: 12px 0; background: #fafbfc; }}
.fingerprint-row {{ margin: 4px 0; }}
.fingerprint-label {{ display: inline-block; min-width: 180px; color: #666; }}
</style></head><body>
<h1>Extraction Report: {title}</h1>
<div class="meta">
  Document ID: {id} | Type: {doc_type} | Status: {status}<br>
  Generated: {generated}
</div>
<div class="summary">
  <div class="summary-card">Total entities<br><strong>{total}</strong></div>
  <div class="summary-card exact">Grounded (exact)<br><strong>{exact}</strong></div>
  <div class="summary-card normalized">Grounded (normalized)<br><strong>{normalized}</strong></div>
  <div class="summary-card not_found">Not found<br><strong>{not_found}</strong></div>
  <div class="summary-card pending">Pending<br><strong>{pending}</strong></div>
  <div class="summary-card">Relationships<br><strong>{rel_count}</strong></div>
</div>
"#,
        title = esc(&bundle.document.title),
        id = esc(&bundle.document.id),
        doc_type = esc(&bundle.document.document_type),
        status = esc(&bundle.document.status),
        generated = report.generated_at.to_rfc3339(),
        total = report.verification.total,
        exact = report.verification.exact,
        normalized = report.verification.normalized,
        not_found = report.verification.not_found,
        pending = report.verification.pending,
        rel_count = report.total_relationship_count,
    );
}

fn write_fingerprints_section(html: &mut String, fp: &ConfigFingerprints) {
    let _ = write!(
        html,
        "<div class=\"fingerprints\">\n<h2>Configuration Fingerprints</h2>\n"
    );
    write_fp_row(html, "profile_name", fp.profile_name.as_deref());
    write_fp_hash_row(html, "profile_hash", fp.profile_hash.as_deref());
    write_fp_row(html, "pass1_template_file", fp.pass1_template_file.as_deref());
    write_fp_hash_row(html, "pass1_template_hash", fp.pass1_template_hash.as_deref());
    write_fp_row(html, "pass2_template_file", fp.pass2_template_file.as_deref());
    write_fp_hash_row(html, "pass2_template_hash", fp.pass2_template_hash.as_deref());
    write_fp_row(html, "global_rules_file", fp.global_rules_file.as_deref());
    write_fp_hash_row(html, "global_rules_hash", fp.global_rules_hash.as_deref());
    write_fp_row(html, "schema_file", fp.schema_file.as_deref());
    html.push_str("</div>\n");
}

fn write_fp_row(html: &mut String, label: &str, value: Option<&str>) {
    let _ = writeln!(
        html,
        "<div class=\"fingerprint-row\"><span class=\"fingerprint-label\">{label}:</span> {value}</div>",
        label = esc(label),
        value = match value {
            Some(v) => esc(v),
            None => "—".to_string(),
        },
    );
}

fn write_fp_hash_row(html: &mut String, label: &str, value: Option<&str>) {
    let _ = writeln!(
        html,
        "<div class=\"fingerprint-row\"><span class=\"fingerprint-label\">{label}:</span> {value}</div>",
        label = esc(label),
        value = render_optional_hash(value),
    );
}

fn write_per_pass_section(html: &mut String, passes: &[PerPassRunMetadata]) {
    let _ = write!(html, "<div class=\"per-pass\">\n<h2>Per-Pass Run Metadata</h2>\n");
    for pass in passes {
        let _ = write!(
            html,
            "<div class=\"pass-block\">\n<h3>Pass {pass_no}</h3>\n",
            pass_no = pass.pass_number,
        );
        write_fp_row(html, "model", Some(pass.model_name.as_str()));
        write_fp_row(
            html,
            "input_tokens",
            pass.input_tokens.map(|n| n.to_string()).as_deref(),
        );
        write_fp_row(
            html,
            "output_tokens",
            pass.output_tokens.map(|n| n.to_string()).as_deref(),
        );
        write_fp_row(html, "cost_usd", pass.cost_usd.as_deref());
        write_fp_row(html, "status", Some(pass.status.as_str()));
        write_fp_row(
            html,
            "started_at",
            Some(pass.started_at.to_rfc3339().as_str()),
        );
        write_fp_row(
            html,
            "completed_at",
            pass.completed_at.map(|d| d.to_rfc3339()).as_deref(),
        );
        write_fp_row(html, "template_file", pass.template_file.as_deref());
        write_fp_hash_row(html, "template_hash", pass.template_hash.as_deref());
        write_fp_row(html, "system_prompt_file", pass.system_prompt_file.as_deref());
        write_fp_hash_row(html, "system_prompt_hash", pass.system_prompt_hash.as_deref());
        if pass.pass_number == 2 {
            write_fp_row(
                html,
                "pass2_cross_doc_entity_count",
                Some(pass.pass2_cross_doc_entity_count.to_string().as_str()),
            );
            write_fp_row(
                html,
                "pass2_source_document_count",
                Some(pass.pass2_source_document_count.to_string().as_str()),
            );
            // Source doc IDs as a comma-joined list — short docs in
            // this codebase, no need for a separate table.
            let ids = pass.pass2_source_document_ids.join(", ");
            let value = if ids.is_empty() { None } else { Some(ids) };
            write_fp_row(html, "pass2_source_document_ids", value.as_deref());
        }
        if let Some(err) = &pass.parse_error {
            // Surface the parse-failure inline on the pass block too —
            // operators looking at this pass should see the degraded
            // state immediately, not just down in Limitations.
            let _ = writeln!(
                html,
                "<div class=\"fingerprint-row\" style=\"color: #b45309;\">⚠ processing_config parse failed: {}</div>",
                esc(err)
            );
        }
        html.push_str("</div>\n");
    }
    html.push_str("</div>\n");
}

fn write_relationship_breakdown_section(
    html: &mut String,
    breakdown: &[RelationshipTypeCount],
) {
    let total: i64 = breakdown.iter().map(|r| r.count).sum();
    let _ = write!(
        html,
        "<div class=\"relbreakdown\">\n<h2>Relationships by Type ({total})</h2>\n<table>\n<tr><th>Type</th><th>Count</th></tr>\n"
    );
    for row in breakdown {
        let _ = writeln!(
            html,
            "<tr><td>{rt}</td><td>{count}</td></tr>",
            rt = esc(&row.relationship_type),
            count = row.count,
        );
    }
    html.push_str("</table>\n</div>\n");
}

fn write_limitations_section(html: &mut String, limitations: &[String]) {
    html.push_str("<div class=\"limitations\">\n<h2>Report Limitations</h2>\n<ul>\n");
    for entry in limitations {
        let _ = writeln!(html, "<li>{}</li>", esc(entry));
    }
    html.push_str("</ul>\n</div>\n");
}

fn write_entity_table(html: &mut String, entity_type: &str, items: &[&ExtractionItemRecord]) {
    let _ = write!(
        html,
        "<h3>{et} ({count})</h3>\n<table>\n<tr><th>ID</th><th>Label</th>\
        <th>Verbatim Quote</th><th>Page</th><th>Grounding</th><th>Review</th></tr>\n",
        et = esc(entity_type),
        count = items.len()
    );

    for item in items {
        let label = item_label(item);
        let status = item.grounding_status.as_deref().unwrap_or("pending");
        let quote = item.verbatim_quote.as_deref().unwrap_or("—");
        let page = item
            .grounded_page
            .map(|p| p.to_string())
            .unwrap_or_else(|| "—".to_string());
        let _ = writeln!(
            html,
            "<tr class=\"{cls}\"><td>{id}</td><td>{label}</td>\
             <td class=\"quote\">{quote}</td><td>{page}</td>\
             <td>{status}</td><td>{review}</td></tr>",
            cls = esc(status),
            id = item.id,
            label = esc(&label),
            quote = esc(quote),
            page = page,
            status = esc(status),
            review = esc(&item.review_status)
        );
    }
    html.push_str("</table>\n");
}

fn write_relationships_table(
    html: &mut String,
    relationships: &[pipeline_repository::ExtractionRelationshipRecord],
    item_labels: &HashMap<i32, String>,
) {
    let _ = write!(
        html,
        "<h2>Extracted Relationships ({count})</h2>\n<table>\n\
         <tr><th>Type</th><th>From</th><th>To</th><th>Tier</th></tr>\n",
        count = relationships.len()
    );

    let unknown = "???".to_string();
    for rel in relationships {
        let from = item_labels.get(&rel.from_item_id).unwrap_or(&unknown);
        let to = item_labels.get(&rel.to_item_id).unwrap_or(&unknown);
        let _ = writeln!(
            html,
            "<tr><td>{rt}</td><td>{from}</td><td>{to}</td><td>{tier}</td></tr>",
            rt = esc(&rel.relationship_type),
            from = esc(from),
            to = esc(to),
            tier = rel.tier
        );
    }
    html.push_str("</table>\n");
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::pipeline_repository::PerPassRunMetadata;

    fn empty_pass(pass_number: i32) -> PerPassRunMetadata {
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
            profile_hash: Some(format!(
                "p{pass_number}_profile_hash_with_extra_chars_for_truncation_check"
            )),
            template_file: Some(format!("pass{pass_number}_template.md")),
            template_hash: Some(format!("p{pass_number}_template_hash_extra_chars")),
            system_prompt_file: Some("legal_extraction_system.md".into()),
            system_prompt_hash: Some(format!("p{pass_number}_sys_hash_extra_chars")),
            global_rules_file: Some("global_rules_v4.md".into()),
            global_rules_hash: Some(format!("p{pass_number}_rules_hash_extra_chars")),
            schema_file: Some("complaint_v4.yaml".into()),
            pass2_cross_doc_entity_count: 0,
            pass2_source_document_count: 0,
            pass2_source_document_ids: Vec::new(),
            parse_error: None,
        }
    }

    fn empty_doc() -> pipeline_repository::DocumentRecord {
        pipeline_repository::DocumentRecord {
            id: "doc-x".into(),
            title: "Doc X".into(),
            file_path: "/tmp/x.pdf".into(),
            file_hash: "deadbeef".into(),
            document_type: "complaint".into(),
            status: "PUBLISHED".into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            assigned_reviewer: None,
            assigned_at: None,
            total_cost_usd: None,
            has_failed_steps: false,
            processing_step: None,
            processing_step_label: None,
            chunks_total: None,
            chunks_processed: None,
            entities_found: None,
            percent_complete: None,
            failed_step: None,
            failed_chunk: None,
            error_message: None,
            error_suggestion: None,
            is_cancelled: false,
            entities_written: None,
            entities_flagged: None,
            relationships_written: None,
            model_name: None,
            run_chunk_count: None,
            run_chunks_succeeded: None,
            run_chunks_failed: None,
            content_type: None,
            page_count: None,
            text_pages: None,
            scanned_pages: None,
            pages_needing_ocr: None,
            total_chars: None,
            mime_type: None,
            original_format: None,
        }
    }

    /// Empty-extraction case: HTML renders the friendly "no runs"
    /// message and does NOT include the per-pass / fingerprints /
    /// limitations sections. Per Roman's Step 1 directive A.
    #[test]
    fn html_empty_document_renders_no_runs_message() {
        let bundle = ReportBundle {
            document: empty_doc(),
            items: Vec::new(),
            relationships: Vec::new(),
            relationship_breakdown: Vec::new(),
            passes: Vec::new(),
        };
        let report = build_report_from_bundle(&bundle);
        let html = render_html(&bundle, &report);
        assert!(
            html.contains("No extraction runs yet"),
            "empty-document HTML must include the friendly no-runs message"
        );
        assert!(
            !html.contains("Configuration Fingerprints"),
            "empty-document HTML must NOT include the fingerprints section"
        );
        assert!(
            !html.contains("Per-Pass Run Metadata"),
            "empty-document HTML must NOT include the per-pass section"
        );
        assert!(
            !html.contains("Report Limitations"),
            "empty-document HTML must NOT include the limitations section"
        );
        // Page is structurally complete.
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.ends_with("</body></html>"));
    }

    /// HTML truncates hashes to 8 chars and embeds the full hash in
    /// a `title` attribute for hover (matches the Configuration
    /// Panel's pattern from commit `ae9976a`).
    #[test]
    fn html_truncates_hash_to_eight_chars_with_full_in_title() {
        let bundle = ReportBundle {
            document: empty_doc(),
            items: Vec::new(),
            relationships: Vec::new(),
            relationship_breakdown: Vec::new(),
            passes: vec![empty_pass(1)],
        };
        let report = build_report_from_bundle(&bundle);
        let html = render_html(&bundle, &report);
        // The pass-1 profile_hash starts with "p1_profi" (8 chars)
        // and the full value contains "extra_chars_for_truncation".
        assert!(
            html.contains("title=\"p1_profile_hash_with_extra_chars_for_truncation_check\""),
            "full hash must be in the title attribute for hover"
        );
        assert!(
            html.contains(">p1_profi<"),
            "displayed hash must be exactly the 8-char prefix"
        );
        assert!(
            !html.contains("p1_profile_hash_with_extra_chars_for_truncation_check<"),
            "the full hash must NOT appear as visible text — only in the title"
        );
    }

    /// Limitations section is absent when the pass parses cleanly.
    #[test]
    fn html_omits_limitations_section_when_no_parse_errors() {
        let bundle = ReportBundle {
            document: empty_doc(),
            items: Vec::new(),
            relationships: Vec::new(),
            relationship_breakdown: Vec::new(),
            passes: vec![empty_pass(1)],
        };
        let report = build_report_from_bundle(&bundle);
        let html = render_html(&bundle, &report);
        assert!(
            !html.contains("Report Limitations"),
            "Limitations section must be omitted when limitations is empty"
        );
    }

    /// Limitations section appears when at least one pass parse
    /// failed. The per-pass block also surfaces an inline ⚠ warning.
    #[test]
    fn html_renders_limitations_section_when_parse_error_present() {
        let mut bad_pass = empty_pass(2);
        bad_pass.parse_error = Some("simulated failure".into());
        bad_pass.profile_hash = None;
        bad_pass.template_hash = None;
        let bundle = ReportBundle {
            document: empty_doc(),
            items: Vec::new(),
            relationships: Vec::new(),
            relationship_breakdown: Vec::new(),
            passes: vec![empty_pass(1), bad_pass],
        };
        let report = build_report_from_bundle(&bundle);
        let html = render_html(&bundle, &report);
        assert!(html.contains("Report Limitations"));
        assert!(html.contains("Pass-2 processing_config could not be parsed"));
        assert!(html.contains("simulated failure"));
        // Inline pass-block warning too.
        assert!(html.contains("⚠ processing_config parse failed"));
    }

    /// The new section headers are present in the HTML for a real
    /// (non-empty) document.
    #[test]
    fn html_renders_new_sections_for_a_real_document() {
        let bundle = ReportBundle {
            document: empty_doc(),
            items: Vec::new(),
            relationships: Vec::new(),
            relationship_breakdown: Vec::new(),
            passes: vec![empty_pass(1)],
        };
        let report = build_report_from_bundle(&bundle);
        let html = render_html(&bundle, &report);
        assert!(html.contains("Configuration Fingerprints"));
        assert!(html.contains("Per-Pass Run Metadata"));
        assert!(html.contains("Relationships by Type"));
    }

    /// `render_hash` returns the explicit "(no hash)" sentinel for
    /// an empty input — never a silent empty string. (No silent fails.)
    #[test]
    fn render_hash_explicit_sentinel_for_empty_input() {
        assert_eq!(render_hash(""), "(no hash)");
    }

    /// `render_hash` truncates correctly even when the hash is
    /// shorter than HASH_DISPLAY_LEN (no panic on slice).
    #[test]
    fn render_hash_does_not_panic_on_short_input() {
        let html = render_hash("abc");
        assert!(html.contains(">abc<"));
        assert!(html.contains("title=\"abc\""));
    }
}
