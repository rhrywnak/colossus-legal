# Proof Matrix — Documented Queries

**Version:** 1.0
**Created:** 2026-07-27
**Surface:** `GET /api/cases/:slug/causes-of-action` → the Proof Matrix page, and
`GET /api/cases/:slug/elements/:element_id/detail` → its expanded row
**Backend:** `api::causes_of_action` · `repositories::causes_of_action_repository` ·
`repositories::causes_of_action_builder` · `repositories::element_detail_repository` ·
`repositories::element_detail_fold`

---

## Why this file exists

Every figure the Proof Matrix displays must be reproducible by pasting a query
from this file into Neo4j and reading the result. If a number on screen ever
disagrees with the query documented for it, one of the two is wrong and that is a
defect, not a rounding quirk.

The Proof Matrix had no such artifact until now. Its queries were traceable only
through a one-line row in `docs/CODE_INVENTORY_2026-05-25.md` — a dated snapshot
that is already wrong (it lists a `PROVES_ELEMENT` edge that does not exist in the
schema). That is precisely the gap this file closes, and it is the same
convention `docs/CASE_HEALTH_QUERIES.md` follows.

**All queries here are read-only.** The Cypher below is extracted from the strings
the code actually builds, not paraphrased.

---

## Conventions

### Parameters

Node labels are bound as query parameters; relationship types are interpolated
from the `neo4j::schema` constants, because Cypher cannot parameterize a
relationship type. Substituted below so the queries paste as-is:

| Parameter | Value | Source constant |
|---|---|---|
| `$count_label` | `'LegalCount'` | `ENTITY_LEGAL_COUNT` |
| `$element_label` | `'Element'` | `ENTITY_ELEMENT` |
| `$allegation_label` | `'Allegation'` | `ENTITY_ALLEGATION` |
| `$evidence_label` | `'Evidence'` | `ENTITY_EVIDENCE` |
| `$document_label` | `'Document'` | `ENTITY_DOCUMENT` |

### The spine

Everything below walks one chain:

```
LegalCount -[:HAS_ELEMENT]-> Element <-[:BEARS_ON]- Allegation <-[:CORROBORATES|REBUTS]- Evidence
```

Evidence reaches an Element **only through an Allegation**. There is no direct
`Evidence → Element` edge and there never was — a query for one is what produced
the false `to_element: 0` finding on 2026-07-26.

### Support and dispute are independent

- **Supporting** = `Evidence -[:CORROBORATES]-> Allegation`
- **Disputes** = `Evidence -[:REBUTS]-> Allegation`

Neither is subtracted from the other, and **only Supporting feeds the Status
pill**. An Element can be well corroborated *and* heavily disputed; that Element
is the one worth arguing about, so the two are shown side by side rather than
netted into one verdict.

"Disputes" is the deliberate word. Not "Contradicts" — that is reserved for the
future evidence-vs-evidence impeachment layer, and one word for two different
relationships would make them read as one. Not "Opposing" — that describes a
party's posture rather than what the record actually disputes.

---

## 1. Element row metrics

**Purpose:** every numeric column of the Proof Matrix, plus the input to the
Status pill.

**Feeds:** Mapped Allegations, Supporting, Disputes, Status.

```cypher
MATCH (lc)-[:HAS_ELEMENT]->(el)
WHERE labels(lc)[0] = 'LegalCount' AND labels(el)[0] = 'Element'
OPTIONAL MATCH (a)-[:BEARS_ON]->(el)      WHERE labels(a)[0]  = 'Allegation'
OPTIONAL MATCH (a)<-[:CORROBORATES]-(ev)  WHERE labels(ev)[0] = 'Evidence'
OPTIONAL MATCH (a)<-[:REBUTS]-(dv)        WHERE labels(dv)[0] = 'Evidence'
RETURN lc.count_number             AS count_number,
       el.id                       AS element_id,
       el.order_in_count           AS order_in_count,
       el.element_name             AS element_name,
       el.what_plaintiff_must_prove AS what_plaintiff_must_prove,
       el.controlling_authority    AS controlling_authority,
       el.theory_variant           AS theory_variant,
       count(DISTINCT a)           AS allegation_count,
       count(DISTINCT ev)          AS supporting_evidence_count,
       count(DISTINCT CASE WHEN ev IS NOT NULL THEN a END) AS covered_allegation_count,
       count(DISTINCT dv)          AS disputing_evidence_count;
```

