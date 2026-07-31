# ENTITY SOURCES DIAGNOSTIC — read and report

**Repo:** `colossus-legal` · **Branch:** `claude/entity-sources-diagnostic-qeh5mt`
**HEAD:** `8d15bf0 merge: scenario-1a arc through v2.0.0-beta.363 into main`
**Date:** 2026-07-31
**Mode:** READ-AND-REPORT ONLY. No code changed, no data changed, no fixes, no migrations
proposed. The only file written is this report.

---

## 0. METHOD AND EVIDENCE LABELS

Every claim below carries one of four labels.

| Label | Means |
|---|---|
| **measured (code)** | I read it directly out of a file in this repo at HEAD. File and line are cited. This is authoritative about *what the code does*, not about *what is currently in DEV*. |
| **measured (live)** | Came out of a query executed against the live DEV Neo4j / Postgres this session. **There are ZERO such findings in this report — see §0.1.** |
| **reported** | Asserted by a doc comment, a commit message, or a prior CC report. Plausible and cited, but not independently verified by me. |
| **inferred** | My reasoning past the evidence. |

### 0.1 The live half could NOT be run — and why

**measured (code/environment).** This session runs in a **remote cloud container**, not on
Roman's homelab network. The DEV hosts are on RFC1918 space that this container has no route
to, and the container has no `ssh`, no `ping`, no `psql`, no `cypher-shell`.

Probes run this session:

```
TCP 10.10.100.200:7687 (Neo4j DEV) → unreachable
TCP 10.10.100.200:5432 (Postgres DEV) → unreachable
TCP 10.10.100.200:6333 (Qdrant DEV) → unreachable
`ssh`  → "No such file or directory" (binary not installed)
`ping` → "No such file or directory" (binary not installed)
```

The environment's egress proxy explicitly excludes `10.0.0.0/8` from proxying (`no_proxy`),
so private-range traffic is not tunnelled either.

This is a **different blocker** from the one recorded in the three earlier reports.
`CC-REPORTS/scenario_refs_diagnostic.md:39-45` records a `.claude/settings.local.json`
`permissions.deny` entry `"Bash(ssh*)"` as the historic cause. That file **does not exist on
this branch** — `.claude/` contains only `agents/`. The block today is pure network topology,
which no permission grant can lift.

**Consequence for the deliverable:** Section 2 (MEASURED DUPLICATION) **cannot be answered
with live counts by this session.** Rather than guess, §2 delivers the thing that makes the
measurement a 20-minute job for whoever has DB access: the **exact query set, derived from
the code's actual write paths** (§4), with the property and label names each query reads
justified by the writer that sets them. Sections 1, 3 and 4 are answered in full — they are
questions about code, and the code is here.

The access path a prior session used, for whoever runs §2's queries
(**reported**, from `CC-REPORTS/scenario_refs_diagnostic.md:26-34`):

- Postgres — `ssh core@10.10.100.200` → `sudo podman exec -i colossus-postgres psql -U postgres -d colossus_legal_v2`
- Neo4j — same host → `sudo podman exec -i colossus-neo4j cypher-shell`
- The app host `10.10.100.220` carries neither client; the DB host `10.10.100.200` hosts both engines as containers.

---

## 1. SOURCES — every place a list of people/entities is produced

### 1.1 The short version

**measured (code).** There are **five physically distinct stores** that hold "who the people
are", and **nine distinct read paths** that produce a list of them. No two stores are
synchronised by any code in this repo.

| # | Store | Physical location | Populated by |
|---|---|---|---|
| S1 | Neo4j `:Person` / `:Organization` nodes | Neo4j (DEV `10.10.100.200:7687`) | Pipeline Ingest step (§4.1) |
| S2 | Postgres `parties` table | **`colossus_legal`** (main pool, `state.pg_pool`) | Hand-loaded case metadata; no writer in this repo |
| S3 | Postgres `extraction_items` rows with effective type Party/Person/Organization | **`colossus_legal_v2`** (pipeline pool, `state.pipeline_pool`) | LLM pass-1 extraction (§4.1 step 1) |
| S4 | Postgres `authored_entities` | **`colossus_legal_v2`** | Canonical-element loader (Elements/Counts only — **no persons today**) |
| S5 | Qdrant vectors for `Person` / `Organization` nodes | Qdrant DEV `:6334` | Embedding job reading S1 |

Note the split-brain risk baked into the topology: **S2 lives in a different database than
S3/S4** (`colossus_legal` vs `colossus_legal_v2`), and **S1 is a different engine entirely**.

### 1.2 The nine read paths

**measured (code).** Every row below was read at HEAD; the "Source & filter" column is the
actual query, not a paraphrase.

