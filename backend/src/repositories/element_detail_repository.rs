//! Composite read for the Element detail floating panel
//! (`GET /api/cases/:slug/elements/:element_id/detail`).
//!
//! This module reaches into two stores in a single endpoint:
//!
//! - **Neo4j** — the Element node itself, its parent `LegalCount` (via
//!   `HAS_ELEMENT`), and every `Allegation` that bears on it (via
//!   `BEARS_ON`). One Cypher with two `OPTIONAL MATCH` hops, decoded
//!   into a flat list of rows.
//! - **Postgres `authored_entities`** — the human-authored `review_notes`
//!   column added by the `add_review_notes_to_authored_entities` migration.
//!   Lives outside the canonical `item_data` JSONB because it is operator-
//!   layer annotation, not part of the canonical entity payload.
//!
//! ## Why a new module
//!
//! `causes_of_action_repository.rs` already serves the list view (all Counts +
//! all Elements with allegation **counts only**). The detail panel needs
//! per-Allegation rows (id, paragraph_number, title, summary, verbatim_quote)
//! which the list query does not return. Adding the new Cypher there would
//! push the file past the 300-line module limit, so the detail read gets its
//! own file. Pattern mirrors `allegation_detail_repository.rs` split off from
//! `decomposition_repository.rs`.

use neo4rs::{query, Graph};
use serde::Serialize;
use sqlx::PgPool;

use super::element_detail_cypher::element_detail_cypher;
use super::element_detail_fold::DetailFold;
use crate::models::document_status::{
    ENTITY_ALLEGATION, ENTITY_DOCUMENT, ENTITY_ELEMENT, ENTITY_EVIDENCE, ENTITY_LEGAL_COUNT,
};
use crate::repositories::pipeline_repository::PipelineRepoError;

// ── Error type ────────────────────────────────────────────────────

/// Errors raised by the detail read. Each variant identifies a distinct
/// failure class so the API handler can map them to 404 vs 500.
///
/// Operator-log context (operation + `#[source]`) is preserved; the API
/// handler renders bland bodies for the client (Rule 1).
#[derive(Debug, thiserror::Error)]
pub enum ElementDetailRepoError {
    /// The Element id did not match any node in Neo4j. Mapped to HTTP 404.
    /// Distinct observable: query succeeded, zero rows.
    #[error("Element not found: {element_id}")]
    NotFound { element_id: String },

    /// Neo4j request failed (network, syntax, server-side error). Mapped to
    /// HTTP 500.
    #[error("Neo4j query failed during {operation}: {source}")]
    Neo4jQuery {
        operation: &'static str,
        #[source]
        source: neo4rs::Error,
    },

    /// A Neo4j row decoded successfully at the transport layer but a column
    /// could not be deserialized into the expected Rust type. Mapped to 500.
    #[error("Failed to decode Neo4j row during {operation}: {source}")]
    Neo4jDecode {
        operation: &'static str,
        #[source]
        source: neo4rs::DeError,
    },

    /// Postgres lookup for `review_notes` failed. Mapped to 500.
    /// Wraps [`PipelineRepoError`] so the underlying SQL error string is
    /// preserved in operator logs.
    #[error("Postgres read failed during {operation}: {source}")]
    Postgres {
        operation: &'static str,
        #[source]
        source: PipelineRepoError,
    },
}

// ── Response DTOs ─────────────────────────────────────────────────

/// Top-level response body for the Element detail endpoint. Field casing is
/// snake_case to match the project-wide DTO convention (see
/// `dto::causes_of_action`).
///
/// ## Domain note: `count_number` is `Option<i64>`
///
/// The Cypher uses `OPTIONAL MATCH (lc:LegalCount)-[:HAS_ELEMENT]->(e)` so an
/// orphan Element (one not currently hung off a Count) decodes the field to
/// `None` rather than silently failing. In well-loaded canonical data this is
/// always `Some(_)`, but Rule 1 says "missing must be distinguishable from
/// failed" — the operator log can tell them apart.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ElementDetailResponse {
    pub element_id: String,
    pub element_name: String,
    pub what_plaintiff_must_prove: String,
    pub order_in_count: Option<i64>,
    pub count_number: Option<i64>,
    pub count_name: Option<String>,
    pub review_notes: Option<String>,
    pub allegations: Vec<AllegationSummary>,
    pub allegation_count: usize,
    /// Number of mapped Allegations in the Common Allegations paragraph
    /// range (¶`COMMON_PARA_START` through ¶`COMMON_PARA_END`).
    pub common_count: usize,
    /// Number of mapped Allegations in the dedicated-Count paragraph range
    /// (¶`DEDICATED_PARA_START`+).
    pub dedicated_count: usize,
    /// How many items each paragraph shows before "N more" — the stored
    /// `matrix_visible_items` (PROOF_MATRIX_v2 §3).
    ///
    /// Served rather than compiled into the browser for the ordinary Rule 2
    /// reason, and served on THIS payload rather than fetched separately because
    /// the same number decides what the Word export prints: one read, one number,
    /// and no way for the page and the document to disagree.
    pub visible_items: usize,
}

