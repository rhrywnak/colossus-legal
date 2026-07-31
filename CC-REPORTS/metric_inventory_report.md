# Metric Inventory — every computed figure in the product

**Instruction:** `CC_METRIC_INVENTORY_READ_AND_REPORT_v1.md`
**Repo/branch:** colossus-legal · `feature/scenario-1a` · read at `5fbcc6a`; HEAD is now
`83f90b5` (two version-bump commits landed mid-investigation, touching only `Cargo.lock`,
`Cargo.toml` and `package.json` — no source file this report cites)
**Report date:** 2026-07-27
**Type:** read-and-report. No file under `backend/` or `frontend/` touched, no commit, no
schema change, no processing, no spend. The only file created is this one.

=== REPORT START ===

VERDICT: **inventory-complete.** 63 displayed figures across 21 surfaces, catalogued below.

BLOCKERS: none for the inventory itself (it is a code read). One caveat: **no live DB
verification** — SSH to the DEV host is refused by this session's permission layer and there
is no local Postgres client, so every claim below is from code at HEAD, labelled **[code]**,
with **[inference]** used where I reason past it. Five figures whose *current value* would
settle a question are listed with their queries in §9.3.

HEADLINE FINDINGS (detail in §7):

1. **Three surfaces display a per-document "evidence → linked" table, with three different
   definitions of "linked."** Case Health (probative / topical), Case Analysis (CORROBORATES ∪
   legacy MotionClaim path), Proof Matrix (CORROBORATES only). They will not agree.
2. **The "evidence strength" percentage on the product's most prominent evidence page is
   fabricated.** `calculate_strength` is a five-row lookup table mapping an evidence *count* to
   a hardcoded percentage (0→25%, 1→60%, 2→80%, 3→90%, 4+→95%). The percentage carries no
   information the count does not, and the page's own InfoPopup documents the invented scale
   to the user as though it were a measurement.
3. **Two figures are permanently zero by construction**, not by data: `proven_count` /
   `all_proven` on Decomposition (tests a property the query hard-codes to `NULL`), and
   `CaseStats.evidence_count` (a literal `0` with a stale comment claiming Evidence nodes do
   not exist — 525 do).
4. **`allegations_proven` measures the wrong thing entirely** — it counts allegations whose own
   *quote was found in its source PDF*, and labels that "proven."
5. **The `document_type` property error is already in shipped product code**, in three Bias
   repository queries. Same class as the two 2026-07-26 alarm findings, but this one is in the
   product, not in an ad-hoc query.
6. **Seven figures are computed in the frontend**, four of them genuine arithmetic on case data.
7. **10 of 21 metric surfaces are unreachable by clicking** — routed but absent from every nav
   and link in the app.

---

## 0. Method, coverage, and conventions

**Provenance labels.** **[code]** = read at HEAD `5fbcc6a`, cited `path:line`. **[inference]** =
reasoning past the evidence. No project-knowledge summary is cited as evidence for a code claim;
I did not read `docs/FRONTEND_BL_AUDIT_RESULTS.md` or any status document for this report.

**How the surfaces were enumerated.** From `frontend/src/App.tsx:57-89` (the complete route
table) and `frontend/src/components/Header.tsx:22-46` (`NAV_ITEMS`), then every page and
component reached from those, then their service clients, then the backend handler →
repository/builder → store for each. Frontend arithmetic was found by grepping for `toFixed`,
`Math.round`, `Math.floor`, `.reduce(`, `.filter(...).length`, and `/ ... * 100` across
`pages/` and `components/`.

**Reachability.** In-app links resolve to only these targets: `/`, `/admin`, `/allegations`,
`/documents`, `/explorer`, `/people`, `/timeline`, `/cases/…` **[code:** grep of every
`to="/…"` and `navigate("/…")` in `frontend/src` **]**, plus the eight `NAV_ITEMS` entries.
**Everything else in the route table is URL-only**: `/analysis`, `/claims`, `/damages`,
`/hearings`, `/decisions`, `/decomposition`, `/contradictions`, `/graph`, `/queries`,
`/search`. Marked **(orphaned)** throughout. This matters for the retire/keep decision: a
surface nobody can click is a cheap retirement.

**A naming trap, stated up front.** The instruction refers to *"Evidence & Analysis"* and its
*"completeness score."* Those are **two different pages**, and neither is named what one might
expect:

- **"Case Evidence & Analysis"** is the `<h1>` of `/explorer` — `EvidenceExplorerPage`
  (`pages/EvidenceExplorerPage.tsx:256`) **[code]**. It is the "Evidence" leaf under the Proof
  Matrix nav group. It displays the **strength percentages**, not a completeness score.
- **"Case Analysis"** is the `<h1>` of `/analysis` — `AnalysisPage` (`pages/AnalysisPage.tsx:652`)
  **[code]** — an orphaned page with three tabs, sharing the same backend payload.
- The only figure in the product called **completeness** is **Admin → Audit → "Complete"**
  (`components/admin/AdminAudit.tsx:76`) **[code]**, which measures *metadata presence on
  Evidence nodes*, not coverage. Full formula in §8.

I have inventoried all three.

---

## 1. Summary table — every displayed figure

Locus: **BE** = backend-computed; **FE** = frontend-computed (a BL-in-frontend finding);
**FE-fold** = frontend fold over rows the backend already returned (arithmetic, but only over
the rendered set). Store: **N** = Neo4j, **P** = Postgres, **S** = static file, **—** = none.

### Case Health · `/cases/:slug/case-health` (nav: top level)

| # | Figure | One-line definition | Locus | Store |
|---|---|---|---|---|
| 1 | Probative connection rate | % of Evidence with ≥1 CORROBORATES/REBUTS/CHARACTERIZES → Allegation | BE | N |
| 2 | Topical connection rate | Same, adding ABOUT → Allegation | BE | N |
| 3 | Inert / Inert % | Evidence reaching no Allegation by any of the four classes | BE | N |
| 4 | Evidence without Document | Evidence with no CONTAINED_IN edge | BE | N |
| 5 | Per-doc Evidence / Probative / Topical / Inert + % | Same four measures, per Document | BE | N |
| 6 | Per-doc per-class item counts | Items carrying ≥1 edge of each class | BE | N |
| 7 | Nodes by populated label | `MATCH (n) … labels(n)[0]`, never `db.labels()` | BE | N |
| 8 | Unlabeled node count | Nodes carrying no label | BE | N |
| 9 | Edges by (from, rel, to) | Full edge taxonomy with endpoints | BE | N |

### Proof Matrix · `/cases/:slug/proof-matrix` (nav)

