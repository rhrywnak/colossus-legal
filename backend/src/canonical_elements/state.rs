//! Reading current Neo4j state for the diff.
//!
//! Split out of [`super::plan`] (to keep each module within the 300-line
//! limit), this holds every read query the planner issues *before* any write:
//! per-node content hashes, the orphan-Element attribution, and each
//! LegalCount's managed properties.

use super::cypher;
use super::schema::CountMetadata;
use super::CanonicalLoaderError;
use neo4rs::Graph;
use std::collections::HashMap;

type LoaderResult<T> = Result<T, CanonicalLoaderError>;

/// Read `key → content_hash` for every existing node of one type.
///
/// A `None` hash means the node predates content hashing (e.g. a wrong Element
/// from the old extraction) — the planner treats that as "needs update", though
/// such nodes are usually orphans bound for deletion anyway.
pub(crate) async fn read_hashes(
    graph: &Graph,
    q: neo4rs::Query,
    operation: &'static str,
) -> LoaderResult<HashMap<String, Option<String>>> {
    let mut stream = graph
        .execute(q)
        .await
        .map_err(CanonicalLoaderError::exec(operation))?;
    let mut out = HashMap::new();
    while let Some(row) = stream
        .next()
        .await
        .map_err(CanonicalLoaderError::exec(operation))?
    {
        let key: String = row
            .get("key")
            .map_err(CanonicalLoaderError::decode(operation))?;
        let hash: Option<String> = row
            .get("hash")
            .map_err(CanonicalLoaderError::decode(operation))?;
        out.insert(key, hash);
    }
    Ok(out)
}

/// Per-Count and unattributed orphan-Element/edge counts.
#[derive(Default)]
pub(crate) struct OrphanAttribution {
    /// `count_number → (orphan Elements, incoming BEARS_ON edges)`.
    pub(crate) per_count: HashMap<u32, (u64, u64)>,
    /// Orphans not attached to any Count: `(elements, edges)`.
    pub(crate) unattributed: (u64, u64),
}

/// Pre-query (before any deletion) how many orphan Elements and incoming
/// `BEARS_ON` edges hang off each Count, plus the unattributed bucket.
pub(crate) async fn read_orphan_attribution(
    graph: &Graph,
    yaml_element_ids: Vec<String>,
) -> LoaderResult<OrphanAttribution> {
    const OP: &str = "orphan_element_attribution";
    let q = cypher::orphan_element_attribution(yaml_element_ids);
    let mut stream = graph
        .execute(q)
        .await
        .map_err(CanonicalLoaderError::exec(OP))?;
    let mut attribution = OrphanAttribution::default();
    while let Some(row) = stream
        .next()
        .await
        .map_err(CanonicalLoaderError::exec(OP))?
    {
        let count_number: Option<i64> = row
            .get("count_number")
            .map_err(CanonicalLoaderError::decode(OP))?;
        let elements: i64 = row
            .get("orphan_elements")
            .map_err(CanonicalLoaderError::decode(OP))?;
        let edges: i64 = row
            .get("proves_edges")
            .map_err(CanonicalLoaderError::decode(OP))?;
        match count_number {
            Some(n) => {
                attribution
                    .per_count
                    .insert(n as u32, (elements as u64, edges as u64));
            }
            None => attribution.unattributed = (elements as u64, edges as u64),
        }
    }
    Ok(attribution)
}

/// The loader-managed properties of a LegalCount, all normalized to
/// `Option<String>` so they can be compared uniformly against desired values.
#[derive(Default)]
struct LegalCountCurrent {
    burden_of_proof: Option<String>,
    template_name: Option<String>,
    m_civ_ji_reference: Option<String>,
    controlling_authorities_json: Option<String>,
    doctrinal_requirements_json: Option<String>,
    chuck_review_required: Option<String>,
    chuck_review_note: Option<String>,
    special_note: Option<String>,
}

/// Read one LegalCount's managed properties. `None` ⇒ the Count does not exist
/// (a hard error — the case-structuring pipeline must run first).
async fn read_legal_count_state(
    graph: &Graph,
    count_number: u32,
) -> LoaderResult<LegalCountCurrent> {
    const OP: &str = "fetch_legal_count_state";
    let mut stream = graph
        .execute(cypher::fetch_legal_count_state(count_number))
        .await
        .map_err(CanonicalLoaderError::exec(OP))?;
    let row = stream
        .next()
        .await
        .map_err(CanonicalLoaderError::exec(OP))?
        .ok_or(CanonicalLoaderError::MissingLegalCount { count_number })?;

    let s = |col: &'static str| -> LoaderResult<Option<String>> {
        row.get::<Option<String>>(col)
            .map_err(CanonicalLoaderError::decode(OP))
    };
    let review: Option<bool> = row
        .get("chuck_review_required")
        .map_err(CanonicalLoaderError::decode(OP))?;

    Ok(LegalCountCurrent {
        burden_of_proof: s("burden_of_proof")?,
        template_name: s("template_name")?,
        m_civ_ji_reference: s("m_civ_ji_reference")?,
        controlling_authorities_json: s("controlling_authorities_json")?,
        doctrinal_requirements_json: s("doctrinal_requirements_json")?,
        chuck_review_required: review.map(|b| b.to_string()),
        chuck_review_note: s("chuck_review_note")?,
        special_note: s("special_note")?,
    })
}

/// Build the `(name, desired, current)` comparison rows for one LegalCount,
/// all normalized to `Option<String>`. The third element borrows `current`.
fn legal_count_comparisons<'a>(
    meta: &CountMetadata,
    current: &'a LegalCountCurrent,
    controlling_authorities_json: &str,
    doctrinal_requirements_json: &Option<String>,
) -> [(&'static str, Option<String>, &'a Option<String>); 8] {
    [
        (
            "burden_of_proof",
            Some(meta.burden_of_proof.clone()),
            &current.burden_of_proof,
        ),
        (
            "template_name",
            Some(meta.template_name.clone()),
            &current.template_name,
        ),
        (
            "m_civ_ji_reference",
            meta.m_civ_ji_reference.clone(),
            &current.m_civ_ji_reference,
        ),
        (
            "controlling_authorities_json",
            Some(controlling_authorities_json.to_string()),
            &current.controlling_authorities_json,
        ),
        (
            "doctrinal_requirements_json",
            doctrinal_requirements_json.clone(),
            &current.doctrinal_requirements_json,
        ),
        (
            "chuck_review_required",
            meta.chuck_review_required.map(|b| b.to_string()),
            &current.chuck_review_required,
        ),
        (
            "chuck_review_note",
            meta.chuck_review_note.clone(),
            &current.chuck_review_note,
        ),
        (
            "special_note",
            meta.special_note.clone(),
            &current.special_note,
        ),
    ]
}

/// Compare desired LegalCount properties against the current node, returning
/// the names of the properties that differ. An empty result means the
/// LegalCount needs no property update.
pub(crate) async fn diff_legal_count(
    graph: &Graph,
    meta: &CountMetadata,
    controlling_authorities_json: &str,
    doctrinal_requirements_json: &Option<String>,
) -> LoaderResult<Vec<String>> {
    let current = read_legal_count_state(graph, meta.count_number).await?;
    let comparisons = legal_count_comparisons(
        meta,
        &current,
        controlling_authorities_json,
        doctrinal_requirements_json,
    );
    // A property is "changed" iff its desired value differs from the current.
    Ok(comparisons
        .into_iter()
        .filter(|(_, desired, current)| desired != *current)
        .map(|(name, _, _)| name.to_string())
        .collect())
}
