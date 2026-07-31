# Case Health — Documented Queries

**Version:** 1.0
**Created:** 2026-07-27
**Surface:** `GET /api/cases/:slug/case-health/inventory` → Case Health page, Pane 1
("Graph Inventory")
**Backend:** `api::case_health` · `repositories::case_health_repository` ·
`repositories::case_health_builder`

---

## Why this file exists

The Case Health dashboard is a measuring instrument, and an instrument nobody can
check is worse than no instrument. **Every number the pane displays is
reproducible by pasting a query from this file into Neo4j and reading the
result.** If a figure on screen ever disagrees with the query documented here for
it, one of the two is wrong and the discrepancy is a defect, not a rounding
quirk.

This discipline is not decorative. On 2026-07-26 two "alarm" findings —
`to_element: 0` and `document_type` NULL on every Document — turned out to be
queries against schema that has never existed. The standing rule that came out of
it: **verification queries are written from the code's write paths, never from
assumed property names.** The queries below are the exact strings the backend
builds, extracted from the code, not paraphrases of them.

**All queries here are read-only.** A unit test
(`case_health_repository_tests::every_query_is_read_only`) scans the generated
Cypher for `CREATE` / `MERGE` / `SET` / `DELETE` / `REMOVE` / `DETACH` and fails
the build if one appears.

---

## Conventions

### Parameters

The backend binds node labels as query parameters and interpolates relationship
types from the `neo4j::schema` constants (Cypher cannot parameterize a
relationship type). The queries below have the parameters substituted with their
production values so they can be pasted as-is:

| Parameter | Value | Source constant |
|---|---|---|
| `$evidence_label` | `'Evidence'` | `models::document_status::ENTITY_EVIDENCE` |
| `$allegation_label` | `'Allegation'` | `models::document_status::ENTITY_ALLEGATION` |
| `$document_label` | `'Document'` | `models::document_status::ENTITY_DOCUMENT` |

### Property names

Taken from the write paths, never assumed:

| What | Property | Written by |
|---|---|---|
| Document type | `doc_type` | `api::pipeline::ingest_helpers::create_document_node` |
| Document display name | `title` | same |
| Document id | `id` | same |

`document_type` and `file_name` are **not** properties of a `Document` node.
A test (`documents_query_reads_the_properties_the_write_path_actually_sets`)
fails the build if either name appears in the generated Cypher.

### The two connection tiers

Defined once in `domain::connection_tier`:

- **Probative** = `CORROBORATES` ∪ `REBUTS` ∪ `CHARACTERIZES` → `Allegation`.
  These edges bear on whether the allegation is **true**. **This is the headline.**
- **Topical** = probative ∪ `ABOUT` → `Allegation`. `ABOUT` asserts only that an
  item is on the subject.

The two are displayed side by side, separately labeled, and **never blended**. A
corpus entirely "about" the case would otherwise report as fully connected while
proving nothing.

### Item counts vs. edge counts

Every connection figure counts **Evidence items**, not edges. An item that
corroborates three allegations contributes `1`. This is why the per-class columns
do **not** sum to the connected totals: an item that both corroborates one
allegation and rebuts another appears under both classes while contributing 1 to
the connected count.

### Rates

Computed in `case_health_builder::percent`: `count * 100 / total`, rounded to one
decimal place (half away from zero). A zero denominator yields **no rate at all**
(rendered `—`), never `0.0%` — "nothing to measure" and "measured, none
connected" are different statements.

---

## 1. Corpus headline

**Purpose:** the two headline rates, the inert count, and the orphaned-Evidence
tally. Measured over **every** Evidence node, not by summing §3 — an Evidence node
with no `CONTAINED_IN` edge is invisible to §3 but real.

**Feeds:** the three cards at the top of the page (`current.corpus`).

