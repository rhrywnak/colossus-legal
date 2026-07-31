# SCENARIO_REFS_DIAGNOSTIC — read and report

**Branch:** `feature/scenario-1a` · **HEAD:** `39e5817 fix(theme-scan): validate the request before recording a run`
**Mode:** read-only. No files written outside this report. No commits.
**Date:** 2026-07-29 · **Empirical half added:** 2026-07-29 (later session, SSH unblocked)

---

## 0. STATUS — the live half HAS NOW BEEN RUN

> **UPDATE 2026-07-29.** SSH was unblocked and every query in §1.6 was executed against
> DEV. The original blocker text is preserved below for the record; it is **no longer in
> effect**. The measured results are in **§1.7**, and the Q1 conclusion in **§1.8**.
>
> **The measured answer overturns the code-only ranking.** H1 (the leading hypothesis) is
> **falsified** — Family B does not exist in this environment at all. **H3 is confirmed**:
> all 6 `included` refs, and in fact all 26 refs of every status, are **dead pointers**
> left behind by the 2026-07-24/25 re-extraction. The join key is correct and provably
> works today (296 of 390 pool members join). See §1.7.

### 0.1 Method for the empirical half

Throughout §1.7–§1.8, **measured** means it came out of a query run this session.
**Inferred** means I am reasoning past the evidence and it is labelled as such.

Access path (read-only, `SELECT`/`MATCH` only, no writes, no DDL, no temp tables):

- Postgres — `ssh core@10.10.100.200` → `sudo podman exec -i colossus-postgres psql -U postgres -d colossus_legal_v2`
  (container `docker.io/library/postgres:17`, reported `PostgreSQL 17.7`).
- Neo4j — same host → `sudo podman exec -i colossus-neo4j cypher-shell` (container `docker.io/library/neo4j:5`).

The app host `10.10.100.220` (`colossus-dev-app1`) carries no `psql`/`cypher-shell`; the
DB host `10.10.100.200` (`colossus-dev-db1`) hosts both engines as containers. Recorded
here so the next session does not have to rediscover it.

*Incidental observation, out of scope:* `colossus-qdrant` on the DB host runs
`docker.io/qdrant/qdrant:latest` — a `:latest` tag, contrary to CLAUDE.md Rule 23. Noted,
not acted on.

### 0.2 The original blocker (historic — resolved)

The instruction budgeted for live read-only DB queries via `ssh core@10.10.100.220`.
**SSH was hard-denied by this repo's permission configuration**, not by a prompt Roman could
approve at runtime:

> `.claude/settings.local.json` → `permissions.deny` contains `"Bash(ssh*)"`,
> alongside `"Bash(scp*)"`, `"Bash(curl*)"`, `"Bash(nc*)"`, `"Bash(sudo*)"`.
> *(provenance: `.claude/settings.local.json`, deny array, read in the first session)*

A `deny` entry is evaluated before any permission prompt, so no approval dialog was ever
raised — three attempts (`ssh core@… podman ps`, with and without `sudo`) were refused
outright. This was the same block recorded in two prior reports:
`CC-REPORTS/case_health_dashboard_read_report.md:19` and `:773`, and
`CC-REPORTS/metric_inventory_report.md:829`.

### 0.3 What §1.1–§1.6 and §2 therefore are

Everything in §1.1–§1.6 and §2 is derived from **source, migrations, and git history** — all
of which are authoritative about *what the code writes* and *what the contract says*.
Provenance is on every claim. **They are left exactly as first written**, including the
hypothesis ranking that the measurement went on to overturn, so the code-only reasoning and
the empirical result can be compared honestly. Where a measurement contradicts them, the
contradiction is called out in place with a `> **MEASURED …**` block.

- **Q2 is answered completely from code and history.** The writers of `undecided` rows are
  enumerable exactly, and the apparent contract violation resolves without live data (§2).
  Live data was needed only to *attribute* the 17 rows among the known writers — that
  attribution is now measured, in §2.5.
- **Q1 was answered structurally but not empirically.** Code proves there are exactly two
  Evidence-node provenance families, one of which *can* join and one of which *can never*
  join (§1.3). Which family the 6 `included` rows belong to was a fact only the database
  held. It is now measured: **neither** — they belong to a *retired generation* of Family A.

---

## 1. Q1 — WHY DOESN'T THE JOIN MATCH?

The join under test:

```sql
FROM scenario_fact_refs r
JOIN extraction_items i ON r.graph_node_id = i.neo4j_node_id
WHERE r.status = 'included'
```

Measured tonight: 6 `included` rows, **0 join rows**.

### 1.1 What each column actually holds

**`scenario_fact_refs.graph_node_id`** — the Neo4j `Evidence` node's **`id` property**
(not `elementId`, not a Postgres key).

> Declared: `backend/pipeline_migrations/20260626122424_create_scenario_fact_refs_table.sql`
> — *"The Neo4j node id (the graph node's id property). Plain TEXT, NO foreign key."*
>
> Confirmed at the write side: every value written traces back to `BiasInstance.evidence_id`
> (`backend/src/api/scenario_gather.rs:139` — `index.get(&content.evidence_id)`), which is
> projected by Cypher as `e.id AS evidence_id`
> (`backend/src/bias/queries.rs:111`, `:159`, `:210` — all three Evidence queries).

**`extraction_items.neo4j_node_id`** — `VARCHAR(255)`, populated **only** by the pipeline
Ingest step, from the in-memory `pg_to_neo4j` map, **after** the Neo4j transaction commits.

> Column: `backend/pipeline_migrations/20260421212806_add_neo4j_node_id_to_extraction_items.sql`
> — *"Ingest populates this column after its Neo4j transaction commits, using the in-memory
> `pg_to_neo4j: HashMap<i32, String>`."*
>
> Write: `backend/src/pipeline/steps/ingest.rs:646-651` (step 14c) →
> `pipeline_repository::batch_update_neo4j_node_ids` →
> `backend/src/repositories/pipeline_repository/extraction_items.rs:195-204`
> (`UPDATE extraction_items SET neo4j_node_id = $1 WHERE id = $2`).
>
> The map is filled at `backend/src/pipeline/steps/ingest.rs:392-399`, where
> `neo4j_id = create_entity_node(...)` returns `stable_entity_id(item, doc_id)`
> (`backend/src/api/pipeline/ingest_helpers.rs:485`).

**Both columns live in `colossus_legal_v2`** (the pipeline DB), so this is not a
cross-database join. `scenario_fact_refs` — pipeline migration, per its own header
*"Target: pipeline database (colossus_legal_v2)"*. `extraction_items` — created in
`backend/pipeline_migrations/20260327_create_pipeline_tables.sql`.

### 1.2 On the happy path the two values are literally the same string

For a pipeline-ingested Evidence node, one function produces the value that goes into
*both* places:

