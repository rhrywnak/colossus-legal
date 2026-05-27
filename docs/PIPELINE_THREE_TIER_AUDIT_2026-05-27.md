# Pipeline Three-Tier Architecture Audit — 2026-05-27

> Read-only research. Every claim is grounded in a `file:line` grep result, a file read, or a
> migration/schema definition. No code, migrations, schemas, templates, or other files were changed.
> Findings are categorised factually (driven by presence/absence of read and write paths). No fixes
> are recommended — architectural decisions are deferred to Roman.
>
> Branch `main` @ `2656b61` ("refactor: remove Element extraction from complaint schema/templates").
>
> **Scope note / path correction:** the instruction referenced `backend/src/bin/canonical_elements/`
> (a directory). It does not exist. The canonical loader is a thin binary
> (`backend/src/bin/load_canonical_elements.rs`) delegating to a library module
> `backend/src/canonical_elements/` (9 sub-modules). All were read.

---

## Executive Summary

**Entity-type tier conflicts:** Of the 12 traced entity types, exactly **one is an active both-tiers
conflict: `LegalCount`.** It is mastered in Tier 1 (canonical loader writes `authored_entities`
`count-{N}` and stamps Neo4j `LegalCount.id = count-{N}`) *and* extracted in Tier 2 (the active
`complaint_schema_v5_1.yaml` defines `LegalCount`; ingest MERGEs a Neo4j node keyed
`{doc_slug}:count:{N}`). Because `count-1` ≠ `doc-awad-…:count:1`, the two paths MERGE **two distinct
Neo4j `LegalCount` nodes** — the duplicate defect named in the instruction.

