# CC_TASK_TEMPLATE_BATCH_HANDOFF_NOTE_v1 — resume point for the id-arm batch

**Written:** 2026-08-14 by the CC session that ran Phase A and built P1a.
**For:** a fresh session with zero prior context. Read this **with**
`~/Documents/colossus-legal/CC-INSTRUCTIONS/CC_TASK_TEMPLATE_BATCH_AND_ID_ARM_v1.md`
(the task file). Everything below is measured or ruled — nothing is assumed.

---

## WHERE THE WORK IS

- **Branch:** `feat/template-batch-id-arm`, off `origin/main`.
- **Base:** `9d50dcd chore: bump version to v2.0.0-beta.395` (the deployed
  release; it contains the Tuesday matrix batch).
- **P1a commit:** `6ee1263 feat(pipeline): give Evidence a stable id arm keyed on
  the document, not the model` — the only CODE commit on the branch; the two
  documentation commits sit on top of it. Nothing is pushed.
  *(An earlier arrangement of these commits was rebuilt on 2026-08-14 — if you
  find references anywhere to `a3344b6` or `f8a1864`, they are dead hashes for
  this same content.)*
- **Reports:** `CC-REPORTS/CC_REPORT_TEMPLATE_BATCH_ID_ARM_PHASE_A.md`, also
  copied to `~/Documents/colossus-legal/CC-REPORTS/`.

**Do not push unless Roman says so.** No version bump, no tag.

## COMPLETE

**Phase A** — measured, reported, ruled. **P1a** — `backend/src/api/pipeline/
evidence_key.rs` + `evidence_key_tests.rs`, wired into `stable_entity_id` in
`ingest_helpers.rs` under a new `ENTITY_EVIDENCE` arm. Key is
`doc_slug + page_number + NFC-normalized verbatim_quote + question (when
present)`; id shape unchanged (`{doc_slug}:evidence:{8 hex}`); a quoteless item is
refused a key and falls back to the blob hash with a `warn` (never taken on the
live corpus). New direct dependency: `unicode-normalization = "0.1"` (was already
in the lock at 0.1.25).

Verified at the time of commit: `cargo test --lib` **1881 passed / 0 failed**,
`cargo fmt --check` clean, `cargo clippy --lib --bins -- -D warnings` clean.

## THE NUMBERS (measured on DEV 2026-08-14, read-only)

| Figure | Value |
|---|---|
| Evidence nodes | **525** |
| Distinct keys under the new arm | **504** |
| Nodes with a unique key → **re-key these** | **483** |
| Collisions | **21 pairs, all ×2, no triples** (42 nodes) |
| Curated rows referencing Evidence ids | **947** (supersedes "~929" in the task file) |
| Distinct Evidence ids referenced | **148** |
| Twin pairs with BOTH twins curated | **7** (0 pairs one-sided, 14 neither) |
| Twin pairs with CONFLICTING rulings | **3** (one `carries`, one `backup`, same scenario) |

The 8 referencing columns across 7 tables:
`scenario_candidate_ordinals.graph_node_id` (444) ·
`scan_run_verdicts.graph_node_id` (226) · `scenario_ruling_anchors.graph_node_id`
(167) · `evidence_allegation_link_events.graph_node_id` (37) ·
`scenario_fact_refs.graph_node_id` (35) ·
`scenario_human_facts.anchor_graph_node_id` (18) ·
`evidence_allegation_links.graph_node_id` (11) ·
`scenario_human_facts.answers_graph_node_id` (9).

Also measured, for a separate future task — **build nothing for it here**:
per-page document text IS fully persisted in `document_text`
(`document_id`, `page_number`, `text_content`) — 160 rows, all 9 documents,
100% page coverage, 266,076 chars.

## THE P1b MECHANISM RULING (2026-08-14, verbatim)