| # | Endpoint / read | Code | Store | Source & filter | UI surface that consumes it |
|---|---|---|---|---|---|
| R1 | `GET /api/bias/available-filters` → `actors[]` | `bias/queries.rs:39-53` (built), `bias/repository.rs:125-154` (executed) | S1 | `MATCH (e:Evidence)-[:STATED_BY]->(actor) WHERE e.pattern_tags IS NOT NULL AND e.pattern_tags <> ''` — **no label filter** on `actor` | Bias Explorer "Actor" dropdown (`BiasExplorerFilters.tsx:178`); **Scenario page "wielder / party" dropdown** (`ScenarioDefinitionForm.tsx:359`) |
| R2 | `GET /api/bias/available-filters` → `subjects[]` | `bias/queries.rs:61-75`, `repository.rs:167-190` | S1 | `MATCH (e:Evidence)-[:ABOUT]->(subject) WHERE e.pattern_tags IS NOT NULL AND e.pattern_tags <> ''` | Bias Explorer "About" dropdown (`BiasExplorerFilters.tsx:216`); **Scenario page "target" dropdown** (`ScenarioDefinitionForm.tsx:338`) |
| R3 | `GET /api/persons` | `repositories/person_repository.rs:48` | S1 | `colossus_graph::get_nodes_by_label(graph, "Person")` — **every Person node, ungated**; sorted by name in Rust | People page (`pages/People.tsx:115`), grouped by the node's `role` property |
| R4 | `GET /api/persons/:id/detail` | `repositories/person_detail_repository.rs:56-71` | S1 | `MATCH (e)-[:STATED_BY]->(p {id: $person_id})` — **label-agnostic on both ends** | Person detail page |
| R5 | `GET /api/case` → `parties{plaintiffs,defendants,other}` | `repositories/case_repository.rs:125` | S1 | `colossus_graph::get_nodes_with_property(graph, "role")` — **any node carrying a `role` property**, bucketed by `role` string in Rust | Case overview |
| R6 | `GET /api/case-summary` → plaintiff/defendant name lists | `repositories/case_summary_repository.rs:291` | S1 | same `get_nodes_with_property("role")`, but returns **bare name strings, no ids** | Case summary |
| R7 | `GET /api/cases/:slug` → `parties` | `repositories/case_header_repository.rs:100-102`, called at `api/case_header.rs:95` with `state.pg_pool` | **S2** | `SELECT party_id, name, role, entity_type, status, … FROM parties WHERE case_id = $1` | Home page `CaseHeader` caption (`components/CaseHeader.tsx:257-262`) |
| R8 | `GET /api/admin/pipeline/.../items` (review panel) | `repositories/pipeline_repository/review_items.rs:64-80` | **S3** | `SELECT … FROM extraction_items WHERE run_id = $1 …`; UI filters on `resolved_entity_type ?? entity_type` | Review panel + **"People & Links" tab** (`components/pipeline/PeopleLinksPanel.tsx:17,55`) |
| R9 | `GET /rebuttals` | `repositories/rebuttals_repository.rs:44` | S1 | `MATCH (e:Evidence)-[:STATED_BY]->(p:Person {name: 'George Phillips'}) WHERE e.id STARTS WITH 'evidence-phillips-coa-'` — **a person's name and an id prefix hardcoded in a `const`** | Decomposition / rebuttals view |

Two further entity-list producers that are not UI dropdowns but matter to the duplication story:

| # | Producer | Code | What it lists |
|---|---|---|---|
| R10 | Embedding job → Qdrant (S5) | `repositories/embedding_repository.rs:150-165` | `MATCH (p:Person) RETURN p.id, p.name, p.role, p.roles, p.description` — one vector **per duplicate node** |
| R11 | Pass-2 cross-document context (fed to the LLM) | `repositories/pipeline_repository/extraction_context.rs:260-274`, whitelist at `:86-96` | Party/Person/Organization rows from **other published documents'** pass-1 runs, injected into the pass-2 prompt as `ctx:<id>` |

### 1.3 Why the two Scenario-page dropdowns offer different lists

**measured (code).** This is the specific question, and it has a specific answer with three
independent causes stacked on top of each other.

Both dropdowns are rendered by the same component from a **single** API call:

```
frontend/src/components/ScenarioDefinitionForm.tsx:211
  Promise.all([getAvailableFilters(), getAllegations()])
    → setSubjects(filters.subjects);   // line 214
    → setActors(filters.actors);       // line 215
```

- **"Target" (who the attack is about)** renders `subjects` — `ScenarioDefinitionForm.tsx:338-342`.
- **"Party / wielder" (who wields the attack)** renders `actors` — `ScenarioDefinitionForm.tsx:359-363`.

`actors` and `subjects` are two different arrays computed by two different Cypher queries
(`bias/repository.rs:101-105` runs all three fetches under `tokio::try_join!`). They differ
for these reasons:

**Cause 1 — different edge, therefore different node set (by design).**
`actors` = `ABOUT`-agnostic, reached via `(e:Evidence)-[:STATED_BY]->(actor)`.
`subjects` = reached via `(e:Evidence)-[:ABOUT]->(subject)`.
`STATED_BY` is *the speaker* and `ABOUT` is *who the statement concerns* — deliberately
distinct relationships (`neo4j/schema.rs:73-80`, which spells out the domain note). A judge
who is spoken *about* but never quoted appears in `subjects` and not in `actors`; a witness
who is quoted but never discussed appears in `actors` and not in `subjects`. **This part is
correct behaviour, not a bug** — but it is invisible to the author, because the form labels
them "Target" and "Party" with no hint that they are drawn from different edges
(`components/scenarioFormLabels.ts:92,100`).

**Cause 2 — both lists are gated on bias tags, so most of the graph is invisible to both.**
Both queries carry `WHERE e.pattern_tags IS NOT NULL AND e.pattern_tags <> ''`. A person who
appears in fifty untagged Evidence nodes appears in **neither** dropdown. There is a
repo-wide test pinning this gate as intentional for the Bias Explorer
(`bias/queries.rs`, test `every_query_is_scoped_to_tagged_evidence`) — the Scenario form is a
**second consumer that inherited a scope designed for a different surface.**
*(inferred: this is why an author can fail to find a party they know exists.)*