**Reading the screen against this:**

| Column | From this query |
|---|---|
| Mapped Allegations | `allegation_count` |
| Supporting | `supporting_evidence_count` |
| Disputes | `disputing_evidence_count` |
| Status | `derive_proof_status(allegation_count, covered_allegation_count)` — see §2 |

### ⚠ The fan-out multiplies — why every aggregate is `count(DISTINCT …)`

The two evidence legs hang off the same `a`, so this query returns one row per
`(allegation × corroborating × disputing)` **combination** — not per allegation,
and **not** the sum of the two legs. An Allegation with 3 corroborations and 2
rebuttals produces 6 rows, and each corroborating item appears on 2 of them.

Every metric survives that only because it is a `count(DISTINCT …)`. Relaxing any
one of them to a plain `count(…)` would multiply the figure by the size of the
other leg and produce a number that is wrong but entirely plausible. If you run
this by hand and drop a `DISTINCT`, expect inflated results — that is the query
misbehaving as designed, not the graph.

A unit test
(`causes_of_action_repository::tests::every_element_aggregate_is_distinct_guarded_against_the_cartesian_fan_out`)
fails the build if any aggregate here loses its `DISTINCT`.

Source: `causes_of_action_repository::elements_query`.

---

## 2. The Status pill

**Not a query** — derived in Rust from two numbers §1 already returned, so it
cannot disagree with the columns beside it.

`causes_of_action_builder::derive_proof_status(T, C)` where `T = allegation_count`
and `C = covered_allegation_count`:

| Condition | Status | Meaning |
|---|---|---|
| `T = 0` | `no_allegations` | nothing is mapped to this Element |
| `C = 0`, `T > 0` | `gap` | allegations are mapped, none corroborated |
| `C ≥ T` | `supported` | every mapped allegation has ≥1 corroboration |
| otherwise | `partial` | some but not all corroborated |

Domain note: this reports **presence of evidence**, not legal sufficiency — there
is deliberately no `proven` state. `disputing_evidence_count` is **not** an input;
disputes never downgrade the pill.

To reproduce `C` alone:

```cypher
MATCH (lc)-[:HAS_ELEMENT]->(el {id: $element_id})
OPTIONAL MATCH (a)-[:BEARS_ON]->(el)     WHERE labels(a)[0]  = 'Allegation'
OPTIONAL MATCH (a)<-[:CORROBORATES]-(ev) WHERE labels(ev)[0] = 'Evidence'
RETURN count(DISTINCT a) AS T,
       count(DISTINCT CASE WHEN ev IS NOT NULL THEN a END) AS C;
```

---

## 3. Expanded row — the items behind the counts

**Purpose:** the mapped Allegations of one Element, each with the Evidence
corroborating it and the Evidence disputing it. This is where the Supporting and
Disputes counts become readable items.

**Feeds:** the expanded detail panel under a Proof Matrix row.

```cypher
MATCH (e) WHERE e.id = $element_id AND labels(e)[0] = 'Element'
OPTIONAL MATCH (lc)-[:HAS_ELEMENT]->(e)  WHERE labels(lc)[0] = 'LegalCount'
OPTIONAL MATCH (a)-[:BEARS_ON]->(e)      WHERE labels(a)[0]  = 'Allegation'
OPTIONAL MATCH (a)<-[:CORROBORATES]-(ev) WHERE labels(ev)[0] = 'Evidence'
OPTIONAL MATCH (ev)-[:CONTAINED_IN]->(d) WHERE labels(d)[0]  = 'Document'
OPTIONAL MATCH (a)<-[:REBUTS]-(dv)       WHERE labels(dv)[0] = 'Evidence'
OPTIONAL MATCH (dv)-[:CONTAINED_IN]->(dd) WHERE labels(dd)[0] = 'Document'
RETURN e.id                          AS element_id,
       e.element_name                AS element_name,
       e.what_plaintiff_must_prove   AS what_plaintiff_must_prove,
       e.order_in_count              AS order_in_count,
       lc.count_number               AS count_number,
       lc.title                      AS count_name,
       a.id                          AS allegation_id,
       a.paragraph_number            AS paragraph_number,
       a.summary                     AS summary,
       a.title                       AS title,
       a.verbatim_quote              AS verbatim_quote,
       ev.id                         AS evidence_id,
       ev.verbatim_quote             AS evidence_quote,
       ev.page_number                AS evidence_page_number,
       ev.paragraph                  AS evidence_paragraph,
       ev.page_note                  AS evidence_page_note,
       d.id                          AS source_document_id,
       d.title                       AS source_document_title,
       dv.id                         AS disputing_id,
       dv.verbatim_quote             AS disputing_quote,
       dv.page_number                AS disputing_page_number,
       dv.paragraph                  AS disputing_paragraph,
       dv.page_note                  AS disputing_page_note,
       dd.id                         AS disputing_document_id,
       dd.title                      AS disputing_document_title;
```

