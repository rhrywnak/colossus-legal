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

use crate::models::document_status::{ENTITY_ALLEGATION, ENTITY_ELEMENT, ENTITY_LEGAL_COUNT};
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
       a.verbatim_quote             AS verbatim_quote",
        has_element = schema::HAS_ELEMENT,
        bears_on = schema::BEARS_ON,
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

// ── Internal row aggregator ───────────────────────────────────────

/// Element + parent-Count columns captured from the first decoded row. We use
/// a named struct rather than a tuple so the call site stays readable and
/// clippy's `type_complexity` lint stays quiet.
struct ElementHeader {
    element_id: String,
    element_name: String,
    what_plaintiff_must_prove: String,
    order_in_count: Option<i64>,
    count_number: Option<i64>,
    count_name: Option<String>,
}

// ── Main read fn ──────────────────────────────────────────────────

/// Decode helper: maps neo4rs row-decode errors into the typed variant.
///
/// ## Rust Learning: returning `impl Fn(_) -> _`
///
/// `decode_err("op")` returns a closure that captures the operation name.
/// Used as `row.get("x").map_err(decode_err(OP))` so every column decode
/// reuses the same captured context without restating it inline. The `move`
/// would be required if the closure outlived the local, but here the
/// returned closure is consumed immediately by `map_err`.
fn decode_err(operation: &'static str) -> impl Fn(neo4rs::DeError) -> ElementDetailRepoError {
    move |source| ElementDetailRepoError::Neo4jDecode { operation, source }
}

/// Fetch an Element with its parent Count, mapped Allegations, and the
/// human-authored `review_notes` from Postgres. The two reads run sequentially
/// (Neo4j first — that determines whether the Element exists at all). On a
/// Neo4j miss the function returns [`ElementDetailRepoError::NotFound`] before
/// touching Postgres.
///
/// The Cypher emits one row per (Element, parent Count, mapped Allegation)
/// triple. We aggregate in Rust: the Element / Count columns repeat across
/// rows (same Element) and the Allegation columns vary per row; an Element
/// with zero mapped Allegations still produces a single row with NULL
/// Allegation columns thanks to `OPTIONAL MATCH`.
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
        .param("allegation_label", ENTITY_ALLEGATION);

    let mut stream =
        graph
            .execute(q)
            .await
            .map_err(|source| ElementDetailRepoError::Neo4jQuery {
                operation: OP_GRAPH,
                source,
            })?;

    // First row carries the Element + Count columns; subsequent rows just add
    // more Allegations. We accumulate Allegations into a Vec, dedup'd by
    // allegation_id (an Allegation could in principle prove the same Element
    // more than once via duplicate edges).
    let mut element_fields: Option<ElementHeader> = None;
    let mut allegations: Vec<AllegationSummary> = Vec::new();

    while let Some(row) =
        stream
            .next()
            .await
            .map_err(|source| ElementDetailRepoError::Neo4jQuery {
                operation: OP_GRAPH,
                source,
            })?
    {
        // Element / Count columns: capture once on the first row. They are
        // identical on every subsequent row (same Element, same Count).
        if element_fields.is_none() {
            // `element_name` and `what_plaintiff_must_prove` are required
            // properties on a canonical Element node. We decode them as
            // non-Option — a missing value here is a data-shape bug, not a
            // recoverable state, so the decode error propagates as 500.
            let row_element_id: String = row.get("element_id").map_err(decode_err(OP_GRAPH))?;
            let element_name: String = row.get("element_name").map_err(decode_err(OP_GRAPH))?;
            let what_plaintiff_must_prove: String = row
                .get("what_plaintiff_must_prove")
                .map_err(decode_err(OP_GRAPH))?;
            let order_in_count: Option<i64> =
                row.get("order_in_count").map_err(decode_err(OP_GRAPH))?;
            let count_number: Option<i64> =
                row.get("count_number").map_err(decode_err(OP_GRAPH))?;
            let count_name: Option<String> = row.get("count_name").map_err(decode_err(OP_GRAPH))?;
            element_fields = Some(ElementHeader {
                element_id: row_element_id,
                element_name,
                what_plaintiff_must_prove,
                order_in_count,
                count_number,
                count_name,
            });
        }

        // Allegation columns: each is Option because OPTIONAL MATCH yields
        // NULLs when no Allegation proves this Element.
        let allegation_id: Option<String> =
            row.get("allegation_id").map_err(decode_err(OP_GRAPH))?;
        let paragraph_number: Option<String> =
            row.get("paragraph_number").map_err(decode_err(OP_GRAPH))?;
        // Only assemble an AllegationSummary when both keys are present —
        // either being NULL means this row carries no Allegation.
        if let (Some(id), Some(paragraph)) = (allegation_id, paragraph_number) {
            let summary: Option<String> = row.get("summary").map_err(decode_err(OP_GRAPH))?;
            let title: Option<String> = row.get("title").map_err(decode_err(OP_GRAPH))?;
            let verbatim_quote: Option<String> =
                row.get("verbatim_quote").map_err(decode_err(OP_GRAPH))?;
            let section = source_section_for(&paragraph);
            allegations.push(AllegationSummary {
                allegation_id: id,
                paragraph_number: paragraph,
                summary,
                title,
                verbatim_quote,
                source_section: section,
            });
        }
    }

    let header = element_fields.ok_or_else(|| ElementDetailRepoError::NotFound {
        element_id: element_id.to_string(),
    })?;

    // De-duplicate by allegation_id. A canonical BEARS_ON MERGE on
    // (a, e) ought to be unique, but two ingest passes can leave a duplicate
    // edge briefly; the panel shouldn't render the same row twice.
    allegations.sort_by(|x, y| x.allegation_id.cmp(&y.allegation_id));
    allegations.dedup_by(|x, y| x.allegation_id == y.allegation_id);

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
mod tests {
    use super::*;
    use serde_json::json;