| # | Figure | Definition | Locus | Store |
|---|---|---|---|---|
| 10 | Mapped Allegations (per Element) | incoming BEARS_ON count | BE | N |
| 11 | Supporting (per Element) | DISTINCT Evidence CORROBORATES→Allegation BEARS_ON→Element | BE | N |
| 12 | **Opposing (per Element)** | **hardcoded empty array in the frontend** | FE | — |
| 13 | Status pill | `derive_proof_status(T, C)` → no_allegations/gap/partial/supported | BE | N |

### Proof Review · `/cases/:slug/proof-review` (nav) — 4 sub-tabs

| # | Figure | Definition | Locus | Store |
|---|---|---|---|---|
| 14 | Corroborating total | count of CORROBORATES edges in scope | BE | N |
| 15 | Corroborating by statement_type | tally over the same rows | BE | N |
| 16 | Corroborating by (statement_type, evidence_strength) | tally over the same rows | BE | N |
| 17 | Excluded total | Evidence whose statement_type ∈ non-answer set with no CORROBORATES | BE | N |
| 18 | Excluded by statement_type | tally over the same rows | BE | N |
| 19 | Sub-tab badges (3) | `payload.<array>.length` — verbatim array lengths | FE-fold | N |

### Case Evidence & Analysis · `/explorer` (nav: "Evidence")

| # | Figure | Definition | Locus | Store |
|---|---|---|---|---|
| 20 | **Strength % per allegation** | **lookup table on evidence count — a fabricated scale** | BE | N |
| 21 | Strength category | strong/moderate/weak/gap, from the same table | BE | N |
| 22 | Supporting evidence count | DISTINCT evidence via MotionClaim path ∪ CORROBORATES | BE | N |
| 23 | **Per-Count strength breakdown + bar widths** | **fold + `(n/total)*100` in the frontend** | FE | N |
| 24 | **"N with evidence"** | **`total - strengthCounts.gap`, in the frontend** | FE | N |
| 25 | Evidence-chain summary (claims / evidence / documents) | counts along the MotionClaim chain | BE | N |

### Case Analysis · `/analysis` **(orphaned)** — 3 tabs

| # | Figure | Definition | Locus | Store |
|---|---|---|---|---|
| 26 | Gap-analysis buckets (strong/moderate/weak/gaps) | counts of #21's categories | BE | N |
| 27 | Strength bar per allegation | #20 rendered as a bar | BE | N |
| 28 | Contradictions total | count of Evidence-[:CONTRADICTS]->Evidence | BE | N |
| 29 | Evidence coverage totals (total / linked / unlinked) | sum over the per-document rows | BE | N |
| 30 | **Per-document linked %** | **`Math.round((linked/evidence)*100)` in the frontend** | FE | N |

### Trial Prep — War Room · `/cases/:slug/trial-prep` (nav)

| # | Figure | Definition | Locus | Store |
|---|---|---|---|---|
| 31 | Scenarios | `cards.len()` | BE | P |
| 32 | Ready | cards with status `ready` | BE | P |
| 33 | Drafted or review | cards with status `draft` | BE | P |
| 34 | Instances | Σ per-card REBUTS∪CORROBORATES evidence on anchor allegations | BE | P+N |
| 35 | **Baseless repeat patterns** | **always 0 — the source field is never populated** | BE | — |
| 36 | **No response yet** | **always = scenario count — responses unwired** | BE | — |
| 37 | Per-card instance count | as #34, per card | BE | P+N |
| 38 | **Per-card response count** | **hardcoded `0`** | BE | — |
| 39 | **Per-card speakers** | **hardcoded `[]`** | BE | — |
| 40 | **Per-card baseless repeat** | **hardcoded `None`** (honest "pending") | BE | — |
| 41 | **Alerts strip** | **hardcoded `[]`** (honest empty) | BE | — |

### Bias Explorer · `/bias-explorer` (nav)

| # | Figure | Definition | Locus | Store |
|---|---|---|---|---|
| 42 | Result count | DISTINCT tagged Evidence matching the filters | BE | N |
| 43 | Total unfiltered | Evidence with a non-empty `pattern_tags` | BE | N |
| 44 | Instances per actor | `group.items.length` | FE-fold | N |

### Scenario workbench · `/cases/:slug/trial-prep/:scenarioId` (nav → card)

| # | Figure | Definition | Locus | Store |
|---|---|---|---|---|
| 45 | **Undecided / included / dropped counts** | **`countByStatus` fold in the frontend** | FE | P+N |
| 46 | **"x of y"** shown-vs-total | **`visible.length + orphans.length`, frontend** | FE | P+N |
| 47 | Candidate confidence % | `Math.round(confidence*100)` — formatter over a stored score | FE-fold | P |
| 48 | Scan-run counts (relevant / not / failed) | stored run tallies, rendered verbatim | BE | P |
| 49 | **Run-vs-run agreement % (relevant, role)** | **Jaccard + role match computed in the frontend** | FE | P |

### Home · `/` (nav)

| # | Figure | Definition | Locus | Store |
|---|---|---|---|---|
| 50 | Elements per Count | `count.elements.length` — array length | FE-fold | N |
| 51 | Allegations per Count | deduped BEARS_ON allegations per Count (proof-matrix rollup) | BE | N |

### Documents · `/documents` and `/documents/:id` (nav)

| # | Figure | Definition | Locus | Store |
|---|---|---|---|---|
| 52 | **Status bucket counts** | **`documents.filter(...).length` ×5, frontend** | FE-fold | P |
| 53 | Errors-needing-attention count | from the errors endpoint | BE | P |
| 54 | Review counts (pending/approved/rejected) | from the items summary | BE | P |
| 55 | Per-document total cost | `SUM(extraction_runs.cost_usd)` | BE | P |
| 56 | Completeness verdict (pass/warn/fail) | node-existence + Qdrant-point existence per approved item | BE | P+N+Qdrant |

### Admin · `/admin` (nav) — 9 tabs

| # | Figure | Definition | Locus | Store |
|---|---|---|---|---|
| 57 | Avg grounding rate | mean of stored per-run `(exact+normalized)/total*100` | BE | P |
| 58 | Total / avg cost | Σ and mean of `extraction_runs.cost_usd` | BE | P |
| 59 | Documents by status | `GROUP BY status` | BE | P |
| 60 | **Audit "Complete" %** | **`complete_evidence / total_evidence` — metadata presence** | BE | N |
| 61 | Audit issue counts (4 checks) | per-check detail-array lengths | BE | N+Qdrant+disk |
| 62 | **Reviewer progress %** | **`Math.round((reviewed/assigned)*100)`, frontend** | FE | P |
| 63 | Step performance (count / avg / min / max duration) | aggregates over `pipeline_steps` | BE | P |

### Other surfaces — list totals only, no derived figures