**Cause 3 — duplicate Person nodes multiply into both lists independently.**
Both queries group by the *node*, not by the human. `MATCH … ->(actor) WITH actor, count(e)`
emits one row per node id. If "Judge Tighe", "Karen A. Tighe" and "Tighe" are three nodes,
they are three separate dropdown choices with three separate counts — **and the split is not
the same in the two dropdowns**, because which variant got a `STATED_BY` edge and which got
an `ABOUT` edge depends on which document each variant came from. That is the mechanism by
which the *same human* produces two different-looking choice lists in two dropdowns on one
page. *(inferred from the two queries + the write path in §4; not live-verified.)*

**Cause 4 — the `id` the form persists is the raw node id.**
`ScenarioDefinition.target` and `Wielder.party_id` are documented as
"Graph party node id (from `available-filters` actors), never free text"
(`dto/scenario_crud.rs:126,181`). So the author's choice of *which duplicate* is frozen into
the scenario row, and everything downstream keys off that one node id (§3.3).

### 1.4 Hardcoded lists and case-specific names in code

**measured (code).** Not a "list" in the dropdown sense, but they are places where a person's
identity is compiled in rather than configured:

| Location | What is hardcoded |
|---|---|
| `repositories/rebuttals_repository.rs:44` | `(p:Person {name: 'George Phillips'})` and `e.id STARTS WITH 'evidence-phillips-coa-'` in a `const` query |
| `repositories/rebuttals_repository.rs:23-25` | The whole `/rebuttals` endpoint is framed as "grouped by George's claims" |
| `config.rs:268` / `bias/repository.rs:502-533` | `CASE_DEFAULT_SUBJECT_NAME` — *correctly* configured, matched **case-sensitively and exactly** against `subject.name`. With duplicate variants present, the code already `warn!`s on both the no-match and multi-match cases (`bias/repository.rs:515,525`) and, on multi-match, silently picks the first by sort order |
| `frontend/public/data/timeline.json` | Person names in prose (`"Judge Tighe Appoints CFS as Guardian"`) — display-only, no ids, feeds no dropdown |

The `CASE_DEFAULT_SUBJECT_NAME` behaviour is worth flagging: **the duplicate-variant problem
already has a logging tripwire in production**, at `bias/repository.rs:523-531`. Whether it
has fired on DEV is a log question I cannot answer from here.

---

## 2. MEASURED DUPLICATION — **BLOCKED**, with the exact query set to run

**Status: not answered.** No live counts. See §0.1 for why. Nothing in this section is a
count; everything here is either the query to run, or a labelled non-measured statement.

### 2.1 What is known without the live DB

**reported** — from the doc comment on the resolver fix, `api/pipeline/ingest_resolver.rs:155-159`:

> "That is the mechanism behind the duplicate Person nodes in the graph (**"Judge Tighe",
> "Karen A. Tighe", "Tighe" and "Karen A." became four separate people**). Organizations
> were unaffected — `"organization"` matches on both sides."

So the four-variant Tighe split is **stated in the codebase as an observed fact**, and names
one more variant ("Karen A.") than the instruction's example. I have not verified it.

**measured (code)** — the ids those variants would carry are fully determined by
`ingest_helpers.rs:29-37` (`slug`) and `ingest_resolver.rs:363-366`:

| Stored name | Derived node id |
|---|---|
| `Judge Tighe` | `person-judge-tighe` |
| `Karen A. Tighe` | `person-karen-a-tighe` |
| `Tighe` | `person-tighe` |
| `Karen A.` | `person-karen-a` |

Corroborating **measured (code)** traces of at least two of these being real: `person-judge-tighe`
appears as a fixture id in `dto/scenario_crud.rs:289,362`, and `person-tighe` in
`frontend/src/components/__tests__/scenarioDefinitionGuard.test.ts:76,86`. Test fixtures are
weak evidence of production data, but they are the ids the code mints.

**measured (code)** — timeline that bounds what DEV can look like: the resolver fix landed in
commit `a56e9e7 "fix(ingest): party_name + aliases persist; person resolution actually runs"`,
dated **2026-07-20**, and is an ancestor of HEAD. Before it, person matching was inert
(§4.2). A prior report notes a re-extraction on **2026-07-24/25**
(`CC-REPORTS/scenario_refs_diagnostic.md:0`, **reported**). *(inferred: whether DEV's current
Person set was written before or after `a56e9e7` decides whether the duplicates on DEV are
historic residue or still being produced — and that is exactly what §2.2 Q1/Q6 settle.)*

### 2.2 The query set — derived from the write paths, not from assumed names

Every property name below is justified by the writer that sets it. **This is the constraint
the instruction imposed, so the justification is explicit for each.**

| Property read | Set by | Line |
|---|---|---|
| `n.id` | Party MERGE key, from `ResolvedParty.neo4j_id` or the `person-<slug>` fallback | `ingest_helpers.rs:301`, `:388-392` |
| `n.name` | `ON CREATE SET n.name = $name`, **never overwritten on match** | `ingest_helpers.rs:302` |
| `n.party_name` | `ON CREATE SET n.party_name = $name`; `ON MATCH SET n.party_name = coalesce(n.party_name, n.name)` — **added only in `a56e9e7`, so pre-fix nodes lack it** | `ingest_helpers.rs:302,305` |
| `n.aliases` (LIST) | `ON CREATE SET n.aliases = $aliases`; `ON MATCH` appends `$aliases` **plus the losing `$name`** — also new in `a56e9e7`, `NULL` on older nodes | `ingest_helpers.rs:303,306-308` |
| `n.role` | `ON CREATE SET n.role = $role` only — **stale on every subsequent document** | `ingest_helpers.rs:302` |
| `n.source_document` (scalar) | `ON CREATE` only — names the FIRST document to create the node | `ingest_helpers.rs:304` |
| `n.source_documents` (LIST) | appended by every document that references the node | `ingest_helpers.rs:304,309-312` |
| `e.speaker` (on Evidence) | generic property loop writes every schema property onto the node | `ingest_helpers.rs:538-586` |
| `e.pattern_tags` | same generic loop (comma-joined string) | `ingest_helpers.rs:538-586` |

