//! Orchestration: read the YAML files, validate them, build the change plan,
//! execute it (unless dry-run), and return the report.
//!
//! Execution order, and why:
//! 1. **Global orphan wipe** — `DETACH DELETE` every Element/theory/declaration
//!    not present in the YAML. This is inherently global (it can't be scoped to
//!    one Count), so it runs first, in its own auto-committed statements, and
//!    removes the wrong Elements plus their `PROVES_ELEMENT` edges.
//! 2. **Per-Count upserts** — each Count's LegalCount update and child upserts
//!    run in their own transaction, so a partial failure can't leave a single
//!    Count half-loaded. Unchanged nodes are skipped (idempotency).
//!
//! Canonical Element ids (`element-1-1`, …) never collide with the wrong
//! Elements' ids, so wiping before upserting is safe.

use super::authored;
use super::plan::{self, ChangeKind, CountPlan, LoadPlan, NodePlan};
use super::report::ChangeReport;
use super::schema::CountFile;
use super::{cypher, CanonicalLoaderError};
use neo4rs::Graph;
use sqlx::PgPool;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::{info, instrument};

type LoaderResult<T> = Result<T, CanonicalLoaderError>;

/// Inputs for one loader invocation.
#[derive(Debug, Clone)]
pub struct RunOptions {
    /// Directory holding the `count_N_*.yaml` files.
    pub yaml_dir: PathBuf,
    /// When true, build and report the plan but write nothing (neither
    /// Postgres nor Neo4j).
    pub dry_run: bool,
    /// Disable ANSI color in the report.
    pub no_color: bool,
    /// Pipeline-DB pool for the Tier-1 authored-entity writes. `None` ⇒ the
    /// loader runs Neo4j-only (the Neo4j-focused integration tests pass
    /// `None`); the binary always supplies it from `--database-url` /
    /// `PIPELINE_DATABASE_URL`. CLI requiredness lives at the binary layer;
    /// the library stays runnable without Postgres for those tests.
    pub pipeline_pool: Option<PgPool>,
    /// Case slug written to `authored_entities` / `authored_relationships`.
    /// `None` alongside `pipeline_pool = None`; required at the CLI layer
    /// (`--case-slug`).
    pub case_slug: Option<String>,
}

/// Read → validate → plan → (execute) → report.
///
/// This is the single entry point the binary and the integration tests call.
/// On `dry_run`, no transaction is ever opened.
#[instrument(skip(graph, opts), fields(step = "run", yaml_dir = %opts.yaml_dir.display(), dry_run = opts.dry_run))]
pub async fn run(graph: &Graph, opts: RunOptions) -> LoaderResult<ChangeReport> {
    let files = read_count_files(&opts.yaml_dir)?;
    info!(
        file_count = files.len(),
        "parsed canonical Element YAML files"
    );
    validate(&files)?;

    // Tier-1 Postgres writes happen BEFORE any Neo4j writes (Option A:
    // Postgres is the system of record, Neo4j the operational copy). When
    // no pool/slug is configured (the Neo4j-only integration tests) this is
    // skipped and the authored section is omitted from the report.
    let authored = match (opts.pipeline_pool.as_ref(), opts.case_slug.as_deref()) {
        (Some(pool), Some(case_slug)) => {
            let counts = authored::count_authored(&files);
            if opts.dry_run {
                info!(
                    authored_entities = counts.entities,
                    authored_relationships = counts.relationships,
                    "dry-run: skipping authored-entity Postgres writes"
                );
            } else {
                authored::write_authored_entities(pool, case_slug, &files).await?;
                info!(
                    authored_entities = counts.entities,
                    authored_relationships = counts.relationships,
                    "wrote authored entities + relationships to Postgres"
                );
            }
            Some(counts)
        }
        _ => {
            info!(
                "Postgres not configured (no --database-url / --case-slug); \
                 skipping authored-entity writes"
            );
            None
        }
    };

    let plan = plan::build_plan(graph, &files).await?;

    if opts.dry_run {
        info!("dry-run: skipping all writes");
    } else {
        execute(graph, &plan).await?;
        info!("canonical Element load complete");
    }

    Ok(ChangeReport::new(
        plan,
        opts.dry_run,
        opts.no_color,
        authored,
    ))
}

