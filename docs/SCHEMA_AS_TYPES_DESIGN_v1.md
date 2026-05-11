# Schema as Types — Design v1

**Status:** Design proposal. NOT a commitment. All scope estimates are estimates.

**Date:** 2026-05-11

**Motivation:** The night of 2026-05-11, a v5.1 complaint extraction (120 Allegations, 4 LegalCounts, 15 Elements, 15 Harms, 12 Parties — all extracted, all ingested) displayed as empty pages on the Allegations and Evidence tabs. Root cause: the extraction layer was renamed `ComplaintAllegation` → `Allegation` and the direct `:SUPPORTS` edge was replaced by a two-hop `PROVES_ELEMENT`/`HAS_ELEMENT` path through a new `Element` node, but **the page-query layer was still hardcoded to the v4 names**. Eighteen production files needed manual migration across commits `2b51d38` (11 files) and `4ba332d` (7 files), plus a follow-up grep audit that surfaced four more dispatch-table sites still pending.

The schema lives in YAML (`backend/extraction_schemas/complaint_schema_v5_1.yaml`). Every consumer copies the strings into Cypher / Rust dispatch / DTO field names by hand. The compiler has zero knowledge of the schema. A schema change is silent until a user navigates to a page that depends on the old shape.

This document proposes moving schema into Rust types so the compiler enforces consumer–schema alignment. After the refactor, the same v4→v5.1 rename would be a single edit to one file plus a `cargo build` that fails everywhere the rename invalidates a query, with type-system pointers to each location.

---

## 1. Current state

### 1.1 Where schema lives

| Artifact | Location | Form |
|---|---|---|
| Authoritative schema | `backend/extraction_schemas/*.yaml` (8 schemas as of beta.222 — affidavit_v4, brief_v4, complaint_v4, complaint_v5, **complaint_schema_v5_1** (active default), court_ruling_v4, discovery_response_v4, general_legal, motion_v4) | YAML, loaded at runtime by `colossus_extract::ExtractionSchema::from_file` |
| Schema struct | `colossus-rs/colossus-extract/src/schema.rs:91-124` (`ExtractionSchema`), `:127-159` (`EntityTypeConfig`), `:162-174` (`RelationshipTypeConfig`), `:177-193` (`PropertyConfig`) | `Vec<EntityTypeConfig>`, `Vec<RelationshipTypeConfig>`, `Vec<PatternConfig>` — flexible but **untyped**. Entity names are `String`. |
| Profile (selects schema) | `backend/profiles/complaint_v5_1.yaml` | YAML, references schema file by filename |
| LLM extraction templates | `backend/extraction_templates/complaint_pass1_v5_1.md`, `complaint_pass2_v5_1.md` | Markdown with hardcoded entity/relationship names and JSON few-shot examples |

The schema is read by:
- `colossus-extract::ExtractionSchema::from_file` at extraction time (Pass-1 prompt assembly)
- Frontend list-schemas endpoint (`backend/src/api/pipeline/config_endpoints/schemas.rs`) to populate the upload dropdown