`/allegations`, `/people`, `/damages` **(orphaned)**, `/claims` **(orphaned)**,
`/contradictions` **(orphaned)**, `/timeline`, `/graph` **(orphaned)**, `/search`, `/ask`.
Each renders a backend-supplied `total` (or a `.length` of the fetched array) plus, in three
cases, a per-detail count. Detail in §2.11. Notable exceptions catalogued there: Person detail's
four counts, Decomposition's summary (§2.9, contains finding #3), Harms' `total_damages`
(string-parsing sum), and Search's `Math.round(score*100)` match percentage.

---

## 2. Detail by surface

### 2.1 Case Health — `/cases/:slug/case-health` (Pane 1, shipped `927d601`)

**Source:** `api/case_health.rs` → `repositories/case_health_repository.rs` (four Cypher reads)
→ `repositories/case_health_builder.rs` (all arithmetic). Neo4j only. **[code]**

| Figure | Definition per the code | Where |
|---|---|---|
| Probative connection rate | Evidence items with ≥1 edge in {CORROBORATES, REBUTS, CHARACTERIZES} to an Allegation, ÷ all Evidence | `domain/connection_tier.rs:89`, `case_health_repository.rs::corpus_query` |
| Topical connection rate | Same with ABOUT added | `connection_tier.rs:107` |
| Inert | `evidence_total − topical_connected` | `case_health_builder.rs::build_corpus` |
| Evidence without Document | Evidence with `size([(e)-[:CONTAINED_IN]->(d:Document)]) = 0` | `corpus_query` |
| Per-document rows | Same measures scoped by CONTAINED_IN; `OPTIONAL MATCH` so zero-yield documents still appear | `documents_query` |
| Per-class item counts | Items carrying ≥1 edge of each class — **item counts, not edge counts; they do not sum to the connected totals** | `documents_query` |
| Nodes by label | `MATCH (n) RETURN labels(n)[0], count(*)` — populated labels only | `LABEL_COUNTS_QUERY` |
| Edges by triple | `MATCH (a)-[r]->(b) RETURN labels(a)[0], type(r), labels(b)[0], count(*)` | `EDGE_TRIPLES_QUERY` |

Rates are `Option<f64>` — `null` (rendered `—`) when there is nothing to measure, never `0.0`.
Every figure is reproducible from `docs/CASE_HEALTH_QUERIES.md`. **Consumers:** this page only.
**Frontend arithmetic: none.**

### 2.2 Proof Matrix — `/cases/:slug/proof-matrix`

**Source:** `GET /api/cases/:slug/causes-of-action` →
`repositories/causes_of_action_repository.rs::elements_query` (`:126-146`) +
`causes_of_action_builder.rs::derive_proof_status` (`:138-147`). Neo4j. **[code]**

- **Mapped Allegations** = `count(DISTINCT a)` over `(a)-[:BEARS_ON]->(el)` — the proof
  denominator `T`.
- **Supporting** = `count(DISTINCT ev)` over `(a)<-[:CORROBORATES]-(ev)` — **CORROBORATES only.**
- **Status pill** = `derive_proof_status(T, C)` where `C` = allegations with ≥1 CORROBORATES:
  `T=0 → no_allegations`; `C=0 → gap`; `C≥T → supported`; else `partial`. Compiled-in, no
  configurable threshold.
- **Opposing** — the column exists in the header template
  (`components/proofMatrixColumns.ts:23-29`) but the page passes `opposingEvidence={[]}`
  **hardcoded** at `components/MatrixRowWithDetail.tsx:62` **[code]**. The comment at `:60`
  says "empty today (no CONTRADICTS/REBUTS edges yet)" — **that comment is now wrong**:
  `Evidence -[:REBUTS]-> Allegation` = 41 edges **[given, §2.2 of the requirement]**. This is a
  permanently blank column over data that exists.

**Consumers of the same payload:** Home (`indexAllegationTotals` for the Count cards),
`CountDetailPage`, `EvidenceExplorerPage` (`legal_count_details` for grouping).
**Also reads `derive_proof_status`:** `services/proofMatrix.ts:61-65` (the TS type).

### 2.3 Proof Review — `/cases/:slug/proof-review`

**Sub-tabs (enumerated per the instruction) —** `pages/ProofReviewPage.tsx:39-46` **[code]**:

| id | Label | Contents |
|---|---|---|
| `summary` | Summary (default) | corroboration + exclusion category tables |
| `proof_edges` | Proof edges | one row per CORROBORATES edge, answer → allegation |
| `excluded` | Excluded | preserved non-answer Evidence with no CORROBORATES edge |
| `borderline` | Borderline | the `proof_edges` subset with `statement_type = partial_admission` |

One fetch drives all four; switching tabs does not refetch (`:170`). The active tab is in the
URL (`?tab=`).

**Math:** `repositories/proof_review_repository.rs` runs two Cypher reads — the CORROBORATES
edges (`:113-117`) and Evidence whose `statement_type IN $non_answer_types` with no such edge
(`:144-149`). `proof_review_builder.rs` derives every summary count **in Rust from the exact
rows the page renders**, explicitly so the summary can never disagree with the detail
(`proof_review_builder.rs:6-12`) **[code]**. Frontend does array-length reads and client-side
filtering only (`pages/proofReviewHelpers.ts`), which changes no count.

**The non-answer vocabulary is compiled in**: `NON_ANSWER_STATEMENT_TYPES = [evasive, objection,
referral, denial]` and `CORROBORATING_STATEMENT_TYPES = [admission, partial_admission]`
(`models/document_status.rs:161-206`) **[code]**. These are **discovery-response** values; the
schemas state outright that `statement_type` is "a HETEROGENEOUS free string across document
types" (`extraction_schemas/correspondence_schema_v5_3.yaml:43`) **[code]**. **[inference]**
Proof Review is therefore a discovery-response surface wearing a case-wide name: a transcript's
`judicial_statement` or a brief's `attorney_assertion` matches neither list, so those items
appear in neither the corroborating nor the excluded tally. This is a reusability-checkpoint
failure as well as a coverage gap.

### 2.4 Case Evidence & Analysis — `/explorer`

**Source:** `GET /allegations` + `GET /analysis` (`pages/EvidenceExplorerPage.tsx:125`) →
`repositories/analysis_repository.rs`. Neo4j. **[code]**

**The strength percentage — `analysis_repository.rs:56-64`:**

```rust
fn calculate_strength(evidence_count: i64) -> (i32, String) {
    match evidence_count {
        0 => (25, "gap"), 1 => (60, "weak"), 2 => (80, "moderate"),
        3 => (90, "strong"), _ => (95, "strong"),
    }
}
```

