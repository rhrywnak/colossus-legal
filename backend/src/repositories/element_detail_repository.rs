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

use super::element_detail_fold::DetailFold;
use crate::models::document_status::{
    ENTITY_ALLEGATION, ENTITY_DOCUMENT, ENTITY_ELEMENT, ENTITY_EVIDENCE, ENTITY_LEGAL_COUNT,
};
use crate::neo4j::schema;
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
    /// How hard this item is to dispute: `strong` / `hedged` / `other`, or `None`
    /// when the stored tier map does not name its pair.
    ///
    /// Domain note: `None` is a real answer and renders as a row with no chip. An
    /// item whose pair is unmapped is still counted as approved and still shown —
    /// a new document type must never make proof vanish from this list.
    ///
    /// Populated by [`rank_supporting_evidence`]; the disputing leg leaves it
    /// `None`, because tiering is a claim about how hard SUPPORT is to dispute.
    pub tier: Option<String>,
    /// How many near-identical statements collapsed into this row — the "×N".
    ///
    /// `1` means no duplicates, which is the overwhelming majority of rows. The
    /// renderer prints the marker only above 1.
    pub occurrences: usize,
}

// ── Cypher and SQL constants ──────────────────────────────────────

/// Build the detail Cypher: Element properties, parent LegalCount (OPTIONAL),
/// and every Allegation that bears on this Element (OPTIONAL).
///
/// ## Why a `fn -> String` and not a `const`
///
/// Relationship types come from `neo4j::schema` so the read stays in lockstep
/// with one constant; a Rust `const` cannot call `format!`, so the query is
/// built by a function (the `fetch_hashes` pattern in
/// `canonical_elements::cypher`). No literal `{ }` braces appear here (node
/// bindings use `labels(x)[0]`, not property maps), so no `{{`/`}}` escaping.
///
/// ## Why label filters on every node binding
///
/// `(a)-[:{bears_on}]->(e)` with no label restriction would match any
/// node-type bearing on an Element. House style — established in
/// `causes_of_action_repository.rs` — is to gate every node binding with
/// `labels(x)[0] = $label` and read the label name from `ENTITY_*`
/// constants, so we never hardcode a domain string in a Cypher clause.
///
/// `e.id` for the Element matches the `id` *property* (not Neo4j's internal
/// id) — that is the canonical, content-stable identifier the loader writes
/// and the one Postgres stores in `authored_entities.entity_id`.
fn element_detail_cypher() -> String {
    format!(
        "MATCH (e) \
       WHERE e.id = $element_id AND labels(e)[0] = $element_label \
     OPTIONAL MATCH (lc)-[:{has_element}]->(e) WHERE labels(lc)[0] = $count_label \
     OPTIONAL MATCH (a)-[:{bears_on}]->(e) WHERE labels(a)[0] = $allegation_label \
     OPTIONAL MATCH (a)<-[:{corroborates}]-(ev) WHERE labels(ev)[0] = $evidence_label \
     OPTIONAL MATCH (ev)-[:{contained_in}]->(d) WHERE labels(d)[0] = $document_label \
     OPTIONAL MATCH (ev)-[:{stated_by}]->(sp) \
     OPTIONAL MATCH (a)<-[:{rebuts}]-(dv) WHERE labels(dv)[0] = $evidence_label \
     OPTIONAL MATCH (dv)-[:{contained_in}]->(dd) WHERE labels(dd)[0] = $document_label \
     OPTIONAL MATCH (dv)-[:{stated_by}]->(dsp) \
     RETURN \
       e.id                         AS element_id, \
       e.element_name               AS element_name, \
       e.what_plaintiff_must_prove  AS what_plaintiff_must_prove, \
       e.order_in_count             AS order_in_count, \
       lc.count_number              AS count_number, \
       lc.title                     AS count_name, \
       a.id                         AS allegation_id, \
       a.paragraph_number           AS paragraph_number, \
       a.summary                    AS summary, \
       a.title                      AS title, \
       a.verbatim_quote             AS verbatim_quote, \
       ev.id                        AS evidence_id, \
       ev.verbatim_quote            AS evidence_quote, \
       ev.page_number               AS evidence_page_number, \
       ev.paragraph                 AS evidence_paragraph, \
       ev.page_note                 AS evidence_page_note, \
       d.id                         AS source_document_id, \
       d.title                      AS source_document_title, \
       ev.statement_type            AS evidence_statement_type, \
       ev.evidence_strength         AS evidence_strength, \
       sp.name                      AS evidence_speaker, \
       ev.question                  AS evidence_question, \
       dv.id                        AS disputing_id, \
       dv.verbatim_quote            AS disputing_quote, \
       dv.page_number               AS disputing_page_number, \
       dv.paragraph                 AS disputing_paragraph, \
       dv.page_note                 AS disputing_page_note, \
       dd.id                        AS disputing_document_id, \
       dd.title                     AS disputing_document_title, \
       dv.statement_type            AS disputing_statement_type, \
       dv.evidence_strength         AS disputing_strength, \
       dsp.name                     AS disputing_speaker, \
       dv.question                  AS disputing_question",
        has_element = schema::HAS_ELEMENT,
        bears_on = schema::BEARS_ON,
        corroborates = schema::CORROBORATES,
        rebuts = schema::REBUTS,
        contained_in = schema::CONTAINED_IN,
        stated_by = schema::STATED_BY,
    )
}

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

    let q = query(&element_detail_cypher())
        .param("element_id", element_id)
        .param("element_label", ENTITY_ELEMENT)
        .param("count_label", ENTITY_LEGAL_COUNT)
        .param("allegation_label", ENTITY_ALLEGATION)
        .param("evidence_label", ENTITY_EVIDENCE)
        .param("document_label", ENTITY_DOCUMENT);

    let mut stream =
        graph
            .execute(q)
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

    // Allegations are already unique (folded by id in `DetailFold::push_row`,
    // which also absorbs duplicate BEARS_ON edges a mid-ingest race could
    // leave), so the historical sort-by-id + `dedup_by` step is no longer needed.
    //
    // Sort by paragraph_number numerically (parse the leading int prefix so
    // ranges like "16-18" sort by 16). Falls back to lexicographic for
    // anything we can't parse — keeps the order stable instead of panicking.
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

    let allegation_count = allegations.len();
    let common_count = allegations
        .iter()
        .filter(|a| a.source_section == "Common")
        .count();
    let dedicated_count = allegations
        .iter()
        .filter(|a| a.source_section == "Dedicated")
        .count();

    // Postgres: fetch the review_notes column. A missing row is not an error
    // here — the canonical loader writes the Element row, but a brand-new
    // deployment whose loader hasn't run yet would have no row.
    //
    // `fetch_optional` returns `Option<Option<String>>`:
    //   None        → no authored_entities row exists (data-load gap)
    //   Some(None)  → row exists, review_notes column is SQL NULL (user
    //                 has not yet written notes, or has cleared them)
    //   Some(Some)  → row exists, notes string present
    //
    // Both `None` states render on the wire as `review_notes: null`, but
    // we keep them distinguishable in operator logs (Rule 1: distinct
    // observables) by emitting a debug span on the row-missing branch.
    let pg_row: Option<Option<String>> = sqlx::query_scalar::<_, Option<String>>(REVIEW_NOTES_SQL)
        .bind(element_id)
        .bind(ENTITY_ELEMENT)
        .fetch_optional(pool)
        .await
        .map_err(|e| ElementDetailRepoError::Postgres {
            operation: OP_PG,
            source: PipelineRepoError::Database(e.to_string()),
        })?;

    let review_notes: Option<String> = match pg_row {
        None => {
            tracing::debug!(
                element_id = %element_id,
                "no authored_entities row for element — review_notes defaulting to None"
            );
            None
        }
        Some(notes) => notes,
    };

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
    })
}