Note `labels(n)[0]` is the house convention for "the label" everywhere in this codebase
(`repositories/case_health_repository.rs:25-33` explains why `db.labels()` is not used).

#### Q1 — how many person/entity records exist, per store

```cypher
// Neo4j (S1) — node counts by label
MATCH (n) WHERE labels(n)[0] IN ['Person','Organization']
RETURN labels(n)[0] AS label, count(n) AS nodes;
```

```cypher
// Neo4j (S1) — the full roster with everything the writers set.
// `party_name` NULL and `aliases` NULL both mean "written before a56e9e7".
MATCH (n) WHERE labels(n)[0] IN ['Person','Organization']
RETURN labels(n)[0]          AS label,
       n.id                  AS id,
       n.name                AS name,
       n.party_name          AS party_name,
       n.aliases             AS aliases,
       n.role                AS role,
       n.source_document     AS first_doc,
       size(coalesce(n.source_documents, [])) AS n_docs
ORDER BY label, n.name;
```

```sql
-- Postgres colossus_legal (S2) — the hand-loaded caption parties
SELECT party_id, name, role, entity_type, status, sort_order
FROM parties ORDER BY role, sort_order;
```

```sql
-- Postgres colossus_legal_v2 (S3) — every extracted party mention, pre-graph.
-- COALESCE mirrors extraction_context.rs:260-274 and review_items.rs exactly.
SELECT COALESCE(resolved_entity_type, entity_type) AS effective_type,
       item_data->'properties'->>'party_name'      AS party_name,
       item_data->'properties'->>'full_name'       AS full_name,
       item_data->'properties'->>'aliases'         AS aliases_raw,
       item_data->'properties'->>'party_type'      AS party_type,
       item_data->'properties'->>'role'            AS role,
       neo4j_node_id,
       document_id,
       review_status,
       count(*) AS mentions
FROM extraction_items
WHERE COALESCE(resolved_entity_type, entity_type) IN ('Party','Person','Organization')
GROUP BY 1,2,3,4,5,6,7,8,9
ORDER BY 2,8;
```

The `party_name` / `full_name` pair is not a guess: `create_party_nodes` reads exactly those
two keys in that precedence order (`ingest_helpers.rs:352-355`), and `party_type` /
`entity_kind` likewise (`:378-381`).

```sql
-- Postgres colossus_legal_v2 (S4) — does anything person-shaped live in authored_entities?
-- Expected empty: the canonical loader authors Elements/Counts/Theories only.
SELECT entity_type, count(*) FROM authored_entities GROUP BY 1 ORDER BY 1;
```

#### Q2 — group the same-human variants, with per-variant usage counts

```cypher
// Per-variant usage: STATED_BY in, ABOUT in, tagged-vs-untagged, total edges.
// The tagged split matters because BOTH scenario dropdowns are gated on
// pattern_tags (§1.3 Cause 2), so `tagged_*` is what the author actually sees.
MATCH (n) WHERE labels(n)[0] IN ['Person','Organization']
OPTIONAL MATCH (se:Evidence)-[:STATED_BY]->(n)
WITH n,
     count(DISTINCT se) AS stated_by_all,
     count(DISTINCT CASE WHEN se.pattern_tags IS NOT NULL AND se.pattern_tags <> ''
                         THEN se END) AS stated_by_tagged
OPTIONAL MATCH (ae:Evidence)-[:ABOUT]->(n)
WITH n, stated_by_all, stated_by_tagged,
     count(DISTINCT ae) AS about_all,
     count(DISTINCT CASE WHEN ae.pattern_tags IS NOT NULL AND ae.pattern_tags <> ''
                         THEN ae END) AS about_tagged
RETURN n.id AS id, n.name AS name, labels(n)[0] AS label,
       stated_by_all, stated_by_tagged,   // → appears in the WIELDER dropdown iff stated_by_tagged > 0
       about_all,     about_tagged,       // → appears in the TARGET  dropdown iff about_tagged  > 0
       size((n)--()) AS total_edges,
       n.aliases AS aliases
ORDER BY toLower(n.name);
```

```cypher
// Candidate same-human clusters, by shared surname token. Blunt on purpose —
// this is a REPORT aid, not a merge rule. Read the output, don't act on it.
MATCH (n) WHERE labels(n)[0] IN ['Person','Organization'] AND n.name IS NOT NULL
WITH n, [t IN split(toLower(n.name), ' ') WHERE size(t) > 3] AS tokens
UNWIND tokens AS token
WITH token, collect({id: n.id, name: n.name, label: labels(n)[0]}) AS variants
WHERE size(variants) > 1
RETURN token, size(variants) AS n_variants, variants
ORDER BY n_variants DESC, token;
```

Run the same clustering explicitly for the three families the instruction names:

```cypher
MATCH (n) WHERE labels(n)[0] IN ['Person','Organization']
  AND ( toLower(coalesce(n.name,''))       CONTAINS 'tighe'
     OR toLower(coalesce(n.name,''))       CONTAINS 'phillips'
     OR toLower(coalesce(n.name,''))       CONTAINS 'awad'
     OR toLower(coalesce(n.party_name,'')) CONTAINS 'tighe'
     OR ANY(a IN coalesce(n.aliases, []) WHERE toLower(a) CONTAINS 'tighe') )
RETURN n.id, n.name, n.party_name, n.aliases, labels(n)[0] AS label,
       n.role, n.source_document, n.source_documents
ORDER BY n.name;
```

