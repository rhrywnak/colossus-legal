//! GET /api/admin/pipeline/documents/:id/report — HTML extraction report.
//!
//! Generates a self-contained HTML page showing all extracted entities
//! with color-coded grounding status, grouped by entity type, plus a
//! relationships table. Designed for human review in a browser.

use std::collections::HashMap;
use std::fmt::Write;

use axum::{extract::Path, extract::State, response::Html};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::pipeline_repository::{self, ExtractionItemRecord};
use crate::state::AppState;

/// GET /api/admin/pipeline/documents/:id/report
pub async fn report_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> Result<Html<String>, AppError> {
    require_admin(&user)?;

    // Fetch all data
    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound { message: format!("Document '{doc_id}' not found") })?;

    let runs = pipeline_repository::get_extraction_runs(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?;

    let items = pipeline_repository::get_all_items(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?;

    let relationships = pipeline_repository::get_all_relationships(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?;

    // Build item_id → label lookup for relationship display
    let item_labels: HashMap<i32, String> = items
        .iter()
        .map(|i| (i.id, item_label(i)))
        .collect();

    // Count grounding statuses
    let (mut exact, mut normalized, mut not_found, mut pending) = (0, 0, 0, 0);
    for item in &items {
        match item.grounding_status.as_deref() {
            Some("exact") => exact += 1,
            Some("normalized") => normalized += 1,
            Some("not_found") => not_found += 1,
            _ => pending += 1,
        }
    }

    // Run metadata
    let (model, input_tok, output_tok, cost) = runs.first().map(|r| {
        (
            r.model_name.as_str(),
            r.input_tokens.unwrap_or(0),
            r.output_tokens.unwrap_or(0),
            r.cost_usd.as_deref().unwrap_or("—"),
        )
    }).unwrap_or(("—", 0, 0, "—"));

    // Build HTML
    let mut html = String::with_capacity(32_000);
    write_header(&mut html, &document.title, &document.id, &document.document_type,
                 &document.status, model, input_tok, output_tok, cost,
                 items.len(), exact, normalized, not_found, pending, relationships.len());

    // Group items by entity_type
    let mut grouped: Vec<(String, Vec<&ExtractionItemRecord>)> = Vec::new();
    let mut current_type = String::new();
    for item in &items {
        if item.entity_type != current_type {
            current_type = item.entity_type.clone();
            grouped.push((current_type.clone(), Vec::new()));
        }
        grouped.last_mut().expect("just pushed").1.push(item);
    }

    for (entity_type, group) in &grouped {
        write_entity_table(&mut html, entity_type, group);
    }

    // Relationships table
    write_relationships_table(&mut html, &relationships, &item_labels);

    html.push_str("</body></html>");
    Ok(Html(html))
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
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

#[allow(clippy::too_many_arguments)]
fn write_header(
    html: &mut String, title: &str, id: &str, doc_type: &str, status: &str,
    model: &str, input_tok: i32, output_tok: i32, cost: &str,
    total: usize, exact: usize, normalized: usize, not_found: usize,
    pending: usize, rel_count: usize,
) {
    let _ = write!(html, r#"<!DOCTYPE html>
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
</style></head><body>
<h1>Extraction Report: {title}</h1>
<div class="meta">
  Document ID: {id} | Type: {doc_type} | Status: {status}<br>
  Model: {model} | Tokens: {input_tok} in / {output_tok} out | Cost: ${cost}
</div>
<div class="summary">
  <div class="summary-card">Total entities<br><strong>{total}</strong></div>
  <div class="summary-card exact">Grounded (exact)<br><strong>{exact}</strong></div>
  <div class="summary-card normalized">Grounded (normalized)<br><strong>{normalized}</strong></div>
  <div class="summary-card not_found">Not found<br><strong>{not_found}</strong></div>
  <div class="summary-card pending">Pending<br><strong>{pending}</strong></div>
  <div class="summary-card">Relationships<br><strong>{rel_count}</strong></div>
</div>
"#, title = esc(title), id = esc(id), doc_type = esc(doc_type), status = esc(status),
    model = esc(model), input_tok = input_tok, output_tok = output_tok, cost = esc(cost),
    total = total, exact = exact, normalized = normalized, not_found = not_found,
    pending = pending, rel_count = rel_count);
}

fn write_entity_table(html: &mut String, entity_type: &str, items: &[&ExtractionItemRecord]) {
    let _ = write!(html, "<h3>{et} ({count})</h3>\n<table>\n<tr><th>ID</th><th>Label</th>\
        <th>Verbatim Quote</th><th>Page</th><th>Grounding</th><th>Review</th></tr>\n",
        et = esc(entity_type), count = items.len());

    for item in items {
        let label = item_label(item);
        let status = item.grounding_status.as_deref().unwrap_or("pending");
        let quote = item.verbatim_quote.as_deref().unwrap_or("—");
        let page = item.grounded_page.map(|p| p.to_string()).unwrap_or_else(|| "—".to_string());
        let _ = writeln!(html,
            "<tr class=\"{cls}\"><td>{id}</td><td>{label}</td>\
             <td class=\"quote\">{quote}</td><td>{page}</td>\
             <td>{status}</td><td>{review}</td></tr>",
            cls = esc(status), id = item.id, label = esc(&label),
            quote = esc(quote), page = page, status = esc(status),
            review = esc(&item.review_status));
    }
    html.push_str("</table>\n");
}

fn write_relationships_table(
    html: &mut String,
    relationships: &[pipeline_repository::ExtractionRelationshipRecord],
    item_labels: &HashMap<i32, String>,
) {
    let _ = write!(html,
        "<h2>Extracted Relationships ({count})</h2>\n<table>\n\
         <tr><th>Type</th><th>From</th><th>To</th><th>Tier</th></tr>\n",
        count = relationships.len());

    let unknown = "???".to_string();
    for rel in relationships {
        let from = item_labels.get(&rel.from_item_id).unwrap_or(&unknown);
        let to = item_labels.get(&rel.to_item_id).unwrap_or(&unknown);
        let _ = writeln!(html,
            "<tr><td>{rt}</td><td>{from}</td><td>{to}</td><td>{tier}</td></tr>",
            rt = esc(&rel.relationship_type), from = esc(from), to = esc(to), tier = rel.tier);
    }
    html.push_str("</table>\n");
}