/// Collapse, tier and rank every Allegation's SUPPORTING evidence, in place.
///
/// ## Why this happens here and not in the Cypher
///
/// Two of the three things it does are outside the graph's reach: the pair→tier
/// map is a settings row, and the near-duplicate collapse keys on a normalized
/// question and answer. Doing it in Rust also means the drill-down and the matrix
/// row's two numbers come from ONE function
/// ([`crate::services::matrix_strength::collapse_and_rank`]) — which is what
/// makes "the counts agree" a property of the code rather than a coincidence.
///
/// ## Why the DISPUTING leg is left alone
///
/// A tier is a claim about how hard a piece of SUPPORT is to dispute. Ranking
/// rebuttals by the same scale would read as a verdict on how badly the Element
/// is damaged, which is a different judgment nobody has made — and collapsing
/// them would quietly reduce the number of things arguing against us.
///
/// ## Rust Learning: `&mut` on the response instead of returning a new one
///
/// The response is already assembled and owns a `Vec` per Allegation; rebuilding
/// the whole tree to change two fields per item would clone every quote. Taking
/// `&mut` lets each `Vec` be replaced in place. The function returns nothing —
/// its whole effect is the mutation, which the name says.
pub fn rank_supporting_evidence(
    response: &mut ElementDetailResponse,
    tier_map: &crate::domain::evidence_tier::EvidenceTierMap,
) {
    use crate::services::matrix_strength::{collapse_and_rank, CorroboratingItem};

    for allegation in &mut response.allegations {
        let items: Vec<CorroboratingItem> = allegation
            .supporting_evidence
            .iter()
            .map(|e| CorroboratingItem {
                id: e.id.clone(),
                statement_type: e.statement_type.clone(),
                evidence_strength: e.evidence_strength.clone(),
                speaker: e.speaker.clone(),
                question: e.question.clone(),
                quote: e.verbatim_quote.clone(),
            })
            .collect();

        let groups = collapse_and_rank(&items, tier_map);

        // Rebuild the leg in ranked order, keeping ONE `EvidenceRef` per group —
        // the group's lead — and stamping it with what the collapse learned. The
        // lookup by id is over a list that is a handful of items long, so the
        // linear scan is cheaper than building a map to avoid it.
        let mut ranked: Vec<EvidenceRef> = Vec::with_capacity(groups.len());
        for group in &groups {
            let Some(source) = allegation
                .supporting_evidence
                .iter()
                .find(|e| e.id == group.lead.id)
            else {
                // Unreachable: every group's lead came from this very list. Logged
                // rather than skipped silently, because if it ever happened it
                // would mean a piece of proof had vanished between two lines of
                // one function, and a missing item on a proof surface must never
                // be something a reader has to notice for themselves.
                tracing::error!(
                    evidence_id = %group.lead.id,
                    allegation_id = %allegation.allegation_id,
                    "ranked group names an evidence id that is not in the allegation's \
                     supporting list — the item has been dropped from the drill-down; \
                     this indicates a defect in collapse_and_rank, not a data problem"
                );
                continue;
            };
            let mut item = source.clone();
            item.tier = group.tier.map(|t| t.code().to_string());
            item.occurrences = group.occurrences;
            ranked.push(item);
        }
        allegation.supporting_evidence = ranked;
    }
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