```
stable_entity_id(item, doc_id)  →  the `id` property MERGEd onto the Neo4j node
                               →  extraction_items.neo4j_node_id
```

> `backend/src/api/pipeline/ingest_helpers.rs:469-520` — `create_entity_node` computes
> `let neo4j_id = stable_entity_id(item, doc_id);` (`:485`) and runs
> `MERGE (n:{entity_type} {{id: $id}}) …` with that exact string (`:496-509`), then
> **returns it** (`:534`). The caller stores the returned value in `pg_to_neo4j`
> (`ingest.rs:399`), and step 14c writes that same map to `neo4j_node_id`.

`Evidence` is not a special-cased entity type; it falls into `stable_entity_id`'s catch-all
arm, so the format is:

```
{doc_slug}:evidence:{sha256_of_item_data[..8]}
```

> `backend/src/api/pipeline/ingest_helpers.rs:149-159` — the `other =>` arm:
> `format!("{}:{}:{}", doc_slug, slug(other), &hash[..8])`.
> `slug()` lowercases (`ingest_helpers.rs:29-38`), so entity_type `"Evidence"` → `"evidence"`.
> The named arms are only `ComplaintAllegation` (`{doc}:para:{n}`), `LegalCount`
> (`count-{n}`), and `Harm` (`{doc}:harm:{hash8}`) — `:80`, `:106`, `:144`.

**So a format/prefix mismatch is ruled out for pipeline-ingested Evidence.** The two sides
cannot disagree on shape; they are the same variable.

### 1.3 There are TWO Evidence provenance families, and only one can ever join

Grepping every `Evidence`-node creation site in the backend
(`MERGE (…:Evidence` / `CREATE (e:Evidence`):

| # | Writer | Node `id` source | Writes an `extraction_items` row? | Can join? |
|---|---|---|---|---|
| A | Pipeline Ingest — `create_entity_node` (`ingest_helpers.rs:469`), driven by `ingest.rs:386-399` | `stable_entity_id()` → `{doc_slug}:evidence:{hash8}` | **Yes** — the item *is* the extraction_items row; `neo4j_node_id` set at step 14c | **Yes** |
| B | Admin authoring — `POST /api/admin/evidence` (`backend/src/api/admin_evidence.rs:157`) | **client-supplied** `item.id`, taken verbatim from the request body | **No** — this path never touches Postgres `extraction_items` at all | **Never** |

> Family B provenance: `backend/src/api/admin_evidence.rs:155-168` —
> `CREATE (e:Evidence { id: $id, title: $title, … })` with `.param("id", item.id.as_str())`.
> The whole handler (`:95-200`) talks only to `state.graph`; there is no `pipeline_pool`
> write anywhere in it. The id is whatever the caller sent — the only constraint is a
> pre-flight duplicate check (`:118-139`), not a format rule.

The other three `:Evidence` matches in the codebase are **readers**, not writers:
`repositories/scenario_repository.rs:368`, `api/admin_evidence_helpers.rs:85`, and
`services/graph_expansion_cypher.rs:21` — all `MATCH`.

**This is the missing link the question asks for.** `extraction_items.neo4j_node_id` is not
a registry of graph nodes; it is a *lineage column on pipeline extraction rows*. An Evidence
node authored through the admin route exists in Neo4j with no Postgres counterpart, so no
join predicate written against `extraction_items` can ever reach it — not with a cast, not
with a normalization, not with a different column.

### 1.4 Ranked hypotheses for the zero-row result

Each is falsifiable by one of the queries in §1.6.

> **MEASURED VERDICTS (2026-07-29).** Ranking below is the code-only prediction, preserved
> as written. The measurement (§1.7) says:
>
> | Hypothesis | Code-only rank | **Measured** |
> |---|---|---|
> | H1 — Family-B admin-authored Evidence | *Most likely* | **FALSIFIED** — all 6 ids match the Family-A shape; zero Family-B nodes exist in the DEV graph |
> | H2 — Family-A with `neo4j_node_id IS NULL` | Plausible | **FALSIFIED** for these 6 — the ids are absent from `extraction_items` entirely, not NULL-columned. (The *condition* does exist elsewhere: 55 NULL rows, 45 of them from run 155.) |
> | H3 — target node retired/renumbered after the ref was written | Possible, secondary | **CONFIRMED** — 0 of 6 resolve to a live `Evidence` node |
> | H4 — whitespace/case/encoding drift | Lowest | **FALSIFIED** — `has_edge_whitespace = false` on all 6; loose match = exact match = 0 |
>
> The secondary hypothesis was the right one. See §1.8 for why the code-only ranking was
> wrong, since that is the reusable lesson.

**H1 — the 6 included refs point at Family-B (admin-authored) Evidence.** *Most likely.*
Structurally sufficient on its own: zero rows is the *predicted* result, not an anomaly.
The tell is the id shape — a Family-A id always contains `:evidence:` and an 8-hex-char
tail; a Family-B id is whatever a human or loader script chose (`ev-…`, `awad-…`, a UUID —
unconstrained by code).
*Falsified if:* the 6 ids all match `%:evidence:________`.

**H2 — Family-A ids, but the matching `extraction_items` rows have `neo4j_node_id IS NULL`.**
Plausible. `neo4j_node_id` is written *only* at ingest step 14c, which runs after the Neo4j
transaction commits (`ingest.rs:646`). Any Evidence node whose row predates the 2026-04-21
migration, or whose ingest run died between the graph commit and step 14c, has a live graph
node and a NULL column. The migration header names this case explicitly: *"a NULL falls back
to the existing recomputation path for rows written before this migration."*
*Falsified if:* the ids match the Family-A shape **and** `SELECT count(*) FROM
extraction_items WHERE neo4j_node_id IS NOT NULL` is non-zero for the same documents.

**H3 — the graph node was retired/renumbered after the ref was written.** Possible but
secondary. `scenario_fact_refs.graph_node_id` has **no FK** (it cannot have one — it points
into Neo4j), so a ref survives its target's deletion. The codebase already acknowledges a
duplicate-node defect whose fix leaves holes:
`backend/src/repositories/pipeline_repository/scenario_candidate_ordinals.rs:28-31` —
*"When the pipeline's duplicate-node defect is fixed, retired duplicates leave holes."*
The same dead-pointer case is a first-class outcome in the read path:
`backend/src/api/scenario_facts.rs:129` — *"A `scenario_fact_refs` row is a pointer into
Neo4j; if that Evidence node is [gone] …"* → `content: null`.
*Falsified if:* every one of the 6 ids resolves to a live `Evidence` node in Neo4j.

**H4 — whitespace / case / encoding drift.** *Lowest.* Both sides are machine-written from
the same `String` on the Family-A path (§1.2), and `slug()` already normalizes case. Only
reachable via Family B, where the id is human-supplied. Included for completeness because it
is cheap to test.