The percentage is **a hardcoded label attached to a bucket**, not a computed ratio. It has
exactly five possible values and carries no information the evidence count does not. The DTO's
field comments describe ranges ("90%+", "70-89%") that no code ever computes
(`dto/analysis.rs:39-43`) **[code]**. The page's InfoPopup then publishes the invented scale to
the user as method — "0 items = Gap (25%) … 3+ items = Strong (90%+)"
(`pages/EvidenceExplorerPage.tsx:262-266`) **[code]**.

**What the evidence count actually counts** (`analysis_repository.rs:97-113`): the union of
(a) `(a)<-[:PROVES]-(:MotionClaim)-[:RELIES_ON]->(e1)` — the **legacy** motion path — and
(b) `(a)<-[:CORROBORATES]-(e2)`. REBUTS, CHARACTERIZES and ABOUT are excluded; the doc comment
at `:73-76` says CONTRADICTS is deliberately excluded as counter-evidence, but is silent on
REBUTS.

**Frontend arithmetic** — `components/EvidenceExplorerParts.tsx:67-128` **[code]**: the
per-Count strength breakdown is folded client-side, `withEvidenceCount = total -
strengthCounts.gap` is computed client-side (`:79`), and each segment's bar width is
`(strengthCounts[cat] / total) * 100` (`:128`). These are the two clearest BL-in-frontend
findings in the product after #30.

**Consumers of `/analysis`:** this page and `/analysis` (§2.5). No exports.

### 2.5 Case Analysis — `/analysis` **(orphaned)**

Same payload as §2.4, three tabs: `gap-analysis`, `contradictions`, `evidence-coverage`
(`pages/AnalysisPage.tsx:16`) **[code]**.

- **Gap-analysis buckets** — counts of the five `calculate_strength` categories
  (`analysis_repository.rs:154-171`).
- **Contradictions total** — `MATCH (a:Evidence)-[r:CONTRADICTS]->(b:Evidence)`
  (`analysis_repository.rs:185-193`). **[inference]** Zero today: zero E→E edges exist.
- **Evidence coverage** — `fetch_evidence_coverage` (`:236-270`): per Document,
  `evidence_count` = DISTINCT `:Evidence` CONTAINED_IN it, `linked_count` = those reached by
  the MotionClaim path **or** a direct CORROBORATES. Corpus totals are summed over the
  document rows (`:274-276`) — note this is the sum-of-rows approach Case Health deliberately
  avoided (§7.1).
- **Per-document linked %** — **computed in the frontend**:
  `Math.round((doc.linked_count / doc.evidence_count) * 100)` (`pages/AnalysisPage.tsx:340-342`),
  with a three-band color threshold at 80/50 hardcoded at `:362-364` **[code]**.

### 2.6 Trial Prep — War Room · `/cases/:slug/trial-prep`

**Source:** `api/trial_prep.rs` → `services/scenario_dashboard.rs`. Postgres `scenarios` +
Neo4j per-card count. **[code]**

`compute_metrics` (`:363-375`) derives the band from the card list. But `record_to_card`
(`:336-352`) hardcodes three of the four card fields:

```rust
response_count: 0,          // "Responses are not wired until a later chunk"
speakers: Vec::new(),       // "Speaker derivation is not sourced yet"
baseless_repeat_count: None // "Pattern analysis is not wired"
```

Consequences, following the metric math through **[code]**:

- **Baseless repeat patterns** counts cards where `baseless_repeat_count.is_some_and(|n| n > 0)`
  → **always 0**, because the field is always `None`.
- **No response yet** counts cards where `response_count == 0` → **always equals the scenario
  count**.
- **Alerts** is `Vec::new()` (`:158`), commented "honest empty — NOT the old hardcoded
  placeholder strings."

The stubs are honestly commented and `baseless_repeat_count: None` correctly distinguishes
"pending" from "analysed, none found" — but the two *metrics band* figures derived from them
present as measurements. **[inference]** An operator reading "Baseless repeat patterns: 0"
cannot tell it from a real zero.

**Instance count** — the one live figure. `count_record_rebuts` (`:209-224`) sums, over each
scenario's `anchor_allegation_ids`, the evidence returned by
`anchored_allegation_evidence(anchor, EvidencePolarity::Both)` — i.e.
`Evidence -[:REBUTS|CORROBORATES]-> Allegation` (`repositories/scenario_repository.rs:121-127`)
**[code]**. A scenario with no anchors scores 0. Note the label: the card says
**"instances"** for what is *corroborating + rebutting evidence on the anchor allegations*.

### 2.7 Bias Explorer · `/bias-explorer`

**Source:** `bias/repository.rs`. Neo4j. **[code]**

- **Result count** = DISTINCT Evidence matching the filters (`execute_filtered_query`, `:314`).
- **Total unfiltered** = `MATCH (e:Evidence) WHERE e.pattern_tags IS NOT NULL AND
  e.pattern_tags <> '' RETURN count(e)` (`:215-219`).
- **Instances per actor** = `g.items.length` (`pages/BiasExplorer/BiasByActorView.tsx:100`) —
  a length read over the returned group.

Full assessment in §6.

### 2.8 Scenario workbench · `/cases/:slug/trial-prep/:scenarioId`

**Source:** `GET .../facts/gather` (`api/scenario_gather.rs`) — the live graph pool from
`BiasRepository::all_evidence_about_subject` reconciled in memory against `scenario_fact_refs`.
Postgres + Neo4j. **[code]**

- **Undecided / included / dropped** — `countByStatus`, a `.reduce` over the candidate array
  (`components/candidateWorkbench.ts:69-83`), called at `components/CandidateFactsPanel.tsx:560`,
  with orphans folded in at `:561-562` **[code]**. **Frontend-computed.**
- **"x of y"** — `visible.length + orphans.length` over `counts.total + orphans.length`
  (`:565, :590`).
- **Confidence %** — `Math.round(confidence * 100)` (`candidateWorkbench.ts:186`) — a formatter
  over a stored `scenario_fact_refs.confidence`, not a derivation.
- **Scan-run counts** — `relevant_count` / `irrelevant_count` / `failed_count` read verbatim
  from `scan_runs` (`components/RunHistoryList.tsx:119-127`). Backend-stored; the migration
  states the partition invariant `relevant + irrelevant + failed = candidates_read`
  (`pipeline_migrations/20260715121130…:75-80`) **[code]**.
- **Run-vs-run agreement** — `relevantPct` = Jaccard of the two relevant sets,
  `rolePct` = role matches ÷ shared, both `Math.round`ed
  (`components/themeScanFormat.ts:61-77`) **[code]**. **Frontend-computed**, and the doc comment
  concedes it is a *partial* agreement because irrelevant verdicts are only sampled in the
  summary.

### 2.9 Decomposition · `/decomposition` **(orphaned)**

**Source:** `repositories/decomposition_repository.rs`. Neo4j. **[code]**