/// One mapped Allegation as it appears in the detail panel's list. The fields
/// are intentionally minimal — the panel renders a card per row and links to
/// the existing Allegation detail page for the full payload.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AllegationSummary {
    pub allegation_id: String,
    pub paragraph_number: String,
    pub summary: Option<String>,
    pub title: Option<String>,
    pub verbatim_quote: Option<String>,
    /// `"Common"`, `"Dedicated"`, or `"Unknown"`. The frontend already knows
    /// the active Count from the panel context, so this is a coarse
    /// classifier, not a precise count attribution. See
    /// [`source_section_for`].
    pub source_section: &'static str,
    /// Evidence items that corroborate this Allegation
    /// (`(Evidence)-[:CORROBORATES]->(Allegation)`), deduped by id.
    ///
    /// Domain note: an **empty vec is the visible gap** — an Allegation with no
    /// corroborating Evidence renders as an explicit "no evidence" row in the
    /// panel, so gaps are honest, not hidden. Never omitted (Rule 1: empty and
    /// absent stay distinguishable).
    pub supporting_evidence: Vec<EvidenceRef>,

    /// Evidence items that DISPUTE this Allegation
    /// (`(Evidence)-[:REBUTS]->(Allegation)`), deduped by id — the exact mirror
    /// of `supporting_evidence`, and the items behind the Proof Matrix's
    /// "Disputes" column.
    ///
    /// Domain note: "Disputes", not "Contradicts" — CONTRADICTS is reserved for
    /// the future evidence-vs-evidence impeachment layer, and one word for two
    /// different relationships would make them read as one.
    ///
    /// Empty is again the honest state, and it means something different from
    /// the supporting side: no evidence disputes this Allegation *yet*. Never
    /// omitted.
    pub disputing_evidence: Vec<EvidenceRef>,
}

