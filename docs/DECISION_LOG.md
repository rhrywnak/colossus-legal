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
