# Colossus-Legal Decision Log

This log tracks decisions made across all roles that may impact other team members. All roles must check this document at session start.

---

## 2026-01-19 | DATA ARCHITECT (Roman Approved)

### Decision
Added **Harm** node type to the data model to track damages caused to Marie.

### Rationale
The lawsuit's core purpose is to prove:
1. Incompetence and negligence by CFS/Phillips
2. **Harm caused to Marie** as a result

Without tracking Harm explicitly, we cannot:
- Quantify damages for court
- Connect misconduct to actual injury
- Answer "what did this cost Marie?"

### Harm Categories Defined

| Category | Type | Description |
|----------|------|-------------|
| `financial_direct` | Sanctions | Money taken directly from Marie (e.g., 100% cost assessment) |
| `financial_estate` | Incompetence | Estate losses that reduced Marie's inheritance |
| `reputational` | Character Attacks | False accusations damaging Marie's reputation |

### New Node Type: Harm

```
Harm {
  id: string,                    // "harm-001"
  title: string,                 // Short description
  category: string,              // financial_direct, financial_estate, reputational
  subcategory: string,           // sanction, incompetence, character_attack
  amount: float | null,          // Dollar amount if quantifiable
  description: date | null,      // Detailed description
  date: date | null,             // When harm occurred
  source_reference: string       // Document/page reference
}
```

### New Relationships

| Relationship | From | To | Meaning |
|--------------|------|-----|---------|
| `CAUSED_BY` | Harm | ComplaintAllegation | This harm resulted from this misconduct |
| `EVIDENCED_BY` | Harm | Evidence | This evidence proves the harm occurred |
| `DAMAGES_FOR` | Harm | LegalCount | This harm supports damages claim for this count |

### Impacts
- Data Architect: Schema updated to v3 with Harm node
- DB Engineer: **MUST IMPLEMENT** - Add Harm nodes for all identified damages
- Software Architect: API must expose harm/damages queries

### Action Required
- [ ] DB Engineer: Create Harm nodes for identified damages
- [ ] DB Engineer: Link Harms to Allegations via CAUSED_BY
- [ ] DB Engineer: Link Harms to Evidence via EVIDENCED_BY
- [ ] Software Architect: Plan API endpoints for damage queries

### Known Harms to Populate

| ID | Title | Category | Amount |
|----|-------|----------|--------|
| harm-001 | 100% Appellate Costs to Marie | financial_direct | $15,246.94 |
| harm-002 | MCR 2.114 Sanction - Lost Reimbursement | financial_direct | $2,345.00 |
| harm-003 | Unnecessary Auction Loss | financial_estate | ~$6,000 |
| harm-004 | Estate Depletion from Fees | financial_estate | TBD |
| harm-005 | Lost 1/3 of $50K Conversion | financial_estate | ~$16,667 |
| harm-006 | "North Korea" Comparison | reputational | N/A |
| harm-007 | "Fanciful Conspiracy Theories" | reputational | N/A |
| harm-008 | "Obstructive" Characterization | reputational | N/A |
| harm-009 | Selective Sanctions vs Sisters | reputational | N/A |

---

## 2026-01-19 | PROJECT MANAGEMENT (Roman)

### Decision
Clarified project mission and team structure.

### Key Points
1. Legal team is THREE people: one attorney, Marie, and Roman
2. Colossus-Legal must function as their "high powered law firm"
3. System must not just organize evidence, but:
   - Prove ALL allegations
   - Quantify HARM to Marie
   - Generate COURT-READY output

### Impacts
- All Roles: Shift from "reference tool" to "litigation engine"
- Phase 4 (Court Output) elevated in priority

### Action Required
- [ ] All: Ensure every feature supports litigation goals
- [ ] Data Architect: Harm tracking added (see above)

---

## 2026-01-14 | PROJECT MANAGEMENT (Roman)

### Decision
Established multi-role coordination structure with three specialized Claude personas.

### Rationale
- Context management: Each role has focused scope, preventing context overflow
- Clear authority: Data model decisions cascade down, preventing conflicts
- Parallel work: Roles can operate concurrently with defined interfaces

### Roles Defined
1. **Senior Data Architect** - Schema, data model, use cases
2. **Senior DB Engineer** - Neo4j, Cypher, document processing
3. **Senior Software Architect** - Rust/Axum, React, implementation

### Authority Hierarchy
```
Roman (Product Owner) → Data Architect → DB Engineer → Software Architect
```

### Impacts
- Data Architect: Now owns architecture/ folder and schema decisions
- DB Engineer: Now owns database/ folder, adapts to schema changes
- Software Architect: Now owns development/ folder, adapts to both above

### Action Required
- [x] Create COORDINATION.md
- [x] Create DECISION_LOG.md
- [x] Create PROJECT_ROADMAP.md
- [ ] Data Architect: Review and formalize DATA_MODEL_v2.md into v3
- [ ] DB Engineer: Document current Neo4j state (81 nodes, 180 relationships)
- [ ] Software Architect: Review current codebase state

---

## 2026-01-14 | DB ENGINEER (Prior Session - Documented Retroactively)

### Decision
Completed evidence chains for 8 of 18 complaint allegations in Neo4j.

### Rationale
Prioritized allegations with strongest evidence from CFS and Phillips interrogatory responses.

### Current Neo4j State
- **Nodes:** 81 total
  - Evidence: 29
  - ComplaintAllegation: 18
  - MotionClaim: 12
  - Person: 7
  - Document: 5
  - LegalCount: 4
  - Organization: 3
  - Case: 2
  - Event: 1

- **Relationships:** 180 total
  - RELIES_ON: 29
  - CONTAINED_IN: 29
  - INVOLVES: 28
  - SUPPORTS: 26
  - IN_CASE: 22
  - APPEARS_IN: 14
  - PROVES: 12
  - Others: 20

### Completed Allegations
| ID | Title | Counts Supported |
|----|-------|------------------|
| complaint-001 | Undisclosed CFS-Court Contract | I |
| complaint-005 | $50K Conversion by Sisters | I, II |
| complaint-007 | Estate Was Unnecessary | I, II |
| complaint-011 | Auction Caused $6K Loss | I, IV |
| complaint-015 | Selective Sanctions | I, IV |
| complaint-016 | 100% Costs to Marie | I, IV |
| complaint-017 | MCL 700.1212 Violation | I |
| complaint-018 | CFS Ultra Vires | III |

### Impacts
- Data Architect: None - implemented existing model
- DB Engineer: Continue with remaining 10 allegations
- Software Architect: Traceability queries available for API

### Action Required
- [ ] DB Engineer: Populate remaining 10 allegations
- [ ] DB Engineer: Add court documents (Judge Tighe, COA rulings) as evidence
- [ ] DB Engineer: Implement Harm nodes per 2026-01-19 decision

---

## 2026-07-18 | SOFTWARE ARCHITECT (Roman Approved)