- **Total allegations / characterizations / rebuttals** — real counts over
  `(charE:Evidence)-[c:CHARACTERIZES]->(a)` and `(rebE:Evidence)-[:REBUTS]->(charE)`
  (`OVERVIEW_CHAR_QUERY`, `:66-78`). Note this is **Evidence→Evidence REBUTS** — an edge class
  with **zero instances** **[given]**, so `total_rebuttals` is 0 and every per-allegation
  `rebuttal_count` is 0. **[inference]**
- **`proof_count`** — `count(DISTINCT mc)` over `(mc:MotionClaim)-[:PROVES]->(a)`
  (`OVERVIEW_PROOF_QUERY`, `:83-88`) — the legacy path only.
- **`proven_count` / `all_proven` — permanently zero/false by construction.** The query returns
  the literal `NULL AS status` (`:74`, a v5.1 migration note says `evidence_status` was
  dropped), and `map_overview_row` computes `is_proven = status.as_deref() == Some("PROVEN")`
  (`:187`) **[code]**. A comparison against a hard-coded `NULL` can never be true. The page
  renders the resulting figures at `pages/DecompositionPage.tsx:131-166`.

### 2.10 Documents · `/documents`, `/documents/:id`

- **Status bucket counts** — `documents.filter(d => d.status_group === …).length` ×5, in a
  `useMemo` (`pages/DocumentsPage.tsx:146-153`) **[code]**. A fold over rows already fetched.
- **Per-document cost** — `SUM(cost_usd::float8) FROM extraction_runs WHERE status='COMPLETED'`
  (`repositories/pipeline_repository/document_records.rs:321-324` for the list, `:372-375` for
  the single read) **[code]**. `cost_usd` is nullable and there is a known NULL population from
  before the columns were populated (`api/pipeline/constants.rs:4-6`) **[code]**.
- **Review counts** (pending / approved / rejected) — from the items summary
  (`components/pipeline/ReviewPanel.tsx:198-200`).
- **Completeness verdict** — `api/pipeline/completeness.rs:228-234` **[code]**:
  `fail` if the Document node is missing **or** any expected Neo4j node id is missing;
  `warn` if any Qdrant point is missing; `pass` otherwise. Entity-level, not count-level — the
  header explains that MERGE dedup made count equality unreachable (`:9-13`).

### 2.11 Everything else

| Surface | Figures | Locus / note |
|---|---|---|
| `/allegations` | `total`, filtered length | BE `total`; filtered count is `displayedAllegations.length` |
| `/people` | `total`, per-role group sizes | BE `total`; group sizes are `.length` |
| `/damages` **(orph.)** | `total`, **`total_damages`** | `total_damages` sums `Harm.amount` by **stripping `$` and `,` from a string and parsing** (`repositories/case_repository.rs:196-201`) — an unparseable amount is silently dropped by `filter_map` **[code]** |
| `/claims` **(orph.)** | `total` | BE |
| `/contradictions` **(orph.)** | `total` | BE; zero today (no E→E edges) |
| `/timeline` | event / phase counts | **static file** `/data/timeline.json` (`pages/TimelinePage.tsx:60`) — not case data |
| `/search` | **`Math.round(hit.score*100)`% match** | FE formatter over a Qdrant score (`pages/SearchPage.tsx:180`) |
| `/ask` | retrieval hit/node counts, `score.toFixed(2)` | FE length reads (`components/RetrievalDetailsPanel.tsx:39,99`) |
| `/graph` **(orph.)** | `nodes.length` · `edges.length` | FE length read (`pages/GraphPage.tsx:715`) |
| Person detail | total statements / documents / characterizations / rebuttals received | BE, summed in Rust from the fetched rows (`repositories/person_detail_repository.rs:215-221`) |
| Allegation detail | characterization / rebuttal / proof-claim counts, per-claim `evidence_count` | BE (`repositories/allegation_detail_repository.rs:110`) + `.length` reads |
| Home | Elements per Count (`.length`), allegations per Count (BE rollup) | see §2.2 |
| `CaseStats` (app-wide context) | see §7.4 — **not rendered**; only `legal_count_details` is consumed | `context/CaseContext.tsx:27`; consumers `AllegationsPage:35`, `EvidenceExplorerPage:145` **[code]** |

---

## 3. Frontend-computed figures — the BL-in-frontend list

Flagged explicitly per the instruction. **Genuine arithmetic on case data (4):**

| Figure | File:line | What it computes |
|---|---|---|
| Per-document linked % (Case Analysis) | `pages/AnalysisPage.tsx:340-342` | `Math.round((linked/evidence)*100)` + 80/50 color thresholds at `:362-364` |
| Per-Count strength breakdown + bar widths | `components/EvidenceExplorerParts.tsx:67-128` | category fold, `withEvidenceCount = total − gap`, `(n/total)*100` |
| Candidate status counts | `components/candidateWorkbench.ts:69-83` → `CandidateFactsPanel.tsx:560-562` | undecided / included / dropped / total, + orphan folding |
| Run-vs-run agreement % | `components/themeScanFormat.ts:61-77` | Jaccard + role-match ratios |

**Folds over already-fetched rows (3)** — arithmetic, but only over the rendered set, so they
cannot disagree with the list: document status buckets (`DocumentsPage.tsx:146-153`), reviewer
progress % (`components/admin/ReviewerWorkloadSection.tsx:72-74`), bias instances-per-actor
(`BiasByActorView.tsx:100`).

**Hardcoded, not computed (1):** the Proof Matrix Opposing column,
`opposingEvidence={[]}` (`MatrixRowWithDetail.tsx:62`).

Pure formatters (`formatRate`, `Math.round(confidence*100)`, `Math.round(score*100)`,
`score.toFixed(2)`) are **not** listed as findings — they render a number the backend computed.

---

## 4. Scenario data reality — the readiness slice

Per the instruction, with store and field provenance. **[code]** throughout.