> CORRECTION ON THE RECORD: "re-key as a migration" was the architect's error —
> sqlx cannot touch the graph or run the Rust arm. Your read is right. RULING:
> option 3.
>
> - Schema stays sqlx: P4's document_date/date_precision columns + all wording
>   rows = normal migrations, as always.
> - The re-key is a Rust ONE-SHOT MAINTENANCE BINARY (not an admin endpoint —
>   nothing clickable, nothing rerunnable by accident):
>   * dry-run by default, --apply to execute; Roman runs it per a runbook step
>     after the batch deploys, before the wave.
>   * per-document unit of work: compute new ids via the arm → update the Neo4j
>     node property and every Postgres referencing row for that document
>     together; on any count mismatch, abort that document, report, leave it
>     untouched. Postgres side transactional; the runbook step takes DB backups
>     immediately before --apply (that is the two-store safety net, stated
>     honestly rather than a pretended cross-store transaction).
>   * count proofs as real tracing output AND a written report file: per
>     document, rows expected/updated per table, nodes re-keyed; final totals
>     against 947 rows / 483 nodes.
>   * idempotent: already-new-format ids are skipped, so a partial run resumes
>     safely.
> - The twin-merge script (built, not run) and the remap script join it as the
>   same family of one-shot tools with the same dry-run/--apply discipline.

**Why it mattered:** `backend/migrations_neo4j/` exists but **nothing in `src/` or
`scripts/` references it** — there is no Neo4j migration runner. And the new ids
cannot be computed in SQL: the key needs NFC + SHA-256 over normalized text,
which is the Rust arm.

## NOT STARTED

- **P1b** — the re-key one-shot binary (per the ruling above). Re-keys **483**
  nodes, not 525/504. The 21 twin pairs keep their current ids, untouched, and
  must be enumerated in the completion report with their curated-row map.
- **Twin-merge script** — built, NOT run. One survivor per pair keyed by the new
  arm, union of edges and curated rows, provenance count ×2. Where the twins'
  rulings CONFLICT (the 3 weight cases) it takes **no default** — emit to the
  human queue. Roman rules the 7 curated pairs in a merge session **after Chuck
  Tuesday**; the 14 uncurated pairs merge mechanically in the same run.
- **Ingest-time dedupe** — two extraction items producing the same stable id
  merge to ONE node at ingest with an occurrence count. Kills the class at the
  door.
- **P2** remap script (built, NOT run) + tests · **P3** ISO paragraph into every
  pass-1 template, v5_3→v5_4 · **P4** `documents.document_date` +
  `date_precision` (+ Neo4j mirror at ingest, intake UI field with precision,
  post-hoc edit path for the 9 ingested; describe field placement in the report —
  **mockup-first flag stands**) · **P5a** brief templates · **P5b** motion
  templates · **P6** party/alias tightening.

**Drop order unchanged: P6 first, then P5b, then P5a. P1–P4 never drop.**

## DISCIPLINE FOR WHOEVER RESUMES

- Gate unchanged: four agents once, full suites once, `cargo check --bins` in the
  verification list. Completion report as a FILE in **both** CC-REPORTS locations
  (repo + `~/Documents/colossus-legal/CC-REPORTS/`).
- `cargo test --workspace` is **broken on main** — `backend/tests/*.rs` have had
  stale `AppState` fields since ~beta.343. `cargo test --lib` is the honest
  baseline. Do not "fix" those files as part of this batch.
- Frontend has no lint script: `npm run typecheck`, `npm test`, `npm run build`.
- Live DEV reads (read-only!) — clients live on the DB host, not the app host:
  ```
  ssh core@10.10.100.200 'sudo podman exec -i colossus-postgres psql -U postgres -d colossus_legal_v2 -X -f -' < q.sql
  ssh core@10.10.100.200 'sudo podman exec -i colossus-neo4j sh -c '"'"'cypher-shell -u neo4j -p "${NEO4J_AUTH#neo4j/}" --format plain'"'"'' < q.cypher
  ```
- **S-5 and S-6 rows are READ-ONLY to this batch.** Nothing here runs a reprocess
  or the remap. The Complaint is do-not-reprocess.
- One thing worth knowing: the `.395` matrix work already collapses the twin class
  at DISPLAY time (same speaker + question + answer → one row with "×2"), so each
  pair already presents to Chuck as one item. That is display-layer mercy over a
  data-layer defect — the merge script is the cure, and there is no Tuesday
  exposure meanwhile.