/// Read and parse every `*.yaml` / `*.yml` file in `yaml_dir`, sorted by Count.
///
/// Public so integration tests can exercise parsing without a Neo4j connection.
pub fn read_count_files(yaml_dir: &Path) -> LoaderResult<Vec<CountFile>> {
    let dir = std::fs::read_dir(yaml_dir).map_err(|source| CanonicalLoaderError::YamlDirRead {
        path: yaml_dir.to_path_buf(),
        source,
    })?;

    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in dir {
        let entry = entry.map_err(|source| CanonicalLoaderError::YamlDirRead {
            path: yaml_dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let is_yaml = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e == "yaml" || e == "yml");
        if is_yaml {
            paths.push(path);
        }
    }
    paths.sort();

    if paths.is_empty() {
        return Err(CanonicalLoaderError::Validation(format!(
            "No .yaml files found in {}",
            yaml_dir.display()
        )));
    }

    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        let content =
            std::fs::read_to_string(&path).map_err(|source| CanonicalLoaderError::FileRead {
                path: path.clone(),
                source,
            })?;
        let file: CountFile =
            serde_yaml::from_str(&content).map_err(|source| CanonicalLoaderError::Parse {
                path: path.clone(),
                source,
            })?;
        files.push(file);
    }
    // Sort by count_number so report and execution proceed in Count order
    // regardless of filename.
    files.sort_by_key(|f| f.count.count_number);
    Ok(files)
}

/// Cross-file consistency checks. All failures map to exit code 4.
///
/// Public so integration tests can exercise validation without a Neo4j
/// connection.
pub fn validate(files: &[CountFile]) -> LoaderResult<()> {
    let mut seen_counts = HashSet::new();
    for f in files {
        if !seen_counts.insert(f.count.count_number) {
            return Err(CanonicalLoaderError::Validation(format!(
                "Duplicate count_number {} across YAML files",
                f.count.count_number
            )));
        }
    }

    // Element ids must be globally unique (they are the cross-Count merge key).
    let mut seen_ids = HashSet::new();
    for f in files {
        for e in &f.elements {
            if !seen_ids.insert(e.id.as_str()) {
                return Err(CanonicalLoaderError::Validation(format!(
                    "Duplicate Element id '{}' (Element ids must be globally unique)",
                    e.id
                )));
            }
        }
    }

    // Theory keys / declaration ids must be unique within their Count.
    for f in files {
        let cn = f.count.count_number;
        check_unique(
            f.breach_theories.iter().map(|t| t.key.as_str()),
            cn,
            "breach theory key",
        )?;
        check_unique(
            f.improper_act_theories.iter().map(|t| t.key.as_str()),
            cn,
            "improper-act theory key",
        )?;
        check_unique(
            f.declarations_sought.iter().map(|d| d.id.as_str()),
            cn,
            "declaration id",
        )?;
    }
    Ok(())
}