| Wanted | Computable today? | Store & provenance |
|---|---|---|
| Included / dropped counts | **Yes** | Postgres `scenario_fact_refs.status ∈ {included, dropped}`, `GROUP BY status` per `scenario_id` (migration `20260706162558…:48-63`) |
| **Undecided count** | **Yes, but NOT as a row count** | **Undecided candidates have no row.** `api/scenario_gather.rs:10-17` states the ratified derive-on-read contract: "A pool node with NO ref row is `Undecided` — persisted nowhere." It must be computed as `pool_size − included − dropped`. Counting rows systematically under-reports it. |
| Candidate pool size | **Yes** (live) | Neo4j via `BiasRepository::all_evidence_about_subject` (`bias/repository.rs:505`), driven by the scenario's resolved subject (`services/scenario_subject.rs`). A *historical* pool size is also stored as `scan_runs.candidates_read` |
| Whether included items connect to allegations | **Yes — but nothing computes it today** | Two hops: `scenario_fact_refs WHERE status='included'` → `graph_node_id` → `MATCH (e:Evidence)-[:CORROBORATES\|REBUTS\|CHARACTERIZES\|ABOUT]->(:Allegation) WHERE e.id IN $ids`. Both halves exist; no code joins them. This is the single highest-value addition for a readiness metric |
| Distinct source documents behind included items | **Yes** | `BiasInstance.document: Option<DocumentRef>` is already on the gather payload (`bias/dto.rs:185`), populated from `(e)-[:CONTAINED_IN]->(d:Document)`. Distinct-count over the included subset |
| Distinct witnesses/speakers behind included items | **Yes** | `BiasInstance.stated_by: Option<ActorOption>` (`bias/dto.rs:182`), from `(e)-[:STATED_BY]->(actor)`. **Caveat:** `execute_filtered_query` makes STATED_BY a *mandatory* MATCH (`bias/repository.rs:321`), so Evidence with no speaker is invisible to the *bias* path — check which the gather path uses before relying on the denominator |
| **Grounding / verified-quote status of included items** | **NO — not on any current payload** | The property EXISTS on the node: `create_entity_node` writes `n.grounding_status` (`api/pipeline/ingest_helpers.rs:502`), and the source of truth is `extraction_items.grounding_status` in Postgres. But `BiasInstance` does not carry it (`bias/dto.rs:153-186`) and no scenario query selects it. Requires either one field added to the gather projection, or a Postgres join on the item id |
| Scan-run history with verdict counts | **Yes** | Postgres `scan_runs` (`run_id, model_id, dry_run, candidates_read, relevant_count, irrelevant_count, failed_count, input/output_tokens, computed_cost, started_at, duration_ms`) and `scan_run_verdicts` (`relevant, proposed_role, confidence, reason, raw_reply, error`) — migration `20260715121130…:42-150`. Already surfaced by `RunHistoryList` |

**Not computable, stated plainly:** *themes.* There is no theme entity anywhere —
`ScenarioDefinition` (schema_v 2) carries only `attack_text`, `attack_meaning`, `target`,
`wielders`, `schema_v` (`dto/scenario_crud.rs:172-195`), and none of `scenarios`,
`scenario_fact_refs`, `scan_runs`, `scan_run_verdicts` has a theme column. Already ratified out
of the requirement; restated here because a readiness metric per theme is the natural next ask.

---

## 5. (folded into §4 and §6)

---

## 6. Bias tab — assessment

**What it computes.** Two endpoints. `GET /api/bias/available-filters` returns the dropdown
vocabularies — distinct actors, distinct `pattern_tags`, distinct subjects — each derived from
the data at query time, none hardcoded (`bias/repository.rs:97-243`) **[code]**. `POST
/api/bias/query` returns every matching Evidence instance plus `total_count` (deduped matches)
and `total_unfiltered` (all tagged Evidence) (`:294-306`).

**Inputs.** `Evidence.pattern_tags`, stored as a **comma-joined string** and split in Cypher
(`UNWIND split(e.pattern_tags, ',')`, `:240`), plus `STATED_BY` and `ABOUT` edges.

**Does its pipeline still run against current schema? — Yes.** `pattern_tags` is declared on the
Evidence entity in **every** Evidence-producing schema at HEAD: `affidavit_schema_v5_1.yaml:114`,
`discovery_response_schema_v5_1.yaml:116`, `court_ruling_schema_v5_2.yaml:123`,
`court_transcript_schema_v5_3.yaml:150`, `motion_schema_v5_3.yaml:145`,
`appellate_brief_schema_v5_3.yaml:165`, `correspondence_schema_v5_3.yaml:182` **[code]**, and it
is referenced by the current pass-1 and pass-2 templates for those types **[code]**. It is
written to the graph by the generic property loop in `create_entity_node`
(`api/pipeline/ingest_helpers.rs:538-560`) **[code]**.

**Assessment: live data, not a stale artifact** — with three caveats an architect should weigh
before making it canonical:

1. **`MATCH (e)-[:STATED_BY]->(actor)` is mandatory in the filtered query**
   (`bias/repository.rs:321`) **[code]**. Evidence with no speaker cannot appear, and
   `total_unfiltered` (which has no such join, `:217-219`) counts it — so the two numbers on
   screen are drawn from different populations.
2. **The tag vocabulary is per-template free text.** Nothing validates it, and no two document
   types are guaranteed to use the same tags. Deduplication is by exact string after `trim`
   (`:240-243`).
3. **`d.document_type` is read in three queries** (`:348`, `:451`, `:560`) — a property that has
   never existed on a `Document` node. See §7.5.

### 6.1 War Room dashboard — per the instruction, each figure and its source

Covered in full at §2.6 and rows 31–41 of §1. Summary: **one live figure** (instance count,
Postgres anchors × Neo4j REBUTS∪CORROBORATES), **three real Postgres counts** (scenarios, ready,
draft), **two derived-from-stubs figures that present as measurements** (baseless repeat
patterns → always 0; no response yet → always = scenario count), and **four honest empties**
(response count, speakers, baseless repeat, alerts).

---

## 7. Overlaps and contradictions — the most valuable findings

### 7.1 Three per-document connection tables, three definitions of "linked"

| Surface | Numerator = Evidence linked when it has… | Denominator | Corpus total |
|---|---|---|---|
| **Case Health** | ≥1 CORROBORATES/REBUTS/CHARACTERIZES (probative) — or +ABOUT (topical) | Evidence CONTAINED_IN the doc | measured over **all** Evidence, independently |
| **Case Analysis** (`/analysis`, orphaned) | CORROBORATES **or** the legacy `MotionClaim -[:PROVES]->` path | Evidence CONTAINED_IN the doc | **summed over the document rows** |
| **Proof Matrix / Explorer** | CORROBORATES only (Proof Matrix); CORROBORATES ∪ MotionClaim (Explorer) | per Element / per Allegation, not per document | n/a |

Three consequences **[inference]**:

- The three surfaces will show **different connection numbers for the same document**, and no
  label on any of them says which definition is in play.
- Case Analysis's corpus total is a **sum of per-document rows**
  (`analysis_repository.rs:274-276`); Case Health measures the corpus independently and reports
  `evidence_without_document` precisely so the gap between the two approaches is visible. An
  Evidence node with no Document is invisible to the Analysis total.
- Only Case Health's definition excludes `ABOUT` from the headline. The other two never
  considered ABOUT at all, so they are neither probative nor topical — they are a third thing.

### 7.2 Two element/allegation verdict vocabularies