#### Q3 — is the split different between the two dropdowns? (the headline question)

```cypher
// Reproduces the two dropdown queries VERBATIM (bias/queries.rs:41-53 and :61-75)
// and diffs them. Any row with in_actors <> in_subjects is a party the author
// can pick in one dropdown and not the other.
CALL {
  MATCH (e:Evidence)-[:STATED_BY]->(a)
  WHERE e.pattern_tags IS NOT NULL AND e.pattern_tags <> ''
  RETURN a.id AS id, a.name AS name, count(e) AS c, 'actor' AS which
  UNION
  MATCH (e:Evidence)-[:ABOUT]->(s)
  WHERE e.pattern_tags IS NOT NULL AND e.pattern_tags <> ''
  RETURN s.id AS id, s.name AS name, count(DISTINCT e) AS c, 'subject' AS which
}
RETURN id, name,
       sum(CASE WHEN which = 'actor'   THEN c ELSE 0 END) AS actor_count,
       sum(CASE WHEN which = 'subject' THEN c ELSE 0 END) AS subject_count,
       sum(CASE WHEN which = 'actor'   THEN 1 ELSE 0 END) AS in_actors,
       sum(CASE WHEN which = 'subject' THEN 1 ELSE 0 END) AS in_subjects
ORDER BY name;
```

#### Q4 — Neo4j vs Postgres `parties` (S1 vs S2) disagreement

```sql
-- run on colossus_legal, then compare names to Q1's Neo4j roster by hand
SELECT name, role, entity_type, status FROM parties ORDER BY name;
```

*(inferred: S2 is the only store with a curated, human-authored party list — the caption. It
is therefore the natural yardstick for "how many humans are actually in this case". Nothing
in this repo joins S2 to S1: `case_header_repository.rs` never touches Neo4j, and no Neo4j
query reads `parties`.)*

#### Q5 — duplicates already reflected downstream

```cypher
// One Qdrant vector is emitted per Person node (embedding_repository.rs:150).
// This count is the number of Person vectors the store will hold.
MATCH (p:Person) RETURN count(p) AS person_vectors_expected;
```

```sql
-- colossus_legal_v2 — do saved scenarios already point at specific variants?
SELECT scenario_id, name, status,
       definition->>'target'   AS target_id,
       definition->'wielders'  AS wielders
FROM scenarios ORDER BY name;
```

That last one is the practical blast radius: every `target_id` / `party_id` is a frozen
choice of one variant (`dto/scenario_crud.rs:126,181`).

#### Q6 — is duplication still being produced, or is it historic?

```cypher
// party_name / aliases exist ONLY on nodes written by a56e9e7 or later
// (ingest_helpers.rs:302-303). This splits the roster by era.
MATCH (n) WHERE labels(n)[0] IN ['Person','Organization']
RETURN CASE WHEN n.party_name IS NULL THEN 'pre-a56e9e7' ELSE 'post-a56e9e7' END AS era,
       count(n) AS nodes
ORDER BY era;
```

Plus a log check on the app host, since the resolver already narrates every near-miss it
refuses to merge (`ingest_resolver.rs:344-351`):

```
grep -F 'Fuzzy/semantic near-match NOT auto-merged' <backend logs>
grep -F 'Canonical-name disagreement between documents' <backend logs>
grep -F 'CASE_DEFAULT_SUBJECT_NAME matched multiple subjects' <backend logs>
```

**measured (code):** those three strings are at `ingest_resolver.rs:345-350`,
`ingest_helpers.rs:410-416`, and `bias/repository.rs:524-529` respectively. The first one
prints both the incoming name and the near-match's stored name — it is, by construction, a
ready-made duplicate report that has been accumulating since 2026-07-20.

---

## 3. SPEAKER STORAGE — how speaker identity is recorded on Evidence

### 3.1 The answer: **both**, and they are written from different things

**measured (code).** Speaker identity exists in **two independent forms on every Evidence
node**, and nothing reconciles them.

**Form A — `e.speaker`, a free-text string property.**

The extraction schemas declare `speaker` as a property of the Evidence entity. From
`backend/extraction_schemas/court_transcript_schema_v5_3.yaml:106-109`:

> `- name: speaker` … "The canonical party_name of the person who spoke this turn, resolved
> from the speaker label via the APPEARANCES block and the introduction lines. NOT the raw
> label — 'George Phillips', not 'MR. PHILLIPS'. **This is the graph's join key to the Party
> node and must match that node's party_name exactly.**"

At ingest, `create_entity_node` walks `item_data["properties"]` and writes **every**
alphanumeric-named property onto the node with `SET n.<key> = $val`
(`ingest_helpers.rs:538-586`). `speaker` is not special-cased anywhere, so it lands on the
node as `e.speaker` — **verbatim, unnormalised, never resolved against any Person node.**

The same file also emits `speaker_role` (`schema:110-113`) and, for attorneys, a
`represents` party name (`schema:116`) — both stored the same free-text way.

**Form B — the `STATED_BY` edge to a Person/Organization node.**

`(:Evidence)-[:STATED_BY]->(:Person|:Organization)`, defined at `neo4j/schema.rs:73-80`:

> "Domain note: `STATED_BY` is the speaker who made the statement under oath; it is distinct
> from `ABOUT` (who the statement concerns) — different relationships, different queries."