/// One corroborating Evidence item, with enough fields for a source-PDF
/// click-through (`page_number` locates the page; the source Document supplies
/// the file).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EvidenceRef {
    pub id: String,
    pub verbatim_quote: Option<String>,
    /// PDF page number where the Q&A appears — the click-through locator.
    pub page_number: Option<i64>,
    /// Interrogatory / request id, e.g. `"Q74"`, `"RFA 9"`.
    pub paragraph: Option<String>,
    /// Page range when the Q&A spans pages, e.g. `"pages 10-11"`.
    pub page_note: Option<String>,
    /// `Document.id` of the source PDF, reached via
    /// `(Evidence)-[:CONTAINED_IN]->(Document)`. `None` when the Evidence node
    /// carries no `CONTAINED_IN` edge — a distinguishable data-gap (logged at
    /// `warn`), not an error and not a dropped item (Rule 1).
    pub source_document_id: Option<String>,
    pub source_document_title: Option<String>,
    /// `Evidence.statement_type` — half the tier key (task 396, P1).
    pub statement_type: Option<String>,
    /// `Evidence.evidence_strength` — the other half.
    pub evidence_strength: Option<String>,
    /// The speaker, via `(Evidence)-[:STATED_BY]->(Party)`. Part of the collapse
    /// key: two parties saying the same words are two pieces of proof.
    pub speaker: Option<String>,
    /// The interrogatory question this answers, or `None` for documentary
    /// evidence. Also part of the collapse key — and it is the component that
    /// keeps three distinct "yes." admissions from merging into one row.
    pub question: Option<String>,
    /// The answer half of a Q&A card, when there is one. Feeds the RFA line.
    pub answer: Option<String>,

    // ── What the linking pass wrote on the EDGE (PROOF_MATRIX_v2 §1) ─────────
    //
    // Served as raw tokens, not as labels: the words a reader sees come from the
    // stored `matrix_*` wording rows, and a backend that sent finished labels
    // would put the vocabulary in two places. `None` on all of them is a real
    // state — 286 edges predate the pass.
    /// `r.rank` — the pass's position within a stance, 1..N.
    pub rank: Option<i64>,
    /// `r.role` — one of the five tokens in
    /// [`crate::domain::matrix_edge::EdgeRole`], or `None` when absent or
    /// unreadable (the fold warns and shows the item; it never hides one it
    /// failed to parse).
    pub role: Option<String>,
    /// `r.confidence` — `high` / `medium` / `low`, or `None` for an older edge.
    pub confidence: Option<String>,
    /// `r.rank_reason` — the pass's one line about why this item ranks here.
    pub rank_reason: Option<String>,
    /// `r.why` — the older linking pass's reason. Shown when there is no
    /// `rank_reason`, so a row is never left with no explanation at all.
    pub why: Option<String>,
    /// `r.conflict` — the item both supports and disputes this Allegation.
    ///
    /// A plain `bool` rather than `Option<bool>`: absent means false, and there
    /// is no third state a reader could act on differently. 28 edges carry it.
    pub conflict: bool,
    /// `r.duplicate_of_card_id` — the item this one restates.
    pub duplicate_of_card_id: Option<String>,
    /// The SOURCE DOCUMENT's date, `YYYY-MM-DD`. §3's fourth sort key. Empty
    /// string on four DEV documents, which the ordering treats as absent.
    pub document_date: Option<String>,

    // ── What a human said, and what this build worked out (§2, §3) ───────────
    /// The human's verdict: `keep` / `remove`, or `None` for an unruled item.
    pub ruling: Option<String>,
    /// Who ruled. `None` exactly when `ruling` is `None`.
    pub ruled_by: Option<String>,
    /// Why this item is not in the default list: `does_not_belong` / `removed`,
    /// or `None` for a visible one.
    ///
    /// Domain note: served rather than derived in the browser, so the page and
    /// the Word export apply ONE set of hide rules.
    pub hidden_reason: Option<String>,
    /// The finished one-line rendering of a Q&A card, composed from the stored
    /// templates. `None` for anything that is not one — the renderer then prints
    /// `verbatim_quote` as usual.
    pub rfa_line: Option<String>,
    /// How many items this row stands for: itself, plus every `duplicate_of`
    /// folded into it — the "×N".
    ///
    /// `1` means nothing folded, which is the overwhelming majority of rows. The
    /// renderer prints the marker only above 1.
    pub occurrences: usize,
}

// ── Cypher and SQL constants ──────────────────────────────────────

/// Defensive Postgres lookup: filter by entity_id (uniquely constrained) AND
/// entity_type to keep a stray id collision with a different entity_type from
/// returning unrelated notes. The `entity_type` discriminator binds to the
/// canonical `ENTITY_ELEMENT` constant (imported above) — same source of
/// truth used by the Cypher's `element_label` parameter.
const REVIEW_NOTES_SQL: &str =
    "SELECT review_notes FROM authored_entities WHERE entity_id = $1 AND entity_type = $2";

// ── Paragraph-classifier constants ────────────────────────────────
//
// CONST: the Awad complaint structure puts "Common Allegations" in ¶7–71 and
// the per-Count "dedicated" allegations from ¶72 onward. These are
// **case-structural constants for Awad** — app-level, not shared library.
// They cannot be runtime configuration today because the only case is Awad
// and there is no per-case YAML loader for ranges of this kind yet. If a
// second case onboards with a different layout, promote these to the case's
// YAML config; for now they stay here as named constants so no magic
// numbers leak into the classifier. (Domain note: see complaint structure.)

/// First paragraph number in the Common Allegations range (inclusive).
const COMMON_PARA_START: u32 = 7;

/// Last paragraph number in the Common Allegations range (inclusive).
const COMMON_PARA_END: u32 = 71;

/// First paragraph number considered Count-dedicated (inclusive).
const DEDICATED_PARA_START: u32 = 72;

