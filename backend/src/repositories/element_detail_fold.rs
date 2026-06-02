//! Row-folding for the Element detail read.
//!
//! `element_detail_repository.rs` owns the endpoint's DTOs, Cypher, SQL, and the
//! `fetch_element_with_allegations` orchestration. The per-row decoding and the
//! fan-out fold live here so neither file crosses the 300-line module limit
//! (Rule 17) — the same split `causes_of_action_builder.rs` makes against
//! `causes_of_action_repository.rs` (pure shaping in its own file).
//!
//! The detail Cypher emits one row per `(Element × Allegation × Evidence ×
//! Document)` tuple. [`DetailFold`] collapses that stream: the Element header is
//! captured once, Allegations are deduped by id, and each Allegation's
//! corroborating Evidence is collected (deduped by evidence id).

use std::collections::HashMap;

use super::element_detail_repository::{
    source_section_for, AllegationSummary, ElementDetailRepoError, EvidenceRef,
};

/// Element + parent-Count columns captured from the first decoded row. A named
/// struct (not a tuple) keeps the call site readable and clippy's
/// `type_complexity` lint quiet. Fields are `pub(crate)` so the orchestrating
/// `fetch_element_with_allegations` can read them when assembling the response.
pub(crate) struct ElementHeader {
    pub(crate) element_id: String,
    pub(crate) element_name: String,
    pub(crate) what_plaintiff_must_prove: String,
    pub(crate) order_in_count: Option<i64>,
    pub(crate) count_number: Option<i64>,
    pub(crate) count_name: Option<String>,
}

/// Decode helper: maps neo4rs row-decode errors into the typed variant.
///
/// ## Rust Learning: returning `impl Fn(_) -> _`
///
/// `decode_err("op")` returns a closure that captures the operation name. Used
/// as `row.get("x").map_err(decode_err(OP))` so every column decode reuses the
/// same captured context without restating it inline. The `move` keeps the
/// captured `&'static str` inside the returned closure.
fn decode_err(operation: &'static str) -> impl Fn(neo4rs::DeError) -> ElementDetailRepoError {
    move |source| ElementDetailRepoError::Neo4jDecode { operation, source }
}

/// Decode the Element + parent-Count columns from a row. The required Element
/// properties decode as non-`Option` — a missing value here is a data-shape
/// bug, not a recoverable state, so its decode error propagates as a 500.
fn decode_header(
    row: &neo4rs::Row,
    op: &'static str,
) -> Result<ElementHeader, ElementDetailRepoError> {
    Ok(ElementHeader {
        element_id: row.get("element_id").map_err(decode_err(op))?,
        element_name: row.get("element_name").map_err(decode_err(op))?,
        what_plaintiff_must_prove: row
            .get("what_plaintiff_must_prove")
            .map_err(decode_err(op))?,
        order_in_count: row.get("order_in_count").map_err(decode_err(op))?,
        count_number: row.get("count_number").map_err(decode_err(op))?,
        count_name: row.get("count_name").map_err(decode_err(op))?,
    })
}

/// Decode one Allegation's scalar columns into a summary with an empty evidence
/// bucket. The caller supplies the already-extracted `id`/`paragraph` (both
/// confirmed non-NULL) and fills `supporting_evidence` from subsequent rows.
fn decode_allegation(
    row: &neo4rs::Row,
    id: String,
    paragraph: String,
    op: &'static str,
) -> Result<AllegationSummary, ElementDetailRepoError> {
    let section = source_section_for(&paragraph);
    Ok(AllegationSummary {
        allegation_id: id,
        paragraph_number: paragraph,
        summary: row.get("summary").map_err(decode_err(op))?,
        title: row.get("title").map_err(decode_err(op))?,
        verbatim_quote: row.get("verbatim_quote").map_err(decode_err(op))?,
        source_section: section,
        supporting_evidence: Vec::new(),
    })
}

/// Decode the Evidence + source-Document columns from a row. Returns `None`
/// when the row carries no Evidence (the `OPTIONAL MATCH` produced NULLs — e.g.
/// an Allegation with no corroboration).
///
/// When an Evidence node is present but has no `CONTAINED_IN` Document,
/// `source_document_id` is `None`: we keep the item (it still corroborates the
/// Allegation) and emit a `warn` so the data gap is observable (Rule 1) — we do
/// not drop the evidence and we do not fail the request.
fn decode_evidence(
    row: &neo4rs::Row,
    op: &'static str,
) -> Result<Option<EvidenceRef>, ElementDetailRepoError> {
    let id: Option<String> = row.get("evidence_id").map_err(decode_err(op))?;
    // No Evidence on this row → nothing to attach.
    let Some(id) = id else {
        return Ok(None);
    };
    let source_document_id: Option<String> =
        row.get("source_document_id").map_err(decode_err(op))?;
    if source_document_id.is_none() {
        tracing::warn!(
            evidence_id = %id,
            "corroborating Evidence has no CONTAINED_IN Document — source-PDF click-through unavailable; \
             re-run discovery pass-2 extraction for the source document or verify its CONTAINED_IN edge was authored"
        );
    }
    Ok(Some(EvidenceRef {
        verbatim_quote: row.get("evidence_quote").map_err(decode_err(op))?,
        page_number: row.get("evidence_page_number").map_err(decode_err(op))?,
        paragraph: row.get("evidence_paragraph").map_err(decode_err(op))?,
        page_note: row.get("evidence_page_note").map_err(decode_err(op))?,
        source_document_title: row.get("source_document_title").map_err(decode_err(op))?,
        // `id` is moved last — it is borrowed by the `warn` above.
        source_document_id,
        id,
    }))
}