**`Element` is a *resolved* conflict with residual machinery.** Element extraction was removed from the
active complaint schema/templates (this branch's HEAD commit). No active schema emits `Element`, so
there is no live duplication today. But dormant Tier-2 machinery remains: a `stable_entity_id`
`ENTITY_ELEMENT` arm, a `graph_migrations` `element_id_unique` constraint, the `ENTITY_ELEMENT`
constant, and an `Element` test corpus. These would re-create the conflict the moment any schema
re-introduces `Element` extraction.

**Tier-1 asymmetry:** The canonical loader writes **4 node types to Neo4j** (`Element`, `BreachTheory`,
`ImproperActTheory`, `DeclarationSought`) and **3 relationships** (`HAS_ELEMENT`, `HAS_THEORY`,
`SEEKS_DECLARATION`), but writes only **2 entity types** (`LegalCount`, `Element`) and **1
relationship** (`HAS_ELEMENT`) to the Postgres system-of-record tables. `BreachTheory`,
`ImproperActTheory`, `DeclarationSought`, `HAS_THEORY`, and `SEEKS_DECLARATION` exist in Neo4j with **no
`authored_entities`/`authored_relationships` backing row**. Conversely `LegalCount` has an
`authored_entities` row but its Neo4j node is *not created* by the loader (only `id`-stamped).

**The central v5 relationship is currently un-persistable by the pipeline.** `PROVES_ELEMENT`
(Allegation→Element) is retained in the schema and is the explicit focus of the complaint Pass-2
template, which instructs the LLM to target canonical Elements at `ctx:element-*`. But authored
entities are injected into the Pass-2 *prompt only* and **excluded from the `id_map` (Option B)**, so
`store_pass2_relationships` skips every edge whose endpoint is a `ctx:`-authored id. No Tier-3 mapping
step writes `PROVES_ELEMENT` to `authored_relationships` (the loader writes only `HAS_ELEMENT`).
Result: PROVES_ELEMENT edges the LLM produces against canonical Elements are dropped-and-logged.

**Dead artifacts found:**
- **10 of 12 `REL_*` constants** in `document_status.rs` are defined but have **zero non-definition
  references** (only `REL_CONTAINED_IN` is used; relationship types otherwise flow as data).
- **`ThematicAllegation`** — no active schema/template emits it (v5.1 schema documents it as REMOVED);
  residual machinery in `verify.rs`, `ingest_helpers.rs` (`stable_entity_id` arm), `graph_migrations.rs`
  (constraint), `document_status.rs` (constant).
- **`Court`, `Proceeding`, `ProceduralEvent`, `Role`, `Assertion`** — constants + `graph_migrations`
  constraints, defined by **no schema**.
- **`CHARACTERIZES`** — instructed by `discovery_response_pass2_v5_1.md` and queried in 5 backend repos,
  but **not declared in `discovery_response_schema_v5_1.yaml`'s `relationship_types`**.
- **Orphaned files:** 5 v4 templates and 3 schemas unreferenced by any profile; **2 broken profile
  references** (`motion.yaml`/`default.yaml` point at template files that do not exist on disk).
- **v4 entity artifacts `MotionClaim`/`Exhibit`** still live in backend RAG repos + v4 schemas.

**Counts:** 1 active entity conflict (`LegalCount`); 1 dormant entity conflict (`Element`); 5 Tier-1
Neo4j node/rel types with no Postgres backing; 10 dead `REL_*` constants; ~6 dead entity-type
constants; 5 orphaned templates; 3 orphaned schemas; 2 broken profile references.

---

## Entity Type Traces

Legend for tier: **T1** = canonical loader only; **T2** = extraction pipeline only; **BOTH** = active
duplication; **BOTH (dormant)** = machinery exists in both but only one is live today.

Active v5.1 profiles → files:
`complaint_v5_1` → `complaint_schema_v5_1.yaml` / `complaint_pass1_v5_1.md` / `complaint_pass2_v5_1.md`;
`affidavit` → `affidavit_schema_v5_1.yaml` / `affidavit_pass1_v5_1.md` / `affidavit_pass2_v5_1.md`;
`discovery_response` → `discovery_response_schema_v5_1.yaml` / `discovery_response_pass1_v5_1.md` /
`discovery_response_pass2_v5_1.md`.

### 1. `Party`

- **A. Schema/Template:** Defined in all three active v5.1 schemas — `complaint_schema_v5_1.yaml:57`,
  `affidavit_schema_v5_1.yaml:43`, `discovery_response_schema_v5_1.yaml:44`. Also v4 schemas. Extracted
  by all active Pass-1 templates; referenced as relationship endpoint in all Pass-2 templates.
- **B. Canonical loader:** No. Loader never creates `Party` (Neo4j or `authored_entities`).
- **C. Pipeline code:** `ENTITY_PARTY` constant (`document_status.rs:107`); whitelist member
  `PARTY_SUBTYPES` (`document_status.rs:139`) and `CROSS_DOC_ENTITY_TYPES`
  (`extraction_context.rs:87`). Ingest resolves `Party` → `Person`/`Organization` via
  `create_party_nodes` (`ingest_helpers.rs:247-332`, filter `ingest_resolver.rs:155`). Verify grounds
  it (`load_grounding_config` reads the schema's modes; test mirror `verify.rs:889`).
- **D. Frontend:** 5 files (`src/pages/EvidenceExplorerPage.tsx`, `src/services/documentEvidence.ts`,
  `src/hooks/useSchema.ts`, `src/pages/BiasExplorer/*`, …).
- **E. Neo4j:** Never persisted as `:Party`; resolved at ingest to `:Person`/`:Organization`.
- **F. Tier: T2.** No conflict (it is mastered by extraction; loader never touches it).

### 2. `LegalCount` — **ACTIVE CONFLICT**

- **A. Schema/Template:** Active extraction entity — `complaint_schema_v5_1.yaml:80`
  (extraction_rules line 244 "Extract ALL legal counts"). Cross-doc context type
  (`extraction_context.rs:90`) and Pass-2 endpoint for `DAMAGES_FOR`. Also `complaint_v5.yaml`,
  `complaint_v4.yaml` (legacy).
- **B. Canonical loader:**
  - `authored_entities`: **Yes** — `authored.rs:138-151` upserts entity_type `LegalCount`, id
    `count-{N}` (`legal_count_entity_id`, `authored.rs:50-52`).
  - Neo4j: **node not created** by loader; loader UPDATEs managed props (`cypher.rs:247-273`) and
    stamps `id = count-{N}` (`cypher.rs:285-289`, called `loader.rs:346-349`). The node itself is
    expected to pre-exist (case-structuring pipeline), keyed by `count_number`.
- **C. Pipeline code:** `ENTITY_LEGAL_COUNT` (`document_status.rs:120`); `stable_entity_id`
  `ENTITY_LEGAL_COUNT` arm → `{doc_slug}:count:{count_number}` (`ingest_helpers.rs:101-118`); ingest
  MERGEs `(n:LegalCount {id:$id})` via `create_entity_node` (`ingest_helpers.rs:380-381`); cross-doc
  whitelist (`extraction_context.rs:90`); `graph_migrations.rs:53` (`legal_count_id_unique`
  constraint); query repos `causes_of_action_repository.rs:105/147`, `case_summary_elements.rs:64`.
- **D. Frontend:** 7 files (`src/utils/countFormat.ts`, `src/pages/AllegationsPage.tsx`,
  `src/services/caseSummary.ts`, `src/pages/EvidenceExplorerPage.tsx`, …).
- **E. Neo4j:** `:LegalCount` written by both the case-structuring pipeline (canonical, `id=count-{N}`)
  and ingest (extracted, `id={doc_slug}:count:{N}`).
- **F. Tier: BOTH (active conflict).** Duplication locations:
  1. `authored_entities` row `count-{N}` (T1) **vs** `extraction_items` `LegalCount` row (T2).
  2. Neo4j node `id=count-{N}` (T1, `cypher.rs:285-289`) **vs** Neo4j node
     `id={doc_slug}:count:{N}` (T2, `ingest_helpers.rs:101-118` + `create_entity_node`). These MERGE
     as two distinct nodes (MERGE matches on `{id}`).

### 3. `Element` — **RESOLVED CONFLICT + RESIDUAL MACHINERY**

- **A. Schema/Template:** **Not** an entity in any active v5.1 schema. `complaint_schema_v5_1.yaml`
  header documents removal (lines 29-40); `extraction_rules` line 245 "Do NOT extract Elements";
  Pass-1 template states it explicitly (`complaint_pass1_v5_1.md:24,71,433,440`); Pass-2 template
  treats Elements as `ctx:`-prefixed authored context (`complaint_pass2_v5_1.md:24`). Still an entity
  in legacy `complaint_v5.yaml:101`.
- **B. Canonical loader:**
  - `authored_entities`: **Yes** — `authored.rs:154-168` upserts entity_type `Element`, id = YAML
    `e.id` (`element-{N}-{M}`).
  - Neo4j: **Yes** — `cypher.rs:121-155` MERGEs `(:Element {id})` + `HAS_ELEMENT`; orphan-wipes
    Elements not in YAML (`cypher.rs:308-310`, `loader.rs:266-271`).
- **C. Pipeline code (residual T2):** `ENTITY_ELEMENT` (`document_status.rs:124`); `stable_entity_id`
  `ENTITY_ELEMENT` arm → `{doc_slug}:element:{hash8}` (`ingest_helpers.rs:130-158`, with full test
  suite `ingest_helpers.rs:1092-1201`); `graph_migrations.rs:55` (`element_id_unique`); query repos
  `causes_of_action_repository.rs:148`, `case_summary_elements.rs:65`. Cross-doc/authored whitelist
  member (`extraction_context.rs:94`). **No active schema feeds these arms.**
- **D. Frontend:** 5 files (`src/components/CountCard.tsx`, `src/services/causesOfAction.ts`,
  `src/services/caseSummary.ts`, `src/components/__tests__/countCardHelpers.test.ts`,
  `src/styles/tokens.css`).
- **E. Neo4j:** `:Element` nodes are now exclusively canonical-loader output (`element-{N}-{M}`).
- **F. Tier: T1 active; BOTH (dormant).** The `stable_entity_id` arm, constraint, and constant are
  the dormant T2 conflict surface; no live path exercises them on this branch.

### 4. `Allegation` (and `ComplaintAllegation`)

- **A. Schema/Template:** `Allegation` is the active v5.1 complaint entity
  (`complaint_schema_v5_1.yaml:118`); extracted by `complaint_pass1_v5_1.md`. `ComplaintAllegation` is
  the **v4-era label** for the same concept — present in v4 schemas (`complaint_v4.yaml`,
  `affidavit_v4.yaml`, `discovery_response_v4.yaml`), `complaint_v5.yaml`, v4 templates, and **still
  referenced in two active v5.1 templates**: `affidavit_pass2_v5_1.md:80,88,158` (CORROBORATES guidance
  says "Evidence → ComplaintAllegation … entities with entity_type 'ComplaintAllegation'"). Both labels
  are deliberately kept readable side-by-side per `document_status.rs:110-119`.
- **B. Canonical loader:** No (extraction-only concept).
- **C. Pipeline code:** `ENTITY_ALLEGATION` (`document_status.rs:119`) and
  `ENTITY_COMPLAINT_ALLEGATION` (`:110`); both in `CROSS_DOC_ENTITY_TYPES`
  (`extraction_context.rs:91-92`). `stable_entity_id` arm keyed on `ENTITY_COMPLAINT_ALLEGATION` →
  `{doc_slug}:para:{paragraph}` (`ingest_helpers.rs:75-100`) — there is **no `Allegation` arm**, so a
  v5.1 `Allegation` falls to the generic `other` arm (`ingest_helpers.rs:187-192`) →
  `{doc_slug}:allegation:{hash8}`. Verify filters `entity_type == "Allegation"` literally
  (`verify.rs:691`); grounding-mode test mirror keys `"ComplaintAllegation"` (`verify.rs:895`).
- **D. Frontend:** `Allegation` 8 files (incl. `src/pages/AllegationDetailPage.tsx`,
  `src/pages/DecompositionPage.tsx`, `src/pages/GraphPage.tsx`); `ComplaintAllegation` 6 files (incl.
  `src/utils/nodeTypeDisplay.ts`, `src/utils/itemProperties.ts`, `src/pages/SearchPage.tsx`).
- **E. Neo4j:** Both `:Allegation` and `:ComplaintAllegation` labels can exist (label = entity_type at
  ingest, `ingest_helpers.rs:381`). Backend queries target `:Allegation`
  (`graph_expansion_queries.rs:76`, `decomposition_repository.rs:68`).
- **F. Tier: T2** for both. **Inconsistencies (not tier conflicts):** (1) `stable_entity_id` lacks an
  `Allegation` arm (v5.1 allegations hash instead of using `:para:{n}`); (2) `affidavit_pass2_v5_1.md`
  instructs matching the v4 `ComplaintAllegation` label, while active complaints emit `Allegation`.

### 5. `Harm`

- **A. Schema/Template:** `complaint_schema_v5_1.yaml:153`; completeness rule min 1 (`:257-260`); three
  hunting paths in Pass-1 (`complaint_pass1_v5_1.md`, extraction_rules line 246). Pass-2 endpoint for
  `CAUSED_BY`/`DAMAGES_FOR`/`SUFFERED_BY`/`EVIDENCED_BY`. Also `complaint_v4.yaml`, `complaint_v5.yaml`.
- **B. Canonical loader:** No.
- **C. Pipeline code:** `ENTITY_HARM` (`document_status.rs:121`); `stable_entity_id` `ENTITY_HARM` arm
  → `{doc_slug}:harm:{hash8}` (`ingest_helpers.rs:119-129`); cross-doc whitelist
  (`extraction_context.rs:95`); `graph_migrations.rs:54` (`harm_id_unique`). Verify grounding mode
  `Derived`, provenance required (`verify.rs:898` test mirror); `verify.rs` provenance handling for
  Harm (`build_para_to_item_id`, `verify.rs:689-735`).
- **D. Frontend:** 3 files (`src/pages/SearchPage.tsx`, `src/services/documentEvidence.ts`,
  `src/hooks/useSchema.ts`).
- **E. Neo4j:** `:Harm` written by ingest.
- **F. Tier: T2.** No conflict.

### 6. `Evidence`

- **A. Schema/Template:** Active entity in `affidavit_schema_v5_1.yaml:66` and
  `discovery_response_schema_v5_1.yaml:67`; extracted by both Pass-1 templates; endpoint for
  CORROBORATES/CONTRADICTS/REBUTS/CHARACTERIZES in Pass-2 templates. **Not** a complaint entity.
- **B. Canonical loader:** No.
- **C. Pipeline code:** `ENTITY_EVIDENCE` (`document_status.rs:122`); cross-doc whitelist
  (`extraction_context.rs:93`). No dedicated `stable_entity_id` arm → generic `other`
  (`{doc_slug}:evidence:{hash8}`).
- **D. Frontend:** 25 files — the most-referenced type (Evidence Explorer, Bias Explorer, admin cards,
  Contradictions, Impeachment, etc.).
- **E. Neo4j:** `:Evidence` heavily queried — `evidence_repository.rs`, `evidence_chain_repository.rs`,
  `contradiction_repository.rs`, `rebuttals_repository.rs`, `graph_expansion_queries.rs:76`,
  `decomposition_repository.rs`, `analysis_repository.rs`, others.
- **F. Tier: T2.** No conflict.

### 7. `BreachTheory`

- **A. Schema/Template:** Defined by **no** extraction schema/template (0 hits in
  `extraction_schemas/`, `extraction_templates/`). It is a canonical YAML concept
  (`schema.rs:201-216` `TheoryDef`; Count I `breach_theories`).
- **B. Canonical loader:** Neo4j **Yes** — `cypher.rs:35` label, `upsert_breach_theory`
  (`cypher.rs:194-202`) MERGEs `(:BreachTheory {key})` + `HAS_THEORY`; orphan-wipe `cypher.rs:313`.
  `authored_entities` **No** — `authored.rs` writes only `LegalCount`/`Element`.
- **C. Pipeline code:** **0 references** outside `canonical_elements/` (no constant, no
  `stable_entity_id` arm, not in any whitelist, no `graph_migrations` constraint, not in verify or
  completeness).
- **D. Frontend:** 0 files.
- **E. Neo4j:** `:BreachTheory` nodes + `HAS_THEORY` edges (loader only). **No backend reader found.**
- **F. Tier: T1 (Neo4j-only).** No conflict, but **no Postgres system-of-record row** (asymmetry).

### 8. `ImproperActTheory`

- Identical shape to `BreachTheory`. Schema/template: none. Loader Neo4j: `cypher.rs:36` label,
  `upsert_improper_act_theory` (`cypher.rs:205-213`), orphan-wipe `cypher.rs:318`. `authored_entities`:
  no. Pipeline code: **0 references** outside `canonical_elements/`. Frontend: 0. **Tier: T1
  (Neo4j-only); no Postgres backing.**

### 9. `DeclarationSought`

- Schema/template: none. Loader Neo4j: `cypher.rs:37` label, `upsert_declaration`
  (`cypher.rs:216-238`) MERGEs `(:DeclarationSought {id})` + `SEEKS_DECLARATION`; orphan-wipe
  `cypher.rs:323`. `authored_entities`: no. Pipeline code: **0 references** outside
  `canonical_elements/`. Frontend: 0. **Tier: T1 (Neo4j-only); no Postgres backing.**

### 10. `Person`

- **A. Schema/Template:** Not a top-level extraction entity; it is the resolved subtype of `Party`
  (`general_legal.yaml` uses `entity_kind`). Property-level only in v5.1 schemas.
- **B. Canonical loader:** No.
- **C. Pipeline code:** `ENTITY_PERSON` (`document_status.rs:108`); `PARTY_SUBTYPES` member (`:139`);
  cross-doc whitelist (`extraction_context.rs:88`); created by `create_party_nodes`
  (`ingest_helpers.rs:281-285,305`); resolution `ingest_resolver.rs:82`; completeness
  `completeness_helpers.rs:77-90`; `graph_migrations.rs:47` (`person_id_unique`).
- **D. Frontend:** 10 files.
- **E. Neo4j:** `:Person` (ingest-resolved). Read by `person_repository.rs`,
  `person_detail_repository.rs`.
- **F. Tier: T2** (resolved subtype). No conflict.

### 11. `Organization`

- Same shape as `Person`. `ENTITY_ORGANIZATION` (`document_status.rs:109`); `PARTY_SUBTYPES`;
  cross-doc whitelist (`extraction_context.rs:89`); `create_party_nodes`
  (`ingest_helpers.rs:281-285`); `ingest_resolver.rs:108`; completeness `completeness_helpers.rs:77-90`;
  `graph_migrations.rs:48` (`organization_id_unique`). Frontend: 7 files. **Tier: T2.** No conflict.

### 12. `ThematicAllegation` — **DEAD (verified)**

- **A. Schema/Template:** Emitted by **no active schema/template**. Active complaint schema documents
  it as REMOVED (`complaint_schema_v5_1.yaml:4,13`). Still defined in legacy `complaint_v5.yaml` and
  `pass1_complaint_v5.md`/`pass2_complaint_v5.md`.
- **B. Canonical loader:** No.
- **C. Pipeline code (residual):** `ENTITY_THEMATIC_ALLEGATION` (`document_status.rs:125`);
  `stable_entity_id` arm → `{doc_slug}:theme:{hash8}` (`ingest_helpers.rs:159-186`, tests
  `:1203-1278`); `graph_migrations.rs:56` (`thematic_allegation_id_unique`); verify special-case
  (`verify.rs:754`, doc `:678,730`, test `:1055`).
- **D. Frontend:** 0 files.
- **E. Neo4j:** `:ThematicAllegation` constraint exists; no active producer or reader on this branch.
- **F. Tier: dead T2 machinery** (no live producer; constant + arm + constraint + verify special-case
  persist).

---

## Relationship Type Traces

Per `document_status.rs:141-159`, relationship types are SCREAMING_SNAKE constants but **flow through
ingest as data** (`rel.relationship_type`, `ingest.rs:526-543`) rather than as literal references —
only `CONTAINED_IN` is hardcoded as a structural edge. Verify references **no** relationship types
(grep of `verify.rs` for all 17 returned nothing). Frontend references almost none (relationship logic
is backend-side). These facts hold for every row below unless noted.

### 1. `HAS_ELEMENT`
- **A. Schema:** Documented REMOVED from extraction (`complaint_schema_v5_1.yaml:35`); Pass-2 template
  says "no longer created by Pass 2 … Do NOT create" (`complaint_pass2_v5_1.md:47`). Active in legacy
  `complaint_v5.yaml`.
- **B. Loader:** **Yes both stores** — `authored_relationships` (`authored.rs:33,129,179`,
  `REL_HAS_ELEMENT`) and Neo4j (`cypher.rs:137` inside `upsert_element`).
- **C. Pipeline/ingest:** Not created by ingest. Not in `document_status.rs` `REL_*` set.
- **E. Tier: T1** (canonical loader; both stores). No conflict.

### 2. `ANCHORED_IN`
- **A. Schema:** Documented REMOVED (`complaint_schema_v5_1.yaml:37`); Pass-2 "no longer created … Do
  NOT create" (`complaint_pass2_v5_1.md:45,47,227,314`). Active in legacy `complaint_v5.yaml:209`.
- **B/C:** Not written by loader or ingest anywhere.
- **E. Tier: dead** (legacy v5 only; explicitly retired in v5.1).

### 3. `PROVES_ELEMENT` — **UN-PERSISTABLE BY PIPELINE**
- **A. Schema/Template:** Declared `relationship_types` (`complaint_schema_v5_1.yaml:180`) and
  `valid_patterns` `Allegation → PROVES_ELEMENT → Element` (`:220-222`). The central focus of
  `complaint_pass2_v5_1.md` (40+ mentions; "THE CENTRAL RELATIONSHIP OF V5", `:49`), instructing the
  LLM to target canonical Elements at `ctx:element-*` (`:24,49`).
- **B. Loader:** Does **not** write `PROVES_ELEMENT` to `authored_relationships` (only `HAS_ELEMENT`)
  nor to Neo4j. (Loader *wipes* incoming `PROVES_ELEMENT` on orphaned Elements, `cypher.rs:104-114`.)
- **C. Pipeline:** Stored as data if endpoints resolve — but the `Element` endpoint is a `ctx:`
  authored id that is **excluded from `id_map`** (Pass-2 §7.4), so `store_pass2_relationships`
  skips-and-logs it (`extraction_relationships.rs:354-361`). Ingest never sees it.
- **E. Tier: intended T3 (mapping), currently NOWHERE persisted.** No code writes it; the edge the
  LLM produces is dropped at Pass-2 storage. (See Pass 2 Context Assembly Analysis.)

### 4. `ABOUT`
- **A. Schema:** All three active v5.1 schemas (`complaint_schema_v5_1.yaml:182`,
  `affidavit_schema_v5_1.yaml:121`, `discovery_response_schema_v5_1.yaml:126`) + v4 schemas. Instructed
  in every active Pass-2 template.
- **C. Pipeline:** Data-flow only. `REL_ABOUT` constant defined (`document_status.rs:150`) but
  **0 non-definition uses**.
- **D. Frontend:** `BiasExplorer` references (`BiasExplorerFilters.tsx`, `index.tsx`).
- **E. Tier: T2** (extraction). No conflict.

### 5. `CAUSED_BY`
- Schema `complaint_schema_v5_1.yaml:184` (pattern `Harm→Allegation`, `:226-228`). Instructed
  `complaint_pass2_v5_1.md`. `REL_CAUSED_BY` defined (`document_status.rs:155`), **0 uses**. **T2.**

### 6. `DAMAGES_FOR` — endpoint-relevant to the LegalCount conflict
- Schema `complaint_schema_v5_1.yaml:186` (pattern `Harm → DAMAGES_FOR → LegalCount`, `:229-231`).
  Instructed `complaint_pass2_v5_1.md:195-205`. `REL_DAMAGES_FOR` defined (`document_status.rs:156`),
  **0 uses**. **T2.** *Targeting note:* a stored `DAMAGES_FOR` resolves at ingest to the **extracted**
  LegalCount id (`{doc_slug}:count:{N}`), not canonical `count-{N}` — unless the LLM targets a
  cross-doc/authored `ctx:` LegalCount (see Ingest analysis §4).

### 7. `SUFFERED_BY`
- Schema `complaint_schema_v5_1.yaml:188` (pattern `Harm→Party`, `:232-234`). `REL_SUFFERED_BY` defined
  (`:157`), **0 uses**. **T2.**

### 8. `EVIDENCED_BY`
- Schema `complaint_schema_v5_1.yaml:190` (pattern `Harm→Allegation`, `:235-237`). `REL_EVIDENCED_BY`
  defined (`:158`), **0 uses**. **T2.**

### 9. `STATED_BY`
- Schema `affidavit_schema_v5_1.yaml:119`, `discovery_response_schema_v5_1.yaml:124` + v4. Instructed in
  affidavit/discovery Pass-2 templates. `REL_STATED_BY` defined (`document_status.rs:149`), **0 uses**.
  Frontend `BiasByActorView.tsx`. **T2.**

### 10. `CORROBORATES`
- Schema `affidavit_schema_v5_1.yaml:123`, `discovery_response_schema_v5_1.yaml:128` + v4. Cross-doc
  edge type (affidavit/discovery → complaint). No `REL_` constant. Read by `evidence_chain_repository`,
  analysis repos. **T2.**

### 11. `CONTRADICTS`
- Schema `affidavit_schema_v5_1.yaml:125`, `discovery_response_schema_v5_1.yaml:130` + v4. `REL_`
  constant defined (`document_status.rs:153`), **0 uses**. Read by `contradiction_repository.rs`. **T2.**

### 12. `REBUTS`
- Schema `affidavit_schema_v5_1.yaml:127`, `discovery_response_schema_v5_1.yaml:132` + v4. `REL_REBUTS`
  defined (`:154`), **0 uses**. Read by `rebuttals_repository.rs`. **T2.**

### 13. `CHARACTERIZES` — **SCHEMA/TEMPLATE INCONSISTENCY**
- **A. Schema:** **Not declared** in any schema's `relationship_types` (absent from
  `discovery_response_schema_v5_1.yaml`). **Instructed** by `discovery_response_pass2_v5_1.md`
  ("### 6. CHARACTERIZES (Evidence → Party)", `:150-176,223-258`).
- **C. Pipeline:** No `REL_` constant. Stored as data if endpoints resolve (no schema validation gate
  rejects unknown rel types in `store_pass2_relationships`).
- **E. Neo4j/backend reads:** Queried in `allegation_detail_repository.rs:55`,
  `decomposition_repository.rs:68`, `person_detail_repository.rs:59`, `graph_expansion_queries.rs:76,139`
  (as **Evidence→Allegation**), and `case_summary_repository.rs:266-269` (returns zeros, "don't exist
  in v2"). **Direction mismatch:** template says Evidence→Party; two query repos read Evidence→Allegation.
- **E. Tier: T2** (extraction, template-only — not in schema). No tier conflict; schema/template/query
  inconsistency noted.

### 14. `CONTAINED_IN`
- **A. Schema:** Not in active v5.1 schemas (it is a structural ingest edge, not LLM-extracted);
  present in legacy `affidavit_v4.yaml`, `motion_v4.yaml`.
- **C. Pipeline:** **Hardcoded** structural edge. `REL_CONTAINED_IN` (`document_status.rs:148`) — the
  **only** used `REL_*` constant: `create_contained_in_relationships` (`ingest_helpers.rs:811-835`,
  uses `:828`) attaches every non-Document node to its Document; `graph_validation.rs:324,354` checks
  it.
- **E. Tier: T2 structural** (added by ingest). Does **not** touch Tier-1 canonical nodes (they are not
  in the ingested `items`); the **extracted** LegalCount/Element nodes do receive `CONTAINED_IN`, the
  canonical ones do not (see Ingest analysis §5).

### 15. `SUPPORTS` (legacy v4)
- Schema: legacy only — `complaint_v4.yaml`, `complaint_v5.yaml` (`:211` "Replaces v4's flat
  SUPPORTS"). Active v5.1 Pass-2 template names it only to say PROVES_ELEMENT replaces it
  (`complaint_pass2_v5_1.md:32,53`). `REL_SUPPORTS` defined (`document_status.rs:151`), **0 uses**.
- **E. Tier: dead** (superseded by `PROVES_ELEMENT`).

### 16. `PART_OF`
- Documented REMOVED (`complaint_schema_v5_1.yaml:12`); active only in legacy `complaint_v5.yaml`. No
  constant, no code, no template instruction. **Tier: dead.**

### 17. `THEME_SUPPORTS`
- Documented REMOVED (`complaint_schema_v5_1.yaml:12`); appears in **no** active or legacy schema as a
  live type (only the v5.1 removal comment). No constant, no code. **Tier: dead.**

---

## Legacy Artifacts

### Dead entity-type references
| Token | Where | Status |
|---|---|---|
| `ThematicAllegation` | `document_status.rs:125`; `ingest_helpers.rs:159-186` (+tests); `graph_migrations.rs:56`; `verify.rs:754` | No active producer; v5.1 documents removal |
| `Element` (as T2) | `document_status.rs:124`; `ingest_helpers.rs:130-158` (+tests); `graph_migrations.rs:55` | Extraction removed this branch; arms dormant |
| `Court` | `document_status.rs:126`; `graph_migrations.rs:57` (`court_id_unique`) | Defined by no schema |
| `Proceeding` | `document_status.rs:127`; `graph_migrations.rs:58` | Defined by no schema |
| `ProceduralEvent` | `document_status.rs:128`; `graph_migrations.rs:59` | Defined by no schema |
| `Role` | `document_status.rs:129`; `graph_migrations.rs:60` | Defined by no schema |
| `Assertion` | `document_status.rs:130`; `graph_migrations.rs:61` | Defined by no schema |
| `MotionClaim` | `motion_v4.yaml`, `brief_v4.yaml`; backend `motion_claim_repository.rs`, `ask.rs`, `claims.rs`, `query_repository.rs`, `evidence_chain_repository.rs`, `embedding_repository.rs`, `allegation_detail_repository.rs`, `admin_document_evidence.rs`; frontend 4 files | v4 entity, live in RAG/query path |
| `Exhibit` | `brief_v4.yaml`, `pass1_brief_v4.md`, `tests/completeness_validation.rs` | v4 entity |
| `FactualAllegation` | **0 hits anywhere** | Never present / fully gone |
| `Claim` | (bare token; subsumed by `MotionClaim` usages) | v4-era |

### Dead relationship-type references
| Token | Where | Status |
|---|---|---|
| `REL_STATED_BY`, `REL_ABOUT`, `REL_SUPPORTS`, `REL_CORROBORATES`, `REL_CONTRADICTS`, `REL_REBUTS`, `REL_CAUSED_BY`, `REL_DAMAGES_FOR`, `REL_SUFFERED_BY`, `REL_EVIDENCED_BY`, `REL_DERIVED_FROM` | Defined `document_status.rs:149-159`; **0 non-definition references** (verified per-constant) | Defined-but-unused constants (10 + DERIVED_FROM = 11; the rel types themselves flow as data and DERIVED_FROM is written via a string literal `ingest_helpers.rs:740`) |
| `SUPPORTS` (rel) | legacy schemas only | Superseded by `PROVES_ELEMENT` |
| `PART_OF`, `THEME_SUPPORTS` | v5.1 removal comments + legacy `complaint_v5.yaml` (PART_OF) | Retired |
| `ANCHORED_IN` | v5.1 removal comment + legacy `complaint_v5.yaml` | Retired |

### v4 artifacts
- v4 schemas still on disk: `complaint_v4.yaml`, `affidavit_v4.yaml`, `discovery_response_v4.yaml`,
  `court_ruling_v4.yaml`, `motion_v4.yaml`, `brief_v4.yaml`, `complaint_v5.yaml`.
- v4 templates: `pass1_complaint_v4.md`, `pass2_complaint_v4.md`, `pass1_affidavit_v4.md`,
  `pass2_affidavit_v4.md`, `pass1_discovery_response_v4.md`, `pass2_discovery_response_v4.md`,
  `pass1_brief_v4.md`, `pass2_brief_v4.md`, `pass1_court_ruling_v4.md`, `pass2_court_ruling_v4.md`,
  `pass1_complaint_v3_restored.md`, `pass1_complaint_v5.md`, `pass2_complaint_v5.md`, `global_rules_v4.md`.
- v4 label `ComplaintAllegation` still referenced by the **active** `affidavit_pass2_v5_1.md:80,88,158`.

### Orphaned templates/schemas (present on disk, not referenced by any profile)
Referenced-by-profile fields scanned: `schema_file`, `template_file`, `pass2_template_file`,
`global_rules_file`, `system_prompt_file`.
- **Orphaned templates (5):** `pass1_affidavit_v4.md`, `pass1_complaint_v3_restored.md`,
  `pass1_discovery_response_v4.md`, `pass2_affidavit_v4.md`, `pass2_discovery_response_v4.md`.
  (`global_rules_v4.md` and `legal_extraction_system.md` are **not** orphaned — referenced via
  `global_rules_file`/`system_prompt_file` in most profiles.)
- **Orphaned schemas (3):** `affidavit_v4.yaml`, `discovery_response_v4.yaml`, `general_legal.yaml`.
  `general_legal.yaml` is **not profile-referenced but IS code-referenced** (`main.rs`, `state.rs`,
  `verify.rs`, `ingest_helpers.rs`, `dto/schema.rs`) — a code-level fallback/default, not a true orphan.
- **Broken profile references (2):** `motion.yaml` → `pass1_motion_v4.md` (**missing on disk**) and
  `pass2_universal_v4.md` (**missing**); `default.yaml` → `pass2_universal_v4.md` (**missing**). These
  profiles would fail template load if selected.
- **Unused-by-active-profile-but-valid:** `complaint.yaml`/`complaint_v5.yaml` profiles (v4/v5
  complaint), `brief.yaml`, `court_ruling.yaml` — all reference existing files but are superseded by
  the v5.1 profiles for the active case.

### Hardcoded entity-type strings (not derived from schema YAML)
- `stable_entity_id` matches hardcoded constants per type (`ingest_helpers.rs:74-194`):
  `ENTITY_COMPLAINT_ALLEGATION`, `ENTITY_LEGAL_COUNT`, `ENTITY_HARM`, `ENTITY_ELEMENT`,
  `ENTITY_THEMATIC_ALLEGATION` — i.e. the ID scheme is type-specific code, not schema-driven.
- `verify.rs:691` hardcodes `entity_type == "Allegation"`; `verify.rs:754` hardcodes
  `== "ThematicAllegation"`.
- `graph_migrations.rs:46-61` hardcodes the full constraint label list (13 labels) independent of any
  schema.
- `CROSS_DOC_ENTITY_TYPES` (`extraction_context.rs:86-96`) is a compile-time list of 9 labels (by
  deliberate design — `extraction_context.rs:46-62` documents why it is `const`, not config).
- `create_entity_node` uses `item.entity_type` **directly** as the Neo4j label
  (`ingest_helpers.rs:381`), so the label set is whatever the schema emits (no central enum gate beyond
  the alphanumeric-injection guard `:361`).

---

## stable_entity_id Analysis

**Location/signature:** `backend/src/api/pipeline/ingest_helpers.rs:71`
`pub fn stable_entity_id(item: &ExtractionItemRecord, doc_id: &str) -> String`.

**Inputs:** an `ExtractionItemRecord` (its `entity_type` selects the match arm; `item_data["properties"]`
supplies the keying fields) and `doc_id` (slugged into the `{doc_slug}` prefix via `slug()`,
`ingest_helpers.rs:30-39,72`).

**ID generation logic (per arm):**
| entity_type (constant) | ID format | Key source | Lines |
|---|---|---|---|
| `ComplaintAllegation` | `{doc_slug}:para:{paragraph}` | `paragraph_number` → `paragraph_ref` → `hash-{sha8}` of `summary`/`allegation_text` | 75-100 |
| `LegalCount` | `{doc_slug}:count:{count_number}` | `count_number` (u64 or str) → `hash-{sha8}` of `legal_basis` | 101-118 |
| `Harm` | `{doc_slug}:harm:{sha8}` | sha256(`doc_id`+`harm_type`+`description`) | 119-129 |
| `Element` | `{doc_slug}:element:{sha8}` (or `:element:hash-{sha8}`) | sha256(`parent_count_id`\|sorted `anchor_paragraph_numbers`\|`element_name`); fallback hashes `item_data` | 130-158 |
| `ThematicAllegation` | `{doc_slug}:theme:{sha8}` (or `:theme:hash-{sha8}`) | sorted `paragraph_numbers`; fallback hashes `item_data` | 159-186 |
| *other* | `{doc_slug}:{slug(type)}:{sha8}` | sha256(`item_data`) | 187-192 |

**Entity types whose IDs it generates:** `ComplaintAllegation`, `LegalCount`, `Harm`, `Element`,
`ThematicAllegation`, plus the generic fallback for everything else (e.g. `Allegation`, `Evidence`,
`LegalCount` legacy aliases). Note **no `Allegation` arm** — v5.1 `Allegation` items take the generic
`other` arm (`{doc_slug}:allegation:{sha8}`), *not* the `:para:{n}` form the `ComplaintAllegation` arm
produces.

**Tier-1 (canonical) types among these — the conflict points:**
- **`LegalCount`** → extracted id `{doc_slug}:count:{N}` (e.g. `doc-awad-…:count:1`) collides
  conceptually with canonical id `count-{N}` (e.g. `count-1`, `cypher.rs:285-289`,
  `authored.rs:50-52`). Different strings ⇒ **two Neo4j nodes**. This is the live conflict.
- **`Element`** → extracted id `{doc_slug}:element:{sha8}` vs canonical `element-{N}-{M}`
  (`authored.rs:154-168`). Dormant: no active schema emits `Element`, so this arm is currently
  unreached, but it is the latent conflict surface.

**Special cases / overrides in the function:**
- Defensive empty-key fallbacks in the `ComplaintAllegation`, `Element`, and `ThematicAllegation` arms
  to avoid collapsing many entities onto the empty-string hash (`:91-98,143-147,171-176`).
- v4/v2 property-name compatibility (`paragraph_number` vs `paragraph_ref`, `summary` vs
  `allegation_text`) (`:84-95`).
- Anchor/paragraph normalisation (split/trim/sort) so order/whitespace don't change the id
  (`:149-153,180-182`).
- `Party` is **not** handled here — Party IDs are generated separately in `create_party_nodes`
  (`person-{slug}` / `org-{slug}`, or the resolver's resolved id; `ingest_helpers.rs:288-294`).

---

## Ingest Step Tier 1 Conflicts

Read in full: `backend/src/pipeline/steps/ingest.rs` (713 lines) and
`backend/src/api/pipeline/ingest_helpers.rs` (1402 lines).

**1. Awareness of `authored_entities` / Tier 1?** **None.** `run_ingest` reads only extraction data:
`get_approved_items_for_document` and `get_approved_relationships_for_document_all_passes`
(`ingest.rs:295-313`). No import of the `authored_entities` repository, no `case_slug`, no query of the
`authored_entities`/`authored_relationships` tables anywhere in the ingest path (grep: 0).

**2. Does ingest check for an existing canonical node before creating?** **No.** `create_entity_node`
unconditionally MERGEs `(n:{entity_type} {id: $id})` with `id = stable_entity_id(...)`
(`ingest_helpers.rs:368-404`). It does not look up canonical ids or `count_number`-keyed nodes.

**3. MERGE behaviour when a canonical `count-1` already exists and ingest MERGEs `doc-…:count:1`:**
MERGE matches solely on the `{id}` property (`ingest_helpers.rs:381`). Canonical id `count-1` and
extracted id `doc-awad-…:count:1` are different strings, so **MERGE does not match the canonical node —
it creates a second `:LegalCount` node.** Both carry label `:LegalCount`; they differ only by `id`.
Net effect: **duplicate LegalCount nodes** (the reported defect). No error, no warning — the duplicate
is silent at the Neo4j layer.

**4. Which LegalCount does a Pass-2 `DAMAGES_FOR` edge target?** Relationship endpoints are resolved
from `extraction_relationships` rows (`ingest.rs:495-519`) through `pg_to_neo4j` (built from this
document's items) or the cross-doc `lookup_neo4j_node_ids` map. A `DAMAGES_FOR` from a local `Harm` to
a **local** extracted `LegalCount` resolves to the **extracted** id `{doc_slug}:count:{N}` — i.e. it
attaches to the **duplicate/extracted** node, *not* canonical `count-{N}`. The only way an edge would
target canonical `count-{N}` is if the LLM emitted the canonical id as the endpoint *and* that id were
in `id_map` — which it is **not**, because authored entities are excluded from `id_map` (§7.4) and so
such an edge would be skipped at Pass-2 storage rather than reaching ingest. Conclusion: pipeline
`DAMAGES_FOR` edges land on the extracted LegalCount node.

**5. `CONTAINED_IN` and Tier 1:** `create_contained_in_relationships` (`ingest_helpers.rs:811-835`)
attaches **every node in `all_nodes_with_runs`** to the Document. `all_nodes_with_runs` is built from
this document's `items` only (`ingest.rs:412-422`). Therefore: the **extracted** LegalCount/Element
nodes receive `CONTAINED_IN`; **canonical Tier-1 nodes do not** (they are never in `items`). This
further separates the two graphs: the duplicate extracted `:LegalCount` is `CONTAINED_IN` the Document,
while the canonical `:LegalCount` is not — they are not merged or cross-linked by ingest.

**Idempotency note:** ingest is cleanup-then-write — `run_ingest` calls `cleanup_neo4j(doc_id)` first
(`ingest.rs:255-265`) because the helpers use CREATE/MERGE without a Tier-1 reconciliation. Cleanup is
scoped to this `doc_id`'s nodes; it does not touch canonical nodes.

---

## Pass 2 Context Assembly Analysis

Read in full: `backend/src/pipeline/steps/llm_extract_pass2.rs` (1078 lines) and
`backend/src/repositories/pipeline_repository/extraction_context.rs` (633 lines); plus
`store_pass2_relationships` (`extraction_relationships.rs:342-380`).

**1. When `CASE_SLUG` is set, which authored types appear in the prompt, and in what format?**
`run_pass2_extraction` calls `load_authored_entities_for_context(db, case_slug,
CROSS_DOC_ENTITY_TYPES)` (`llm_extract_pass2.rs:494-501`). The filter is the 9-type whitelist
(`extraction_context.rs:86-96`): `Party, Person, Organization, LegalCount, ComplaintAllegation,
Allegation, Evidence, Element, Harm`. In practice the loader only writes `LegalCount` and `Element`
rows to `authored_entities` (`authored.rs:138-168`), so only **`LegalCount` and `Element`** authored
rows can surface. Each renders via `authored_record_to_cross_doc` → `CrossDocEntity::to_prompt_value`
(`extraction_context.rs:332-361,184-211`): id = `ctx:<item_data.id or entity_id>` (e.g.
`ctx:element-1-1`, `ctx:count-1`), `source_document = "canonical"`,
`source_document_type = "canonical_element_library"` (`extraction_context.rs:303,312`), and a per-type
property allowlist. They are appended **last** to `entities_prompt` (`llm_extract_pass2.rs:563-567`).

**2. When `CASE_SLUG` is NOT set:** the `None` arm logs
`"Pass 2: CASE_SLUG not configured — authored entity context disabled"` and returns an empty `Vec`
(`llm_extract_pass2.rs:510-516`). **Logged, not silent.** `case_slug` originates from
`std::env::var("CASE_SLUG").ok()` (`config.rs:237`) → `AppContext.case_slug: Option<String>`
(`context.rs:87`). The `authored_context_entities` count (0 here) is also recorded in
`processing_config` and the step result (`llm_extract_pass2.rs:117,197-201`), so "set, 0 injected"
stays distinguishable from "unset" in the DB.

**3. Does Pass 1 receive authored context?** **No.** `steps/llm_extract.rs` has no
`load_authored_entities`, `load_cross_document_context`, or `case_slug` call; the only `cross_doc`
references are `pass2_cross_doc_entities: &[]` / `Vec::new()` snapshot fields for Pass-1 runs
(`llm_extract.rs:659,2610,2646,2776,2791,2819,2854`). Authored/cross-doc context is Pass-2-only.

**4. Could the `id_map` accidentally include authored entities?** **No.** `id_map` is built from local
Pass-1 entities (`llm_extract_pass2.rs:570-574`) plus cross-doc extracted entities
(`:575-577`). `authored_context_entities` is a separate `Vec` only `extend`ed into `entities_prompt`
(`:563-567`), never into `id_map` (Option B; defended further by the negated `item_id = -record.id`,
`extraction_context.rs:352`). A residual collision risk exists only if an authored `prefixed_id` exactly
equals a cross-doc extracted `prefixed_id` (e.g. both `ctx:count-1`); the cross-doc insert would then
win the `id_map` slot. In practice extracted LegalCounts use LLM ids like `count-001` while authored use
`count-1`, so no collision is observed, but the de-dup is by string equality, not by tier.

**5. Full path of a Pass-2 relationship targeting a `ctx:`-prefixed entity:**
LLM output → `parse_chunk_response` → `store_pass2_relationships` (`llm_extract_pass2.rs:656-665`) →
for each relationship, `resolve_relationship_fields` extracts `from`/`to`/`type`
(`extraction_relationships.rs:352`) → **`id_map` lookup** of both endpoints (`:354`). If either endpoint
is absent from `id_map`, the edge is **skipped-and-logged**:
`tracing::warn!("Pass 2: skipping relationship with unresolved endpoint(s)")` then `continue`
(`extraction_relationships.rs:354-361`). Therefore:
- `ctx:<extracted-cross-doc-id>` → **in `id_map`** → stored, FK to the cross-doc `extraction_items.id`.
- `ctx:<authored-id>` (e.g. `ctx:element-1-1`, `ctx:count-1`) → **not in `id_map`** → **dropped**.

This is the mechanism by which **`PROVES_ELEMENT` (Allegation→`ctx:element-*`) edges are dropped at
storage**: the `Element` endpoint is authored, hence absent from `id_map`. The Pass-2 template instructs
the LLM to create exactly these edges (`complaint_pass2_v5_1.md:49,118`), but the pipeline cannot persist
them, and no Tier-3 mapping step writes them to `authored_relationships` (only `HAS_ELEMENT` is written
there, by the loader).

---

## Conflict Summary Table

| entity_type | tier_1 source | tier_2 source | conflict locations | fix required (per instruction: decision deferred) |
|---|---|---|---|---|
| `LegalCount` | `authored.rs:138-151` (`count-{N}`); Neo4j `id` stamp `cypher.rs:285-289` | `complaint_schema_v5_1.yaml:80`; `stable_entity_id` `ingest_helpers.rs:101-118` (`{doc_slug}:count:{N}`); MERGE `ingest_helpers.rs:381` | (a) `authored_entities` row vs `extraction_items` row; (b) Neo4j `count-{N}` vs `{doc_slug}:count:{N}` → 2 nodes | YES (active duplicate) |
| `Element` | `authored.rs:154-168` (`element-{N}-{M}`); Neo4j `cypher.rs:121-155` | none active (removed `complaint_schema_v5_1.yaml:29-40`); dormant `stable_entity_id` arm `ingest_helpers.rs:130-158`; constraint `graph_migrations.rs:55` | dormant `stable_entity_id`/constraint/constant only | LATENT (no live producer) |
| `BreachTheory` | Neo4j only `cypher.rs:194-202` (no Postgres row) | none | T1 Neo4j w/o system-of-record row | N/A (asymmetry, not duplication) |
| `ImproperActTheory` | Neo4j only `cypher.rs:205-213` | none | same | N/A |
| `DeclarationSought` | Neo4j only `cypher.rs:216-238` | none | same | N/A |
| `Party`/`Person`/`Organization` | none | v5.1 schemas; `create_party_nodes` | — | NO |
| `Allegation` | none | `complaint_schema_v5_1.yaml:118` | label coexists with v4 `ComplaintAllegation`; no `stable_entity_id` arm | NO (consistency note) |
| `Harm`/`Evidence` | none | v5.1 schemas | — | NO |
| `ThematicAllegation` | none | none active | dead machinery only | N/A (dead) |

## Dead Code Summary Table

| artifact | location | last used (evidence) | safe to remove (factual; decision deferred) |
|---|---|---|---|
| 10 `REL_*` constants (`STATED_BY,ABOUT,SUPPORTS,CORROBORATES,CONTRADICTS,REBUTS,CAUSED_BY,DAMAGES_FOR,SUFFERED_BY,EVIDENCED_BY`) | `document_status.rs:149-158` | 0 non-definition refs (verified per-constant); types flow as data | constants unused; removing them changes no runtime behaviour |
| `REL_DERIVED_FROM` | `document_status.rs:159` | 0 refs; ingest uses literal `"DERIVED_FROM"` (`ingest_helpers.rs:740`) | unused constant |
| `ThematicAllegation` machinery | `document_status.rs:125`; `ingest_helpers.rs:159-186`; `graph_migrations.rs:56`; `verify.rs:754` | no active schema/template producer | no live producer on this branch |
| `Element` T2 machinery | `ingest_helpers.rs:130-158`; `graph_migrations.rs:55`; `document_status.rs:124` | extraction removed @ HEAD commit | no live producer; query repos still READ `:Element` (canonical) |
| `Court/Proceeding/ProceduralEvent/Role/Assertion` | `document_status.rs:126-130`; `graph_migrations.rs:57-61` | defined by no schema | no producer/reader found |
| `SUPPORTS`/`PART_OF`/`THEME_SUPPORTS`/`ANCHORED_IN` (rels) | v5.1 removal comments; legacy `complaint_v5.yaml` | retired in v5.1 | not produced by active pipeline |
| Orphaned templates (5) | `pass1_affidavit_v4.md`, `pass1_complaint_v3_restored.md`, `pass1_discovery_response_v4.md`, `pass2_affidavit_v4.md`, `pass2_discovery_response_v4.md` | no profile reference | not loaded by any profile |
| Orphaned schemas (2) | `affidavit_v4.yaml`, `discovery_response_v4.yaml` | no profile reference | superseded by v5.1 |
| `general_legal.yaml` | profiles: none; code: `main.rs`, `state.rs`, `verify.rs`, `ingest_helpers.rs`, `dto/schema.rs` | code-level fallback | NOT removable without code change |
| Broken profile refs | `motion.yaml`→`pass1_motion_v4.md`+`pass2_universal_v4.md`; `default.yaml`→`pass2_universal_v4.md` | template files missing on disk | profiles non-functional as written |
| `MotionClaim`/`Exhibit` (v4 entities) | v4 schemas + RAG repos (`motion_claim_repository.rs` etc.) | live in RAG/query path for v4 docs | NOT dead in RAG path |
| `CHARACTERIZES` schema gap | instructed `discovery_response_pass2_v5_1.md`; absent from `discovery_response_schema_v5_1.yaml` `relationship_types` | live template instruction | schema/template inconsistency (not removable; under-declared) |

---

*End of audit.*