`derive_proof_status` → `no_allegations / gap / partial / supported`, from
`(T = BEARS_ON count, C = allegations with ≥1 CORROBORATES)`
(`causes_of_action_builder.rs:138-147`).

`calculate_strength` → `gap / weak / moderate / strong` **+ a percentage**, from a raw evidence
count with a **different edge set** (`analysis_repository.rs:56-64`).

Both are per-Allegation-or-Element coverage verdicts. Both use the word **"gap"** for different
conditions: `derive_proof_status` means "allegations mapped, none corroborated"; `calculate_strength`
means "zero evidence items found by the MotionClaim∪CORROBORATES union." An allegation can be
`gap` on one page and `weak` on the other. **[inference]** The ratified rule ("one element-verdict
vocabulary; Pane 2 wraps `derive_proof_status`", `docs/DECISION_LOG.md` 2026-07-27) resolves the
*future* case but does not retire `calculate_strength`, which is live on the nav-reachable
`/explorer` page today.

### 7.3 The same word means three different things

- **"Complete"** — Admin Audit: metadata presence on Evidence nodes (§8).
- **"Completeness"** — per-document pipeline check: expected graph nodes and Qdrant points exist
  (`api/pipeline/completeness.rs:228-234`).
- **"Connected"** — Case Health: has an edge to an Allegation.

None measures the others. **[inference]** An operator seeing "Complete: 96%" on Admin and "19.6%
probative" on Case Health has no way to know these are unrelated axes.

Likewise **"instances"**: on the War Room card it means corroborating+rebutting evidence on the
anchor allegations (`scenario_dashboard.rs:209-224`); on the Bias Explorer it means a tagged
Evidence node (`BiasByActorView.tsx:100`).

### 7.4 Figures that are structurally incapable of being non-zero

| Figure | Why | Provenance |
|---|---|---|
| Decomposition `proven_count` / `all_proven` | Query returns literal `NULL AS status`; code tests `status == Some("PROVEN")` | `decomposition_repository.rs:74` vs `:187` |
| `CaseStats.evidence_count` | Hardcoded `0` with the comment `// Evidence nodes don't exist in v2` — **525 do** | `case_repository.rs:222` |
| War Room "Baseless repeat patterns" | Counts cards with `baseless_repeat_count > 0`; the field is always `None` | `scenario_dashboard.rs:350` vs `:369-372` |
| Proof Matrix "Opposing" column | Frontend passes `[]` | `MatrixRowWithDetail.tsx:62` |
| Decomposition `total_rebuttals`, per-allegation `rebuttal_count` | Counts `Evidence-[:REBUTS]->Evidence` — an edge class with zero instances | `decomposition_repository.rs:70` |
| Contradictions total (`/contradictions`, `/analysis`) | Counts `Evidence-[:CONTRADICTS]->Evidence` — zero instances | `analysis_repository.rs:187` |

The last two are honest zeros over a real (empty) edge class — they will become non-zero when
G1 lands. The first four are **defects**: they would stay zero after the edge layer ships.

### 7.5 `allegations_proven` measures quote-verifiability and calls it proof

```rust
// "grounding_status of 'exact' or 'normalized' means the allegation is proven"
let allegations_proven = allegations.iter().filter(|a| {
    a.properties.get("grounding_status")… == "exact" || … == "normalized"
}).count()
```
`repositories/case_repository.rs:203-217` (duplicated at `case_summary_repository.rs:187`) **[code]**

`grounding_status` records whether the allegation's **own verbatim quote was located in its own
source PDF**. It says nothing about evidentiary support. Calling it "proven" inverts the meaning
of the word this product exists to measure. **Mitigating:** `CaseStats` is fetched app-wide via
`CaseContext` but **only `legal_count_details` is ever read** (`AllegationsPage.tsx:35`,
`EvidenceExplorerPage.tsx:145`) **[code]** — so the figure is computed on every page load and
displayed nowhere. It is a landmine, not a live wrong number.

### 7.6 The `document_type` error is already in shipped product code

`bias/repository.rs` projects **`d.document_type`** in three queries — `:348`
(`execute_filtered_query`, the Bias Explorer), `:451` (`evidence_by_ids`, the curation hydrate
path called from `api/scenario_facts.rs:415`), and `:560` (`all_evidence_about_subject`, the
candidate gather at `api/scenario_gather.rs:303` **and** Theme Scan at
`services/theme_scan.rs:449`) **[code]**.

No write path in the repo ever sets `document_type` on a `Document` node — every writer sets
`doc_type` (`api/pipeline/ingest_helpers.rs:186`, `repositories/document_repository.rs:94,147,271`)
**[code]**. Therefore **`DocumentRef.document_type` is `None` on every Bias card, every
candidate-workbench card, and every Theme Scan card.** The DTO comment at `bias/dto.rs:190-193`
even reasons about why `Option` distinguishes "no property" from "empty string" — a distinction
that has only ever resolved one way.

**Blast radius is currently latent**: the field is declared on the TS type
(`services/bias.ts:47`) but **never rendered** — no component reads it **[code]**. So this is a
dead field, not a wrong number on screen. It becomes a live defect the moment anything tries to
group scenario candidates or bias results by document type. **Fix is one word in three places.**

### 7.7 Compiled-in vocabularies that are not case-generic

`CORROBORATING_STATEMENT_TYPES = [admission, partial_admission]` and
`NON_ANSWER_STATEMENT_TYPES = [evasive, objection, referral, denial]`
(`models/document_status.rs:198-206`) **[code]** are **discovery-response** values living in
shared code and driving the Proof Review Excluded and Borderline tabs. The schemas state
`statement_type` is heterogeneous across document types
(`correspondence_schema_v5_3.yaml:43`) **[code]**. Any Case State service that needs a
statement-type axis must not inherit this list.

---

## 8. The completeness score, by name

**Where:** Admin → Audit tab, the fourth summary card, labelled **"Complete"**, rendered as
`${data.summary.completeness_pct.toFixed(0)}%` (`components/admin/AdminAudit.tsx:76`) **[code]**.

**Formula** (`api/admin_audit_health.rs:88-92`) **[code]**:

```rust
let completeness_pct = if total_evidence > 0 {
    (complete_evidence as f64 / total_evidence as f64) * 100.0
} else { 100.0 };
```

**Inputs** (`services/audit_checks.rs:95-145`) **[code]** — one Cypher read over every Evidence
node:

```cypher
MATCH (e:Evidence)
OPTIONAL MATCH (e)-[:STATED_BY]->(p)
OPTIONAL MATCH (e)-[:CONTAINED_IN]->(d)
RETURN e.id, e.verbatim_quote IS NOT NULL AS has_quote,
       e.page_number IS NOT NULL AS has_page,
       p IS NOT NULL AS has_speaker, d IS NOT NULL AS has_document
```

An Evidence node counts toward `complete_evidence` **only if all four hold**: it has a
`verbatim_quote`, a `page_number`, a `STATED_BY` edge, and a `CONTAINED_IN` edge.

**What it actually measures:** *metadata hygiene* — is each extracted item citable and
attributable? It measures **nothing about connection, coverage, or proof.**

**Two properties worth flagging for the keep/retire decision:**

1. **The empty case returns `100.0`** — a graph with zero Evidence reports "100% Complete."
   That is the opposite of the Case Health convention, where nothing-to-measure yields `null`.
2. It is the **only** figure in the product that would catch an Evidence node missing its quote
   or page — the precondition for every citation the case will ever make. **[inference]** That
   makes it a genuine candidate to become canonical under an honest name (*citability* /
   *attributability*), rather than a retirement. But "Complete" must go: it reads as a coverage
   claim and is not one.

---

## 9. Recommendation shape, and what I could not close

### 9.1 Per-figure disposition — my read

**Canonical (keep the computation, one home):** Case Health's connection tiers (#1-9);
`derive_proof_status` and its inputs (#10, 11, 13); Proof Review's corroborating/excluded
tallies (#14-18) *once the statement-type vocabulary is de-hardcoded*; per-document cost (#55);
grounding rate (#57); the audit metadata check (#60) *renamed*; scan-run tallies (#48).

**Becomes a view of a canonical figure:** #19, #29, #44, #50, #52 (length reads);
#30 and #23-24 (move the arithmetic to the backend, render the result);
#45-46 (move `countByStatus` to the backend as part of a scenario-state payload).

**Retire:** #20-21 and #26-27 (`calculate_strength` — a fabricated scale);
#12 (the hardcoded Opposing column — replace with the real REBUTS data or drop the column);
Decomposition `proven_count` / `all_proven` (#2.9); `CaseStats.evidence_count` and
`allegations_proven` (§7.4, §7.5); War Room #35-36 (either wire the sources or stop showing
figures derived from stubs).

**Decide separately:** #49 (run-vs-run agreement is a benchmark metric, not case state — it may
belong outside Case State entirely); #28 and Decomposition's rebuttal counts (honest zeros that
become real with G1).

### 9.2 The one structural recommendation

Every contradiction in §7 has the same root: **a metric's definition lives next to its query,
and each surface grew its own.** The Case State service fixes this only if the *edge-class
partition* and the *verdict vocabulary* live in one place that all three subjects read —
`domain/connection_tier.rs` and `derive_proof_status` are the two existing candidates, and
neither currently knows about the other. **[inference]** If Case State ships with those two
still separate, it will be the fourth per-document connection table rather than the last one.

### 9.3 What I could not close — five queries

Live DB access was unavailable (see BLOCKERS). These would settle the remaining value questions:

```cypher
// 1. How many Evidence nodes are invisible to the Bias Explorer (no STATED_BY)?
MATCH (e:Evidence) WHERE NOT (e)-[:STATED_BY]->() RETURN count(e);

// 2. Does the Proof Review statement_type vocabulary cover the corpus?
MATCH (e:Evidence) RETURN e.statement_type AS t, count(*) AS n ORDER BY n DESC;

// 3. What the Opposing column would show if it were wired.
MATCH (ev:Evidence)-[:REBUTS]->(a:Allegation)-[:BEARS_ON]->(el:Element)
RETURN el.id, count(DISTINCT ev) ORDER BY 2 DESC;

// 4. Confirms §7.5 — is allegations_proven non-zero, i.e. is the landmine armed?
MATCH (a:Allegation) RETURN a.grounding_status AS s, count(*) ORDER BY 2 DESC;
```

```sql
-- 5. (colossus_legal_v2) Grounding status of scenario-included items — the one
--    readiness input with no current payload (see §4).
SELECT r.scenario_id, i.grounding_status, count(*)
FROM scenario_fact_refs r
JOIN extraction_items i ON i.neo4j_node_id = r.graph_node_id
WHERE r.status = 'included'
GROUP BY r.scenario_id, i.grounding_status;
```

---

FINDINGS (what the code otherwise forces)

1. **`calculate_strength` is the single worst metric in the product** — a five-value lookup
   table published to the user as a measurement, on a nav-reachable page, with an InfoPopup
   documenting the invented scale as method.
2. **Three per-document connection tables with three definitions of "linked," none labelled.**
   Case Health is the only one that separates probative from topical.
3. **Four figures cannot be non-zero regardless of data** — and would stay zero after the E→E
   edge layer ships. Two more (contradictions, Evidence→Evidence rebuttals) are honest zeros
   that G1 will make real; do not confuse the two groups.
4. **`allegations_proven` calls quote-verifiability "proven."** Computed on every page load,
   rendered nowhere. Delete it before something starts rendering it.
5. **`d.document_type` is read in three shipped Bias queries** — the same never-existed property
   as the 2026-07-26 alarm findings, this time in product code. Latent (never rendered), one
   word to fix, and it blocks any by-document-type grouping of scenario candidates.
6. **Seven frontend-computed figures**, four of them real arithmetic on case data. The
   candidate-status counts (#45) matter most: they are the readiness slice's own inputs.
7. **"Complete", "Completeness" and "Connected" are three unrelated axes** sharing a word
   family. The Admin metadata check is worth keeping under an honest name — it is the only
   thing guarding citability.
8. **Undecided candidates have no database row.** Any scenario-state computation must derive
   undecided as `pool − included − dropped`. Counting rows silently under-reports it.
9. **Grounding status of scenario-included items is the one readiness input with no payload.**
   The data exists in both stores; nothing selects it.
10. **10 of 21 metric surfaces are unreachable by clicking** — `/analysis`, `/decomposition`,
    `/contradictions`, `/claims`, `/damages`, `/hearings`, `/decisions`, `/graph`, `/queries`,
    `/search`. Retiring an orphaned surface costs nothing; three of them are the only consumers
    of `calculate_strength`'s bucket counts.

=== REPORT END ===

STOP — read and report only. No code was written, nothing was committed, no document was
processed, no API spend was incurred. The working tree carries only this report and the
pre-existing `backend/Cargo.lock` version-string drift.

Two things worth your ruling before the Case State design proceeds:

- **`calculate_strength` is live on a nav-reachable page today.** The ratified "one
  element-verdict vocabulary" rule governs Pane 2's future; it does not retire this. Retiring
  `/explorer`'s strength display is a small, self-contained change that could land well before
  Case State — or it can wait and be absorbed. Your call which.
- **The five queries in §9.3** are the open value questions. If you want me to run them, Bash
  permission for `ssh core@10.10.100.220` would do it; otherwise they are ready to paste.