The edge is **not** derived from `e.speaker` by any code in this repo. It is created only
when the **LLM emits a relationship object** in its pass-1/pass-2 JSON whose endpoints are
two entity ids from the same response. `store_entities_and_relationships`
(`extraction_relationships.rs:297-310`) resolves those endpoint keys through an in-response
`id_map` and, when either side does not resolve, **logs and drops the edge**:

```rust
tracing::warn!(run_id, document_id, from = %from_key, to = %to_key,
    "Skipping relationship with unresolved endpoint(s)");
```

At ingest, `from_item_id` / `to_item_id` are mapped to Neo4j ids through `pg_to_neo4j`
(`pipeline/steps/ingest.rs:496-527`), so the edge lands on **whichever Party node the
resolver picked for that document's mention** — see §4.

**Therefore (inferred, but tightly):** `e.speaker` and the `STATED_BY` target can disagree,
and there is no code path that would notice. `e.speaker` is the LLM's canonical string;
`STATED_BY` is the resolver's chosen node. They agree only when the resolver matched the
variant the LLM named — which, before `a56e9e7`, it never could for people (§4.2).

### 3.2 A third form exists on one write path only

**measured (code).** `POST /api/admin/evidence` (`api/admin_evidence.rs:150-200`) is a
separate, hand-driven import path. It takes `stated_by` as an **explicit node id**
(`admin_evidence.rs:34`: "Person or org ID who made the statement (e.g., \"george-phillips\")"),
creates the edge with a **label-free MATCH** (`admin_evidence_helpers.rs:76-105`), and
**hard-fails with a 400 if the id names no node**. It does not create Person nodes and does
not write `e.speaker`. So Evidence created through this path has Form B and no Form A.

### 3.3 Who reads which form — and this is where the corruption enters

**measured (code).**

| Reader | Reads | Consequence of a duplicate |
|---|---|---|
| Bias Explorer cards / filters | Form B (`STATED_BY`, `actor.name`) | Card shows whichever variant the edge points at; filtering by one variant hides the others' statements |
| Scenario theme scan + candidate gather | Form B via `ABOUT` (`bias/repository.rs:428-476`, `services/scenario_subject.rs:88-108`) | **The candidate pool is scoped to ONE subject node id.** Evidence about a sibling variant is not in the pool at all |
| Scenario rebuttal facts | Form B, **label-pinned to `:Person`** — `(targetE)-[:STATED_BY]->(w:Person {id: $wielder_id})` (`scenario_repository.rs:320`) | Returns nothing for the variants the author did not pick; also returns nothing if the wielder is an `:Organization` |
| Scenario contradictions | Form B, label-pinned — `(a)-[:STATED_BY]->(:Person {id: $wielder_id})` (`scenario_repository.rs:342-343`) | same |
| Scenario timeline turns (`speaker` shown on each card) | Form B — `CASE WHEN spk:Person OR spk:Organization THEN spk.name ELSE null END` (`scenario_repository.rs:412`), surfaced at `services/scenario_dashboard.rs:283` | Card displays the stored variant name, so one exchange can show three spellings of one human |
| Semantic search / RAG embeddings | Form B — `COALESCE(speaker.name, '')` folded into the embedded text (`embedding_repository.rs:45,56`) | Each variant embeds separately; retrieval fragments |
| Admin document evidence table | Form B, **`:Person`-only** — `OPTIONAL MATCH (n)-[:STATED_BY]->(p:Person) … p.name AS speaker` (`api/admin_document_evidence_queries.rs:72-78`) | An Organization speaker renders as blank, not as the org |
| Audit health check | Form B presence only — `p IS NOT NULL AS has_speaker` (`services/audit_checks.rs:103-140`) | Reports "no STATED_BY relationship" — cannot see a *wrong* one |
| Person detail page | Form B, **label-agnostic** — `(e)-[:STATED_BY]->(p {id: $person_id})` (`person_detail_repository.rs:56`) | Shows only the statements attached to that one variant |
| **Nothing at all** | **Form A (`e.speaker`)** | The free-text speaker string is written to every Evidence node and, as far as I can find, **is never read back by any query in this repo** |

That last row is the one I would flag hardest. `e.speaker` is declared by the schemas as
"the graph's join key to the Party node", is dutifully written to every Evidence node, and
has **no reader** — I grepped for `e.speaker`, `.speaker AS`, and `speaker_role` across
`backend/src` and the only `speaker` projection is `p.name AS speaker` from the *edge*
(`admin_document_evidence_queries.rs:78`). The join key exists in the data and is
never joined on. *(measured (code) for the absence of readers; inferred that this was
intended to be the reconciliation point.)*

### 3.4 Query to measure Form A vs Form B disagreement

```cypher
// Does the free-text speaker string agree with the node the edge points at?
MATCH (e:Evidence) WHERE e.speaker IS NOT NULL AND e.speaker <> ''
OPTIONAL MATCH (e)-[:STATED_BY]->(p)
RETURN e.speaker                                AS speaker_string,
       coalesce(p.name, '<no STATED_BY edge>')  AS edge_target_name,
       coalesce(p.id, '')                       AS edge_target_id,
       e.speaker = p.name                       AS agrees,
       count(*)                                 AS evidence_count
ORDER BY agrees, speaker_string;
```

```cypher
// The same human under different speaker STRINGS, ignoring the edge entirely.
MATCH (e:Evidence) WHERE e.speaker IS NOT NULL AND e.speaker <> ''
RETURN e.speaker AS speaker_string, count(e) AS evidence_count
ORDER BY toLower(speaker_string);
```