### Decision
Added a **tightly-scoped best-effort carve-out to the "No silent failures" frontend
rule** (Standing Rule 1 in `CLAUDE.md`, and Rule 9 in `.claude/agents/rules-enforcer.md`).
A `catch` around reading/writing a **cosmetic UI preference to browser storage**
(`localStorage`/`sessionStorage` — e.g. remembering a panel's collapsed state) may
degrade to a default WITHOUT a user-facing banner, provided it (a) carries a
`// best-effort:` comment and (b) stays observable via `console.warn`. Worded as the
direct parallel to the existing `.ok()` `// best-effort:` carve-out (rules-enforcer
Rule 5).

### Rationale
The Theme Scan card's per-scenario collapse state is persisted to `localStorage`
(the ratified "remember collapse" decision). A storage failure (privacy mode / quota)
has no user recovery action, and a user-facing banner for a lost cosmetic preference
is disproportionate noise. The rule previously admitted no exception, forcing either a
disproportionate UI surface or dropping persistence. The carve-out resolves that while
keeping the failure observable (`console.warn`) and keeping the **full** explicit
`.catch()` + error-UI requirement for `fetch`/`authFetch` and ANY data read/write — a
failed *data* operation is never best-effort.

### Impacts
- Data Architect: None
- DB Engineer: None
- Software Architect: Governing-doc change — `CLAUDE.md` Standing Rule 1 and the
  `rules-enforcer` agent now both carry the carve-out. Applies ONLY to cosmetic
  browser-storage preferences; not a general catch-suppression loophole.

### Action Required
- [x] Update `CLAUDE.md` Standing Rule 1 frontend bullet
- [x] Update `.claude/agents/rules-enforcer.md` Rule 9
- [x] Log here

---

## 2026-07-18 | SOFTWARE ARCHITECT (Roman Approved)

**Decision:** Track `.claude/agents/` in git (`.gitignore`: `.claude/*` + `!.claude/agents/`) so the four-agent enforcement gate is identical on every machine/CI; `settings.local.json`, its `.bak`, and `settings.json` stay ignored. Fixes governance drift where the Rule 9 carve-out lived only in the local working tree.

---

## 2026-07-27 | SOFTWARE ARCHITECT (Roman Approved)

### Decision
**Verification queries are written from the code's write paths, never from assumed
property names.** Any figure used to judge the knowledge base — in an assessment, a
report, or a dashboard — must be produced by a query whose property and label names
were read out of the code that WRITES them.

### Rationale
Two of the four alarm findings in the 2026-07-26 graph assessment were measurement
errors, not data problems:

- **`document_type` NULL on all 9 Document nodes.** The property has never existed.
  Every write path — `api::pipeline::ingest_helpers::create_document_node` (both the
  full and delta ingest paths) and `repositories::document_repository` — writes
  `doc_type`. A live query on 2026-07-27 confirmed `doc_type` populated on all nine
  nodes (discovery_response ×2, court_ruling ×2, affidavit ×2, complaint,
  correspondence, court_transcript). `file_name` is likewise not a real property; the
  display name is `title`.
- **`to_element: 0`.** A query for direct `Evidence → Element` edges, which were never
  part of the design. The spine is
  `LegalCount → HAS_ELEMENT → Element ← BEARS_ON ← Allegation ← Evidence` and is intact.

Both errors pointed at real-looking crises and cost real investigation time. A wrong
property name returns NULL rather than an error, so the failure mode is a plausible
finding, not a visible fault — which is precisely why it needs a rule rather than
care.

### Impacts
- Data Architect: The v1 §9 "hard prerequisite" (populate `document_type` before the
  by-type rollup) is **DELETED**. The by-type rollup is unblocked.
- DB Engineer: None. No schema change; no data patch was made or is needed.
- Software Architect: New surfaces that display graph figures carry a documented-query
  artifact (see the Case Health entry below). Case Health pins the correct spellings by
  test — `documents_query_reads_the_properties_the_write_path_actually_sets` fails the
  build if `document_type` or `file_name` appears in its generated Cypher.

### Action Required
- [x] Correct the requirement document (`CASE_HEALTH_DASHBOARD_REQUIREMENT_v1_1.md` §1)
- [x] Pin the correct property names by test in `case_health_repository_tests.rs`
- [x] Record the spot-check query in `docs/CASE_HEALTH_QUERIES.md` §5.3
- [ ] Data Architect: apply the same discipline when authoring future assessment queries

---

## 2026-07-27 | SOFTWARE ARCHITECT (Roman Approved)

### Decision
**The corpus connection headline is the PROBATIVE rate, and `ABOUT` is excluded from
it.** Two tiers, defined once in `backend/src/domain/connection_tier.rs`:

- **Probative** — Evidence with ≥1 `CORROBORATES` / `REBUTS` / `CHARACTERIZES` edge to
  an `Allegation`. **This is the headline.**
- **Topical** — probative plus `ABOUT` → `Allegation`. Displayed beside the headline,
  separately labeled.

The two are never blended into a single number, on any surface.

### Rationale
`CORROBORATES`, `REBUTS` and `CHARACTERIZES` each say something about whether an
allegation is TRUE — they move the proof one way or the other. `ABOUT` deliberately
says only that an item is on the subject. Counting `ABOUT` in the headline inflates
precisely the number this instrument exists to keep honest: a corpus of 500 items all
merely "about" the case would report as fully connected while proving nothing.

The 24% figure from the 2026-07-26 assessment blended the two and is superseded. The
honest headline is lower than 24%. That is the point.

The topical rate is not noise and is not hidden — it is the honest measure of "material
the E→E edge layer could promote". It is simply a different claim, so it gets its own
label and its own one-line explanation on screen.

### Impacts
- Data Architect: The connection metric is two-tier wherever it appears.
- DB Engineer: None.
- Software Architect: `CONNECTION_TIER_LOOKUP_V` (currently `1`) versions the partition
  and ships in the payload, so a future snapshot delta can refuse to compare rates
  computed under different definitions. The partition lives in code, not config —
  which edges are probative is a fact about the v5 schema, not about a case or a
  deployment, so another Colossus case renders with zero code changes.

### Action Required
- [x] Implement in `domain::connection_tier` with the ABOUT-exclusion pinned by test
- [x] Ship both rates, separately labeled, in Pane 1
- [ ] Software Architect: apply the same two-tier treatment in Panes 2–4 (later chunks)

---

## 2026-07-27 | SOFTWARE ARCHITECT (Roman Approved)

### Decision
**One element-verdict vocabulary in the codebase.** Pane 2 of Case Health (a later
chunk) will WRAP the existing `causes_of_action_repository::elements_query` +
`causes_of_action_builder::derive_proof_status` computation and its four-state verdict
(`no_allegations` / `gap` / `partial` / `supported`). No second element-verdict
vocabulary may be introduced. REBUTS exposure is layered on as additional columns
BESIDE the existing verdict, never altering it. The thresholds stay compiled-in for v1;
configurability is deferred, not redesigned.

### Rationale
The Proof Matrix page already ships `derive_proof_status` to the frontend
(`services/proofMatrix.ts`). A parallel `covered / thin / naked` vocabulary computed
from the same graph would give the same Element two verdicts on two pages, and they
would eventually disagree — the reader would then have no way to know which page was
lying. One vocabulary, extended where it is thin, is strictly better than two that
agree today.

Chunk 1 (Pane 1) introduces **no** verdict at all: it reports rates and counts and
renders no judgement, so it cannot violate this rule.

### Impacts
- Data Architect: None.
- DB Engineer: None.
- Software Architect: Binding constraint on the Pane 2 chunk. Threshold configurability
  is a separate, later decision.

### Action Required
- [x] Chunk 1 ships no verdict vocabulary
- [ ] Software Architect: Pane 2 chunk wraps `derive_proof_status` rather than
      duplicating it

---

## 2026-07-27 | DATA ARCHITECT (Roman Approved)

### Decision
**"Themes with zero candidates" is deleted from the Case Health Pane 3 requirement.**
No theme concept exists in storage.

### Rationale
A read of the code found no theme entity anywhere: `ScenarioDefinition`
(`dto::scenario_crud`, schema_v 2) carries only `attack_text`, `attack_meaning`,
`target`, `wielders`, `schema_v` — the D1 rebuild retired `seed_phrases` /
`anti_seed_phrases` / `notes` with no successor; the `scenarios`,
`scenario_fact_refs`, `scan_runs` and `scan_run_verdicts` tables have no theme column.
"Theme Scan" names the ACTIVITY (judging candidates against a scenario's single
`attack_text`), not a stored structure.

Introducing themes would be a `ScenarioDefinition` schema_v bump — a change to the
scenario workbench, which the requirement itself declares complete and closed. Not
worth it to satisfy one dashboard bullet.

### Impacts
- Data Architect: Requirement §6 amended. Pane 3 reports what is actually stored:
  candidate pool size, include/drop/undecided counts, source-document distribution of
  the included set, scan-run history with verdict counts, plus the structural
  capability flags (which query the graph, not scenario storage).
- DB Engineer: None — no migration, and explicitly no new theme table.
- Software Architect: Pane 3 chunk scoped accordingly. Note for that chunk: **undecided
  candidates have no `scenario_fact_refs` row** (derive-on-read is the ratified
  contract), so the undecided count must be computed as
  `pool − included − dropped`, never as a row count.

### Action Required
- [x] Amend the requirement (`CASE_HEALTH_DASHBOARD_REQUIREMENT_v1_1.md` §6)
- [x] Ensure nothing in Chunk 1's payload anticipates a theme concept
- [ ] Software Architect: honour the derive-on-read note when Pane 3 is built

---

## 2026-07-27 | SOFTWARE ARCHITECT (Roman Approved)

### Decision
**New read-only API surface: `GET /api/cases/:slug/case-health/inventory`**, rendered at
the top-level frontend route `/cases/:slug/case-health` ("Case Health"). Chunk 1 of the
Case Health dashboard — Pane 1, Graph Inventory. Every figure it displays is
reproducible by a query documented in **`docs/CASE_HEALTH_QUERIES.md`**.

### Rationale
Graph health was invisible: eight documents were processed and judged on grounding rate
while the metric that matters — how much extracted Evidence is wired into an Allegation
— existed nowhere in the product and had to be hand-queried. Pane 1 alone makes that
class of blindness structurally impossible.

### Impacts
- Data Architect: None — read-only, no schema change, no write path.
- DB Engineer: None — Neo4j reads only, no Postgres, no migration.
- Software Architect: New API contract (see below). Payload shaped so a snapshot/delta
  attaches later without a breaking change: the measurement lives in one `GraphInventory`
  struct carried as `current`, with a `previous` sibling that is always absent today.

### API contract
`GET /api/cases/:slug/case-health/inventory` → `200`
`{ case_slug, connection_tier_lookup_v, edge_classes[], current{…}, previous? }`.
`current` = `{ computed_at, corpus{…}, node_labels[], unlabeled_node_count,
edge_triples[], documents[] }`. Rates are `number | null` — `null` means "nothing to
measure", rendered `—`, and is never collapsed to `0`. Only failure mode is `500`
`{"error":"internal server error"}`; an empty graph is a legitimate `200`, not a `404`.

### Action Required
- [x] Backend, frontend, tests, documented-query artifact
- [ ] Roman: build + deploy the next beta and verify on DEV

---

## 2026-07-27 | SOFTWARE ARCHITECT (Roman Approved) — honesty batch

### Decision
Retire every displayed figure the metric inventory found to be fabricated,
permanently zero by construction, or mislabeled; wire the one column that was
blank over real data. No replacements invented — a real coverage verdict is Case
State work, and a substitute made up here would repeat the mistake being fixed.

**Retired:** `calculate_strength` — the "evidence strength" percentage on
`/explorer` came from a five-row lookup table keyed on the evidence count
(0 → 25%, 1 → 60%, 2 → 80%, 3 → 90%, 4+ → 95%), carrying no information the count
does not while presenting as a measurement, with an InfoPopup publishing the
invented scale to the user as method. Gone with it: the strength categories, the
strong/moderate/weak/gap buckets, the badge, the bars, and the client-side
per-Count breakdown. **Deleted:** `allegations_proven` (it counted Allegations
whose own quote was locatable in their own PDF and called that "proven" — that is
verifiability, not evidentiary support); `CaseStats.evidence_count` and its twins
`evidence_total` / `evidence_grounded` (hardcoded `0` with the false comment
"Evidence nodes don't exist in v2"); Decomposition `proven_count` / `all_proven`
(compared against a `status` property the v5.1 migration dropped and the query
returned as literal NULL, so both were zero/false by construction — the whole
unreachable `GET /decomposition` endpoint went with them). **Removed from the War
Room band:** "Baseless repeat patterns" and "No response yet", both derived from
hardcoded stub card fields, so one was structurally 0 and the other always
equalled the scenario count; the stub fields themselves are untouched, and the
figures return when their sources are real. **Fixed:** `d.document_type` in
**six** Cypher queries (three in `bias`, plus `embedding_repository`,
`graph_expansion_minor`, `graph_expansion_cypher`) — a property no write path has
ever set. **Wired:** the Proof Matrix's "Opposing" column, which received a
hardcoded `[]` over 41 real `Evidence -[:REBUTS]-> Allegation` edges, is now
**"Disputes"**, fed by a real per-Element count with the items in the expanded row.

### Rationale
These figures were live in a trial-prep product in active use. A fabricated
percentage and a structural zero are worse than a blank: both read as
measurements, and a reader has no way to tell them from real results. The
`document_type` class is subtler still — reading a Postgres column name off a
graph node returns NULL rather than erroring, so the failure mode is a plausible
empty field. It survived for months in six queries.

"Disputes" is deliberate: not "Contradicts", reserved for the future
evidence-vs-evidence impeachment layer, since one word for two relationships
would make them read as one; and not "Opposing", which describes a party's
posture rather than what the record disputes.

### Impacts
- **Data Architect:** None to the graph — read-only throughout, no schema change,
  no migration, no data patch.
- **DB Engineer:** None.
- **Software Architect:** API contract changes below. The Proof Matrix keeps ONE
  element-verdict vocabulary: `disputing_evidence_count` is layered beside
  `derive_proof_status` and is deliberately **not** an input to it — support and
  dispute are independent readings, and an Element that is both well corroborated
  and heavily disputed is the one worth arguing about.

### API contract changes
`GET /api/cases/:slug/causes-of-action` — each element gains
`disputing_evidence_count: i64`. `GET /api/cases/:slug/elements/:id/detail` —
each allegation gains `disputing_evidence: EvidenceRef[]` (empty array present,
never omitted). `GET /api/analysis` — loses `strength_percent`,
`strength_category`, `gap_notes`, the four bucket counts, `contradictions_summary`
and `evidence_coverage`. `GET /api/case` / `/api/case-summary` — lose
`allegations_proven`, `evidence_count`, `evidence_total`, `evidence_grounded`.
`GET /api/cases/:slug/trial-prep/dashboard` — metrics band loses
`baseless_repeat_patterns` and `no_response_yet`. `GET /decomposition` — removed.
Routes `/analysis` and `/decomposition` retired (both were unreachable from any
link or nav item).

### Action Required
- [x] Retire `calculate_strength` and its displays; delete the three false figures
- [x] `d.document_type` → `d.doc_type` in all six queries
- [x] Rule-21 filesystem scan in `neo4j::schema` failing the build on recurrence
- [x] Ship the Disputes column and its expanded items
- [x] Document the Proof Matrix queries — `docs/PROOF_MATRIX_QUERIES.md`
- [ ] Roman: build + deploy the next beta and verify on DEV
- [ ] Software Architect: Case State owns the replacement coverage verdict

---

## 2026-08-01 | SOFTWARE ARCHITECT (Roman Approved) — A0: Case State skeleton

### Decision
Establish `backend/src/domain/case_state/` as the sole home for case-state
computation, and move the shipped connection-tier partition into it as
`partition.rs` before any verdict work begins. The move carries no new
computation: the file's content is unchanged apart from its header and the
visibility narrowing below. Alongside it, ratify the **layering law** — verdicts
consume only the partition's output types; the three tier sets
(probative, topical-only, topical) lose `pub` and become private to
`partition.rs`, so no module outside `case_state` can name one. Every caller now
routes through `ConnectionTier::edge_types()`: `case_health_repository`,
`case_health_builder`, both their test modules, `dto/case_health` and
`api/case_health`. A Rule-21 filesystem scan
(`case_state::visibility_invariants`) fails the build if the visibility widens or
if a tier-set name appears outside the family. No shim was left at the old path.

### Rationale
Phases 1–2 are about to build readiness verdicts, hazard/ammunition and
conservation identities. Landing that beside scenario code means relocating it
later, and — the real cost — it leaves an interval in which a fourth definition
of "connected" can grow in whatever module happens to need one. Establishing the
family first means every later tenant arrives into a structure that already
answers "where does this go?"

The law's scope is notions of CONNECTEDNESS, not the schema vocabulary. Roughly
eight repository modules legitimately interpolate `schema::CORROBORATES` and
friends into Cypher because they are walking the graph; imprisoning the
`neo4j::schema` constants was never the intent and would forbid ordinary query
construction. It is the tier SETS that constitute a definition of connected, so
it is the sets that are private. Visibility is the enforcement — the scan exists
because adding `pub` is a one-character change that review can miss and the
compiler would then accept silently.

### Impacts
- **Data Architect:** None. No schema change, no query change, no migration.
- **DB Engineer:** None.
- **Software Architect:** No API contract change and no public surface added —
  the change is a net narrowing. Every displayed figure is bit-identical: the
  generated Cypher is asserted string-for-string by the pre-existing
  `case_health_repository_tests`, which were re-pointed but not re-baselined.

### Action Required
- [x] Create the `case_state` family and move the partition in unchanged
- [x] Narrow the tier sets to private; re-point all callers to the accessor
- [x] Rule-21 scan pinning the visibility law
- [ ] Roman: G0 merge to main after review
- [ ] Software Architect: A-remainder owns the probative-triple traversals that
      still bypass the partition (`scenario_repository::EvidencePolarity`,
      `causes_of_action`, `element_detail`, `allegation_detail`,
      `proof_review`), plus `derive_proof_status`, post-B

---

## 2026-08-01 | SOFTWARE ARCHITECT (Roman Approved) — task 1.1: codes + anchored rulings

### Decision
Implement the two ratified laws that had no implementation.

**§2a codes.** Every scenario carries `S-n`, allocated at creation and never
reused. The ordinal is a column on `scenarios`; the high-water mark is a separate
`case_code_sequences` row. Allocation is a single data-modifying-CTE statement
inside `insert_scenario`, so creation and allocation are atomic with no explicit
transaction, and the sequence's row lock serializes concurrent creations.
Existing scenarios were backfilled in `created_at` order per case and the
sequence seeded to that maximum, in the same migration. The code renders on the
dashboard card and the detail header, formatted by the BACKEND (`scenario_code`)
so no client re-derives the prefix. Candidate `C-n` codes already existed with
the same discipline and were verified, not rebuilt.

**§12.1 anchors.** Every ruling now records document, page, verbatim + normalized
quote, and speaker, captured server-side from the graph at ruling time and
written to an append-only `scenario_ruling_anchors` ledger in the same
transaction as the state row. `FactAction` gains `defer` (backend only; the UI is
1.3), which persists `status = undecided` plus a `defer_reason` — the field that
makes a parked candidate distinguishable from one nobody has opened.

### Rationale
On 2026-07-24/25 a re-extraction destroyed all 26 rulings including 6 human
includes. Nothing errored: rulings keyed on `graph_node_id`, that id is a content
hash, and re-extraction minted new ones. The anchor records what did NOT change
across the re-extraction — the document, the page, the words, the speaker — so a
ruling describes the RECORD rather than the graph's current encoding of it.

Three consequences worth recording, because each was a decision:

- **The anchor is read server-side, never accepted from the client.** A
  client-supplied anchor records what the caller *claimed*; a stale tab gets that
  wrong and a bad actor can forge it. One round trip on a low-frequency human
  action buys an unforgeable record and one contract for every future machine
  path.
- **An unanchorable ruling is REFUSED, not written partially.** A half-anchor is
  worse than no ruling because it looks recorded while being unrecoverable. A
  missing document refuses every ruling; a missing quote refuses include and
  exclude only (citability law, v2 §9/§17) with a message naming defer as the way
  forward. Defer is always permitted — it is how a human parks an item they
  cannot cite.
- **The ledger has NO foreign key on `scenario_id`**, unlike every sibling table.
  A cascade would mean deleting a scenario also erases the record of every ruling
  made inside it — the 2026-07-24 failure wearing a foreign key. Ledger rows may
  outlive their scenario; that is the point.

Normalization is casefold + whitespace-collapse and nothing else. Curly quotes
are deliberately NOT folded to straight ones, and a test pins that: widening the
rule silently changes which historical anchors match, so it is a versioned law
change, never a quiet edit inside a future matcher.

### Impacts
- **Data Architect:** None to the graph — the anchor is read-only against Neo4j.
- **DB Engineer:** Two pipeline migrations. `scenarios.code_ordinal` (NOT NULL
  after backfill) + `case_code_sequences`; `scenario_ruling_anchors` +
  `scenario_fact_refs.defer_reason`.
- **Software Architect:** API contract changes below. `insert_scenario` now
  returns `(scenario_id, code_ordinal)`; `upsert_fact_ref` takes `defer_reason`.
  Anchors are WRITTEN only — the matching pass that consumes them is task 2.5.

### API contract changes
`POST /api/cases/:slug/scenarios` — response gains `code: string`. Scenario cards
(`ScenarioSummary`) and the detail payload (`ScenarioDetail`) gain `code`.
`POST …/facts/:graph_node_id/action` — `action` accepts `defer`, and the body
accepts an optional `reason` (required for defer, refused otherwise). Both ruling
routes can now return **400** (no document / not citable / reason mismatch) and
**409** (the graph no longer holds the node) where they previously always
succeeded.

### Action Required
- [x] Migrations, allocation, anchor ledger, defer verb, code display
- [ ] Roman: verify on DEV after the Phase-1 batch deploys at G1
- [ ] Software Architect: task 1.3 wires the defer key and the queue view, and
      surfaces `defer_reason` on the candidate payload
- [ ] Software Architect: task 2.5 consumes the anchors (the re-anchor pass); the
      re-processing embargo holds until it is DONE
- [x] Software Architect: `undrop` in the ledger — **RATIFIED 2026-08-01**.
      §12.1's list of rulings is read as NON-EXHAUSTIVE: any verb that changes a
      candidate's ruling state writes a ledger row.
- [x] Software Architect: under that ratification, `DELETE /facts/:id` was the
      last traceless path and now goes through `record_removal`
      (`RulingKind::Remove`). A removal is the one ruling never refused for
      missing content — a vanished node is often WHY the reference is being
      discarded — so it records an empty anchor rather than failing. This
      required `scenario_ruling_anchors.document_id` to become nullable with a
      paired `document_state` (migration `20260801130419`), and `delete_fact_ref`
      to join `upsert_fact_ref` in being withheld from the repository re-export.
      Task 2.5 then needs no special case: reading the ledger forward
      distinguishes "included, still live" from "included, later removed".
- [ ] Software Architect: scenario ARCHIVE status does not exist (only hard
      delete) — tracked as its own task, not part of 1.1

---

## 2026-08-01 | SOFTWARE ARCHITECT (Roman Approved) — task 1.2: card payload backend

### Decision
Serve the complete §7 card payload from one new endpoint,
`GET /api/cases/:slug/scenarios/:id/facts/cards`, with every element assembled
and translated server-side. The frontend (task 1.3) will render it and compute
nothing.

Three pieces carry the weight:

**The naming canon in code** (`domain::card_language`). Every user-visible string
is produced by one module, each mapping tagged with the canon row
(CASE_STATE_UNIFICATION_REQUIREMENT_v1 §9) it implements: CORROBORATES →
"supports", REBUTS → "disputes", CHARACTERIZES → "comments on", ABOUT →
"mentions". "contradicts" is reserved for the future evidence-vs-evidence layer
and appears in no payload string. A banned-word test sweeps every string the
module and the assembler can produce.

**The stance carries its object, or there is no stance.** `CardStance` requires
both verb and object, so a bare "contradicts" is not expressible. The stance is
built from the Evidence→Allegation EDGE — which carries its object inherently —
never from a role in isolation. An item with no such edge serves `stance: null`,
`defer_required: true`, and a plain-language reason naming what would unblock it.

**The band seam** (`domain::confidence_band::band_for_score`). The raw score never
crosses the wire. Cutoffs are in-code defaults behind one function, marked
`TODO(1.6)`, so the settings store swaps one seam.

Also in scope: the Q3 honesty correction — `services::scenario_dashboard`
hardcoded `grounded: true` on every timeline turn. It now reports the node's real
`grounding_status`.

### Rationale
The July failure was cards that could not be ruled on: C-222 showed "contradicts ·
70%" with no object, no context and no citation. §7 makes a card missing any
element a defect, so the payload is the enforcement point and the §7 contract is
written as a test (`every_section_seven_element_is_present_on_a_complete_card`) —
a card missing an element fails the build.

Four decisions worth recording:

- **The edge is the stance, not the scan's proposed role.** A role is a claim
  about the scenario; an edge is a fact about the graph and comes with its object
  attached. The proposed role is used only to explain a defer.
- **Sort stays C-ordinal** (Q1 ruling (b)). §7.8's binding clause — confidence is
  never the default — is satisfied; confidence is not a sort key at all.
  Relevance-to-definition ordering is tracker task 3.7.
- **`bears_on` groups by accusation with an element LIST.** Measured on DEV: ¶12
  bears on three elements of Count 2. The first implementation deduped by
  allegation and silently kept one — the quiet loss this task exists to remove.
  The live query caught it; the unit tests had not.
- **Confidence bands are high/medium/low, not strong/moderate/weak.** Those three
  are the retired element-verdict vocabulary (canon §9); reusing them would put
  one word on two meanings. Band labels also name the SCAN as their subject
  ("Scan was fairly confident") so no reader mistakes them for a claim about the
  evidence.

### Impacts
- **Data Architect:** None. Read-only against Neo4j and Postgres; no schema change,
  no migration.
- **DB Engineer:** None.
- **Software Architect:** One new endpoint; no existing endpoint's shape changed
  except `AnchoredEvidenceFact`, which gains `grounding_status` so the timeline's
  `grounded` can stop lying. The card query reads its stance edge classes through
  A0's `ConnectionTier::Topical.edge_types()` accessor rather than assembling a
  sixth hand-written edge set.

### API contract changes
`GET /api/cases/:slug/scenarios/:scenario_id/facts/cards` — new. Returns
`{ pool: ScenarioCard[], set_aside: ScenarioCard[] }`. The gather endpoint is
unchanged and still serves the shipped workbench until 1.3 switches it over.
`AnchoredEvidenceFact` gains an optional `grounding_status`.

### Action Required
- [x] Card payload endpoint, canon module, band seam, defer detection
- [x] Q3: `grounded: true` replaced with the node's real state + its test
- [ ] Roman: verify on DEV after the Phase-1 batch deploys at G1
- [ ] Software Architect: task 1.3 renders these cards and wires the defer queue
- [ ] Software Architect: task 1.6 serves BOTH tunables from the settings store,
      each behind one seam — the confidence-band cutoffs
      (`domain::confidence_band::band_for_score`) and the quote-in-context window
      (`services::scenario_card::context_window_chars`). Ruled 2026-08-01: the
      window is a tunable, not a structural constant — "how much context flanks
      the quote" is exactly the number Roman may want to turn up without a
      rebuild, and if the card's layout constrains it, that constraint belongs to
      the same setting rather than to a second config source. Neither gets an env
      var in the interim; both are in-code defaults behind their seam. The band
      cutoffs stand as a known Rule-13 standing exception until 1.6 lands.
      **RETIRED 2026-08-01 by task 1.6** — both are now `app_settings` rows;
      `HIGH_CONFIDENCE_CUTOFF`, `MEDIUM_CONFIDENCE_CUTOFF` and
      `DEFAULT_CONTEXT_WINDOW_CHARS` are deleted from the source.
- [ ] Software Architect: task 3.7 owns relevance-to-definition ordering
- [ ] Software Architect: §7.1 thinness — quote-in-context locates the quote by
      substring search in the stored page text. A very short quote ("Yes.") could
      in principle match the wrong occurrence; on the measured DEV row it matched
      uniquely. Defect-filed against §7.1, not widened here.

---

## 2026-08-01 | SOFTWARE ARCHITECT (Roman Approved) — task 1.3: card UI and keyboard triage

### Decision
Replace the candidate workbench with a keyboard TRIAGE QUEUE rendering the §7
card contract from the 1.2 payload. `I` include · `E` exclude · `D` defer ·
`U` undo, auto-advance, single-step undo, an inline defer prompt, a defer queue
with visible reasons, a running ruled/remaining count, and no page navigation to
rule. The source PDF sits in a split pane beside the focused card.

One backend change, the only one this task permits: **`FactAction::Reopen` /
`RulingKind::Reopen`**, the verb that takes back a ruling.

### Rationale
The pool is 148 candidates per scenario and 399 of 525 Evidence nodes are
unrulable as they stand, so the surface has to clear items at keyboard speed or
it will not be used. The proven patterns are Rayyan one-key screening and
Relativity save-and-advance, rendered in the Casefleet card layout.

Four decisions worth recording:

- **Reopen is a new word, not a reuse of `undrop`.** `undrop` already produces
  the right STATE for an undo — undecided, defer reason cleared — but it would
  write "undrop" to the ruling ledger for an item that was never dropped. That is
  a false entry in the forensic record, which is exactly what the 2026-08-01
  ratification closed when it made the ledger's vocabulary the truth of the act.
  Same transition, different word.
- **The split-pane viewer stays** (deviation from the study's new-tab reading,
  stated per §2c): verifying a quote against its page is the action a triager
  repeats constantly, and a shipped `PdfViewer` already does it in place. The
  pinpoint chip still carries `viewer_href` for a full new-tab read.
- **The logic is a pure reducer, not a component.** CLAUDE.md rule 30 records
  that DOM test infrastructure is deliberately absent; rather than reverse a
  standing convention mid-phase, the keyboard state machine and the §7 card
  descriptor were extracted as pure functions and tested exhaustively (31 tests).
  What that cannot cover — that the JSX faithfully walks the descriptor — is what
  the G1 DEV verify line covers. RTL + jsdom rides task 2.7 if wanted.
- **The orphan guarantee moved with the surface.** The card endpoint is
  pool-driven like gather, so a saved fact whose graph node vanished is absent
  from it. The queue makes the same second `listScenarioFacts` read and renders
  the orphan strip below. Dropping it would have regressed a ratified policy.

### Impacts
- **Data Architect:** None.
- **DB Engineer:** None — no migration. `Reopen` writes through the existing
  `record_ruling` path, so it is anchored and ledgered by construction.
- **Software Architect:** `CandidateFactsPanel` (763 lines) is no longer mounted
  anywhere. It is left in the tree deliberately: removing it plus its shipped
  helper tests is its own reviewable change, and keeping it through G1 means the
  old surface is one line away if the queue disappoints on DEV. Flagged for
  removal after G1.

### API contract changes
`POST …/facts/:graph_node_id/action` — `action` accepts `reopen`. No response
shape changed. The frontend `FactAction` union widened to include `defer` and
`reopen`; the old workbench's button maps were narrowed to a `WorkbenchAction`
subtype rather than given entries for verbs it never renders.

### Action Required
- [x] Reopen verb, the triage queue, the defer prompt and queue, the orphan strip
- [ ] Roman: verify on DEV after the Phase-1 batch deploys at G1 — "rule 10
      candidates without touching the mouse or leaving the queue"
- [ ] Software Architect: remove `CandidateFactsPanel` and its now-unused helpers
      after G1 confirms the queue
- [ ] Software Architect: task 2.7 may add RTL + jsdom if DOM-level component
      tests are wanted; 1.3 deliberately did not reverse the no-DOM convention

---

## 2026-08-01 | SOFTWARE ARCHITECT (Roman Approved) — task 1.4: working view and augmentation

### Decision
Give the three human-authored components of a scenario (v2 §2) their storage and
their surface: **C1 identity** (`theme_statement`, `motivation` on `scenarios`),
**C4 human facts** (a new `scenario_human_facts` table), **C5 talking points**
(the existing `scenario_responses` trio, wired at last, plus `authored_by`).

The working view renders the INCLUDED evidence as Casefleet Facts-table rows
above the 1.3 queue; the augmentation panel is where human content is authored,
one modal, everything on one screen.

### Rationale
Phase A found the instruction's premise half wrong in a useful way: **C5's
storage already existed.** The `scenario_responses` / `response_items` /
`response_item_fact_refs` trio shipped 2026-06-26 with a complete repository and
**zero callers** — nothing wrote it, nothing read it, no surface showed it. So C5
needed wiring and one column, not tables. C4 genuinely had nothing.

Five decisions worth recording:

- **Three sentences, not one.** `attack_text` is the attack as the other side
  frames it; `theme_statement` is our one-line answer; `motivation` is what they
  want the jury to believe. Task 1.5's rehearsal mode reads the second beside
  `direction`, so collapsing any two would destroy the distinction it depends on.
  Both new columns are nullable — a scenario is created before it is framed, and
  a NOT NULL would put invented prose in the record.
- **The §8 invariant is structural, and stated POSITIVELY.**
  `scan_and_merge_paths_write_only_their_own_tables` pins the allowlist of tables
  the scan/gather/merge paths may write (`scenario_fact_refs`, the `scan_run*`
  trio, `scenario_candidate_ordinals`). A denial-only test would pass vacuously
  the day someone renamed a table; an allowlist makes ANY new write in those
  paths visible. The other half — `augmentation_never_gathers` — matters because
  gather MEMOIZES candidate ordinals: an edit that triggered it would mint
  identity as a side effect of typing a sentence.
- **C5's "per attack" = per scenario** (ratified). One response row whose ordered
  items are the ≤cap points. `origin` is always `'human'`; the schema's
  `'suggested'` value stays unwritten rather than migrated away.
- **A date needs a TYPE.** "Around 4/21/2009" renders differently from
  "4/21/2009", and flattening them would state more precision than the human
  claimed. An unreadable stored type falls back to the bare date — understating
  precision is the safe direction.
- **The card payload gained `status` beside `status_label`** (a 1.2 addendum,
  approved). The working view filters on the token; filtering on the display
  string would be the frontend reading state out of prose, which 1.2 exists to
  forbid.

### Impacts
- **Data Architect:** None to the graph.
- **DB Engineer:** One migration — two columns on `scenarios`, the
  `scenario_human_facts` table, `authored_by` on the two C5 tables. No ruling or
  anchor table touched.
- **Software Architect:** Four new routes. `update_scenario` and the card DTO
  gained fields. `neo4j/human_facts.rs` is marked DEAD CODE in its header — zero
  callers, a graph-level writer for a different concept, removal tracked as 3.8.

### API contract changes
`GET …/scenarios/:id/augmentation` (new) · `POST …/human-facts` ·
`DELETE …/human-facts/:fact_id` · `PUT …/talking-points`. `PUT /scenarios/:id`
accepts `theme_statement` and `motivation`; `ScenarioDto` returns them. Card
payload gains `status`.

### Action Required
- [x] Schema, repositories, augmentation service, four routes, both surfaces
- [x] §8 invariants asserted by source scan in both directions
- [ ] Roman: verify on DEV after the Phase-1 batch deploys at G1
- [ ] Software Architect: task 1.5 reads `theme_statement` + `direction` +
      talking points for rehearsal mode — the columns landed here for that
- [ ] Software Architect: task 1.6 adds `talking_points_cap` to the settings
      store. **Rule-13 standing exception, signed off 2026-08-01** —
      `DEFAULT_TALKING_POINTS_CAP = 3` sits in code behind
      `domain::human_authored::talking_points_cap()` with a `TODO(1.6)`, the
      shape this task's instruction specified. The exception list is now four
      entries, all retiring when 1.6 moves them to the settings store:
      `HIGH_CONFIDENCE_CUTOFF`, `MEDIUM_CONFIDENCE_CUTOFF`,
      `DEFAULT_CONTEXT_WINDOW_CHARS`, `DEFAULT_TALKING_POINTS_CAP`. Re-flags of
      any of the four are auto-signed; a NEW constant still stops for sign-off.
      **RETIRED 2026-08-01 by task 1.6.** All four constants are deleted; the
      exception list is EMPTY and the auto-sign-off rule expires with it.
- [ ] Software Architect: task 3.8 removes `neo4j/human_facts.rs`

---

## 2026-08-01 | SOFTWARE ARCHITECT (Roman Approved) — task 1.5: rehearsal mode and the ready gate

### Decision

Built v2 §10 rehearsal mode and the §5/§6 ready gate that guards it.

1. **The ready gate is SCENARIO-level and has exactly one recorded path.**
   `POST /cases/:slug/scenarios/:id/ready` takes an explicit target (`{ready:
   bool}`, not a toggle) and writes the status change plus its transition row in
   ONE transaction. `PUT /cases/:slug/scenarios/:id` now **REFUSES** `status`
   with a 400 naming the ready route — previously a rename could carry a
   readiness change with no actor recorded. Refused rather than silently ignored:
   dropping the field would report success for a change that never happened.

2. **`scenario_status_transitions`** (new table, pipeline DB) records every
   `drafted ⇄ ready` act: `from_status`, `to_status`, `actor`, `at`. Append-only,
   no FK on `scenario_id` (the record of a human act must outlive the row), a
   `from_status <> to_status` CHECK. Deliberately NOT the ruling ledger, which is
   for evidence rulings and requires an anchor.

3. **`scenario_responses.status` is VESTIGIAL and must never gate rehearsal.**
   Every 1.4 write path sets it to `'draft'`, so filtering on it would show an
   empty rehearsal forever with no error. It is also wrong in principle: §6 has
   one `drafted ⇄ ready` transition and it is scenario-level. Recorded as a
   doc-comment on the field citing this ruling.

4. **The exclusion law holds by CONSTRUCTION.** `dto::rehearsal` carries only
   code, theme, attack, points (each with an optional plain exhibit label) and
   watch-list. There is no field for motivation, confidence, verdicts, status,
   graph ids, documents or pages. A test serializes a fully-populated payload and
   asserts eleven banned substrings are absent, so ADDING such a field fails the
   build rather than reaching a witness.

5. **Watch-list notes are a `kind` discriminator on `scenario_human_facts`**
   (`fact` | `watch_list`, DEFAULT `'fact'` backfilling existing rows truthfully),
   not a sibling table. One table means one write path, one scan-allowlist entry,
   and §8's invariants cover the new kind for free; Phase 2's computed watch-list
   then merges as a filter rather than a union. The panel receives the two as
   SEPARATE lists so a client cannot render a watch-list note as a fact.

6. **`draft` and `needs_evidence` both read as v2 "drafted".** The gate tests FOR
   `ready` rather than against the two drafted values, so a fourth status added
   later is excluded from rehearsal by default. No CHECK migration.

### Rationale

The gate exists because §5 makes it mandatory and §6 makes both directions human
acts. A column pair (`ready_by` / `ready_at`) holds only the latest act, so one
promote → demote → promote cycle erases the demotion — and "who took S-2 out of
rehearsal?" asked at 11pm the night before trial is the only question this record
is ever used for.

The exclusion law is enforced by type shape rather than by review because the
failure mode is a witness reading our strategy off a screen. Shape is checkable;
discipline is not.

### Impacts

- Data Architect: `scenario_status_transitions` is a new pipeline-DB table.
  `scenario_human_facts` gains `kind`.
- DB Engineer: migration `20260801215024_add_ready_transitions_and_watch_list_kind`,
  applied at backend boot. Forward-only, no down migration.
- Software Architect: `PUT /scenarios/:id` is a BREAKING contract change — it
  rejects `status`. No frontend caller sent it; a curl that did now gets a 400
  naming the replacement route.

### Action Required

- [ ] Software Architect: task 1.6 moves the four recorded standing-exception
      constants to the settings store. Task 1.5 added no new ones.
- [ ] Software Architect: task 3.9 builds the talking-point → exhibit pairing
      authoring UI. The field ships now and renders when present; it is `None`
      until then, and no label is derived from the record (deriving one would put
      words in the witness's mouth and drag pinpoint sourcing into a payload §10
      keeps it out of).
- [ ] Software Architect: task 3.10 builds the READ surface for
      `scenario_status_transitions`. The rows are written from today; nothing can
      display them yet, so "who pulled S-2 and when" currently takes a psql query
      against `colossus_legal_v2`. `list_status_transitions` is written and
      tested and awaits a handler. Deferred deliberately (ruled 2026-08-01):
      where the history belongs — the scenario header, the working view, or a
      case-level audit page — is a design decision, not a sidebar in a commit
      that already carries a mode, a gate and a schema change.
- [ ] Roman: DEV verify — demote S-2 to drafted → it vanishes from the rehearsal
      list; Marie's login still sees the full working view.

---

## 2026-08-01 | SOFTWARE ARCHITECT (Roman Approved) — task 1.6: settings store, and the end of the exception era

### Decision

Built the configuration store v2 §2b requires, and **deleted every compiled-in
parameter in the scenario function**.

1. **`app_settings`** (pipeline DB) — key · value · declared kind · default ·
   bounds · plain-language meaning · `consumed_by` · last-changed. One TEXT value
   column with a `value_kind` (`float` | `count` | `ratio`) rather than typed
   columns or JSONB: one edit widget on the page, one parser per kind, readable
   in psql, bounds enforced numerically. Seeded with all SEVEN parameters at
   today's values, so the migration changes no behaviour.

2. **`app_setting_changes`** — append-only, key · old · new · actor · at. The
   fourth append-only table in two days, and for the same reason: "who changed N
   the night before trial" is the question, and the night before trial is exactly
   when a value gets changed twice. `updated_by`/`updated_at` stay on the row for
   the page's "last changed by" line; the ledger is the history.

3. **The snapshot is THREADED, not global.** `Settings` is loaded at boot into
   `AppState` and handed to the pure functions that need it. A process-global
   would have let the four seams keep their signatures — and would have put
   hidden state inside `build_card`, which this codebase documents as "Pure — no
   I/O" and whose output the §7 completeness test asserts against. Worse: a unit
   test calling it without booting would find the global empty, leaving a panic
   or a compiled-in fallback, *the exact defect this task deletes*. Four
   signatures changed instead.

4. **The freshness law.** A write updates the row, appends the ledger entry,
   re-reads the whole store, and swaps the snapshot — all before the response is
   sent. "Edits take effect on next read", literally, with **zero database reads
   on the card path**. This assumes a single-process backend; if a second process
   ever serves this API the swap needs a cross-process story. Recorded in the
   module doc so tomorrow's reader knows it was seen, not missed.

5. **The failure law.** A parameter missing, unreadable, out of bounds, or
   self-contradictory REFUSES — at boot (process exits, naming the key) and on
   write (400, naming the parameter and the limit). There is deliberately no
   fallback: after this task no compiled-in default exists to fall back to, and
   inventing one at the moment of failure would silently reinstate the defect.
   The `high > medium` invariant spans two rows, so no column CHECK can express
   it; it is enforced in the write path AND re-checked at load, because a psql
   edit bypasses the write path.

6. **`/settings`**, behind the existing `is_admin` gating, listing every
   parameter with value, default, meaning, bounds and input hint. The three
   dormant parameters are listed and editable, each labelled *"Read by task 2.4,
   which is not built yet — changing this has no effect today."* A page that
   listed inert knobs silently would be lying about its own reach.

### Rationale

Roman's law, born from months of parameter changes requiring rebuilds. The seam
discipline established in tasks 1.2 and 1.4 is what made this a one-line change
in each of four files rather than a hunt: every tunable already lived behind
exactly one function, so repointing it was mechanical.

`reanchor_close_match_tolerance` is seeded at 0.85 as a normalized similarity —
bounded, direction-obvious, algorithm-agnostic. Its meaning text says PROVISIONAL
and licenses task 2.5 to re-seed it with a different unit if its matching design
needs one.

### Impacts

- Data Architect: two new pipeline-DB tables, `app_settings` and
  `app_setting_changes`.
- DB Engineer: migration `20260801225147_create_app_settings_store`, applied at
  boot. Forward-only. **The backend now refuses to start if the seed is absent** —
  the migration and the binary must deploy together.
- Software Architect: `build_card`, `assemble`, `check_talking_points`,
  `set_talking_points`, `rehearsal_payload` and `band_for_score` all take a
  `&Settings`. No wire contract changed.

### THE EXCEPTION ERA IS CLOSED

`HIGH_CONFIDENCE_CUTOFF`, `MEDIUM_CONFIDENCE_CUTOFF`,
`DEFAULT_CONTEXT_WINDOW_CHARS` and `DEFAULT_TALKING_POINTS_CAP` are **deleted**.
The Rule-13 standing-exception list is **empty**, and the auto-sign-off rule for
re-flags **expires with it** — from here, any config-shaped constant stops for
sign-off, with no exceptions to cite.

A source scan (`the_four_retired_constants_are_gone_and_stay_gone`) walks every
`.rs` file and fails the build if any of the four names returns, as a fallback, a
fixture promoted to production, or a "temporary" default. The law is now a test.

### Action Required

- [ ] Software Architect: tracker 3.11 — the config-law sweep OUTSIDE the
      scenario surface. The `rules-enforcer` gate on this task returned FAIL on
      five PRE-EXISTING config-shaped constants; they are deferred here by ruling
      (2026-08-01), because migrating the RAG pipeline's configuration inside the
      settings-store commit is the scope-bleed this process exists to prevent.
      The complete list, so 3.11 inherits a list and not a vibe:

      | Location | Constant / value | Why it is config-shaped |
      |---|---|---|
      | `main.rs:24` | `DEFAULT_CHAT_MODEL = "claude-sonnet-4-6"` | a model id — availability and cost tier are deployment decisions |
      | `main.rs:30` | `CHAT_MAX_TOKENS = 4096` | a per-deployment token budget |
      | `main.rs:~127` | `Duration::from_secs(90)` (HTTP client timeout) | a tunable timeout |
      | `main.rs:~128` | `Duration::from_secs(5)` (HTTP connect timeout) | a tunable timeout |
      | `main.rs:~708` | `DEFAULT_STARTUP_SCHEMA_FILE = "general_legal.yaml"` | a filename selecting an asset |
      | `config.rs:20` | `rerank_threshold` | an env var with an `unwrap_or(0.3)` — a compiled-in default behind a config read |

      Two comment-only violations from the same gate WERE fixed in task 1.6, being
      zero-risk: the missing `// best-effort:` marker on `dotenvy::dotenv().ok()`
      and the missing serde rationale comments on `SchemaMetadata` /
      `EntityTypeInfo` / `RelationshipTypeInfo`. `deny_unknown_fields` was
      deliberately NOT added to those three — it is a silent deserialization
      behaviour change and has no business riding a configuration commit.

      3.11 also owes a ruling on `rag_config`: a seeded table with zero readers in
      colossus-legal AND colossus-rs is either a future feature or a corpse, and
      someone should say which.
- [ ] Software Architect: tracker 2.7 inherits a NAMED flaky test.
      `pipeline::registry::tests::test_registry_from_env_with_registry_file`
      failed once during this task's verification and passed on every rerun. The
      cause is now identified rather than folklore: several tests in
      `src/pipeline/registry_tests.rs` call `std::env::set_var` /
      `std::env::remove_var` on the SAME variables — `PROCESSING_PROFILE_DIR` at
      lines 513, 535 and 557 — and Rust runs tests in parallel threads, so one
      test clears the variable another is depending on. Process-wide environment
      is shared state; the fixture needs serialising (a mutex) or the tests need
      to stop mutating the real environment. This is the intermittent observed
      but never captured during tasks 1.2–1.5. NOT fixed in 1.6 — it rides 2.7
      with the fixture repair.
- [ ] Roman: DEV verify — change a value on the Settings page; the API serves the
      new value on next read; no rebuild happened.

---

## 2026-08-02 | SOFTWARE ARCHITECT (Roman Approved) — hotfix: the beta.364 boot failure, and a new standing review rule

### Decision

**1. `app_settings.min_value` / `max_value` become `DOUBLE PRECISION`.**
Migration `20260802092813_app_settings_bounds_to_double_precision.sql`, append-only.
Task 1.6 declared both columns `NUMERIC` while `AppSettingRecord` decodes them as
`Option<f64>`. v2.0.0-beta.364 applied all 51 migrations, seeded the store, and
then refused to boot on the first live read:

```
error occurred while decoding column "min_value": mismatched types; Rust type
`core::option::Option<f64>` (as SQL type `FLOAT8`) is not compatible with SQL
type `NUMERIC`
```

The refusal itself was correct (§16, and §2b leaves nothing to fall back to) and
is unchanged by this fix. No seed value, read semantic, parameter or refusal rule
was touched.

**2. NEW STANDING REVIEW RULE — the migration↔struct type table.** Any task that
adds or alters a migration must include, in its Phase A report, a side-by-side
table of migration column types vs. the Rust types that decode them, and the
four-agent gate checks that seam explicitly. Filed from this defect: the seam
between a green migration and a green struct is where beta.364 died.

### Rationale

`sqlx` implements `Type<Postgres>` for `f64` against `FLOAT8` alone; `NUMERIC`
decodes only into `Decimal` / `BigDecimal`, and neither `rust_decimal` nor
`bigdecimal` is in this workspace's dependency tree. A bare `NUMERIC` column here
therefore has **no decodable Rust type at all** — this was never a
lossy-coercion warning.

Every other `NUMERIC` column in the pipeline database is read through a
`::float8` cast in its projection (`llm_models.cost_per_*_token`,
`default_temperature`, `processing_profiles.temperature`,
`scan_runs.computed_cost`, `extraction_runs.cost_usd`). That is the right pattern
*there*: those columns are money or carry a declared 2-decimal contract, so exact
decimal storage is deliberate. The bounds columns never earned it — 1.6's own
comment shows the choice under consideration was NUMERIC-vs-TEXT — and both
become `f64` the moment they leave the row. Moving the schema closes the seam at
the source; a projection cast would have to be remembered at every future query
site. Measured before the ALTER: the entire numeric content of the table is
`{0, 1}` and NULL across all seven rows, so the cast is provably lossless.

**Why no test caught it:** the seed test parses the migration as TEXT (values,
not types); the SQL-shape tests read the `const` statements with no server; every
unit test builds `AppSettingRecord` from a fixture. `query_as` (not the
`query_as!` macro) type-checks at RUNTIME against a live server. 1240 green tests
were all blind to the one place the two sides meet.

### Impacts

- Data Architect: None — no node, property or relationship changes.
- DB Engineer: one migration, 52 total. 7-row table rewrite; no data change.
- Software Architect: two new tests, and the standing review rule above.
  - `the_migration_declares_only_types_this_build_can_decode` (lib target, CI-runnable,
    no database) parses the declared SQL types out of the migrations and refuses
    any column whose type its decoding Rust type cannot read. Verified red against
    the pre-hotfix schema.
  - `tests/app_settings_decode_integration.rs` (`#[ignore]`, live) creates a scratch
    database, applies the store's migrations with the real `Migrator`, and reads
    through the real `load_settings` path. Run live before commit; passed.

### Action Required

- [ ] Roman: bump 2.0.0-beta.365, build, deploy via Semaphore. Verify: boot log
      shows `migrations=52`, `configuration store read rows=7 required=7`,
      preconditions PASS; `/health` 200; the Settings page renders all seven
      parameters.
- [ ] Software Architect / DB Engineer: **NEW DEFECT, filed not fixed — the
      pipeline migration chain cannot be applied from zero.** `sqlx` orders
      migrations by the filename prefix parsed as `i64`, and 16 of the 52 files
      carry the old 8-digit `YYYYMMDD` prefix. `20260513` is twenty million;
      `20260422214842` is twenty trillion — so every 8-digit migration sorts
      before every 14-digit one regardless of when it was written. On a fresh
      database `20260513_consolidate_model_columns_and_add_overrides` runs before
      the April migration that adds the column it edits and fails with
      `column "pass2_extraction_model" does not exist`. Measured 2026-08-02
      against a scratch database on the DEV server. DEV and PROD never saw it
      because they were migrated incrementally, one pending migration at a time.
      The chain's CONTENT is sound: all 52 files applied in filename order to an
      empty database succeed and produce the full 29-table schema. Only the order
      sqlx derives is wrong. **This means no new environment can be built from
      the repository today**, and a PROD rebuild would fail. The fix is a project,
      not a line: renaming an applied migration changes its version and checksum,
      which the running DEV and PROD databases already record. Not touched by this
      hotfix.
- [ ] Roman: `scripts/bump-version.sh` updates `Cargo.toml` but not `Cargo.lock`,
      so the lock has been one version behind since at least beta.363. The
      corrected lock rides this commit; the script is a separate fix.

---

## 2026-08-02 | SOFTWARE ARCHITECT (Roman Approved) — task 1.7A: three G1 defects measured on beta.365

### Decision

**D1 — the app shell stops painting every screen grey (v2 §2c).** New
`--bg-canvas: #ffffff` token. The shell (`App.tsx`) and the two screens that
paint their own full-height canvas (`EvidenceExplorerPage`, `GraphPage`) use it.

`--bg-page` (#f4f5f7) is NOT changed to white: it has 126 remaining uses, every
one an element tint — table stripes, code blocks, nav hover states, badge fills,
and three sites where it is a hairline BORDER colour. Setting it white would have
satisfied §2c by making all of those invisible. The token has been doing two jobs
under one name; it now does one, and the naming debt (`--bg-page` → `--bg-tint`,
126 sites) is recorded in `tokens.css`. Task 1.7B's Phase A decides whether the
rename rides the page restructure or earns its own cleanup task.

**D3 — candidate context is recomputed at assembly with an index-mapped
normalizer.** New pure module `domain/quote_match.rs`: `normalize_with_map`
(the existing normalization, carrying the byte span each normalized character
came from) and `locate` (exact tier, then normalized tier, returning a span in
the ORIGINAL text). `normalize_text` MOVED there from
`api::pipeline::canonical_verifier`, which re-exports it — one implementation, so
grounding and display cannot drift into two definitions of "found".

**D4 — a parameter's input hint uses its OWN default as the worked example.**
`ValueKind::hint(default)` composes it from the row; every whole-number field
used to advertise "e.g. 240", including `talking_points_cap`, whose default is 3
and whose minimum is 1 — an example the field itself would have refused.

### Rationale

**D3 is the substantive one.** The card assembler located context with
`page.find(quote)` — an exact, case-sensitive substring search — while the ingest
path grounds in TWO tiers: exact, then normalized (smart quotes, dashes,
ligatures, collapsed whitespace, hyphenated line breaks, transcript gutter
numerals). Every quote grounded on the second tier failed that search and was
served with two empty strings, silently.

Measured on DEV before the fix, corpus-wide, joining each item to its own
grounded page's stored text: `exact` 325 of 325 found; `normalized` **0 of 320
found — every one of them bare**. §7.1 exists because a ruling made on a fragment
is a ruling made blind, and 320 cards were being ruled blind.

Grounding does not persist a position — `CanonicalGroundingResult` carries a
match type and a page number, and the normalized tier asks `contains`, a bool —
so there was nothing to reuse. Three options were weighed; recomputing at
assembly was chosen because it needs no migration, no backfill and no
re-verification, and repairs every card already in the database the moment it
ships. Measured recovery: at least 269 of the 320 (84%), the residue being
quotes that span a page boundary.

Slicing the NORMALIZED text was rejected though it needs no map at all:
normalization lowercases, collapses whitespace and rewrites punctuation, so the
context under a quote would have rendered as de-punctuated lowercase prose. On a
surface a lawyer reads to decide what a quote means, that is a different kind of
lie from an empty box. The test asserts the page's own smart quotes survive into
the window, which is what stops a later "simplification" from doing it.

### Impacts

- Data Architect: None.
- DB Engineer: None — no migration, no schema change, read path only.
- Software Architect: `normalize_text` now lives in `domain::quote_match`;
  `canonical_verifier` re-exports it and its 46 tests are the equivalence proof
  that the move changed no behaviour. That file also shrank 569 → 519 lines.

### Action Required

- [ ] **Task 2.5 (re-anchoring, §12.1) inherits `normalize_with_map` as its
      primitive, and persisting `(offset, len)` at grounding time is its design
      default.** That was the rejected option here — rejected on vehicle, not on
      merit: it needs a migration on `extraction_items`, the span carried into the
      Neo4j node the card is built from, and a re-verify of all 782 grounded items
      to backfill. It is the better long-term answer, and it cannot be built
      without exactly the mapping this task added.
- [ ] Task 2.5 also owns the residue this fix cannot reach: a quote spanning a
      page boundary grounds on the LEFT page of the pair while the assembler loads
      that page alone, so the words genuinely are not all there. Those cards stay
      context-less by design — a window drawn around the nearest similar words
      would read as evidence. They are now COUNTED (`without_context` on the
      "served scenario cards" log line) rather than merely absent, because such a
      card reports itself grounded and looks like every other one.
- [ ] Task 1.7B Phase A: decide the `--bg-page` → `--bg-tint` rename (126 sites)
      — rider on the page restructure if mechanical and safe, else its own task.
- [ ] Roman: DEV verify after the 1.7A+1.7B batch deploys — every screen sits on
      pure white; C-1 (normalized-grounded) shows surrounding context at the
      configured width; `talking_points_cap`'s hint reads "e.g. 3".

---

## 2026-08-02 | SOFTWARE ARCHITECT (Roman Approved) — task 1.7B: the scenario page restructure

### Decision

The scenario detail page is rebuilt to `SCENARIO_PAGE_RESTRUCTURE_DESIGN_v1`, so
every Phase-2 feature lands on a stable baseline instead of being retrofitted
after ten features are wired in. Five structural changes:

1. **Lean one-line header** (study §1.7) — code · name · direction chip · status
   chip · edit · rehearsal link. The permanent full-width definition form and the
   inline title-rename machine are gone.
2. **One identity modal** (study §1.6) — name, the three texts, allegation
   CHIPS with an inline picker. Nothing opens a second page.
3. **Compact scan control** — one line: Run · model select · last-run summary.
   The radio grid is gone.
4. **Working view** stays the Facts-table pattern; the augmentation panel keeps
   its behaviour.
5. **The split-pane PDF viewer is removed** (defect D2). Pinpoints open the
   dedicated viewer at the cited page in a new tab.

**`llm_models.billing_class`** (migration `20260802134438`, TEXT NOT NULL,
`local` | `billed`, backfilled from `provider`) makes "who pays for this model"
a stored fact. The scan control orders local first, defaults to a local model,
and labels the rest "(API — billed)" — all composed server-side.

### Rationale

**Why billing needed a column** (Roman's ruling, option C of three). Three
proxies existed and each was rejected: `provider` and `api_endpoint` describe
which client speaks to the endpoint, not who pays — a self-hosted
OpenAI-compatible gateway breaks both; and `cost_per_input_token > 0`, though
correct on all seven rows today, fails in the expensive direction, because a
NULL cost means UNKNOWN, never FREE. A model whose costs were never filled in
would have read as local and told a human a 148-candidate scan was free minutes
before it billed them. Encoding "anthropic means billed" in Rust would also have
put a deployment fact in the binary (Rule 13). The default for an unclassified
row is `billed` — the cautious side.

**Why the modal cannot edit direction.** The design note listed it as editable;
that was an error and the note is amended (§2.2 direction = display-only). The
backend refuses direction on the update route — a scenario's offense/defense
stance is its identity, and flipping it would make it a different scenario. The
chip states it with the reason on hover; the cure for a wrong direction is
archive-and-recreate (task 3.6).

**Why C1 moved out of the augmentation panel.** `theme_statement` and
`motivation` were edited there AND belong to the identity the modal now owns. One
field, one editor. The §8 invariant is untouched: the modal writes through the
same PUT.

**Why the split-pane died.** It rendered every page from the cited one to the end
of the document, stacked (D2) — and a zoomed legal page in half a column is
unreadable regardless. Popup-only viewing supersedes the 1.3 deviation.

### Impacts

- Data Architect: None.
- DB Engineer: one migration (53 total), one column, 7 rows reclassified. Dry-run
  verified in a rolled-back transaction against live DEV: 2 local, 5 billed.
- Software Architect: `ScenarioDetail` gains `direction` (display only). Three
  files become dead and are deleted by Roman's `git rm` with this commit:
  `ScenarioDefinitionForm.tsx`, `CandidateFactsPanel.tsx`, `candidateSeed.ts` (+
  its test). `candidateWorkbench.ts` and `shared/PdfViewer.tsx` STAY — the first
  is still imported by the scan panel, the second is the dedicated viewer that
  pinpoints now route to.

### Action Required

- [ ] Roman: merge `feature/scenario-phase-1b` to main, bump beta.366, build,
      deploy, and run the amended G1 slice on the new page.
- [ ] **Filed, not fixed — `viewer_href` can compose a dead link.** `build_card`
      defaults a missing document id to `""`, producing
      `/documents/?tab=document`. It cannot fire on today's data (measured
      2026-08-02: 525 of 525 Evidence nodes carry both a document and a page), so
      it is latent. The cure when a payload task next opens that file:
      `Option<String>` on `viewer_href`, and a pinpoint chip that renders as
      plain text when there is nothing to open.
- [ ] **Filed — the `--bg-page` → `--bg-tint` rename.** 135 occurrences across 61
      files, mechanical and safe (a token rename with no dynamic construction),
      but deliberately NOT ridden on this commit: 61 files of noise would bury a
      structural review, and several of those files are edited here for real. Its
      own one-hour cleanup task, immediately after this batch merges. The debt
      note is in `tokens.css`.
- [ ] Task 2.4 fills the readiness slot the header leaves empty. Until then the
      header renders NOTHING there — asserted by a test, because a verdict is a
      claim about whether a scenario can be taken into a courtroom.
- [ ] **Filed — the Ask page ignores the catalog's `warnings`.** A model the
      backend refuses to list (unreadable `billing_class`) is reported on
      `ChatModelsResponse.warnings` and SHOWN by the scan control, but
      `services/ask.ts::fetchChatModels` reads only `res.models`, so the chat
      picker would be one row shorter with nothing on screen saying why. Left as
      filed rather than fixed: the Ask page is not task 1.7B's surface, and the
      gap is unreachable on real data (the column is `NOT NULL DEFAULT 'billed'`
      with a single writer, and a drop is logged server-side by name). Whoever
      next opens `ask.ts` / `AskPage.tsx` wires the same alert-box pattern the
      scan control now has.
- [ ] **Filed — `billing_class` has no write path yet.** `InsertModelInput` and
      `UpdateModelInput` do not carry it, so a model added through the admin API
      after this migration is classified `billed` (the cautious default) and can
      only be reclassified by direct SQL. Deliberate for now — the write surface
      for the LLM-config columns was already deferred to a later chunk (R4 of the
      LLM Configuration Method), and this column joins that queue rather than
      opening it. Nothing is misrepresented in the meantime: an unclassified
      model reads as billed, never as free.

---

## Template for Future Entries

```markdown
## YYYY-MM-DD | [ROLE]

### Decision
[Clear statement of what was decided]

### Rationale
[Why this decision was made]

### Impacts
- Data Architect: [impact or "None"]
- DB Engineer: [impact or "None"]
- Software Architect: [impact or "None"]

### Action Required
- [ ] [Role]: [Specific action needed]
```

---

## Quick Reference: What Requires Logging?

**Always Log:**
- Schema changes (new node types, relationships, properties)
- API contract changes
- Query pattern changes that affect API
- Processing methodology changes
- Authority/process changes

**Don't Log:**
- Routine data entry following established patterns
- Bug fixes that don't change interfaces
- Documentation updates within owned files
- Exploratory work that doesn't result in decisions