**Reading the rows.** This is the same cartesian fan-out as §1, so the raw result
repeats items. `element_detail_fold::DetailFold` collapses it: the Element header
is captured once, Allegations are deduped by `allegation_id`, and each leg's
Evidence is deduped by evidence id into its own bucket. Running it by hand you
will see repeats — count `DISTINCT evidence_id` / `DISTINCT disputing_id` per
allegation to match the panel.

An Evidence item whose `source_document_id` is null is **kept**, not dropped —
the click-through to the source PDF is simply unavailable, and the backend emits
a `warn` naming the leg so the data gap is observable.

The panel also reads `review_notes` from Postgres
(`colossus_legal_v2.authored_entities`), which is not part of this Cypher:

```sql
SELECT review_notes FROM authored_entities
WHERE entity_id = $element_id AND entity_type = 'Element';
```

Source: `element_detail_repository::element_detail_cypher` + `REVIEW_NOTES_SQL`.

---

## 4. Spot-check queries

Not used by the page — these exist so a figure can be confirmed from a second
direction.

### 4.1 List the disputing evidence for one Element

Names the items behind the Disputes count, so a number can be read rather than
merely trusted.

```cypher
MATCH (ev:Evidence)-[:REBUTS]->(a:Allegation)-[:BEARS_ON]->(el:Element {id: $element_id})
OPTIONAL MATCH (ev)-[:CONTAINED_IN]->(d:Document)
RETURN DISTINCT ev.id AS evidence_id, ev.verbatim_quote AS quote,
       ev.page_number AS page, d.title AS source_document
ORDER BY d.title, ev.page_number;
```

### 4.2 Elements that are both supported and disputed

The set the Disputes column exists to surface — Elements where the pill says
`supported` or `partial` while the record also carries rebuttals.

```cypher
MATCH (lc:LegalCount)-[:HAS_ELEMENT]->(el:Element)
OPTIONAL MATCH (a)-[:BEARS_ON]->(el)     WHERE labels(a)[0]  = 'Allegation'
OPTIONAL MATCH (a)<-[:CORROBORATES]-(ev) WHERE labels(ev)[0] = 'Evidence'
OPTIONAL MATCH (a)<-[:REBUTS]-(dv)       WHERE labels(dv)[0] = 'Evidence'
WITH lc, el, count(DISTINCT ev) AS supporting, count(DISTINCT dv) AS disputing
WHERE supporting > 0 AND disputing > 0
RETURN lc.count_number AS count, el.element_name AS element, supporting, disputing
ORDER BY disputing DESC, supporting DESC;
```

### 4.3 Confirm the corpus-wide REBUTS→Allegation total

The number the Disputes column sums to across all Elements will normally be
**lower** than this: an Evidence item rebutting an Allegation that bears on no
Element is real but unreachable from the Proof Matrix. A large divergence is a
finding about the `BEARS_ON` layer, not about this query.

```cypher
MATCH (:Evidence)-[r:REBUTS]->(:Allegation) RETURN count(r) AS rebuts_to_allegation;
```

---

## Change discipline

If a query here and the code diverge, **the code is authoritative and this file is
a defect.** When a Cypher builder in `causes_of_action_repository` or
`element_detail_repository` changes, update the corresponding section in the same
commit — the provenance rule is only worth anything if the documentation is
current.
