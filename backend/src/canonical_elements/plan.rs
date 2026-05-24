//! Build the change plan: read current Neo4j state and diff it against the
//! parsed YAML, so the loader knows what to create/update/delete and the
//! report can show exactly what changed.
//!
//! ## Why diff before writing?
//!
//! A bare `MERGE … SET` can't tell you whether it created or merely re-set a
//! node, and it bumps `updated_at` every run. To report created-vs-updated
//! honestly *and* be genuinely idempotent (a second run = zero changes), we
//! compute a deterministic **content hash** of each node's managed properties,
//! store it on the node, and compare it on the next run. Unchanged nodes are
//! skipped entirely, so nothing is touched needlessly.

use super::diff::{self, content_hash};
use super::schema::{CountFile, CountMetadata, DeclarationDef, ElementDef, TheoryDef};
use super::state::{self, OrphanAttribution};
use super::{cypher, CanonicalLoaderError};
use neo4rs::Graph;
use std::collections::{HashMap, HashSet};
use tracing::instrument;

// Re-export the diffing value types so the rest of the crate keeps referring to
// them as `plan::ChangeKind` / `plan::NodePlan` / `plan::Tally`.
pub use super::diff::{ChangeKind, NodePlan, Tally};

type LoaderResult<T> = Result<T, CanonicalLoaderError>;

/// The full plan for one Count: the LegalCount property diff, the child node
/// plans, and the orphan deletions attributable to this Count.
#[derive(Debug, Clone)]
pub struct CountPlan {
    /// Full parsed metadata, retained so the LegalCount-update Cypher can be
    /// rebuilt at execution time (and so the report can read count_number/name).
    pub meta: CountMetadata,
    /// Names of LegalCount properties that differ (empty ⇒ no property update).
    pub changed_legal_count_props: Vec<String>,
    /// Pre-encoded JSON, reused at execution time to avoid re-serializing.
    pub controlling_authorities_json: String,
    pub doctrinal_requirements_json: Option<String>,
    pub elements: Vec<NodePlan<ElementDef>>,
    pub breach_theories: Vec<NodePlan<TheoryDef>>,
    pub improper_act_theories: Vec<NodePlan<TheoryDef>>,
    pub declarations: Vec<NodePlan<DeclarationDef>>,
    /// Orphan `Element` nodes (and their `PROVES_ELEMENT` edges) currently
    /// attached to this Count that the wipe will delete.
    pub orphan_elements: u64,
    pub orphan_proves_edges: u64,
}

/// The complete load plan across all Counts, plus the unattributed orphan
/// bucket (orphans not attached to any Count).
#[derive(Debug, Clone)]
pub struct LoadPlan {
    pub counts: Vec<CountPlan>,
    pub unattributed_orphan_elements: u64,
    pub unattributed_orphan_proves_edges: u64,
    /// Theory/declaration orphans (existing keys/ids absent from every YAML).
    /// These node types are introduced by this loader, so these are normally
    /// zero — non-zero means a previously-loaded theory/declaration was
    /// removed from the YAML and is being cleaned up.
    pub orphan_breach_theories: u64,
    pub orphan_improper_act_theories: u64,
    pub orphan_declarations: u64,
}