```cypher
MATCH (e) WHERE labels(e)[0] = 'Evidence'
WITH e,
     size([ (e)-[:CORROBORATES]->(x1)  WHERE labels(x1)[0] = 'Allegation' | x1 ]) AS n_corroborates,
     size([ (e)-[:REBUTS]->(x2)        WHERE labels(x2)[0] = 'Allegation' | x2 ]) AS n_rebuts,
     size([ (e)-[:CHARACTERIZES]->(x3) WHERE labels(x3)[0] = 'Allegation' | x3 ]) AS n_characterizes,
     size([ (e)-[:ABOUT]->(x4)         WHERE labels(x4)[0] = 'Allegation' | x4 ]) AS n_about,
     size([ (e)-[:CONTAINED_IN]->(d)   WHERE labels(d)[0] = 'Document'   | d  ]) AS n_documents
RETURN count(e) AS evidence_total,
       sum(CASE WHEN n_corroborates + n_rebuts + n_characterizes > 0 THEN 1 ELSE 0 END)
           AS probative_connected,
       sum(CASE WHEN n_corroborates + n_rebuts + n_characterizes + n_about > 0 THEN 1 ELSE 0 END)
           AS topical_connected,
       sum(CASE WHEN n_documents = 0 THEN 1 ELSE 0 END)
           AS evidence_without_document;
```

**Reading the screen against this:**

| Screen | From this query |
|---|---|
| Probative connection rate | `probative_connected * 100 / evidence_total`, 1 d.p. |
| Topical connection rate | `topical_connected * 100 / evidence_total`, 1 d.p. |
| Inert | `evidence_total - topical_connected` |
| Inert % | `inert * 100 / evidence_total`, 1 d.p. |
| "N Evidence items are not linked to any Document" banner | `evidence_without_document` (banner shown only when > 0) |

Source: `case_health_repository::corpus_query`. The only difference from the code
is that the inner pattern-comprehension variables are named `x1…x4` here for
readability; the code derives them as `x_CORROBORATES` etc. so they can never
collide.

---

## 2. Nodes by populated label

**Purpose:** the label inventory. **Derived from the nodes themselves.** Neo4j
retains label *names* from earlier schema generations forever — `db.labels()`
lists twelve labels with zero nodes on this database — so `db.labels()` must never
be used here. A test enforces this.

**Feeds:** the "Nodes by label" table and the `unlabeled_node_count` banner.

```cypher
MATCH (n)
RETURN labels(n)[0] AS label, count(*) AS node_count
ORDER BY node_count DESC, label ASC;
```

A row with `label = null` is a node carrying **no** label. The backend counts
those into `unlabeled_node_count` rather than dropping them, so the label counts
plus the unlabeled tally always account for every node in the graph.

Source: `case_health_repository::LABEL_COUNTS_QUERY` (verbatim).

---

## 3. Edges by endpoint

**Purpose:** the full edge taxonomy. The endpoints are the point — a count by
relationship type alone cannot distinguish `Evidence ABOUT Person` from
`Evidence ABOUT Allegation`, and only the second is a connection.

**Feeds:** the "Edges by endpoint" table.

```cypher
MATCH (a)-[r]->(b)
RETURN labels(a)[0] AS from_label,
       type(r)      AS rel_type,
       labels(b)[0] AS to_label,
       count(*)     AS edge_count
ORDER BY edge_count DESC, from_label ASC, rel_type ASC, to_label ASC;
```

A `null` endpoint label renders as `(unlabeled)` on screen; the payload omits the
field rather than inventing a placeholder.

Source: `case_health_repository::EDGE_TRIPLES_QUERY` (verbatim).

---

## 4. Per-document connection

**Purpose:** what each document yielded and how much of it is wired in.

**Feeds:** every row of the "Per-document connection" table.

`OPTIONAL MATCH` on the Evidence leg is load-bearing: a Document that produced
**no** Evidence still appears, as a row of zeros. That row is a finding — a
document that cost money and yielded nothing — and a plain `MATCH` would hide it.