/// Coarse source-section classifier. The frontend knows the active Count from
/// panel context, so this is just a Common-vs-Dedicated marker plus a fallback
/// for malformed inputs.
///
/// ## Rust Learning: `&'static str` return for enum-like text
///
/// We return a `&'static str` (not a `String`) because the three outputs are
/// compile-time literals — no heap allocation needed. The `'static` lifetime
/// says "this reference lives for the program's entire lifetime", which is
/// exactly the case for a string literal embedded in the binary.
///
/// ## Range handling
///
/// `paragraph_number` is a string because Neo4j Allegation nodes sometimes
/// carry ranges like `"16-18"`. `str::parse::<u32>()` rejects those; the
/// helper falls back to parsing the leading numeric prefix (everything up to
/// the first non-digit) so ranges classify by their starting paragraph. A
/// fully non-numeric value yields `"Unknown"` rather than panicking or
/// silently defaulting (Rule 1).
pub(crate) fn source_section_for(paragraph_number: &str) -> &'static str {
    let leading: String = paragraph_number
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    // best-effort: a parse failure here is the documented "Unknown" path
    // (the `_` arm below) — converting to Option is the conversion, not a
    // swallowed error. Empty / non-numeric input is a defined input class.
    let parsed = leading.parse::<u32>().ok();
    match parsed {
        Some(p) if (COMMON_PARA_START..=COMMON_PARA_END).contains(&p) => "Common",
        Some(p) if p >= DEDICATED_PARA_START => "Dedicated",
        _ => "Unknown",
    }
}

// ── Main read fn ──────────────────────────────────────────────────

/// Fetch an Element with its parent Count, mapped Allegations, and the
/// human-authored `review_notes` from Postgres. The two reads run sequentially
/// (Neo4j first — that determines whether the Element exists at all). On a
/// Neo4j miss the function returns [`ElementDetailRepoError::NotFound`] before
/// touching Postgres.
///
/// The Cypher emits one row per (Element, parent Count, mapped Allegation,
/// corroborating Evidence) tuple. We aggregate in Rust via
/// [`DetailFold::push_row`]: the Element / Count columns repeat across rows
/// (same Element), Allegations are folded by id, and each Allegation's
/// corroborating Evidence is collected (deduped) from its rows. An Element with
/// zero mapped Allegations still produces a single row with NULL Allegation
/// columns thanks to `OPTIONAL MATCH`.
///
/// Final allegation ordering is by parsed-integer `paragraph_number` — see
/// the in-fn comment for why we sort in Rust rather than `ORDER BY` in Cypher.
pub async fn fetch_element_with_allegations(
    graph: &Graph,
    pool: &PgPool,
    element_id: &str,
) -> Result<ElementDetailResponse, ElementDetailRepoError> {
    const OP_GRAPH: &str = "fetch_element_with_allegations";
    const OP_PG: &str = "fetch_review_notes";

    let mut stream = graph
        .execute(detail_query(element_id))
        .await
        .map_err(|source| ElementDetailRepoError::Neo4jQuery {
            operation: OP_GRAPH,
            source,
        })?;

    // Fold the fanned-out rows: Element header once, Allegations deduped by id,
    // each Allegation's corroborating Evidence collected (see `DetailFold`). The
    // stream loop stays here because `DetachedRowStream`'s type is not nameable
    // in a helper signature (see the `DetailFold` doc comment).
    let mut fold = DetailFold::default();
    while let Some(row) =
        stream
            .next()
            .await
            .map_err(|source| ElementDetailRepoError::Neo4jQuery {
                operation: OP_GRAPH,
                source,
            })?
    {
        fold.push_row(&row, OP_GRAPH)?;
    }

    let header = fold
        .header
        .ok_or_else(|| ElementDetailRepoError::NotFound {
            element_id: element_id.to_string(),
        })?;
    let mut allegations = fold.allegations;

    sort_by_paragraph(&mut allegations);

    let (allegation_count, common_count, dedicated_count) = section_counts(&allegations);

    let review_notes = fetch_review_notes(pool, element_id, OP_PG).await?;

    Ok(ElementDetailResponse {
        element_id: header.element_id,
        element_name: header.element_name,
        what_plaintiff_must_prove: header.what_plaintiff_must_prove,
        order_in_count: header.order_in_count,
        count_number: header.count_number,
        count_name: header.count_name,
        review_notes,
        allegations,
        allegation_count,
        common_count,
        dedicated_count,
        // A placeholder the handler overwrites from the settings snapshot. The
        // repository has no settings handle, and inventing a default here is
        // exactly the compiled-in number Rule 2 forbids — so it is zero, which is
        // visibly wrong if the handler ever stops filling it in, rather than
        // plausibly right.
        visible_items: 0,
    })
}