    /// Pins the JSON shape of `AllegationSummary`. The frontend reads
    /// exactly these snake_case keys; a typo in the struct field name would
    /// silently break the panel.
    #[test]
    fn allegation_summary_serializes_with_expected_keys() {
        let summary = AllegationSummary {
            allegation_id: "allegation-42".to_string(),
            paragraph_number: "10".to_string(),
            summary: Some("Defendant did the thing.".to_string()),
            title: Some("Title".to_string()),
            verbatim_quote: None,
            source_section: "Common",
        };
        let value = serde_json::to_value(&summary).expect("serializes cleanly");
        assert_eq!(
            value,
            json!({
                "allegation_id": "allegation-42",
                "paragraph_number": "10",
                "summary": "Defendant did the thing.",
                "title": "Title",
                "verbatim_quote": null,
                "source_section": "Common",
            })
        );
    }

    /// ¶10 falls inside `COMMON_PARA_START..=COMMON_PARA_END`. Documents the
    /// classifier's lower-half behavior.
    #[test]
    fn source_section_common_for_paragraph_10() {
        assert_eq!(source_section_for("10"), "Common");
    }

    /// ¶73 falls above `DEDICATED_PARA_START`. Documents the classifier's
    /// upper-half behavior.
    #[test]
    fn source_section_dedicated_for_paragraph_73() {
        assert_eq!(source_section_for("73"), "Dedicated");
    }

    /// A non-numeric paragraph_number must classify as "Unknown" rather than
    /// silently defaulting (Rule 1: distinct observables for distinct
    /// states).
    #[test]
    fn source_section_unknown_for_non_numeric() {
        assert_eq!(source_section_for("abc"), "Unknown");
    }

    /// A range like "16-18" must classify by its starting paragraph (16 →
    /// Common). This is the case the in-code comment explicitly calls out.
    #[test]
    fn source_section_handles_range_prefix() {
        assert_eq!(source_section_for("16-18"), "Common");
    }

    /// Boundary pins so a future paragraph-range tweak shows up as a test
    /// failure rather than a silent classification drift.
    #[test]
    fn source_section_boundaries_pinned() {
        // Just below the Common range — pre-Common (¶6) is undefined territory
        // for the panel, classify as "Unknown".
        assert_eq!(source_section_for("6"), "Unknown");
        // Lower edge of Common.
        assert_eq!(source_section_for("7"), "Common");
        // Upper edge of Common.
        assert_eq!(source_section_for("71"), "Common");
        // Lower edge of Dedicated.
        assert_eq!(source_section_for("72"), "Dedicated");
    }

    /// Empty string is not a number; classify as "Unknown".
    #[test]
    fn source_section_empty_string() {
        assert_eq!(source_section_for(""), "Unknown");
    }
}