```cypher
MATCH (doc) WHERE labels(doc)[0] = 'Document'
OPTIONAL MATCH (e)-[:CONTAINED_IN]->(doc) WHERE labels(e)[0] = 'Evidence'
WITH doc, e,
     size([ (e)-[:CORROBORATES]->(x1)  WHERE labels(x1)[0] = 'Allegation' | x1 ]) AS n_corroborates,
     size([ (e)-[:REBUTS]->(x2)        WHERE labels(x2)[0] = 'Allegation' | x2 ]) AS n_rebuts,
     size([ (e)-[:CHARACTERIZES]->(x3) WHERE labels(x3)[0] = 'Allegation' | x3 ]) AS n_characterizes,
     size([ (e)-[:ABOUT]->(x4)         WHERE labels(x4)[0] = 'Allegation' | x4 ]) AS n_about
WITH doc,
     count(e) AS evidence_total,
     sum(CASE WHEN e IS NOT NULL AND n_corroborates + n_rebuts + n_characterizes > 0
              THEN 1 ELSE 0 END) AS probative_connected,
     sum(CASE WHEN e IS NOT NULL AND n_corroborates + n_rebuts + n_characterizes + n_about > 0
              THEN 1 ELSE 0 END) AS topical_connected,
     sum(CASE WHEN e IS NOT NULL AND n_corroborates  > 0 THEN 1 ELSE 0 END) AS n_corroborates_items,
     sum(CASE WHEN e IS NOT NULL AND n_rebuts        > 0 THEN 1 ELSE 0 END) AS n_rebuts_items,
     sum(CASE WHEN e IS NOT NULL AND n_characterizes > 0 THEN 1 ELSE 0 END) AS n_characterizes_items,
     sum(CASE WHEN e IS NOT NULL AND n_about         > 0 THEN 1 ELSE 0 END) AS n_about_items
RETURN doc.id       AS document_id,
       doc.title    AS title,
       doc.doc_type AS doc_type,
       evidence_total, probative_connected, topical_connected,
       n_corroborates_items, n_rebuts_items, n_characterizes_items, n_about_items
ORDER BY evidence_total DESC, document_id ASC;
```

**Reading the screen against this:**

| Column | From this query |
|---|---|
| Document | `title` (falls back to `document_id` when the node has no title) |
| Type | `doc_type` |
| Evidence | `evidence_total` |
| Probative / Probative % | `probative_connected`, and its share of `evidence_total` |
| Topical / Topical % | `topical_connected`, and its share |
| Inert / Inert % | `evidence_total - topical_connected`, and its share |
| CORROBORATES / REBUTS / CHARACTERIZES / ABOUT | `n_*_items` |

Source: `case_health_repository::documents_query`.

---

## 5. Spot-check queries

Not used by the dashboard — these exist so a figure can be confirmed from a
second direction.

### 5.1 List the Evidence a document failed to connect

Names the inert items for one document, so "126 inert" can be read rather than
merely counted.

```cypher
MATCH (e)-[:CONTAINED_IN]->(d:Document {id: $document_id})
WHERE NOT (e)-[:CORROBORATES|REBUTS|CHARACTERIZES|ABOUT]->(:Allegation)
RETURN e.id AS evidence_id, e.title AS title, e.page_number AS page
ORDER BY e.page_number, e.id;
```

### 5.2 Confirm the probative/topical gap is exactly the ABOUT-only items

The difference between the two headline rates should be precisely the items whose
ONLY connection is `ABOUT`. If this count does not equal
`topical_connected - probative_connected` from §1, the tiers are being computed
inconsistently somewhere.

```cypher
MATCH (e:Evidence)
WHERE (e)-[:ABOUT]->(:Allegation)
  AND NOT (e)-[:CORROBORATES|REBUTS|CHARACTERIZES]->(:Allegation)
RETURN count(e) AS about_only_items;
```

### 5.3 Confirm `doc_type` is populated

The query whose earlier, misspelled form (`d.document_type`) produced a false
alarm on 2026-07-26. Both spellings are returned so the difference is visible.

```cypher
MATCH (d:Document)
RETURN d.id            AS id,
       d.doc_type      AS doc_type,
       d.document_type AS document_type_does_not_exist,
       d.title         AS title
ORDER BY d.id;
```

---

## Change discipline

If a query in this file and the code diverge, **the code is authoritative and
this file is a defect.** When a Cypher builder in `case_health_repository`
changes, update the corresponding section here in the same commit — the
provenance rule is only worth anything if the documentation is current.

The connection-tier partition is versioned:
`domain::connection_tier::CONNECTION_TIER_LOOKUP_V` (currently `1`), and the
payload carries it. If the probative/topical split ever changes, bump that
constant — rates computed under different partitions are not comparable, and a
future snapshot delta needs to be able to refuse the comparison.