Everything else — every query, every Rust struct field name, every DTO — uses string literals or hand-coded Rust types that are coincidentally aligned (until they're not).

### 1.2 Where schema-derived strings appear in code

Inventoried across the v4→v5.1 migration. Counts are **distinct hardcoded reference sites in production code**, excluding test fixtures and pure documentation comments.

#### Labels (`(:Allegation)`, `(:LegalCount)`, etc.)

| Pattern | Sites |
|---|---|
| `:ComplaintAllegation` in Cypher MATCH | ~14 production sites pre-migration (commits 2b51d38 + 4ba332d migrated all). Examples: `repositories/allegation_repository.rs:58`, `:74`, `repositories/allegation_detail_repository.rs:31,37,46,71`, `repositories/case_summary_repository.rs:157,198,206`, `repositories/evidence_chain_repository.rs:45,46`, `repositories/motion_claim_repository.rs:48`, `repositories/case_repository.rs:207`, `repositories/decomposition_repository.rs:62,76`, `repositories/document_repository.rs:217`, `repositories/embedding_repository.rs:71,72`, `repositories/graph_repository.rs:47,58`, `repositories/analysis_repository.rs:88,235,236`, `services/graph_expansion_queries.rs:76,135,172,195,293,349`, `services/graph_expansion_minor.rs:26,59`, `api/admin_document_evidence_queries.rs:82,83`, `api/admin_evidence.rs:274,278`, `api/pipeline/delete.rs:283` |
| `"ComplaintAllegation"` as `colossus_graph::get_nodes_by_label` arg | 2 sites: `repositories/case_summary_repository.rs:157`, `repositories/case_repository.rs:207` |
| `"ComplaintAllegation"` as `entity_type` dispatch key | 4 sites: `services/graph_expander.rs:107,180,240`, `services/embedding_text.rs:34`. Plus `models/document_status.rs:110` `pub const ENTITY_COMPLAINT_ALLEGATION: &str = "ComplaintAllegation"`. (Test fixtures: `verify.rs:842,868,883`, `completeness_helpers.rs:245`, `pipeline_repository/extraction.rs:1592,1602`, plus 10 sites in `ingest_helpers.rs:867+` test module.) |
| `:LegalCount` in Cypher | ~12 production sites |
| `:Element` | 6 production sites (new in v5) |
| `:Evidence`, `:MotionClaim`, `:Harm`, `:Document`, `:Person`, `:Organization`, `:Case` | ~40 production sites combined |

**Total entity-label string occurrences in production code: ~70 sites.**

#### Relationship types (`-[:SUPPORTS]->`, `-[:PROVES_ELEMENT]->`, etc.)

| Pattern | Sites |
|---|---|
| `:SUPPORTS` (v4 direct Allegation→LegalCount) | 6 production sites pre-migration (all migrated to two-hop in 2b51d38/4ba332d) |
| `:PROVES_ELEMENT` (v5.1) | 6 sites added by the migration |
| `:HAS_ELEMENT` | 6 sites added |
| `:PROVES` (MotionClaim→Allegation) | 7 production sites |
| `:CHARACTERIZES`, `:REBUTS`, `:CONTRADICTS`, `:CORROBORATES`, `:CAUSED_BY`, `:EVIDENCED_BY`, `:DAMAGES_FOR`, `:SUFFERED_BY`, `:CONTAINED_IN`, `:STATED_BY`, `:ABOUT`, `:APPEARS_IN`, `:RELIES_ON`, `:HAS_ELEMENT`, `:ANCHORED_IN`, `:DERIVED_FROM` | ~55 production sites combined |
| Synthetic edge labels rendered to UI (e.g. `"SUPPORTS"` literal pushed into `ExpandedRelationship::new`) | 4 sites: `services/graph_expansion_queries.rs:242`, `repositories/graph_repository.rs:108` (inside `nodes_map.entry().or_insert()`), plus 2 in `services/graph_expansion_minor.rs` |

**Total relationship-type string occurrences in production code: ~75 sites.**

#### Property names (`a.paragraph_number`, `lc.count_number`, etc.)

| Pattern | Sites |
|---|---|
| Allegation properties (`paragraph_number`, `summary`, `kind`, `category`, `severity`, `applies_to`, `amount`, `event_date`, `title`, `verbatim_quote`, `grounding_status`, `id`, `source_document`) | ~80 production sites across Cypher RETURN clauses and `row.get(...)` calls |
| LegalCount properties (`count_name`, `count_number`, `legal_basis`, `legal_theory`, `paragraph_range`, `statutory_anchor`, `damages_claimed`, `applies_to`, plus inherited) | ~50 sites |
| Element properties (`element_name`, `parent_count_id`, `anchor_paragraph_numbers`, `order_in_count`) | ~20 sites |
| Harm properties (`description`, `kind`, `subcategory`, `amount`, `date`) | ~30 sites |
| Other entity properties (Evidence, MotionClaim, Party as Person/Organization, Document) | ~120 sites |

**Total property-name string occurrences in production code: ~300 sites.**

#### Cross-reference: schema YAML → consumer

For one entity (Allegation v5.1), the YAML declares:

```yaml
- name: Allegation
  required: true
  min_count: 1
  grounding_mode: verbatim
  category: evidence
  properties:
    - { name: paragraph_number, type: string, required: true }
    - { name: kind, type: string, required: true }
    - { name: summary, type: string, required: true }
    - { name: category, type: string }
    - { name: severity, type: integer }
    - { name: applies_to, type: string }
    - { name: amount, type: string }
    - { name: event_date, type: string }
```

8 properties. Each appears in:
- LLM prompt assembly (`colossus_extract` reads the YAML and renders a JSON-shaped guide for the model)
- ~5–10 Cypher RETURN clauses per property across the repositories
- ~5–10 `row.get("<name>")` Rust calls
- Frontend TypeScript types (`AllegationDto` in `frontend/src/services/allegations.ts`)
- The Neo4j ingest writer at `api/pipeline/ingest_helpers.rs` (writes the property to `(n:Allegation { ... })` via parameterized Cypher)

Order of magnitude: **each schema property has 15–25 string-literal touchpoints in the codebase.** Rename one property → break 15–25 places, no compile-time signal.

### 1.3 The failure mode this enables

**Concrete instance — the symptom of 2026-05-11:**

Upload of the Awad complaint via the v5.1 profile ran the full pipeline cleanly. Postgres `extraction_items` recorded 120 Allegations / 4 LegalCounts / 15 Elements. Neo4j `MATCH (n) WHERE n.source_document='doc-awad-…' RETURN labels(n)[0], count(*)` confirmed the same counts under the v5.1 labels:

```
"Allegation"   115   (115 of 120 passed the derived_invalid filter)
"Element"      15
"Harm"         13
"Person"       9
"LegalCount"   4
"Organization" 3
```

Frontend Allegations page: "No allegations found." Frontend Allegations-page header per Count: "0 allegations."

Why: `repositories/allegation_repository.rs:51-77` had `MATCH (a) WHERE labels(a)[0] = $allegation_label` with `$allegation_label = "ComplaintAllegation"` (v4 name) and `(a)-[:SUPPORTS]->(c)` (v4 direct edge). The first predicate excluded every v5.1 node; even if it hadn't, the second predicate referenced an edge type that v5.1 doesn't emit.

The schema change had been in place for weeks. The bug was invisible until a v5.1 document actually landed in Neo4j. The compiler had no opinion. No test caught it. No type signature linked the schema's `name: Allegation` to the consumer's `"ComplaintAllegation"` literal.

**Why this keeps happening:**

The schema is the contract. Today, the contract is enforced by:
- Human eyes during code review
- Manual grep when something looks broken
- User reports of empty pages

It is NOT enforced by:
- The compiler
- Static analysis
- Schema-aware tests

Every schema bump (v3 → v4 → v5 → v5.1) has produced the same defect class. The instinct of "v6 will be different" is unsupported by the pattern.

---

## 2. Target state

**One Rust module owns the schema. Everything else derives from it.** Compile-time linkage from schema to query to DTO to frontend type.

### 2.1 Single-source-of-truth module

Proposed location: **`backend/src/schema/v5_1.rs`** (or `colossus_legal::schema::v5_1` if extracted to a workspace crate). One module per schema version. Active version exported as `pub use v5_1::*;` from `backend/src/schema/mod.rs`.

Concrete proposal — Rust shape for v5.1 Allegation:

```rust
use serde::{Deserialize, Serialize};

/// v5.1 Allegation entity.
///
/// Source of truth for: the Neo4j label, the entity_type string emitted
/// by the LLM extractor, the property set, the YAML schema generated for
/// `colossus-extract`, the frontend `AllegationDto`, and every query in
/// the repositories layer.
///
/// A rename here would produce ~70 compile errors (one per current
/// hardcoded string-literal site) that the type system points to.
#[derive(Debug, Clone, Serialize, Deserialize, Entity)]
#[entity(
    label = "Allegation",
    required = true,
    min_count = 1,
    grounding_mode = "verbatim",
    category = "evidence",
)]
pub struct Allegation {
    /// Stable id assigned by the ingest layer at write time.
    #[entity(id)]
    pub id: NodeId<Self>,

    /// Paragraph number in the complaint, e.g. "16" or "16-18".
    /// Required by schema (LLM must emit it).
    #[entity(property, required)]
    pub paragraph_number: String,

    /// "common_allegation" or "count_section" — distinguishes factual
    /// narrative paragraphs from in-count restatements.
    #[entity(property, required)]
    pub kind: AllegationKind,

    /// One-sentence summary of what is alleged.
    #[entity(property, required)]
    pub summary: String,

    /// Verbatim quote of the source paragraph text.
    #[entity(property, required, grounding)]
    pub verbatim_quote: String,

    /// Short LLM-authored label, e.g. "Pattern of Unauthorized
    /// Withdrawals". Used by the frontend as the display heading.
    #[entity(property)]
    pub title: String,

    /// financial, procedural, defamation, fiduciary, conversion,
    /// abuse_of_process, fraud, negligence, breach_of_duty
    #[entity(property)]
    pub category: Option<String>,

    /// 1-10 scale of severity.
    #[entity(property)]
    pub severity: Option<i32>,

    /// Party/parties this allegation is against (comma-separated names).
    #[entity(property)]
    pub applies_to: Option<String>,

    /// Dollar amount if specified.
    #[entity(property)]
    pub amount: Option<String>,

    /// Date of the alleged conduct.
    #[entity(property)]
    pub event_date: Option<String>,

    /// Ingest-populated provenance properties.
    #[entity(system_property)]
    pub source_document: DocumentId,

    #[entity(system_property)]
    pub grounding_status: GroundingStatus,

    #[entity(system_property)]
    pub page_number: Option<i32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, EntityEnum)]
#[serde(rename_all = "snake_case")]
pub enum AllegationKind {
    CommonAllegation,
    CountSection,
}
```

### 2.2 Relationships as a typed enum

```rust
#[derive(Debug, Clone, RelationshipSet)]
pub enum Relationship {
    /// Element-level granularity replacing v4's flat SUPPORTS.
    /// THE central reasoning relationship of v5+.
    #[relationship(name = "PROVES_ELEMENT", from = Allegation, to = Element)]
    ProvesElement,

    /// Reconstructed mechanically from Element.parent_count_id in Pass 2.
    #[relationship(name = "HAS_ELEMENT", from = LegalCount, to = Element)]
    HasElement,

    /// Element anchored in the count-section paragraph where the drafter
    /// declares the element. Reconstructed mechanically.
    #[relationship(name = "ANCHORED_IN", from = Element, to = Allegation)]
    AnchoredIn,

    #[relationship(name = "ABOUT", from = Allegation, to = Person)]
    AboutPerson,

    #[relationship(name = "ABOUT", from = Allegation, to = Organization)]
    AboutOrganization,

    #[relationship(name = "CAUSED_BY", from = Harm, to = Allegation)]
    CausedBy,

    #[relationship(name = "DAMAGES_FOR", from = Harm, to = LegalCount)]
    DamagesFor,

    #[relationship(name = "SUFFERED_BY", from = Harm, to = Person)]
    SufferedBy,

    #[relationship(name = "EVIDENCED_BY", from = Harm, to = Allegation)]
    EvidencedBy,

    #[relationship(name = "DERIVED_FROM", from = Harm, to = Allegation)]
    DerivedFrom,

    #[relationship(name = "CONTAINED_IN", from = AnyEntity, to = Document)]
    ContainedIn,

    // Cross-document (added by Pass-2 after multiple docs ingested):
    #[relationship(name = "CHARACTERIZES", from = Evidence, to = Allegation)]
    Characterizes,

    #[relationship(name = "REBUTS", from = Evidence, to = Evidence)]
    Rebuts,

    #[relationship(name = "CORROBORATES", from = Evidence, to = Allegation)]
    Corroborates,

    #[relationship(name = "PROVES", from = MotionClaim, to = Allegation)]
    MotionClaimProves,

    // (full list — ~20 relationship types in v5.1)
}
```

The derive macro emits:
- `impl Relationship` with `fn label() -> &'static str` and `fn endpoints() -> (&'static str, &'static str)`
- Type-level proof that `ProvesElement` from-endpoint is `Allegation` and to-endpoint is `Element`

### 2.3 The query builder

A type-safe Cypher builder. Compile-time guarantee that:
- The MATCH label exists in the schema
- The relationship endpoints type-match the surrounding pattern
- The RETURN field names are properties of the matched entity

Concrete API — what `list_allegations` looks like after the refactor (compare to `repositories/allegation_repository.rs:51-77` today):

```rust
use crate::schema::v5_1::{Allegation, LegalCount, Relationship};
use crate::query::CypherQuery;

pub async fn list_allegations(&self) -> Result<AllegationsResponse, AllegationRepositoryError> {
    // Type-checked at compile time:
    //   - Allegation::LABEL == "Allegation" — change the schema, fail to build.
    //   - .traverse(...) endpoint types match.
    //   - .return_(...) field selectors must be properties of the matched type.
    //   - DISTINCT enforced automatically on multi-hop paths.
    let q = CypherQuery::match_node::<Allegation>("a")
        .optional_traverse(Relationship::ProvesElement, "el")
        .optional_traverse(Relationship::HasElement.reversed(), "c")
        .order_by(Allegation::paragraph_number_as_int(), "a")
        .return_(|a: AllegationProj, c: LegalCountProj| (
            a.id,
            a.paragraph_number,
            a.title,
            a.summary,                       // formerly a.allegation in v4 / aliased as `allegation` in JSON
            a.category,
            a.severity,
            collect_distinct(c.id),
            collect_distinct(c.title),
        ))
        .build();

    let rows = q.execute(&self.graph).await?;

    let allegations: Vec<AllegationDto> = rows
        .into_iter()
        .map(|row| row.into())   // From<Row> impl derived from Allegation type
        .collect();

    // … summary aggregation …
    Ok(AllegationsResponse { allegations, total, summary })
}
```

What changed semantically:
- No string literal anywhere in the file. Every name comes from the type.
- Property renames (`a.paragraph` → `a.paragraph_number`) become compile errors, not runtime "no allegations found."
- The two-hop traversal is one call chain: `.optional_traverse(ProvesElement, "el").optional_traverse(HasElement.reversed(), "c")`. The reversed-edge case is type-checked (HasElement has from=LegalCount; `.reversed()` flips for the Allegation-anchored traversal).
- DISTINCT is handled by the builder — collect operations on multi-hop projections default to DISTINCT (the bug we hit in Q9 of `case_summary_repository.rs`).

The build step emits the actual Cypher. Approximately:

```cypher
MATCH (a:Allegation)
OPTIONAL MATCH (a)-[:PROVES_ELEMENT]->(el)<-[:HAS_ELEMENT]-(c:LegalCount)
WITH a, collect(DISTINCT c.id) AS legal_count_ids,
     collect(DISTINCT c.title) AS legal_counts
RETURN a.id AS id, a.paragraph_number AS paragraph_number,
       a.title AS title, a.summary AS summary,
       a.category AS category, a.severity AS severity,
       legal_count_ids, legal_counts
ORDER BY toInteger(a.paragraph_number)
```

— matching today's hand-written Cypher in `allegation_repository.rs:58-72` exactly, but generated from types.

### 2.4 YAML schemas as derived artifacts

The schema YAMLs at `backend/extraction_schemas/*.yaml` exist primarily so `colossus_extract` can render the LLM prompt. Move that to codegen:

- Add a `build.rs` (or a `cargo run --bin generate-schema-yaml`) that emits the YAML by serializing the `EntityType` registry.
- The build script writes to a known path; `cargo build` regenerates on change.
- Hand-editing the YAML becomes a CI failure (the generated YAML would differ from the checked-in one if a developer edited only the YAML).

Result: the YAML at `backend/extraction_schemas/complaint_schema_v5_1.yaml` becomes a snapshot of what the Rust types describe, not an independent contract.

### 2.5 Frontend types from the same source

Today: `frontend/src/services/allegations.ts:4-14` has a hand-maintained `AllegationDto` type that must be kept in sync with the backend's `AllegationDto` struct (`backend/src/dto/allegation.rs:13`). Drift between them is a runtime failure when the frontend tries to access a field the backend stopped sending.

Proposal: use **typeshare** (or specta) on the Rust DTO structs to codegen TypeScript types. `typeshare-cli` walks the Rust source, finds `#[typeshare]`-annotated types, and emits `.ts` files. The codegen runs on every backend build; CI fails if the checked-in frontend `.ts` types diverge from what would regenerate.

Codegen surface needed for the schema-as-types work: the entity structs themselves (`Allegation`, `LegalCount`, etc.), the relationship enum, and the DTO structs that wrap them for API responses.

---

## 3. Contracts the compiler enforces (after refactor)

✓ **Caught at compile time** after the migration:

1. **Renamed entity label.** Change `#[entity(label = "Allegation")]` to `#[entity(label = "Claim")]` and every query referencing `Allegation` either fails to compile (type still exists, label change reflected automatically) — or compiles correctly because the type name is the only handle. There's no string literal to forget.
2. **Renamed property.** Rename `paragraph_number` to `paragraph_num` on the struct → every `.return_(|a| a.paragraph_number)` site fails to compile.
3. **Removed property.** Delete the `category` field → every consumer that reads it fails to compile.
4. **Renamed relationship.** Rename `ProvesElement` enum variant → every `Relationship::ProvesElement` reference fails to compile.
5. **Endpoint type mismatch.** Write `.optional_traverse(HasElement, ...)` from an `Allegation`-rooted pattern (where the from-endpoint is `LegalCount`) → compile error, "expected from=LegalCount, found Allegation."
6. **DTO/struct drift.** Frontend `AllegationDto.ts` regenerated from the Rust DTO; CI fails if checked-in version differs.
7. **Schema YAML drift.** Generated YAML differs from checked-in YAML → CI fails.

✗ **Still NOT caught at compile time:**

1. **LLM output drift.** The LLM might emit `paragraph` instead of `paragraph_number`, or `Allegation` with no `summary` field. The Rust types describe the contract; the LLM may violate it. The Verify step at `backend/src/api/pipeline/verify.rs` already handles this — `derived_invalid` status for entities that don't match the expected shape. Type system gives the verifier a stronger schema to validate against, but doesn't prevent the LLM mistake.
2. **Existing Neo4j data divergence.** If v5.1 nodes were ingested under a now-renamed property, the new query types won't find them. Type system can't introspect production Neo4j. A migration step (re-ingest, or one-time Cypher `SET a.new_name = a.old_name REMOVE a.old_name`) is still required.
3. **Semantic correctness of relationships.** The types prove `ProvesElement` connects `Allegation` to `Element`. They don't prove that a specific Pass-2 run correctly identified which allegations prove which elements.
4. **Profile/template/prompt alignment.** The Pass-1 markdown template at `backend/extraction_templates/complaint_pass1_v5_1.md` contains JSON few-shot examples with field names. If the template names a property the schema doesn't have, the LLM emits garbage that the verifier rejects. Codegen could keep the template's example block synced; this is a stretch goal.
5. **YAML profiles' schema_file pointer.** `backend/profiles/complaint_v5_1.yaml` references `complaint_schema_v5_1.yaml`. The pointer's validity is the disk/code-consistency test at `backend/src/pipeline/config.rs:tests::all_extraction_schemas_load_successfully` — that doesn't change.

---

## 4. Migration path

Five phases, additive — each is independently shippable. Phase A unblocks B; B unblocks C; D and E can run in parallel with C.

### Phase A — Define Rust types for v5.1 schema

**Scope:** small-to-medium (1–2 days of focused work — **estimate, not commitment**).

- Add `backend/src/schema/mod.rs` and `backend/src/schema/v5_1.rs`.
- Define structs for: `Allegation`, `LegalCount`, `Element`, `Harm`, `Party` (or split `Person`/`Organization`), `Evidence`, `MotionClaim`, `Document`, `Case`.
- Define the `Relationship` enum covering ~20 relationship types from the v5.1 schema.
- Add `derive` macros (`Entity`, `EntityEnum`, `RelationshipSet`) — these can start as `proc-macro2`-based or just hand-written impls without a derive macro for the initial pass. Derive macros are a quality-of-life improvement, not a Phase-A requirement.
- **No existing code changes.** Pure addition. Tests verify the type-level proofs (e.g., `assert_eq!(Allegation::LABEL, "Allegation")`).

Blockers: none.

Verification: cargo build clean, `cargo test --lib schema::` passes.

### Phase B — Query builder

**Scope:** medium (3–5 days — **estimate**). Most of the time goes into the builder API, not the type system.

- Add `backend/src/query/mod.rs` and `backend/src/query/cypher_builder.rs`.
- Builder API:
  - `CypherQuery::match_node::<T: Entity>(alias: &str) -> MatchBuilder<T>`
  - `.optional_traverse(rel: Relationship, alias: &str) -> MatchBuilder<OtherEnd>`
  - `.where_eq(prop_selector, value)`, `.where_gt`, etc.
  - `.return_(closure: impl Fn(Proj1, Proj2, ...) -> ResultTuple) -> Query<ResultTuple>`
  - `.order_by`, `.limit`, `.distinct` (default on collect)
  - `.build() -> CompiledQuery` — emits the Cypher string and the params map
  - `.execute(&Graph) -> Result<Vec<Row<ResultTuple>>>` — wraps `neo4rs::execute`
- Property selectors: `Allegation::paragraph_number()` returns a `PropertySelector<Allegation, String>`. Type-checked against the entity in the MATCH.
- The `Row<T>` type implements `From<neo4rs::Row>` via the same `Entity` derive that defines the struct.

**New queries written during this phase** use the builder. Existing queries stay as raw Cypher. No production behavior change.

Blockers: Phase A.

Verification:
- `cargo test --lib query::` exercises the builder against an in-memory mock graph or a real DEV Neo4j fixture.
- Hand-write one query the new way, snapshot the emitted Cypher, compare to the corresponding hand-written Cypher in `repositories/`.

### Phase C — Migrate existing queries to the builder

**Scope:** large (1–2 weeks of focused work — **estimate**), one repository file at a time.

- Walk the 18 production files migrated in 2b51d38 + 4ba332d (the full list is in this doc's §1.2 and in those commits' messages).
- Per file: rewrite each Cypher query against the builder. Tests for the affected page or admin tool must keep passing.
- DTO mapping: the `From<Row<T>> for AllegationDto` impl handles the wire-format. If the builder produces the same projection shape, the DTO doesn't change.
- The four dispatch sites (`graph_expander.rs:107,180,240`, `embedding_text.rs:34`, `models/document_status.rs:110`) become `match entity_label { Allegation::LABEL => ..., LegalCount::LABEL => ..., }` — typed dispatch, no more dead `"ComplaintAllegation"` arms.

After Phase C: zero hardcoded entity/relationship/property name strings in production code (excluding the YAML codegen output and the LLM template).

Blockers: Phase B.

Verification: full test suite green; smoke-test each migrated page against DEV Neo4j; visual regression check for graph rendering.

### Phase D — YAML schemas as codegen output

**Scope:** small-to-medium (1–2 days — **estimate**).

- Add a build-script or a `cargo run --bin generate-schemas` that walks the `Entity` registry and emits a YAML matching the current `colossus_extract::ExtractionSchema` format.
- Either:
  - **Pre-commit hook / CI check:** generated YAML must match checked-in YAML; CI fails on drift.
  - **Or:** delete the checked-in YAML and have `cargo build` write it to `target/schemas/`; downstream (Ansible deploy, Pass-1 prompt assembly) reads from the generated location. Riskier — needs `colossus_extract` to know the path.
- Decision deferred to implementation. CI-checked checked-in YAML is the safer default.

Blockers: Phase A. Independent of Phases B and C.

Verification: `cargo run --bin generate-schemas` then `git diff backend/extraction_schemas/` must be empty.

### Phase E — Frontend types codegen

**Scope:** small-to-medium (1–2 days for setup; ongoing maintenance is just `cargo build` + commit the generated `.ts`).

- Choose tool: **typeshare** (proven, simple), **specta** (richer but newer), or a custom build.rs walking the `#[typeshare]`-annotated structs.
- Annotate the DTO structs in `backend/src/dto/*.rs` with `#[typeshare]`.
- Configure typeshare to emit to `frontend/src/types/generated/`.
- Frontend imports from `generated/` instead of from hand-maintained types.
- CI step: `typeshare-cli` + git-diff check, parallel to Phase D's YAML check.

Blockers: Phase A. Independent of Phases B, C, D. Can be done early (before C) to get the wire-contract guarantees without touching queries.

Verification: regenerate `.ts`, `npm run typecheck` passes, `git diff frontend/src/types/generated/` empty after regeneration.

---

## 5. What this does NOT fix

1. **LLM extraction quality.** A schema that says `summary: required` doesn't make the LLM emit summaries. The Verify step's `derived_invalid` machinery (`backend/src/api/pipeline/verify.rs:714-775`) still catches LLM omissions at extract time. Type system gives that verifier a more precise schema to validate against, but the LLM-vs-schema gap remains.

2. **Existing Neo4j data divergence.** If beta.222 was deployed with v5.1 nodes and a future v5.2 renames a property, the old nodes in PROD still carry the old property name. The new query types won't find them. A Cypher migration script — or a re-ingest — is still needed. Type system doesn't introspect runtime data.

3. **Profile / template / prompt configuration.** `backend/profiles/complaint_v5_1.yaml` maps document_type → schema_file → template_file. The mappings themselves are configuration, not schema. The proposal doesn't change profile resolution (PR 1b at `select_profile_for_document_type` already covered that).

4. **Pass-1 markdown template ↔ schema alignment.** `complaint_pass1_v5_1.md` has JSON few-shot examples that mention property names. If a property is renamed in Rust, the template's example doesn't update. Codegen of the template's example blocks is possible but out of scope here — it's a different problem (LLM prompt engineering, not type safety).

5. **Cross-version query support.** This is intentional. Pure v5.1 only — no back-compat with v4 ComplaintAllegation. Per the 2b51d38/4ba332d commit messages, "There are no v4 documents that need to be queryable. ComplaintAllegation references go away."

---

## 6. What this replaces

1. **The 18 files just migrated in 2b51d38 + 4ba332d.** They become a couple hundred lines of builder calls against typed schemas. The string literals disappear. Future schema bumps touch one Rust file.

2. **The pattern of "find every hardcoded reference manually."** Every schema bump in the project's history has produced the same defect: schema renames, query consumers stay, user reports empty pages. v3→v4 had the same shape. v4→v5 had it. v5→v5.1 had it tonight. The schema-as-types refactor ends the pattern by making "find every hardcoded reference" a `cargo build` instead of a `grep`.

3. **The four production dispatch sites** (`services/graph_expander.rs:107,180,240`, `services/embedding_text.rs:34`, `models/document_status.rs:110`) — currently flagged as deferred back-compat in the 4ba332d self-review. Under the typed dispatch model, dead arms are visible at the type level (the `Match` is exhaustive over the entity enum), so leftover v4 names can't accumulate.

4. **Drift between the YAML and the consumer.** Today both sides exist independently; they happen to agree. Tomorrow the YAML is the byproduct.

5. **Drift between backend DTOs and frontend TS types.** Today both sides are hand-maintained. Tomorrow the TS comes from `typeshare` codegen.

---

## 7. Open questions

### 7.1 Code generation tool

Three candidates for the derive-macros + codegen side:

| Tool | Strength | Weakness | Verdict |
|---|---|---|---|
| **typeshare** | Production-ready, used by Mozilla/1Password, simple TS output | TS-only; would need a separate codepath for YAML | **Recommend for Phase E (frontend types)**. Adopt as the proven tool. |
| **specta** | Richer (handles enums-with-data better than typeshare), Tauri-blessed | Newer, smaller ecosystem | Consider if typeshare's enum handling proves limiting for `AllegationKind`-style variants. |
| **Custom `build.rs`** | Total control, can emit both TS and YAML from one walk | Maintenance burden | **Reluctant recommend for Phase D (YAML)** — no off-the-shelf Rust→ExtractionSchema-YAML tool exists, and the YAML shape is colossus-rs-specific. |
| `proc-macro2`-based `#[derive(Entity)]` | Powers the type-checked query builder; orthogonal to TS/YAML generation | Macro complexity, slower compile | **Required for Phase A**. The query builder needs `LABEL: &'static str` and similar trait impls. |

Decision in this design: typeshare for Phase E, custom build.rs for Phase D, proc-macro for Phase A. Could collapse to one tool later if the maintenance burden warrants — out of scope for v1.

### 7.2 How LLM prompts get the schema after the refactor

Today: `colossus_extract::ExtractionSchema::from_file` reads the YAML at extract time and inlines the schema into the prompt's `{{schema_json}}` placeholder.

After refactor: the YAML at `backend/extraction_schemas/complaint_schema_v5_1.yaml` is codegen output. `colossus_extract` reads it at extract time exactly as today. **Nothing changes for the LLM-extract path.** The YAML just stops being hand-edited.

Alternative considered: have `colossus_extract` take a Rust object directly instead of a YAML path. Rejected: colossus-rs is a shared crate that shouldn't depend on colossus-legal-specific types.

### 7.3 How existing v5.1 data in Neo4j is validated against new types

Three options, increasing strength:

1. **Trust the ingest layer.** v5.1 ingest writes data that matches the v5.1 types because the types ARE the schema and the ingest writer uses them. Pre-existing v5.1 nodes (written before the refactor) would already match the types because the property/label names didn't change inside v5.1 — only the consumer code did.
2. **Add a Cypher-side validation tool.** `cargo run --bin validate-graph` walks every Neo4j node, checks its label is in the schema, checks its property keys are a subset of the type's properties. Useful for one-time verification at deploy.
3. **Continuous validation.** Have the worker / a scheduled job periodically run `validate-graph` and surface divergences in the admin UI.

Phase C deploy needs (1) at minimum. Option (2) is a small additional tool. Option (3) is operations work outside this scope.

### 7.4 Whether `colossus-rs::ExtractionSchema` folds into this

`colossus-rs/colossus-extract/src/schema.rs` defines `ExtractionSchema`/`EntityTypeConfig`/etc. as the shape colossus-rs reads from YAML. This is the API contract between colossus-legal (or any future colossus-* consumer) and the colossus-rs extraction crate.

**Recommendation: leave `colossus-rs::ExtractionSchema` as-is.** It's a YAML-shaped struct designed for runtime-loaded, application-agnostic schemas. colossus-rs has its own evolution path and should remain independent.

colossus-legal's typed schema layer would be a **superset** that:
- Generates YAML matching `colossus_extract::ExtractionSchema`'s shape for `colossus_extract` to consume
- Adds typed Rust handles for the colossus-legal query layer

The two coexist: colossus-rs handles the LLM-facing schema and prompt assembly; colossus-legal handles the Rust-side typed contracts. The YAML is the wire format between them.

If a future colossus-* application wants the same compile-time guarantees, it duplicates the typed-schema pattern in its own crate. The pattern, not the types, is the reusable artifact.

### 7.5 Where the typed schema crate lives

Three options:

1. **Inside `backend/src/schema/`** — start here, lowest friction. If a second consumer ever needs it, extract.
2. **A new workspace crate `colossus-legal/schema/`** — clean module boundary, longer setup.
3. **In `colossus-rs/colossus-legal-schema/`** — closest to the LLM-facing schema. Argues against §7.4's recommendation. Rejected.

**Recommend (1).** Migrate to (2) when a second consumer appears.

### 7.6 Backward compatibility during the migration

Phase C migrates queries one repository file at a time. During the transition, half the queries use the new builder and half use raw Cypher. Both produce the same wire format (DTO unchanged). Both read the same Neo4j data.

**Risk:** a property rename mid-Phase-C breaks queries not yet migrated. **Mitigation:** schema versions are immutable. Bumping v5.1 → v5.2 in the type system is a new version with its own struct. Half-migrated state stays valid as long as we don't ship new schema versions mid-Phase-C.

### 7.7 Test pyramid

Phase B adds a "snapshot the emitted Cypher" test category. Each builder-emitted query has a corresponding snapshot test. CI fails if the Cypher diverges from the snapshot. Combined with the integration tests against DEV Neo4j, this gives:

- **Type tests** (Phase A): trait-level proofs.
- **Snapshot tests** (Phase B+C): builder emits the expected Cypher.
- **Integration tests** (existing): query returns expected rows against fixture data.
- **Disk-consistency tests** (existing): YAMLs match the runtime expectation.

---

## 8. Cost / value summary

**Cost of doing it:** 2–3 weeks of focused work (estimate), one engineer, scoped across phases A–E. Most of the time is Phase C (repository migration) and Phase B (builder API design). The risk is well-bounded: each phase is additive and shippable; no big-bang refactor.

**Cost of NOT doing it:** every schema bump produces another night like tonight. The v6 design is already nascent; that's the next test. Each bump's debug-and-fix cycle is ~4–8 hours of grep, query rewriting, smoke testing, and follow-up commits to catch the misses. Plus the user-visible window where pages are empty.

**Value delivered:**

| What changes | Before | After |
|---|---|---|
| Schema rename | ~70 hand-edits across 18 files, 1 follow-up commit, user reports the bug | 1 edit in `backend/src/schema/v5_1.rs`, cargo build points at every needed change |
| Property rename | ~15–25 hand-edits per property, runtime-only signal | Rust struct field rename, every consumer fails to compile |
| Adding a new entity type | Manual: YAML + ingest helper + repositories + DTOs + frontend types | Add a struct with `#[derive(Entity)]`, codegen handles YAML + TS |
| Frontend/backend DTO drift | Silent until a user hits the empty field | CI fails on `typeshare` diff |
| YAML/Rust drift | Silent until LLM extraction fails | CI fails on YAML codegen diff |

**The pattern of finding every hardcoded reference manually ends.** That's the deliverable. Everything else is downstream of it.

---

## 9. Recommended next steps

1. Roman reads this doc. Pushes back on anything that doesn't match his sense of the project's trajectory.
2. If approved: a Phase A spike — define types for Allegation, LegalCount, Element only (3 entities, ~6 relationships). Goal: prove the derive-macro shape and exercise the type-level proofs in tests. Estimate: 1 day.
3. If the Phase A spike validates the approach: scope Phase B (query builder) as a separate design doc. The builder is where most of the technical risk lives — borrow patterns from `diesel`, `sea-orm`, or `cypher-rs` as starting points.
4. Phase C is the muscle. Sequence the repository migration by user-facing impact: `case_summary_repository` (Allegations-page header) and `allegation_repository` (Allegations list) first; admin/embedding/graph last.

---

## Appendix A — File:line map of touched code in the recent migration

Citations grounded in commits `2b51d38` (page-query layer) and `4ba332d` (services + admin + dispatch).

| File | Queries | Notes |
|---|---|---|
| `repositories/allegation_repository.rs` | Q1 (`list_allegations`) | Two-hop + 3 property renames |
| `repositories/allegation_detail_repository.rs` | Q2, Q3, Q4 + 3 param sites | Q2 two-hop; Q3/Q4 label-only |
| `repositories/case_summary_repository.rs` | Q8 (colossus_graph), Q9 (Cypher) | Q9 critical for "0 allegations" symptom |
| `repositories/evidence_chain_repository.rs` | Q11 | Inline Cypher |
| `repositories/motion_claim_repository.rs` | Q12 | Label-only |
| `repositories/case_repository.rs` | Q17 (colossus_graph) | Label-only |
| `repositories/query_repository.rs` | Q26 | Pre-registered Cypher, label-only |
| `repositories/embedding_repository.rs` | Q30 | Namespace rename + property renames |
| `repositories/graph_repository.rs` | Q37, Q38 | RETURN DISTINCT added |
| `repositories/analysis_repository.rs` | Q39, Q41 | Q39 property renames; Q41 label-only |
| `api/pipeline/delete.rs` | (comment update) | Already label-agnostic |
| `repositories/decomposition_repository.rs` | OVERVIEW_CHAR_QUERY, OVERVIEW_PROOF_QUERY | In 4ba332d |
| `repositories/document_repository.rs` | `list_documents_with_evidence_counts` | In 4ba332d |
| `repositories/person_detail_repository.rs` | STATEMENTS_QUERY | In 4ba332d |
| `services/graph_expansion_queries.rs` | `expand_allegation`, `expand_evidence`, `expand_motion_claim` | In 4ba332d + post-grep |
| `services/graph_expansion_minor.rs` | `expand_harm` | In 4ba332d |
| `api/admin_document_evidence_queries.rs` | `CONTENT_QUERY` (UNION ALL) | In 4ba332d |
| `api/admin_evidence.rs` | `create_relationship` site for PROVES | In 4ba332d |

**Deferred (still hardcoded after 4ba332d):**

| File:line | Site | Notes |
|---|---|---|
| `services/graph_expander.rs:107,180,240` | Dispatch on `entity_type` string | 3 sites; dead arms for `"ComplaintAllegation"` |
| `services/embedding_text.rs:34` | Embedding text builder dispatch | 1 site |
| `models/document_status.rs:110` | `pub const ENTITY_COMPLAINT_ALLEGATION` | Constant — referenced by ingest/verify pipeline for back-compat |
| Test fixtures across `verify.rs`, `completeness_helpers.rs`, `pipeline_repository/extraction.rs`, `ingest_helpers.rs` | ~16 sites | Vestigial test artifacts; not blocking |

Total deferred: 4 production dispatch sites + 1 constant + ~16 test sites.

---

End of design.