**Ruled out, do not re-investigate:**
- *Wrong database.* Both tables are in `colossus_legal_v2` (§1.1).
- *`elementId` vs `id` property.* Every Cypher projection uses `e.id`
  (`bias/queries.rs:111`, `:159`, `:210`); nothing in the scenario path reads `elementId()`.
- *Type mismatch.* `graph_node_id` is `TEXT`, `neo4j_node_id` is `VARCHAR(255)` — freely
  comparable in Postgres.

### 1.5 What the correct join is

**If H1 holds (Family B), there is no join, and that is the finding.** The readiness slice's
grounding input cannot be sourced from `extraction_items` for admin-authored Evidence,
because the grounding evidence for those nodes was never recorded in Postgres. What is
missing is not a join key — it is the row. The options at that point are: (a) read grounding
from the graph node's own properties, which Family A already writes
(`ingest_helpers.rs:499-503` sets `n.grounding_status`, `n.verbatim_quote`; `:527-536` sets
`n.page_number`) and which Family B **does not populate at all**
(`admin_evidence.rs:157-163` sets `id, title, content, verbatim_quote, page_number, date,
topic` — **no `grounding_status`**); or (b) treat "no grounding record" as its own
observable state rather than collapsing it into "ungrounded" (Standing Rule 1).