/// The detail query with every node label bound as a parameter.
///
/// Split out so the labels are named in one place: six `.param()` calls inline
/// are six chances to bind the Document label to the Evidence gate, which would
/// silently return zero evidence rows rather than fail.
fn detail_query(element_id: &str) -> neo4rs::Query {
    query(&element_detail_cypher())
        .param("element_id", element_id)
        .param("element_label", ENTITY_ELEMENT)
        .param("count_label", ENTITY_LEGAL_COUNT)
        .param("allegation_label", ENTITY_ALLEGATION)
        .param("evidence_label", ENTITY_EVIDENCE)
        .param("document_label", ENTITY_DOCUMENT)
}

/// `(total, common, dedicated)` for one Element's Allegations.
///
/// ## Why three numbers and not two
///
/// `common + dedicated` does NOT have to equal the total: an Allegation whose
/// paragraph number is unparseable classifies as `"Unknown"` and belongs to
/// neither range. Returning the total separately is what keeps the panel's
/// header honest — "N allegations mapped (X common · Y dedicated)" with X + Y < N
/// is a true sentence, and deriving N from X + Y would quietly lose those rows.
fn section_counts(allegations: &[AllegationSummary]) -> (usize, usize, usize) {
    let common = allegations
        .iter()
        .filter(|a| a.source_section == "Common")
        .count();
    let dedicated = allegations
        .iter()
        .filter(|a| a.source_section == "Dedicated")
        .count();
    (allegations.len(), common, dedicated)
}

/// Sort Allegations by parsed-integer `paragraph_number`, in place.
///
/// Split out of [`fetch_element_with_allegations`] to keep that function inside
/// the 50-line limit, and because the ordering deserves its own name: it is the
/// order the panel reads in, and it is NOT the §3 evidence order — this one is
/// about which accusation comes first, not which proof under it does.
///
/// Allegations are already unique (folded by id in `DetailFold::push_row`, which
/// also absorbs duplicate `BEARS_ON` edges a mid-ingest race could leave), so no
/// dedup step is needed here.
///
/// Sorts by the leading integer prefix so ranges like `"16-18"` sort by 16, and
/// falls back to lexicographic for anything unparseable — keeping the order
/// stable rather than panicking. Non-numeric values sort LAST.
fn sort_by_paragraph(allegations: &mut [AllegationSummary]) {
    allegations.sort_by(|a, b| {
        let pa = leading_int(&a.paragraph_number);
        let pb = leading_int(&b.paragraph_number);
        match (pa, pb) {
            (Some(x), Some(y)) => x
                .cmp(&y)
                .then_with(|| a.paragraph_number.cmp(&b.paragraph_number)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.paragraph_number.cmp(&b.paragraph_number),
        }
    });
}

/// The human-authored `review_notes` for one Element, or `None`.
///
/// A missing row is not an error: the canonical loader writes the Element row,
/// but a brand-new deployment whose loader has not run yet would have none.
///
/// ## Rust Learning: `Option<Option<String>>` from `fetch_optional`
///
/// The outer `Option` is "was there a ROW"; the inner is "was the COLUMN null".
/// Three states collapse to two on the wire — both `None` shapes serialize as
/// `review_notes: null` — so they are kept distinguishable in the operator log
/// instead (Rule 1: distinct observables), with a `debug` span on the
/// row-missing branch.
///
/// # Errors
/// Returns [`ElementDetailRepoError::Postgres`] if the query fails.
async fn fetch_review_notes(
    pool: &PgPool,
    element_id: &str,
    op: &'static str,
) -> Result<Option<String>, ElementDetailRepoError> {
    let pg_row: Option<Option<String>> = sqlx::query_scalar::<_, Option<String>>(REVIEW_NOTES_SQL)
        .bind(element_id)
        .bind(ENTITY_ELEMENT)
        .fetch_optional(pool)
        .await
        .map_err(|e| ElementDetailRepoError::Postgres {
            operation: op,
            source: PipelineRepoError::Database(e.to_string()),
        })?;

    Ok(match pg_row {
        None => {
            tracing::debug!(
                %element_id,
                "no authored_entities row for element — review_notes defaulting to None"
            );
            None
        }
        Some(notes) => notes,
    })
}

/// Parse the leading numeric prefix of a paragraph_number string. Returns
/// `None` if there is no leading digit at all.
fn leading_int(s: &str) -> Option<u32> {
    let leading: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    // best-effort: `None` is the documented "non-numeric, sort last"
    // contract for the caller (see ordering match in `fetch_*`). Converting
    // parse Err → None is the type-level expression of that contract, not
    // a silently-swallowed error.
    leading.parse::<u32>().ok()
}

// ── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "element_detail_repository_tests.rs"]
mod tests;