/// Append `ev` to `bucket` unless an item with the same `id` is already present.
///
/// The fan-out Cypher can surface the same Evidence on more than one row (e.g. a
/// duplicate `CORROBORATES` edge from a mid-ingest race), so the panel must not
/// render an Evidence card twice. Pulled out of [`DetailFold::push_row`] as a
/// pure `Vec` operation so the dedup invariant is unit-testable without a
/// `neo4rs::Row` (which neo4rs exposes no constructor for).
fn push_evidence_deduped(bucket: &mut Vec<EvidenceRef>, ev: EvidenceRef) {
    if !bucket.iter().any(|e| e.id == ev.id) {
        bucket.push(ev);
    }
}

/// Accumulator that folds the fanned-out `(Element × Allegation × Evidence)`
/// rows into the Element header plus a deduped Allegation list, each Allegation
/// carrying its deduped supporting Evidence.
///
/// ## Why an accumulator struct and not a `fn(stream)` helper
///
/// `Graph::execute` returns a `DetachedRowStream` whose type neo4rs does not
/// re-export, so a helper cannot name it in its signature. Instead the caller
/// keeps the 3-line `while let` stream loop and feeds each decoded `Row` here
/// via [`DetailFold::push_row`]. All the folding logic lives in this `impl`,
/// satisfying the 50-line function limit without naming the un-nameable stream
/// type.
///
/// ## Why a row index instead of sort-then-dedup
///
/// The two extra `OPTIONAL MATCH` hops fan one Allegation out to one row per
/// corroborating Evidence (and Evidence to its source Document). An earlier
/// version pushed one row per Allegation then `dedup_by`'d; that no longer works
/// because we must *accumulate* evidence across an Allegation's rows. The
/// `allegation_id → index` map lets the first row for an Allegation create its
/// summary (scalar fields are identical across its rows) while every row's
/// Evidence is appended to that summary, deduped by evidence id. An Allegation
/// with no Evidence keeps an empty `supporting_evidence` vec — the visible gap.
#[derive(Default)]
pub(crate) struct DetailFold {
    pub(crate) header: Option<ElementHeader>,
    pub(crate) allegations: Vec<AllegationSummary>,
    /// `allegation_id` → its position in `allegations`, so repeat rows for the
    /// same Allegation append evidence instead of duplicating the row.
    index: HashMap<String, usize>,
}

impl DetailFold {
    /// Fold one decoded row into the accumulator: capture the Element header
    /// once, find-or-create the row's Allegation, and attach its Evidence.
    pub(crate) fn push_row(
        &mut self,
        row: &neo4rs::Row,
        op: &'static str,
    ) -> Result<(), ElementDetailRepoError> {
        // Element / Count columns are identical on every row — capture once.
        if self.header.is_none() {
            self.header = Some(decode_header(row, op)?);
        }

        // Allegation keys are Option because OPTIONAL MATCH yields NULLs for an
        // Element with no mapped Allegations. Either key NULL ⇒ no Allegation.
        let allegation_id: Option<String> = row.get("allegation_id").map_err(decode_err(op))?;
        let paragraph_number: Option<String> =
            row.get("paragraph_number").map_err(decode_err(op))?;
        let (Some(id), Some(paragraph)) = (allegation_id, paragraph_number) else {
            return Ok(());
        };

        // Find or create the Allegation, then attach this row's Evidence (if any).
        let idx = match self.index.get(&id) {
            Some(&i) => i,
            None => {
                let i = self.allegations.len();
                self.allegations
                    .push(decode_allegation(row, id.clone(), paragraph, op)?);
                self.index.insert(id, i);
                i
            }
        };
        if let Some(ev) = decode_evidence(row, op)? {
            push_evidence_deduped(&mut self.allegations[idx].supporting_evidence, ev);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(id: &str, page: i64) -> EvidenceRef {
        EvidenceRef {
            id: id.to_string(),
            verbatim_quote: None,
            page_number: Some(page),
            paragraph: None,
            page_note: None,
            source_document_id: None,
            source_document_title: None,
        }
    }

    /// A second push with the SAME evidence id is a no-op — the panel must not
    /// render an Evidence card twice when a duplicate `CORROBORATES` edge fans
    /// the same Evidence onto two rows.
    #[test]
    fn push_evidence_deduped_skips_duplicate_id() {
        let mut bucket: Vec<EvidenceRef> = Vec::new();
        push_evidence_deduped(&mut bucket, evidence("evidence-074", 22));
        // Same id, different page_number — still treated as the same item.
        push_evidence_deduped(&mut bucket, evidence("evidence-074", 99));
        assert_eq!(bucket.len(), 1);
        assert_eq!(bucket[0].id, "evidence-074");
        assert_eq!(bucket[0].page_number, Some(22), "first write wins");
    }

    /// Distinct evidence ids both land in the bucket, preserving insertion order.
    #[test]
    fn push_evidence_deduped_keeps_distinct_ids() {
        let mut bucket: Vec<EvidenceRef> = Vec::new();
        push_evidence_deduped(&mut bucket, evidence("evidence-074", 22));
        push_evidence_deduped(&mut bucket, evidence("evidence-041", 15));
        let ids: Vec<&str> = bucket.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["evidence-074", "evidence-041"]);
    }
}