```cypher
// Evidence with a speaker string but NO edge — statements orphaned from their speaker.
MATCH (e:Evidence)
WHERE e.speaker IS NOT NULL AND e.speaker <> ''
  AND NOT (e)-[:STATED_BY]->()
RETURN e.speaker AS speaker_string, count(e) AS orphaned_evidence
ORDER BY orphaned_evidence DESC;
```

The transcript template predicts exactly this failure and warns about it
(`extraction_templates/court_transcript_pass1_v5_3.md:104`): *"Every `speaker` value you emit
must be a canonical `party_name` that exactly matches a Party entity you also emit. That
string is the graph's join key. **If they differ by so much as a title, the statement is
orphaned from its speaker.**"*

---

## 4. WRITE PATHS — where entity names enter, and what normalisation exists

### 4.1 The one real write path, end to end

**measured (code).** Names enter the system at exactly one place per document, and travel
through five stages.

**Stage 1 — LLM pass-1 extraction → Postgres `colossus_legal_v2`.**
The model emits a JSON body with `entities[]` and `relationships[]`.
`store_entities_and_relationships` (`extraction_relationships.rs:248-310`) inserts one
`extraction_items` row per entity, keeping an in-response `id_map` so the relationship
endpoints resolve. **No normalisation of any kind happens here** — the name is whatever the
model wrote, in `item_data.properties.party_name` (or `full_name`).

The templates *instruct* the model to canonicalise, and to carry every label variant as
aliases. From `court_transcript_pass1_v5_3.md:93`:

> "**Speaker-label drift:** the same person may be labeled `NADIA AWAD:` early and `NADIA:`
> later; a lay speaker may be introduced by full name and then labeled by surname. These are
> ONE person and ONE canonical `speaker`. Every label variant goes in that Party's `aliases`."

That instruction is **per document and per chunk**. Nothing constrains document B's canonical
form to match document A's. *(inferred: this is the upstream origin of the variants — two
documents legitimately following the same rule can produce "Judge Tighe" and "Karen A. Tighe".)*

**Stage 2 — human review.** `extraction_items.review_status` gates what reaches the graph
(`extraction_relationships.rs:94-115` requires `approved` on both endpoints). The review UI
shows parties (`PeopleLinksPanel.tsx`), but **it has no merge, rename, or link-to-existing
action** — a reviewer can approve, reject or edit an item's fields, not reconcile two items
to one person. *(measured (code): `review_items.rs` / `review_actions` expose approve /
reject / edit / bulk-approve / undo, and nothing else.)*

**Stage 3 — entity resolution.** `pipeline/steps/ingest.rs:325-337`:

```rust
let existing_parties = ingest_resolver::fetch_existing_parties(&context.graph).await?;
let (resolution_map, _resolution_summary) =
    ingest_resolver::resolve_parties(&items, &existing_parties).await?;
```

`fetch_existing_parties` (`ingest_resolver.rs:61-118`) reads **all** `:Person` and
**all** `:Organization` nodes out of Neo4j — id, name, role — as `KnownEntity` candidates.
`resolve_parties` (`:274-410`) runs them through `NormalizedEntityResolver` from the
`colossus-extract` crate (a git dependency on `colossus-rs` v0.15.0 — **not in this repo, and
not vendored in this container, so its internals are `reported`, not measured**).

**Stage 4 — the merge decision.** This is the interesting part, and it is deliberate policy,
not an accident. `is_auto_mergeable` (`ingest_resolver.rs:246-252`) allows **only** exact and
normalized matches to bind two mentions to one node. Fuzzy and semantic matches are
**demoted** — they create a new node and log a warning. The rationale, `ingest_resolver.rs:229-244`:

> "**Ruling 2026-07-20: only EXACT and NORMALIZED matches auto-merge.** In a legal knowledge
> graph a duplicate fragments a person's evidence — visible, and fixable by a human-approved
> dedup pass. A FALSE merge silently welds two real people into one node and attributes one
> person's sworn statements to another … Jaro-Winkler similarity is a good metric and a bad
> adjudicator of identity, so it does not get a vote."

*(inferred: the duplication the instruction is chasing is, in part, a **chosen trade-off** —
the system is designed to prefer visible duplicates over silent false merges, on the
assumption that a human-approved dedup pass would follow. That pass does not exist yet.)*

**Stage 5 — the graph write.** `create_party_nodes` (`ingest_helpers.rs:335-448`) MERGEs on
the resolved id, or falls back to `person-<slug(name)>` / `org-<slug(name)>` when the party
is absent from the resolution map (`:388-392`). The MERGE
(`build_party_merge_cypher`, `:299-315`) is first-writer-wins on `name` and `role`, and
append-only on `aliases` and `source_documents`.

Non-party entities go through `create_entity_node` (`ingest_helpers.rs:469-585`), which is
where `e.speaker` gets written (§3.1).

### 4.2 What normalisation / dedup exists **today**

**measured (code).** Six mechanisms exist. Four are real, two are decorative.

