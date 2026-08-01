# A0 Inventory — Probative-Set Drift Outside the Partition

**Date:** 2026-08-01
**Task:** CC_TASK_0_1_A0_CASE_STATE_SKELETON_v1, Q1 addition (read-only, report-only)
**Branch:** `feature/case-state-a0`
**Status:** Worklist for **A-remainder** (post-B). Nothing in this document was
changed by A0. A0 moved the partition into `domain/case_state/` and made the
tier sets private; it did not re-route any of the traversals below.

---

## Scope of the scan

Every occurrence outside `backend/src/domain/case_state/` of the inline
pipe-pattern `CORROBORATES|REBUTS|CHARACTERIZES`, or of any hand-assembled
probative-triple list. Searched: all `.rs`, `.ts`, `.tsx`, `.md`, `.yaml`,
`.yml` under the repo, excluding `target/` and `node_modules/`.

**Headline result:** no file outside `domain/case_state/` contains the live
pipe-pattern in *executable* Cypher. What exists instead is **hand-assembled
edge-class sets in Rust**, plus pipe-patterns in doc comments and markdown.

**Five mutually inconsistent probative sets** are in production use. That
inconsistency is the drift the partition exists to end, and it is the
justification for A-remainder.

| # | Set | Where |
|---|-----|-------|
| 1 | `{CORROBORATES, REBUTS, CHARACTERIZES}` | the partition (ratified) |
| 2 | `{CORROBORATES, REBUTS}` | causes of action, element detail |
| 3 | `{CHARACTERIZES, REBUTS}` | allegation detail |
| 4 | `{CORROBORATES}` | proof review, analysis |
| 5 | `{ABOUT, CHARACTERIZES, REBUTS}` | graph expansion |

A sixth, `{CORROBORATES, REBUTS, CONTRADICTS}`, appears in a doc comment only.

---

## 1. Hand-assembled sets that function as a connectedness notion

These are A-remainder's core worklist: each decides what "connected" or
"bearing on the allegation" means by listing edge classes, rather than asking
the partition.

| Location | Set assembled | Note |
|---|---|---|
| `backend/src/repositories/scenario_repository.rs:123-125` | `{REBUTS}` / `{CORROBORATES}` / `{REBUTS, CORROBORATES}` | **The most partition-shaped construct outside `case_state`.** `EvidencePolarity::rel_types()` is a typed vocabulary that returns edge-class sets — structurally the same thing `ConnectionTier::edge_types()` is, built independently. Missing CHARACTERIZES. If any single item in this inventory should be folded into the partition family, it is this one |
| `backend/src/repositories/causes_of_action_repository.rs:167-168` | `{CORROBORATES, REBUTS}` | Feeds the proof computation — the same code path as `derive_proof_status`, which is itself A-remainder work |
| `backend/src/repositories/element_detail_repository.rs:233-234` | `{CORROBORATES, REBUTS}` | |
| `backend/src/repositories/allegation_detail_repository.rs:90,93` | `{CHARACTERIZES, REBUTS}` | A third distinct pair — note it *excludes* CORROBORATES while including CHARACTERIZES, the inverse of the row above |
| `backend/src/repositories/proof_review_repository.rs:130,157` | `{CORROBORATES}` | Its test at `:340` asserts `NOT (e)-[:CORROBORATES]->()` — i.e. **"unconnected" defined as CORROBORATES-only**. A single-edge notion of connectedness, and the furthest from the ratified partition |
| `backend/src/repositories/analysis_repository.rs:69` | `{CORROBORATES}` | |
| `backend/src/services/graph_expansion_cypher.rs:38-41` | `{ABOUT, CHARACTERIZES, REBUTS}` | Expansion traversal |
| `backend/src/services/graph_expansion_queries.rs:106,130,136` | `{ABOUT, CHARACTERIZES, REBUTS}` | |

---

## 2. Adjacent but NOT a tier — do not fold into the partition

`backend/src/neo4j/human_facts.rs:107-113` (mirrored in its test at `:696-701`)
assembles `{ABOUT, CONTRADICTS, CHARACTERIZES, REBUTS}`.

This is the **human-authorable relationship allowlist**, not a connection tier.
It is a different vocabulary answering a different question ("which rel types
may a human create by hand?"). Note it has **no CORROBORATES** and **adds
CONTRADICTS** — it is not a probative set that drifted, it is a distinct
concept that happens to overlap. Flagged here so A-remainder does not mistake
it for a tier and fold it in.

---

## 3. Pipe-patterns — all in prose or documentation

| Location | Pattern | Note |
|---|---|---|
| `docs/CASE_HEALTH_QUERIES.md:262` | `CORROBORATES\|REBUTS\|CHARACTERIZES` | **The exact probative triple.** Ranked first — see below |
| `docs/CASE_HEALTH_QUERIES.md:247` | `CORROBORATES\|REBUTS\|CHARACTERIZES\|ABOUT` | The topical four |
| `docs/PROOF_MATRIX_QUERIES.md:52` | `CORROBORATES\|REBUTS` | |
| `backend/src/repositories/scenario_repository.rs:219` | `CORROBORATES\|REBUTS\|CONTRADICTS` | Doc comment — a sixth variant, and the only one naming CONTRADICTS |
| `backend/src/dto/scenario.rs:169` | `REBUTS\|CORROBORATES` | Doc comment |
| `CC-REPORTS/case_health_dashboard_read_report.md:679` | all four | Historical report — no action |
| `CC-REPORTS/metric_inventory_report.md:381,505` | `REBUTS\|CORROBORATES`, all four | Historical report — no action |

---

## Ranked first for A-remainder: `docs/CASE_HEALTH_QUERIES.md`

The pair at `:247` and `:262` is the highest-priority item in this inventory,
ahead of any code site.

Those are Pane 1's **published provenance queries** — the documented method a
human is told to run to reproduce the dashboard headline by hand. They are a
second copy of the partition, written in prose, that **no test pins to the
code**. Every code site in section 1 is at least visible to the compiler and
covered by its own query-shape tests; this one is not. If the partition ever
changes and the document does not, the provenance rule breaks silently: a human
runs the documented query, gets a different number from the screen, and has no
way to tell which is wrong.

The fix is a Rule-21 disk/code test asserting the documented query text matches
the partition's tier sets — the same shape as `sql_invariants.rs` and the
`neo4j::schema` scan.

---

## What A0 did NOT do

A0's visibility law makes the tier **sets** private to `partition.rs`, so no
module can read the definition directly. It does **not** stop a module from
spelling `schema::CORROBORATES` and assembling its own set — that is ordinary
Cypher construction, and forbidding it would forbid graph traversal. Closing
the gap between "cannot read the definition" and "does not build a rival
definition" is exactly what the re-routing work in this inventory is for.