/// Assert that an iterator of keys has no duplicates within a Count.
fn check_unique<'a>(
    items: impl Iterator<Item = &'a str>,
    count_number: u32,
    kind: &str,
) -> LoaderResult<()> {
    let mut seen = HashSet::new();
    for key in items {
        if !seen.insert(key) {
            return Err(CanonicalLoaderError::Validation(format!(
                "Duplicate {kind} '{key}' within Count {count_number}"
            )));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Execution
// ---------------------------------------------------------------------------

/// Wipe orphans, then upsert each Count.
#[instrument(skip(graph, plan), fields(step = "execute", count_count = plan.counts.len()))]
async fn execute(graph: &Graph, plan: &LoadPlan) -> LoaderResult<()> {
    wipe_orphans(graph, plan).await?;
    for count in &plan.counts {
        apply_count(graph, count).await?;
    }
    Ok(())
}

/// `DETACH DELETE` every managed node whose key is absent from the YAML.
#[instrument(skip(graph, plan), fields(step = "wipe_orphans"))]
async fn wipe_orphans(graph: &Graph, plan: &LoadPlan) -> LoaderResult<()> {
    let breach_keys: Vec<String> = plan
        .counts
        .iter()
        .flat_map(|c| c.breach_theories.iter().map(|t| t.def.key.clone()))
        .collect();
    let improper_keys: Vec<String> = plan
        .counts
        .iter()
        .flat_map(|c| c.improper_act_theories.iter().map(|t| t.def.key.clone()))
        .collect();
    let declaration_ids: Vec<String> = plan
        .counts
        .iter()
        .flat_map(|c| c.declarations.iter().map(|d| d.def.id.clone()))
        .collect();

    run_wipe(
        graph,
        cypher::wipe_orphan_elements(plan.all_element_ids()),
        "wipe_orphan_elements",
    )
    .await?;
    run_wipe(
        graph,
        cypher::wipe_orphan_breach_theories(breach_keys),
        "wipe_orphan_breach_theories",
    )
    .await?;
    run_wipe(
        graph,
        cypher::wipe_orphan_improper_act_theories(improper_keys),
        "wipe_orphan_improper_act_theories",
    )
    .await?;
    run_wipe(
        graph,
        cypher::wipe_orphan_declarations(declaration_ids),
        "wipe_orphan_declarations",
    )
    .await?;
    Ok(())
}

/// Run one wipe statement (auto-committed) and log how many nodes it removed.
async fn run_wipe(graph: &Graph, q: neo4rs::Query, op: &'static str) -> LoaderResult<()> {
    let mut stream = graph
        .execute(q)
        .await
        .map_err(CanonicalLoaderError::exec(op))?;
    let deleted: i64 = match stream
        .next()
        .await
        .map_err(CanonicalLoaderError::exec(op))?
    {
        Some(row) => row
            .get("deleted")
            .map_err(CanonicalLoaderError::decode(op))?,
        None => 0,
    };
    info!(operation = op, deleted, "orphan wipe complete");
    Ok(())
}

/// Apply one Count inside a single transaction: update the LegalCount (if its
/// properties changed), then upsert each created/updated child node.
#[instrument(skip(graph, count), fields(step = "apply_count", count_number = count.meta.count_number))]
async fn apply_count(graph: &Graph, count: &CountPlan) -> LoaderResult<()> {
    // Why: one transaction per Count keeps each Count all-or-nothing. neo4rs
    // `Txn` has no `Drop`-based rollback, and we only call `commit()` on the
    // success path; if any `?` below returns early the `Txn` is dropped without
    // a COMMIT, so its writes are never persisted and Neo4j rolls the open
    // transaction back when the pooled connection resets. We therefore omit an
    // explicit `rollback()` on the error path.
    let mut txn = graph
        .start_txn()
        .await
        .map_err(CanonicalLoaderError::exec("start_txn"))?;

    if !count.changed_legal_count_props.is_empty() {
        let q = cypher::upsert_legal_count(
            &count.meta,
            count.controlling_authorities_json.clone(),
            count.doctrinal_requirements_json.clone(),
        );
        txn.run(q)
            .await
            .map_err(CanonicalLoaderError::exec("upsert_legal_count"))?;
    }

    let cn = count.meta.count_number;

    // Stamp the cross-tier `id` (`count-{N}`) on the LegalCount node
    // unconditionally — see `cypher::set_legal_count_id`. Done inside the
    // per-Count transaction so it's atomic with the property update, and
    // independent of the property-diff guard above so the id is present
    // even on a run where no managed property changed.
    let count_id = authored::legal_count_entity_id(cn);
    txn.run(cypher::set_legal_count_id(cn, &count_id))
        .await
        .map_err(CanonicalLoaderError::exec("set_legal_count_id"))?;

    for e in writable(&count.elements) {
        txn.run(cypher::upsert_element(cn, &e.def, &e.hash))
            .await
            .map_err(CanonicalLoaderError::exec("upsert_element"))?;
    }
    for t in writable(&count.breach_theories) {
        txn.run(cypher::upsert_breach_theory(cn, &t.def, &t.hash))
            .await
            .map_err(CanonicalLoaderError::exec("upsert_breach_theory"))?;
    }
    for t in writable(&count.improper_act_theories) {
        txn.run(cypher::upsert_improper_act_theory(cn, &t.def, &t.hash))
            .await
            .map_err(CanonicalLoaderError::exec("upsert_improper_act_theory"))?;
    }
    for d in writable(&count.declarations) {
        txn.run(cypher::upsert_declaration(cn, &d.def, &d.hash))
            .await
            .map_err(CanonicalLoaderError::exec("upsert_declaration"))?;
    }

    txn.commit()
        .await
        .map_err(CanonicalLoaderError::exec("commit"))?;
    Ok(())
}

/// Iterate only the nodes that actually need a write (created or updated),
/// skipping unchanged ones so a re-run touches nothing.
fn writable<T>(nodes: &[NodePlan<T>]) -> impl Iterator<Item = &NodePlan<T>> {
    nodes.iter().filter(|n| n.kind != ChangeKind::Unchanged)
}