impl LoadPlan {
    /// All canonical Element ids across every Count — the keep-list the wipe
    /// uses to identify orphans.
    pub fn all_element_ids(&self) -> Vec<String> {
        self.counts
            .iter()
            .flat_map(|c| c.elements.iter().map(|e| e.def.id.clone()))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Plan assembly
// ---------------------------------------------------------------------------

/// Build node plans for a slice, hashing each via `fields` and classifying
/// against `existing`.
fn plan_nodes<T: Clone>(
    items: &[T],
    count_number: u32,
    existing: &HashMap<String, Option<String>>,
    key_of: impl Fn(&T) -> &str,
    fields_of: impl Fn(&T, u32) -> Vec<(&'static str, Option<String>)>,
) -> Vec<NodePlan<T>> {
    items
        .iter()
        .map(|item| {
            let hash = content_hash(&fields_of(item, count_number));
            let kind = diff::classify(key_of(item), existing, &hash);
            NodePlan {
                def: item.clone(),
                hash,
                kind,
            }
        })
        .collect()
}

/// Encode a list property to JSON, mapping failure to a typed error.
fn encode_json<T: serde::Serialize>(field: &'static str, value: &T) -> LoaderResult<String> {
    serde_json::to_string(value).map_err(|source| CanonicalLoaderError::JsonEncode {
        field: field.to_string(),
        source,
    })
}

/// JSON-encode the two LegalCount list properties. `doctrinal_requirements`
/// encodes to `None` when empty, so the property is left unset on Counts that
/// have none.
fn count_json(meta: &CountMetadata) -> LoaderResult<(String, Option<String>)> {
    let controlling = encode_json("controlling_authorities", &meta.controlling_authorities)?;
    let doctrinal = if meta.doctrinal_requirements.is_empty() {
        None
    } else {
        Some(encode_json(
            "doctrinal_requirements",
            &meta.doctrinal_requirements,
        )?)
    };
    Ok((controlling, doctrinal))
}

/// The four classified child-node groups for one Count.
struct ChildPlans {
    elements: Vec<NodePlan<ElementDef>>,
    breach: Vec<NodePlan<TheoryDef>>,
    improper: Vec<NodePlan<TheoryDef>>,
    declarations: Vec<NodePlan<DeclarationDef>>,
}

/// Classify every child node of a Count against the current graph hashes.
fn plan_children(file: &CountFile, hashes: &Hashes) -> ChildPlans {
    let cn = file.count.count_number;
    ChildPlans {
        elements: plan_nodes(
            &file.elements,
            cn,
            &hashes.elements,
            |e| e.id.as_str(),
            diff::element_fields,
        ),
        breach: plan_nodes(
            &file.breach_theories,
            cn,
            &hashes.breach,
            |t| t.key.as_str(),
            diff::theory_fields,
        ),
        improper: plan_nodes(
            &file.improper_act_theories,
            cn,
            &hashes.improper,
            |t| t.key.as_str(),
            diff::theory_fields,
        ),
        declarations: plan_nodes(
            &file.declarations_sought,
            cn,
            &hashes.declarations,
            |d| d.id.as_str(),
            diff::declaration_fields,
        ),
    }
}

/// Assemble the plan for one Count.
async fn build_count_plan(
    graph: &Graph,
    file: &CountFile,
    hashes: &Hashes,
    attribution: &OrphanAttribution,
) -> LoaderResult<CountPlan> {
    let meta = &file.count;
    let (controlling_authorities_json, doctrinal_requirements_json) = count_json(meta)?;
    let changed_legal_count_props = state::diff_legal_count(
        graph,
        meta,
        &controlling_authorities_json,
        &doctrinal_requirements_json,
    )
    .await?;
    let (orphan_elements, orphan_proves_edges) = attribution
        .per_count
        .get(&meta.count_number)
        .copied()
        .unwrap_or((0, 0));
    let children = plan_children(file, hashes);

    Ok(CountPlan {
        meta: meta.clone(),
        changed_legal_count_props,
        controlling_authorities_json,
        doctrinal_requirements_json,
        elements: children.elements,
        breach_theories: children.breach,
        improper_act_theories: children.improper,
        declarations: children.declarations,
        orphan_elements,
        orphan_proves_edges,
    })
}

/// Existing content hashes for every managed node type.
struct Hashes {
    elements: HashMap<String, Option<String>>,
    breach: HashMap<String, Option<String>>,
    improper: HashMap<String, Option<String>>,
    declarations: HashMap<String, Option<String>>,
}

/// Read the stored content hashes for all four managed node types up front.
async fn read_all_hashes(graph: &Graph) -> LoaderResult<Hashes> {
    Ok(Hashes {
        elements: state::read_hashes(
            graph,
            cypher::fetch_element_hashes(),
            "fetch_element_hashes",
        )
        .await?,
        breach: state::read_hashes(
            graph,
            cypher::fetch_breach_theory_hashes(),
            "fetch_breach_theory_hashes",
        )
        .await?,
        improper: state::read_hashes(
            graph,
            cypher::fetch_improper_act_theory_hashes(),
            "fetch_improper_act_theory_hashes",
        )
        .await?,
        declarations: state::read_hashes(
            graph,
            cypher::fetch_declaration_hashes(),
            "fetch_declaration_hashes",
        )
        .await?,
    })
}

/// Build the full [`LoadPlan`] from the parsed Count files.
///
/// Reads all current state up front (hashes + orphan attribution), verifies
/// each prerequisite `LegalCount` exists (via `state::diff_legal_count`), then
/// diffs each Count. Performs no writes.
#[instrument(skip(graph, files), fields(step = "build_plan", count_files = files.len()))]
pub async fn build_plan(graph: &Graph, files: &[CountFile]) -> LoaderResult<LoadPlan> {
    let hashes = read_all_hashes(graph).await?;

    let yaml_element_ids: Vec<String> = files
        .iter()
        .flat_map(|f| f.elements.iter().map(|e| e.id.clone()))
        .collect();
    let attribution = state::read_orphan_attribution(graph, yaml_element_ids).await?;

    let mut counts = Vec::with_capacity(files.len());
    for file in files {
        counts.push(build_count_plan(graph, file, &hashes, &attribution).await?);
    }

    // Keep-sets of every theory key / declaration id present in the YAML.
    // Anything in the graph but not in these sets is an orphan to be wiped.
    let breach_kept: HashSet<&str> = files
        .iter()
        .flat_map(|f| f.breach_theories.iter().map(|t| t.key.as_str()))
        .collect();
    let improper_kept: HashSet<&str> = files
        .iter()
        .flat_map(|f| f.improper_act_theories.iter().map(|t| t.key.as_str()))
        .collect();
    let decl_kept: HashSet<&str> = files
        .iter()
        .flat_map(|f| f.declarations_sought.iter().map(|d| d.id.as_str()))
        .collect();

    Ok(LoadPlan {
        counts,
        unattributed_orphan_elements: attribution.unattributed.0,
        unattributed_orphan_proves_edges: attribution.unattributed.1,
        orphan_breach_theories: count_orphans(&hashes.breach, &breach_kept),
        orphan_improper_act_theories: count_orphans(&hashes.improper, &improper_kept),
        orphan_declarations: count_orphans(&hashes.declarations, &decl_kept),
    })
}

/// Count existing nodes whose key is absent from the YAML keep-set.
fn count_orphans(existing: &HashMap<String, Option<String>>, kept: &HashSet<&str>) -> u64 {
    existing
        .keys()
        .filter(|k| !kept.contains(k.as_str()))
        .count() as u64
}