| # | Mechanism | Where | Verdict |
|---|---|---|---|
| N1 | `slug()` lowercases and collapses non-alphanumerics before minting an id | `ingest_helpers.rs:29-37` — `"MARIE AWAD"` and `"Marie Awad"` both → `marie-awad` | **Real, but weak.** Only collapses case and punctuation. `"Judge Tighe"` and `"Tighe"` slug differently, so they mint different ids and MERGE never meets |
| N2 | Exact + normalized matching in `NormalizedEntityResolver` | `colossus-extract` v0.15.0 (external) | **Real** — but see N3; it was inert for people until 2026-07-20 |
| N3 | `normalize_party_type`: rewrites `party_type: "person"` → `"individual"` at the crate boundary | `ingest_resolver.rs:189-198`, constants at `:128-129` | **Real, and load-bearing.** Its doc comment (`:141-162`) explains that upstream `compatible_type` matches Person nodes on the token `"individual"` and returns `false` for anything else, while every template emits `"person"` — so **the candidate list for every human party came back EMPTY and each person resolved as a brand-new entity.** Landed in `a56e9e7`, 2026-07-20 |
| N4 | Fuzzy/semantic demotion + WARN log | `ingest_resolver.rs:246-252`, `:326-352` | **Real** — deliberately does NOT dedup, but produces the near-match log that a dedup pass would consume |
| N5 | `aliases` list, seeded on create, appended on match, folding in the losing canonical name | `ingest_helpers.rs:299-315`, parse at `:239-262` | **Real but not yet used for matching.** The doc comment says it is stored as a LIST "so Cypher can test membership directly, which is what alias-aware matching **will** need". **No query in this repo matches on `aliases` today** — I grepped; the only readers are the writer's own tests |
| N6 | In-batch dedup: `resolution_map.contains_key(party_name)` skip, and the `seen` HashSet on node id | `ingest_resolver.rs:314-317`, `ingest_helpers.rs:333,398-401` | **Real but scoped to one document's batch.** Keyed on the exact name string, so two variants inside one document still produce two nodes |

**Explicitly absent (measured (code) — I looked for each):**

- No cross-document canonical-name authority. `n.name` is first-writer-wins forever (`ingest_helpers.rs:302`, ON CREATE only).
- No alias-based matching at resolution time (N5).
- No use of the `parties` table (S2) — the one curated, human-authored roster — as resolver input. `fetch_existing_parties` reads Neo4j only (`ingest_resolver.rs:61-118`).
- No merge/dedup UI action anywhere in the review panel or admin surface.
- No merge/dedup endpoint. There is no `POST /admin/.../merge`, no `persons/:id/merge`; the API router (`api/mod.rs:79-93`) has no such route.
- No uniqueness constraint on `Person.name` in `backend/migrations_neo4j/` (the only file there is `001_add_document_status.cypher`).
- No reconciliation of `e.speaker` against `STATED_BY` (§3.3).

### 4.3 One more write-path behaviour that churns identity

**measured (code).** Document delete / re-process runs `cleanup_neo4j`
(`pipeline/steps/cleanup.rs:141-161`), which DETACH DELETEs nodes whose scalar
`source_document` matches the doc id — **guarded** so a node shared with another document
survives (`build_party_delete_cypher`, `:231-238`), with the surviving node's
`source_documents` array stripped of the dead doc id (`:171-198`).

The guard is well-built. But `source_document` is ON CREATE only, so it names the *first*
document — meaning **which Person nodes survive a re-process depends on ingest order**.
*(inferred: on a full re-extraction, the surviving canonical form for a given human can
change, which is a plausible mechanism for the variant set on DEV drifting over time. Q6 in
§2.2 is the query that would confirm or kill this.)*

---

## 5. THE MECHANISM, ASSEMBLED

**inferred** (each step's evidence is cited above and is measured (code)):

1. Each document's pass-1 LLM independently picks a canonical name for each human, per the template rule. Two documents legitimately produce two spellings. *(§4.1 stage 1)*
2. Before 2026-07-20, the type-vocabulary mismatch meant the resolver's candidate list for **every human** was empty, so **every person mention in every document created a fresh node** with a slug-derived id. *(§4.2 N3, reported at `ingest_resolver.rs:150-159`)*
3. After 2026-07-20 the resolver works — but only for exact and normalized matches, by explicit ruling. `"Judge Tighe"` vs `"Tighe"` is neither, so it is demoted to a new node and logged. *(§4.2 N4)*
4. The variants that already exist are never merged: there is no dedup pass, no merge endpoint, no merge UI, and alias-based matching is stored-but-unused. *(§4.2, absent list)*
5. Every read surface groups by *node*, so each variant is a separate dropdown row, a separate Qdrant vector, a separate person page, a separate scenario candidate pool. *(§1.2, §3.3)*
6. The two Scenario dropdowns split the *same* human differently because they traverse different edges (`STATED_BY` vs `ABOUT`) over the same duplicated node set, both gated on `pattern_tags`. *(§1.3)*
7. A scenario freezes one variant's node id into `definition.target` / `wielders[].party_id`, and every downstream query — rebuttal facts, contradictions, theme-scan pool, gather — is scoped to that single id, silently excluding the sibling variants' evidence. *(§3.3)*

---

## 6. WHAT THIS REPORT DOES NOT ANSWER

Stated plainly rather than papered over:

1. **All of §2's counts.** No live DB access (§0.1). The queries are written and justified; someone with SSH to `10.10.100.200` can produce the numbers.
2. **Whether DEV's Person set predates or postdates `a56e9e7`.** §2.2 Q6 settles it in one query.
3. **Whether the near-match / canonical-disagreement warnings have fired on DEV.** §2.2 Q6's `grep`s settle it; I cannot reach the app host.
4. **`NormalizedEntityResolver`'s internals.** It lives in `colossus-rs` v0.15.0, a git dependency, not vendored in this container. Everything I say about `compatible_type` is `reported` from `ingest_resolver.rs:141-162`. Reading it is a `colossus-rs` task — and per CLAUDE.md rule 3, a separate instruction in a separate repo.
5. **Whether any variant pair is genuinely two different humans.** Nothing here should be read as a merge recommendation. The codebase's own ruling (`ingest_resolver.rs:229-244`) is that a false merge is worse than a duplicate, and I have no basis to overrule it.

Per the instruction: **stopping here.** No remediation, no design, no migration.