**If H2 holds (Family A with NULLs),** the join key is already correct and the fix is
backfill, not a predicate change — recompute via `stable_entity_id` exactly as
`resolve_extraction_neo4j_id` already does for the cross-tier-edge path
(`backend/src/pipeline/steps/llm_extract_pass2.rs:929-932`: prefer the stored
`neo4j_node_id`, else recompute from `item_data` + the item's own `document_id`).

**A note on the mixed case.** Nothing prevents a single scenario from holding refs of both
families — the pool is assembled by subject, not by provenance
(`bias/repository.rs:428` `all_evidence_about_subject`). So the readiness slice should not
assume one uniform source. Per-ref, grounding is either *recorded*, *absent because the node
was authored outside the pipeline*, or *absent because the pointer is dead* — three distinct
states that must stay distinguishable.

I am not recommending which of (a)/(b) to build; that is a design call and outside this
task's scope.

### 1.6 Paste-ready — the queries that decide Q1

Read-only. Run on `colossus_legal_v2` @ `10.10.100.200`. Each is followed by how to read it.

**Q1-a — the 6 included ids, verbatim, with invisible characters exposed:**
```sql
SELECT scenario_id,
       graph_node_id,
       length(graph_node_id)                                   AS len,
       graph_node_id <> btrim(graph_node_id)                   AS has_edge_whitespace,
       graph_node_id ~ ':evidence:[0-9a-f]{8}$'                AS looks_family_a,
       source_run_id,
       confidence,
       tagged_at
FROM scenario_fact_refs
WHERE status = 'included'
ORDER BY tagged_at;
```
> `looks_family_a = true` for all 6 → **H1 falsified**, go to Q1-b.
> `looks_family_a = false` → **H1 confirmed**: admin-authored Evidence, join impossible by
> construction. `has_edge_whitespace = true` anywhere → H4, trivially fixable.

**Q1-b — is the id present in `extraction_items` at all, and is the column NULL?**
```sql
SELECT r.graph_node_id,
       (SELECT count(*) FROM extraction_items i
         WHERE i.neo4j_node_id = r.graph_node_id)              AS exact_matches,
       (SELECT count(*) FROM extraction_items i
         WHERE btrim(lower(i.neo4j_node_id)) = btrim(lower(r.graph_node_id)))
                                                               AS loose_matches
FROM scenario_fact_refs r
WHERE r.status = 'included';
```
> `exact = 0, loose > 0` → **H4**, a normalization bug.
> both 0 → the id is not in `extraction_items` under any spelling → **H1 or H2**.

**Q1-c — H2: how much of `extraction_items` has a NULL lineage column?**
```sql
SELECT count(*)                                                        AS total_items,
       count(*) FILTER (WHERE neo4j_node_id IS NULL)                   AS null_node_id,
       count(*) FILTER (WHERE neo4j_node_id IS NOT NULL)               AS have_node_id,
       count(*) FILTER (WHERE entity_type = 'Evidence')                AS evidence_items,
       count(*) FILTER (WHERE entity_type = 'Evidence'
                          AND neo4j_node_id IS NULL)                   AS evidence_null
FROM extraction_items;
```
> `evidence_items = 0` → the pipeline never produced Evidence extraction items in this
> environment, which **confirms H1 decisively** and closes Q1 on its own.
> `evidence_null` large → **H2**.

**Q1-d — Neo4j: which family does each id actually belong to, and is the node alive?**
Run against the DEV graph (`bolt://10.10.100.200:7687`). Substitute the 6 ids from Q1-a.
```cypher
MATCH (e:Evidence)
WHERE e.id IN ['<id1>','<id2>','<id3>','<id4>','<id5>','<id6>']
RETURN e.id                        AS id,
       e.source_document           AS source_document,   // Family A only
       e.grounding_status           AS grounding_status,  // Family A only
       e.verbatim_quote IS NOT NULL AS has_quote,
       e.page_number                AS page_number
ORDER BY e.id;
```
> Fewer than 6 rows → **H3**: the missing ids are dead pointers.
> `source_document`/`grounding_status` NULL on a returned row → **Family B**, since
> `create_entity_node` always sets both (`ingest_helpers.rs:499-503`) and
> `admin_evidence.rs:157` sets neither.

**Q1-e — the shape census, to see whether the two families coexist:**
```cypher
MATCH (e:Evidence)
RETURN e.id =~ '.*:evidence:[0-9a-f]{8}' AS family_a_shape,
       e.grounding_status IS NOT NULL    AS has_grounding_status,
       count(*)                          AS n
ORDER BY n DESC;
```
> Tells you at a glance whether this graph is all-A, all-B, or mixed — which decides whether
> the readiness slice needs to handle both.

---

## 1.7 MEASURED — the §1.6 queries, run against DEV

All results below are **measured** on 2026-07-29 via the access path in §0.1. Queries were
run in the order the report specifies, starting with the family query.

### 1.7.1 Q1-c — the family query (Evidence rows in `extraction_items`)

```
 total_items | null_node_id | have_node_id | evidence_items | evidence_null
-------------+--------------+--------------+----------------+---------------
         849 |           55 |          794 |            574 |            49
```

**`evidence_items = 574`, not 0.** The report's pre-stated reading — *"`evidence_items = 0`
→ confirms H1 decisively and closes Q1"* — **does not fire**. The pipeline has produced
Evidence extraction items in this environment, in quantity. H1 does not close here.

Shape census of the lineage column, by entity type (**measured**, supplementary):

```
 entity_type |  n  | null_col | family_a_shape
-------------+-----+----------+----------------
 Evidence    | 574 |       49 |            525
 Party       | 138 |        5 |              0
 Allegation  | 120 |        0 |              0
 Harm        |  13 |        1 |              0
 LegalCount  |   4 |        0 |              0
```

Every non-NULL Evidence lineage id (525/525) matches `:evidence:[0-9a-f]{8}$`, confirming
§1.2's format derivation empirically. The other entity types use their own `stable_entity_id`
arms, as §1.2 predicted.

### 1.7.2 Q1-a — the 6 included ids, verbatim

```
graph_node_id                                                len  ws  family_a  run  conf  tagged_at
doc-george-phillips-response-to-discovery:evidence:5fd83f22   59  f   t         —    —     2026-07-07 12:44:16+00
doc-george-phillips-response-to-discovery:evidence:1ef2612f   59  f   t         —    —     2026-07-07 12:44:27+00
doc-george-phillips-response-to-discovery:evidence:700af54c   59  f   t         —    —     2026-07-19 15:29:56+00
doc-george-phillips-response-to-discovery:evidence:839a18ab   59  f   t         —    —     2026-07-19 15:29:58+00
doc-george-phillips-response-to-discovery:evidence:51d482bb   59  f   t         —    —     2026-07-19 15:30:00+00
doc-cfs-interrogatory-response-08-08-16:evidence:5e73af15     57  f   t         —    —     2026-07-19 15:30:12+00
```
*(all 6 on scenario `797bc26b-2831-4218-9dea-a4eb12865204`; `source_run_id` and `confidence`
NULL on all 6 — consistent with a human include, which passes `None` for both.)*

`looks_family_a = true` on all 6 → **H1 falsified**, per the report's own pre-stated reading.
`has_edge_whitespace = false` on all 6 → **H4 falsified**. The doc slugs are real, current
documents, and the ids are well-formed. Proceeded to Q1-b as instructed.

### 1.7.3 Q1-b — present in `extraction_items` at all?

```
exact_matches = 0  and  loose_matches = 0   — for all 6 ids
```

Both zero → not a normalization bug (**H4 falsified again, independently**), and the id is
absent from `extraction_items` **under any spelling**. Per §1.6's reading this leaves "H1 or
H2" — but H1 is already falsified by Q1-a, and H2 requires the row to *exist* with a NULL
column. It does not exist. **The decision tree as written runs out here**, which is itself a
finding (§1.8.3).

### 1.7.4 Beyond the script — where the ids actually went

The 6 ids carry the slugs `doc-george-phillips-response-to-discovery` and
`doc-cfs-interrogatory-response-08-08-16`. Both slugs are present in `extraction_items` in
volume (131 and 107 lineage rows respectively). So the *document* matches and only the
8-char content hash differs. That points at a regeneration, and the run table confirms it
(**measured**):

```
 run_id | document_id                                | items | evidence | started_at
--------+--------------------------------------------+-------+----------+------------------------
    113 | doc-awad-v-catholic-family-complaint-11-1-13|   149 |        0 | 2026-05-27 18:22:45+00
    114 | (same doc, 2nd pass)                        |     0 |        0 | 2026-05-27 18:29:01+00
    139 | doc-george-phillips-response-to-discovery   |   161 |      131 | 2026-07-24 13:50:12+00
    141 | doc-cfs-interrogatory-response-08-08-16     |   131 |      107 | 2026-07-24 19:04:18+00
    143 | doc-court-of-appeals-rulling-01-12-2012     |    51 |       41 | 2026-07-24 19:32:10+00
    145 | doc-judge-tighe-opinion-and-order-041212    |    79 |       64 | 2026-07-24 19:42:26+00
    147 | doc-sabrina-morris-affidavit                |    33 |       27 | 2026-07-24 19:52:51+00
    149 | doc-jeffrey-humphrey-affidavit              |    32 |       26 | 2026-07-24 19:57:33+00
    151 | doc-certified-letter-to-george-phillips…    |    75 |       55 | 2026-07-24 20:08:23+00
    155 | doc-hearing-to-approve-plan-for-admin…      |   138 |      123 | 2026-07-25 12:07:49+00
```
*(18 runs total, ids 113–156; even-numbered companions are the 0-item pass-2 runs.)*

Two facts decide Q1:

1. **The runs that produced today's Evidence for those two documents started 2026-07-24** —
   *after* every one of the 6 refs was tagged (2026-07-07 and 2026-07-19).
2. **Run ids jump 114 → 139.** Runs 115–138 are gone. Their `extraction_items` went with
   them (FK `extraction_items_run_id_fkey`). That deleted generation is what the refs were
   tagged against.

### 1.7.5 Q1-d — are the 6 nodes alive in Neo4j?

```cypher
MATCH (e:Evidence) WHERE e.id IN [ …the 6 ids… ] RETURN …
```
```
(0 rows)
```

**Zero of six.** Per §1.6's pre-stated reading — *"Fewer than 6 rows → H3: the missing ids
are dead pointers"* — **H3 is confirmed, at the maximum strength the test allows.**

Control, to prove the query mechanism was sound rather than silently matching nothing
(**measured**):
```
control_id                                                    grounding_status  source_document
"doc-george-phillips-response-to-discovery:evidence:92ee6f59" "normalized"      "doc-george-phillips-response-to-discovery"
```
A live id from the same document, same label, same query form, returns a row. The empty
result for the 6 is a real absence, not a broken query.

### 1.7.6 Q1-e — the shape census in the graph

```
 family_a_shape | has_grounding_status |  n
----------------+----------------------+-----
 TRUE           | TRUE                 | 525
```

One bucket. **All 525 Evidence nodes are Family A with `grounding_status` set. There are
zero Family-B nodes in the DEV graph.** The `POST /api/admin/evidence` route is real code
(§1.3) but has never been used here — so the entire Family-A/Family-B framing, while
correct about the code, was irrelevant to this defect.

Exact set comparison, Evidence ids in Neo4j vs `extraction_items.neo4j_node_id`
(**measured**, 525 ids pulled from each side and compared):

```
graph = 525   pg = 525   in-graph-only = 0   in-pg-only = 0   in-both = 525
```

**Perfect 1:1.** The current generation is fully consistent across the two stores. There is
no drift, no orphan, no unjoinable live node.

### 1.7.7 The decisive measurement — the join key works

All refs, by status, against the same join predicate the question asked about:

```
   status  | refs | joinable
-----------+------+----------
 undecided |   17 |        0
 included  |    6 |        0
 dropped   |    3 |        0
```

**All 26 refs are non-joinable, not just the 6.** The zero-row result was never specific to
`status = 'included'`.

Now the same predicate against the append-only candidate pool, split by when the pool member
was first assigned an ordinal:

```
 joinable |  n  |           earliest            |            latest
----------+-----+-------------------------------+-------------------------------
 f        |  94 | 2026-07-19 20:30:35.943274+00 | 2026-07-19 20:30:35.943274+00
 t        | 296 | 2026-07-27 16:48:36.398796+00 | 2026-07-27 19:42:55.816206+00
```

This is the cleanest result in the report. **296 of 390 pool members join successfully.**
The split is perfect and it is purely chronological: everything assigned on 2026-07-19
(before the re-extraction) is dead; everything assigned on 2026-07-27 (after it) is alive.
The re-extraction on 2026-07-24/25 falls exactly in the gap, with no straddling rows.

And all 26 refs sit in the dead 94 (**measured**: `refs_total = 26`, `in_ordinals = 26` —
every ref id is a known pool member, so these are genuinely retired candidates, not foreign
or malformed ids).

---

## 1.8 Q1 CONCLUSION (measured)

**The join predicate is correct. It returns zero rows because every row it was asked about
points at a generation of Evidence nodes that no longer exists.**

`scenario_fact_refs.graph_node_id = extraction_items.neo4j_node_id` is the right key, joining
the right two columns, in the right database. It demonstrably works: **296 of 390 candidate
pool members join today.** Nothing about format, prefix, casing, type, database, `elementId`
vs `id`, or provenance family is wrong.

What happened is a **stale-pointer / generation-skew defect**:

1. Between 2026-07-07 and 2026-07-19, a human ruled on 9 candidates (6 include, 3 drop) and
   a scan/merge wrote 17 `undecided` rows, all against the Evidence generation then in the
   graph — the one produced by extraction runs 115–138.
2. On **2026-07-24/25** those documents were re-extracted. Runs 115–138 and their
   `extraction_items` were deleted; runs 139–156 replaced them. Because a Family-A id is
   `{doc_slug}:evidence:{sha256(item_data)[..8]}`, re-extracting the same document with any
   change in extracted content yields a **different id for the same underlying fact**. The
   old Evidence nodes went away; new ones appeared under new ids.
3. `scenario_fact_refs.graph_node_id` **has no foreign key and cannot have one** — it points
   into Neo4j (`20260626122424_create_scenario_fact_refs_table.sql`: *"Plain TEXT, NO foreign
   key"*). So all 26 rulings survived the deletion of everything they referred to.
4. The result: 26 refs, 100% dead. Six of them are **human curation decisions that have been
   silently orphaned** — the most expensive data in the table.

This is exactly the case the codebase already anticipated in the read path
(`backend/src/api/scenario_facts.rs:129` — *"A `scenario_fact_refs` row is a pointer into
Neo4j; if that Evidence node is [gone] …"* → `content: null`) and in the ordinals module
(`scenario_candidate_ordinals.rs:28-31` — *"retired duplicates leave holes"*). The
anticipation is correct; what is missing is any **observable** that fires when the hole
opens.

### 1.8.1 Consequences

- **§1.5's two branches are both moot.** The Family-B branch (a) and the NULL-backfill branch
  (b) each answered a question that turned out not to be the question. No join predicate
  change and no `stable_entity_id` recomputation would recover these 26 rows — recomputation
  needs an `extraction_items` row to recompute *from*, and those rows were deleted.
- **The 6 human rulings are unrecoverable by id.** The old ids cannot be mapped to new ones
  mechanically, because the id *is* a hash of the content that changed. Recovering the
  curation would mean matching old→new by verbatim quote or page, and the old side no longer
  exists in either store. **Inferred:** the rulings are lost, and re-curation against the
  current pool is the only path. I did not attempt any recovery — read-only task.
- **The readiness slice must treat a dead pointer as its own state.** Per Standing Rule 1,
  "ruled included, target missing" is operationally distinct from "ruled included, target
  grounded" and from "never ruled". Today all three can collapse into a zero-row join.
  That collapse is the actual bug to fix in the slice.

### 1.8.2 The gap this exposes (not fixed — read-only)

Re-extraction deletes `extraction_items` and their graph nodes, but **nothing notices that
`scenario_fact_refs` and `scenario_candidate_ordinals` still point at them.** No log line, no
count, no failed constraint, no warning in the UI. A curator's include/drop decisions can be
invalidated wholesale by a routine re-run and the only symptom is a query that quietly
returns fewer rows than expected — which is precisely how this investigation started.

That is a Standing-Rule-1 violation at the system level: an operationally distinct state
(*"your rulings now reference nothing"*) produces no observable. **Flagged, not fixed.** The
smallest honest fix is a post-ingest reconciliation count — *"N scenario_fact_refs rows now
reference Evidence nodes that no longer exist"* — logged at `warn` and surfaced in the
scenario view. Whether to go further (remap, archive, or block re-extraction on curated
documents) is a design decision and outside this task.

### 1.8.3 Why the code-only ranking was wrong — the reusable lesson

The first-session ranking put H1 first because it was the only hypothesis that made zero rows
the *predicted* outcome rather than an anomaly, and because it was the one thing code alone
could prove was possible. That reasoning was sound and the conclusion was still wrong.

The defect is **temporal**, and code at HEAD carries no information about *when* rows were
written relative to each other. Every static fact in §1.1–§1.3 remains accurate — it just
could not see the 2026-07-24 re-extraction, which is the whole cause. The tell that would
have raised H3 was available and was not used: `scenario_fact_refs.tagged_at` is right there
in Q1-a's own projection, and **§1.6 never compares it to anything.** No query in the script
looks at `extraction_runs.started_at` at all.

Concretely, the decision tree in §1.6 has a hole: Q1-b's reading is *"both 0 → H1 or H2"*,
but the measured state (`exact = 0, loose = 0`, Family-A shape, row absent entirely) is
**neither**, and the script offers no next step. Q1-d rescued it only because it was run
anyway. **For the next diagnostic of this kind: when a pointer table has no FK, check the
timestamps against the referent's write history before ranking anything structural.**

---

## 2. Q2 — WHY DO 17 UNDECIDED ROWS EXIST?

**Short answer: they do not violate the ratified contract. The contract was misquoted.**

### 2.1 The contract says an implication, not a biconditional

The `scenario_gather` module doc states:

> *"A pool node with NO ref row is `Undecided` — persisted nowhere. There is no
> `upsert_fact_ref` and no fact-ref `INSERT` on this path; only 1a.3's include/drop ever
> writes a ruling."*
> — `backend/src/api/scenario_gather.rs:15-17`

That is **no row ⇒ undecided**, and it is a statement about *this route*, whose scope is
fixed by the sentence that follows ("on this path"). It does not say **undecided ⇒ no row**.
Reading it as a biconditional is what makes the 17 rows look anomalous.

The authoritative, converse-inclusive statement of the contract is in the ordinals module,
written by the same commit that introduced the current merge model:

> *"`scenario_fact_refs` is **derive-on-read**: a row exists there if and only if a candidate
> has been ruled on (include/drop) **or scored by a merge**."*
> — `backend/src/repositories/pipeline_repository/scenario_candidate_ordinals.rs:7-9`
> (emphasis added; commit `31f43b3`)

"**or scored by a merge**" is the clause that accounts for the 17 rows. A merged, not-yet-ruled
candidate is *supposed* to have an `undecided` row.

### 2.2 Every writer of an `undecided` row — complete enumeration

There are exactly two functions that can `INSERT`/`UPDATE` `scenario_fact_refs`
(`upsert_fact_ref` and `merge_scan_run_into_scenario`, both in
`backend/src/repositories/pipeline_repository/scenario_store.rs`), plus one historic
migration. That gives four ways an `undecided` row can exist:

| # | Producer | Live today? | Fingerprint on the row |
|---|---|---|---|
| **W1** | **Merge** — `merge_scan_run_into_scenario`, `MERGE_SCAN_RUN_SQL` binds `$2 = FactStatus::Undecided.code()` on INSERT | **YES** | `source_run_id` **NOT NULL**; `confidence` NOT NULL; `role_in_this_scenario` NOT NULL; `note` NULL |
| **W2** | **Un-drop** — `apply_fact_action` with `FactAction::Undrop` → `action_to_status` → `FactStatus::Undecided` | **YES** | `role`, `note`, `confidence` all **NULL** (the route passes `None` for all three); `source_run_id` **preserved** from whatever was there |
| **W3** | **Migration backfill** — `confirmed = FALSE → 'undecided'` | historic, one-off (2026-07-06) | `source_run_id` NULL; `confidence` NULL; `tagged_at` **< 2026-07-06** |
| **W4** | **Retired scan write** — the Theme Scan used to write every RELEVANT verdict as `undecided` at scan time | **NO** — removed 2026-07-19 | `source_run_id` **NULL** (predates the column) but `confidence`/`role` **NOT NULL** — the giveaway combination |

Provenance for each:

- **W1** — `scenario_store.rs:496-508` (`MERGE_SCAN_RUN_SQL`, the `SELECT $1, v.graph_node_id,
  v.proposed_role, $2, v.confidence, $3` projection) and `:566-573` (the call binding
  `FactStatus::Undecided.code()` as `$2`). Confirmed by its own doc comment at `:519-521`:
  *"the picks that landed as `undecided` suggestions (new, or an existing `undecided` row
  refreshed)."* **This is the answer to "does anything still write undecided rows today" — yes.**
- **W2** — `backend/src/api/scenario_facts.rs:284-290` (`action_to_status`: `Undrop =>
  FactStatus::Undecided`), called at `:346` and written at `:351-359` with `None` for role,
  note, and confidence. Pinned by test `action_to_status_maps_each_verb_to_its_state`
  (`scenario_facts.rs:578-585`), whose own comment says *"`undrop` returns a candidate to the
  pool as `Undecided`, NOT `Included`"*.
- **W3** — `backend/pipeline_migrations/20260706162558_replace_confirmed_with_status_on_scenario_fact_refs.sql`:
  `SET status = CASE WHEN confirmed THEN 'included' ELSE 'undecided' END`, with the header's
  explicit mapping *"`confirmed = FALSE` → `'undecided'` (a Theme Scan suggestion awaiting
  review)"*.
- **W4** — removed by commit `31f43b3` (2026-07-19, *"feat(scenario): unify the merge model on
  one pick-keyed write path"*), whose message states: *"remove the scan's direct write into
  `scenario_fact_refs` (`maybe_write_relevant` / `write_relevant`)"* and *"delete the now-dead
  `reconcile_fact_ref` + `RECONCILE_FACT_REF_SQL`."* The current code asserts the absence:
  `backend/src/services/theme_scan_persist.rs:11` — *"This module deliberately does NOT write
  `scenario_fact_refs`"* — with a no-live-DB regression test at
  `backend/src/services/theme_scan_persist_tests.rs:156-178`.

### 2.3 A stale doc comment that will mislead the next reader

`backend/src/services/theme_scan.rs:11` still describes the W4 behavior:

> *"3. writes each RELEVANT verdict to `scenario_fact_refs` as an `undecided` …"*

This is **false as of `31f43b3`** and directly contradicts `theme_scan_persist.rs:11` two
modules away. It is documentation only — no behavior depends on it — but it is exactly the
kind of line that would send the next investigation down the wrong path, and it is plausibly
part of why the 17 rows read as unexplained. Flagged, not fixed (read-only task).

### 2.4 What the 17 rows do to a computation that assumes "undecided ⇒ no row"

Concretely, for each assumption a consumer might make:

**Derived status — no effect.** `reconcile_candidates` is driven by the **graph pool**, and
emits exactly one `CandidateDto` per live pool node
(`backend/src/api/scenario_gather.rs:126-140`). A ref row is consulted via `index.get()`; a
hit returns the stored status, a miss returns `Undecided`. An `undecided` row and no row
produce the **same** output. **No double-count, no misclassification** — the derive-on-read
design already absorbs this.

**Counting from `scenario_fact_refs` alone — undercount, not double-count.** Any
`SELECT count(*) … WHERE status='undecided'` returns 17, but the *true* undecided population
is every pool member without an `included`/`dropped` row. The 17 are the subset that has been
merge-scored. A readiness metric computed as `included / (included + undecided + dropped)`
= 6/26 would be wrong in the denominator: the real pool is the graph pool, which is almost
certainly larger. **This is the actual failure mode to guard against**, and it is the opposite
of the double-count the question anticipated.

**Re-scan eligibility — correct as-is.** The scan judges only the undecided remainder
(`20260706162558…sql` header: *"the Theme Scan later judges only the undecided remainder"*).
A merged-but-unruled candidate genuinely is still awaiting a human ruling, so its presence in
that set is right.

**Merge idempotency — protected.** `MERGE_SCAN_RUN_SQL`'s `ON CONFLICT … WHERE
scenario_fact_refs.status = $2` (`scenario_store.rs:508`) confines updates to rows that are
*already* `undecided`, so re-merging refreshes a suggestion and never resurrects a curated
row. `rows_affected` deliberately excludes the skipped ones (`:519-523`).

**Provenance freeze — intact.** `source_run_id` is written only inside that undecided-gated
tail, so once a human includes or drops, the attributing run is frozen
(`scenario_store.rs:384-390`).

### 2.5 Attribute the 17 rows to their writer — MEASURED

> **MEASURED 2026-07-29.** These queries were run alongside the §1.6 set. Results are
> inline below each query. They **confirm §2's code-only conclusion in full**: the 17 rows
> are contract-compliant, and both surviving buckets are accounted for by known writers.

Read-only, `colossus_legal_v2`. The `writer` column implements the fingerprint table in §2.2
directly.

```sql
SELECT CASE
         WHEN source_run_id IS NOT NULL                        THEN 'W1 merge (live)'
         WHEN confidence IS NOT NULL
           OR role_in_this_scenario IS NOT NULL                THEN 'W4 retired scan write (pre-31f43b3)'
         WHEN tagged_at < TIMESTAMPTZ '2026-07-06'             THEN 'W3 migration backfill'
         ELSE                                                       'W2 undrop, or W3 post-dated'
       END                                    AS writer,
       count(*)                               AS n,
       min(tagged_at)                         AS earliest,
       max(tagged_at)                         AS latest
FROM scenario_fact_refs
WHERE status = 'undecided'
GROUP BY 1
ORDER BY n DESC;
```
**Measured:**
```
               writer                | n  |           earliest            |            latest
-------------------------------------+----+-------------------------------+-------------------------------
 W4 retired scan write (pre-31f43b3) | 16 | 2026-07-19 15:32:23.770553+00 | 2026-07-19 15:32:23.770553+00
 W1 merge (live)                     |  1 | 2026-07-19 20:38:34.993907+00 | 2026-07-19 20:38:34.993907+00
```

> **16 W4 + 1 W1 = 17.** No W2 (un-drop) and no W3 (migration backfill) rows survive. The 16
> W4 rows were written in a single batch at 15:32:23 on 2026-07-19 — one scan, before the
> scan's direct write was removed in `31f43b3` later that same day. The single W1 row was
> written five hours later at 20:38:34 by `merge_scan_run_into_scenario`, the live writer.
>
> This is a **clean confirmation of §2.2's fingerprint table**: every one of the 17 rows falls
> into a bucket the code-only enumeration predicted, with timestamps consistent with the
> commit history. Q2 needed no revision.
>
> Note the interaction with §1.8: these 17 were tagged on 2026-07-19, so they are part of the
> 94 dead pool members. They are contract-compliant *and* dead pointers — two independent
> facts about the same rows.

Row-level detail, if the buckets need disambiguating:
```sql
SELECT scenario_id, graph_node_id, role_in_this_scenario,
       confidence, source_run_id, note IS NOT NULL AS has_note, tagged_at
FROM scenario_fact_refs
WHERE status = 'undecided'
ORDER BY tagged_at;
```

And the denominator check that §2.4 flags as the real risk — compare the ref-table total
against the actual candidate pool:
```sql
SELECT s.scenario_id,
       count(*) FILTER (WHERE r.status = 'included')  AS included,
       count(*) FILTER (WHERE r.status = 'undecided') AS undecided,
       count(*) FILTER (WHERE r.status = 'dropped')   AS dropped,
       count(r.*)                                     AS rows_total,
       (SELECT count(*) FROM scenario_candidate_ordinals o
         WHERE o.scenario_id = s.scenario_id)         AS pool_size_seen
FROM scenarios s
LEFT JOIN scenario_fact_refs r ON r.scenario_id = s.scenario_id
GROUP BY s.scenario_id
ORDER BY rows_total DESC;
```
**Measured:**
```
             scenario_id              | included | undecided | dropped | rows_total | pool_size_seen
--------------------------------------+----------+-----------+---------+------------+----------------
 797bc26b-2831-4218-9dea-a4eb12865204 |        6 |        17 |       3 |         26 |            242
 259ca9a8-abe2-4a7f-9eae-7b26fe2582fd |        0 |         0 |       0 |          0 |            148
```

> `pool_size_seen` ≫ `rows_total` is the **expected and correct** shape under derive-on-read.
> It is the number any readiness percentage must use as its denominator — not `rows_total`.
> (`scenario_candidate_ordinals` is append-only and covers every pool member ever seen:
> `scenario_candidate_ordinals.rs:14-31`.)
>
> **Measured confirmation, and it is worse than §2.4 estimated.** The naive ratio
> `included / rows_total` = 6/26 = **23%**. Against the pool it is 6/242 = **2.5%** — an
> order of magnitude apart. §2.4's warning is correct and the magnitude is large.
>
> **But neither number is usable**, because of §1.8: all 26 refs are dead pointers and 94 of
> the 242 pool members are retired. The only honest readiness figure for this scenario today
> is **0 valid inclusions out of 148 live pool members** — and the second scenario
> (`259ca9a8…`, 148 pool members, zero rulings) shows what an untouched scenario looks like
> for comparison. Any denominator taken from `scenario_candidate_ordinals` **must** be
> filtered to pool members that still resolve, or it inherits the same staleness.
>
> Per-scenario live/dead split of the pool (**measured**, so this is not inferred from the
> 390/296/94 totals):
> ```
>              scenario_id              | pool_total | live | dead
> --------------------------------------+------------+------+------
>  797bc26b-2831-4218-9dea-a4eb12865204 |        242 |  148 |   94
>  259ca9a8-abe2-4a7f-9eae-7b26fe2582fd |        148 |  148 |    0
> ```
> All 94 dead pool members belong to the curated scenario; the second is entirely live and
> entirely unruled. Both now see a 148-member live pool, consistent with a single shared
> candidate population rebuilt on 2026-07-27.

### 2.6 Recommended disposition — recommendation only, no fix in this task

**Do not delete or migrate the 17 rows.** Whatever their bucket:

- **W1 rows are live, correct, load-bearing data.** They carry `source_run_id` and
  `confidence` — the provenance that drives the per-run "applied" state and the
  delete-restriction pre-check (`scenario_store.rs:376-390`;
  `repositories/pipeline_repository/scan_run_merges.rs:170`, `:214`). Deleting them would
  silently un-apply merges and corrupt the run-results view.
- **W4 rows (if any) are still meaningful** — a real model judgment awaiting a human ruling.
  Their only defect is a NULL `source_run_id`, i.e. lost attribution, not wrong state. If
  you want them attributable, backfill `source_run_id` from `scan_run_verdicts` by
  `(scenario_id, graph_node_id)`; if you don't, leaving them NULL is honest ("no scan is
  recorded as having put this here"), which is exactly what the column's doc comment says a
  NULL means (`scenario_store.rs:380-383`).
- **W3 rows are historically accurate** — they encode the pre-2026-07-06 `confirmed = FALSE`
  state faithfully.

**What I would fix instead, in order:**

1. **Correct the stale comment** at `theme_scan.rs:11` (§2.3). Zero risk, removes a live
   contradiction between two modules. Smallest possible change with the largest effect on
   the next reader.
2. **Tighten the contract sentence** at `scenario_gather.rs:15-17` so it states the
   implication direction explicitly, matching the accurate phrasing already at
   `scenario_candidate_ordinals.rs:7-9`. This is the sentence that generated tonight's
   question; leaving it as-is guarantees the question recurs.
3. **Pin the merge-writes-undecided invariant with a test.** `merge_scan_run_into_scenario`
   writing `undecided` is currently asserted only by SQL-shape tests
   (`scenario_store.rs:921-995`). A named test — *"a merge creates undecided rows, and that
   is contract-compliant"* — converts this diagnostic into a permanent artifact and satisfies
   Rule 21's spirit (invariants that review alone misses).
4. **Whatever consumes these counts must take its denominator from the pool**
   (`scenario_candidate_ordinals`, or the live graph pool), never from
   `count(scenario_fact_refs)`. §2.4 shows this is the one place the 17 rows can actually
   produce a wrong number.

None of these are in scope for this task.

---

## 3. SUMMARY

> **This section was rewritten 2026-07-29 after the queries were run.** The superseded
> code-only Q1 summary is preserved at §3.1 for comparison.

**Q1 — why doesn't the join match? (MEASURED)**
**The join predicate is correct and works.** 296 of 390 candidate pool members join today
(§1.7.7). The zero-row result is a **stale-pointer defect, not a key defect**: all 26
`scenario_fact_refs` rows — the 6 `included`, all 17 `undecided`, and all 3 `dropped` — were
tagged on 2026-07-07 and 2026-07-19, and the documents they reference were **re-extracted on
2026-07-24/25**. That re-extraction deleted runs 115–138 and their `extraction_items`, and
replaced the Evidence nodes with new ids — because a Family-A id embeds
`sha256(item_data)[..8]`, so the same fact re-extracted gets a different id. `graph_node_id`
has no FK (it cannot; it points into Neo4j), so all 26 rulings outlived their targets.
Measured: **0 of 6 ids resolve to a live Evidence node** (§1.7.5, control query confirms the
lookup works), **0 of 6 appear in `extraction_items` under any spelling** (§1.7.3), and the
live/dead split of the pool is perfectly chronological around the re-extraction (§1.7.7).

The code-only ranking was **wrong**: H1 (Family-B admin-authored Evidence) is falsified —
there are **zero Family-B nodes in the DEV graph**; all 525 Evidence nodes are Family A with
`grounding_status` set, in exact 1:1 correspondence with the 525 non-NULL lineage rows
(§1.7.6). H2 and H4 are falsified too. **H3 — the secondary hypothesis — is confirmed.** The
static analysis in §1.1–§1.3 remains accurate; it simply could not see a temporal cause, and
§1.6's own decision tree had no branch for the state that actually obtains (§1.8.3).

**The finding worth acting on** (§1.8.2): re-extraction silently orphans curated rulings.
Six human include decisions were invalidated by a routine re-run with **no log line, no
count, no warning** — a Standing-Rule-1 gap at the system level. Flagged, not fixed.

### 3.1 Superseded — the code-only Q1 summary (first session, before measurement)

*Preserved verbatim. Its structural claims hold; its conclusion about which family the 6 refs
belong to was overturned by §1.7.*

> Not a format, prefix, casing, type, or cross-database problem: on the pipeline path both
> columns receive the *same string from the same function call*, so they cannot disagree
> (§1.2). The join fails because `extraction_items.neo4j_node_id` is a **lineage column on
> pipeline extraction rows, not a registry of graph nodes** — and there is a second, entirely
> separate Evidence provenance family (`POST /api/admin/evidence`,
> `admin_evidence.rs:157`) that creates graph nodes with caller-supplied ids and **writes no
> Postgres row at all** (§1.3). For that family no join can work, and the missing thing is the
> row, not the key. Which family the 6 `included` refs belong to is decided by **Q1-a** and
> **Q1-c** in §1.6; `evidence_items = 0` in Q1-c closes it outright. I could not run them —
> see §0 — and I have not asserted a working join I did not execute.
>
> *(Measured outcome: `evidence_items = 574`, not 0 — the pre-stated closing condition never
> fired. The second provenance family exists in code but has never been used in this
> environment.)*

**Q2 — why do 17 undecided rows exist?**
Answered fully from code. They are **contract-compliant**, not a violation. The ratified
contract is *"a row exists if and only if a candidate has been ruled on (include/drop) **or
scored by a merge**"* (`scenario_candidate_ordinals.rs:7-9`); the `scenario_gather.rs:15`
sentence states only the "no row ⇒ undecided" direction and was read as a biconditional.
Something absolutely still writes undecided rows today: **`merge_scan_run_into_scenario`**
binds `FactStatus::Undecided` on every merged pick (`scenario_store.rs:496-508`, `:566-573`),
and **un-drop** does too (`scenario_facts.rs:288`). The retired scan-time writer was removed
on 2026-07-19 in `31f43b3` — but `theme_scan.rs:11` still documents the removed behavior
(§2.3), which is itself a plausible source of the confusion. Effect on computations: **no
double-count and no misclassification** in derived status (the pool drives the output, and a
miss and an `undecided` row produce identical results). The real hazard is the **opposite**
one — any readiness ratio using `count(scenario_fact_refs)` as its denominator will
undercount the pool (§2.4). Disposition: **keep all 17**; fix the two stale/ambiguous
comments and pin the invariant with a test (§2.6).

**Q2 — measured confirmation.** The attribution query was run (§2.5). Result: **16 W4 +
1 W1 = 17**, no W2 and no W3 rows surviving. The 16 W4 rows landed in one batch at
2026-07-19 15:32:23 (a scan running hours before `31f43b3` removed that writer the same day);
the 1 W1 row at 20:38:34 the same evening. Every row falls in a bucket §2.2's code-only
fingerprint table predicted. **Q2 needed no revision.** The §2.4 denominator warning is also
confirmed and is large: naive `6/26` = 23% vs pool-based `6/242` = 2.5%. Given §1.8, neither
is usable — the honest figure is 0 valid inclusions against a 148-member live pool.

**Blocked:** nothing. SSH was unblocked and every query in §1.6 and §2.5 was executed
read-only against DEV on 2026-07-29. Access path recorded in §0.1.

**Files read in the first (code-only) session:** `backend/pipeline_migrations/{20260626122424,20260706112745,20260706162558,20260421212806}*.sql`,
`backend/src/repositories/pipeline_repository/{scenario_store,scenario_candidate_ordinals,extraction_items}.rs`,
`backend/src/api/{scenario_gather,scenario_facts,admin_evidence}.rs`,
`backend/src/api/pipeline/ingest_helpers.rs`, `backend/src/pipeline/steps/{ingest,llm_extract_pass2}.rs`,
`backend/src/bias/{queries,repository}.rs`, `backend/src/services/{theme_scan,theme_scan_persist}.rs`,
`.claude/settings.local.json`. Git history: `31f43b3`, `cd4c44b`.

**Queried in the second (empirical) session — all read-only:**
`colossus_legal_v2` @ `10.10.100.200` — `scenario_fact_refs`, `extraction_items`,
`extraction_runs`, `scenario_candidate_ordinals`, `scenarios`; plus `\d` on
`extraction_items` and `scenario_candidate_ordinals`. Neo4j @ `bolt://10.10.100.200:7687` —
`MATCH (e:Evidence)` only. **No `INSERT`/`UPDATE`/`DELETE`/DDL, no temp tables, no Cypher
write clause was issued.** No code read or changed in this session.

**Files written:** this report only. No code, no commits, no DB writes.
