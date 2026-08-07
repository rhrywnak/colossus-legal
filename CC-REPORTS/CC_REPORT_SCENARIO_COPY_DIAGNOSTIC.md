# CC_REPORT_SCENARIO_COPY_DIAGNOSTIC — why did a new scenario arrive full?

**Task:** `CC_TASK_SCENARIO_COPY_DIAGNOSTIC_v1` · **Date:** 2026-08-07
**Type:** READ-AND-REPORT. No code changed, no branch created, no writes.
Every DB statement issued was a `SELECT` or a `\d`; every API call was a `GET`.
No DDL, no temp tables, no scan runs, EMBARGO untouched.

**Working tree at time of reading** — `git branch --show-current` →
`fix/rehearsal-scenario-link`, HEAD `bad51ec`. Nothing checked out, nothing
stashed, nothing modified.

---

## SHORT ANSWER

The title is **not** the key. Nothing anywhere resolves a scenario by its name.
S-3 is a real, distinct, **empty** row, and no content was copied into it. What
Roman saw is the **candidate workbench**, and the workbench pool is not a
property of the scenario — it is derived from the scenario's *subject*. S-3's
`definition` is `{}`, so it names no `target`, so subject resolution falls
through to the **case-default subject** (`CASE_DEFAULT_SUBJECT_NAME = "Marie
Awad"` → `person-marie-awad`) — which is precisely the target S-2 names
explicitly. Same subject → byte-identical 148-card pool. Hypothesis **2
(display-level illusion)**, confirmed by measurement; hypotheses 1 and 3 are
falsified below.

---

## 1. THE THREE HYPOTHESES, DECIDED

### Hypothesis 3 — same-row collision: **FALSIFIED** [measured]

A second row was created. Three scenarios exist, two of them sharing a name, and
the schema permits that — the only unique constraint is on `(case_slug,
code_ordinal)`, not on `name`.

```
ssh core@10.10.100.200 'sudo podman exec -i colossus-postgres \
  psql -U postgres -d colossus_legal_v2 -X -c "SELECT scenario_id, name, code_ordinal, definition, created_at FROM scenarios"'
```

| scenario_id | name | code | definition | created_at |
|---|---|---|---|---|
| `259ca9a8-abe2-4a7f-9eae-7b26fe2582fd` | Marie is obstructive and uncooperative | S-1 | `{}` | 2026-06-29 20:14:31Z |
| `797bc26b-2831-4218-9dea-a4eb12865204` | Refused to divide property amicably | S-2 | `{"target":"person-marie-awad", "schema_v":2, …}` | 2026-07-05 18:03:42Z |
| `6db1d2f2-da33-418b-93d7-a96370a17b05` | Refused to divide property amicably | S-3 | `{}` | **2026-08-07 17:10:52Z** |

`\d scenarios` [measured]: `PRIMARY KEY (scenario_id)`;
`UNIQUE (case_slug, code_ordinal)`; index on `case_slug`. **No index, no
constraint, and no unique key on `name`.** S-3 got its own UUID and its own
code ordinal 3, and the API confirms it renders as itself, not as S-2:

```
GET /api/cases/awad_v_catholic_family_service/scenarios/6db1d2f2…/augmentation
→ 200 {"identity":{"code":"S-3","name":"Refused to divide property amicably",
        "theme_statement":null,"motivation":null,"attack_text":null}, …}
```

Against S-2 the same endpoint returns `"code":"S-2"` with `attack_text` present.
Two rows, two codes, two payloads. Not a collision.

### Hypothesis 1 — data-level copy: **FALSIFIED** [measured]

Every child table was counted for S-3's id:

| table | rows for S-3 |
|---|---|
| `scenario_fact_refs` | **0** |
| `scenario_human_facts` | **0** |
| `scenario_responses` | **0** |
| `scan_runs` | **0** |
| `scan_run_merges` | **0** |
| `scenario_ruling_anchors` | **0** |
| `scenario_status_transitions` | **0** |
| `scenario_candidate_ordinals` | **148** ← see §3, this is identity memoization, not content |

And the read surfaces agree — S-3 is empty everywhere content is persisted
[measured, direct `GET` against `colossus-backend` on 10.10.100.220:3403]:

| endpoint | S-3 | S-2 |
|---|---|---|
| `…/facts` | `[]` (2 bytes) | 75 480 bytes, 83 refs |
| `…/accusation` | `accusation_text: null`, `instances: []`, notice "0 included facts" | `accusation_text` present |
| `…/augmentation` | all identity fields null | populated |

The create path cannot copy: `create_scenario`
([backend/src/api/scenarios.rs:226](backend/src/api/scenarios.rs:226)) validates
name/direction/status, defaults an absent `definition` to `json!({})`
([scenarios.rs:244](backend/src/api/scenarios.rs:244)), and calls
`insert_scenario` — one statement, `INSERT_SCENARIO_SQL`
([scenario_store.rs:150](backend/src/repositories/pipeline_repository/scenario_store.rs:150)),
which inserts eight literal column values plus a code ordinal from the
`case_code_sequences` CTE. **It contains no `SELECT` from any other scenario, no
`ON CONFLICT`, and no reference to `name` other than as the value of `$1`.** No
subsequent statement runs on the create path — the 201 response is built from
the request plus the minted id ([scenarios.rs:272-287](backend/src/api/scenarios.rs:272)),
with no read-back. Nothing to copy from, nothing that copies.

### Hypothesis 2 — display-level illusion: **CONFIRMED** [measured]

All three scenarios return the **same 148-candidate pool**, differing only in the
derived per-scenario status:

```
GET …/scenarios/<id>/facts/gather
```

| scenario | pool | dropped | status histogram |
|---|---|---|---|
| S-1 `259ca9a8` | 148 | 0 | 148 undecided |
| S-2 `797bc26b` | 148 | 0 | 46 included · 102 undecided |
| **S-3 `6db1d2f2`** | **148** | 0 | **148 undecided** |

S-3's workbench is full of the very same evidence cards Roman has been curating
in S-2 — same nodes, same quotes, same order — but **carrying none of S-2's
rulings**. That is exactly the report Roman gave: "it appeared to contain all of
S-2's content." The cards are shared; the curation is not.

The overlap is total, not partial [measured]:

```sql
SELECT count(*) FROM scenario_candidate_ordinals a
  JOIN scenario_candidate_ordinals b USING (graph_node_id)
 WHERE a.scenario_id='797bc26b…' AND b.scenario_id='6db1d2f2…';
→ 148
```

---

## 2. THE EXACT MECHANISM — file, function, query

The candidate pool is derived per request from the scenario's **subject**, and
S-3 has no subject of its own:

1. **[backend/src/api/scenario_gather.rs:283](backend/src/api/scenario_gather.rs:283)**
   `gather_scenario_candidates` — the workbench endpoint. Line 300 resolves the
   subject; line 303-304 reads the pool as
   `BiasRepository::all_evidence_about_subject(&subject_id)`
   ([bias/repository.rs:428](backend/src/bias/repository.rs:428)). The pool
   query is parameterised **by subject id only** — the scenario id never enters
   it. [measured]

2. **[backend/src/api/scenario_gather.rs:406](backend/src/api/scenario_gather.rs:406)**
   `resolve_gather_subject` — re-reads the row and parses `definition`. S-3's
   `{}` fails `ScenarioDefinition::from_value`, and lines 427-437 treat that as
   "not yet authored": logged at `debug`, then substituted with
   `fallback_definition()` — a **target-less** definition. This is documented,
   deliberate policy ("Gather must still show that scenario's pool"), not a bug
   in itself.

3. **[backend/src/services/scenario_subject.rs:86](backend/src/services/scenario_subject.rs:86)**
   `resolve_scenario_subject` — with no target (line 92 short-circuit not taken),
   falls to lines 99-107: `BiasRepository::available_filters(CASE_DEFAULT_SUBJECT_NAME)`
   → `default_subject_id`.

4. **[backend/src/bias/repository.rs:502](backend/src/bias/repository.rs:502)**
   `resolve_default_subject_id` — line 511, `subjects.iter().filter(|s| s.name == configured)`.
   Live value [measured]:

   ```
   colossus-backend env → CASE_DEFAULT_SUBJECT_NAME=Marie Awad
   GET /api/bias/available-filters → default_subject_id = "person-marie-awad"
     subjects: [{"id":"person-marie-awad","name":"Marie Awad","tagged_statement_count":84}]
   ```

   And S-2's stored `definition.target` is **`person-marie-awad`** — the same id.
   The two scenarios therefore gather over one subject. [measured]

**So the causal chain is:** `definition = {}` → parse fails → target-less
fallback → case-default subject → `person-marie-awad` → the identical
148-node pool that S-2 (explicit target `person-marie-awad`) gathers. [inferred
from the four measurements above; each link is individually measured]

The title played **no part**. A scenario created today with any other title and
an empty definition would show the same 148 cards. [inferred — the resolution
chain never reads `name`; the S-1 row is the standing control: different title,
empty definition, same 148-card pool, 148 ordinals minted in one instant.]

---

## 3. WHY 148 ORDINAL ROWS EXIST FOR AN "EMPTY" SCENARIO

`scenario_candidate_ordinals` is the one deliberate write on this read path —
`ensure_candidate_ordinals`
([scenario_gather.rs:352](backend/src/api/scenario_gather.rs:352)), documented at
[scenario_gather.rs:22-41](backend/src/api/scenario_gather.rs:22) as **identity
memoization**: a candidate needs a speakable handle (`C-14`) from first sight,
independent of any ruling. The timestamps confirm it fired on Roman's first page
view, not at creation [measured]:

| scenario | ordinal rows | assigned_at (min → max) |
|---|---|---|
| S-3 | 148 | 2026-08-07 **17:10:57**Z → 17:10:57Z (one instant, **5 s after** the 17:10:52 row insert) |
| S-1 | 148 | 2026-07-27 19:42:55Z (one instant) |
| S-2 | 242 | 2026-07-19 20:30:35Z → 2026-07-27 16:48:36Z (accumulated) |

These rows carry `(scenario_id, graph_node_id, ordinal, assigned_at)` and **no
status, no note, no score** — no curated state was copied. The behaviour matches
the ratified derive-on-read contract exactly.

---

## 4. THE NAME/TITLE-AS-KEY AUDIT (the whole family)

Every site where a scenario's `name` is read, and every scenario lookup key,
enumerated across `backend/src` and `frontend/src`.

### 4a. Scenario lookups — all keyed by UUID or `case_slug` [measured]

| Site | Predicate |
|---|---|
| [scenario_store.rs:231](backend/src/repositories/pipeline_repository/scenario_store.rs:231) `get_scenario` | `WHERE scenario_id = $1` |
| [scenario_store.rs:248](backend/src/repositories/pipeline_repository/scenario_store.rs:248) `list_scenarios_for_case` | `WHERE case_slug = $1` |
| [scenario_store.rs:271](backend/src/repositories/pipeline_repository/scenario_store.rs:271) delete-for-case | `WHERE case_slug = $1` |
| [scenario_store.rs:313](backend/src/repositories/pipeline_repository/scenario_store.rs:313) delete-one | `WHERE scenario_id = $1 AND case_slug = $2` |
| [scenario_store.rs:396](backend/src/repositories/pipeline_repository/scenario_store.rs:396), [:619](backend/src/repositories/pipeline_repository/scenario_store.rs:619) updates | `WHERE scenario_id = … AND case_slug = …` |
| [scenario_status_transitions.rs:83](backend/src/repositories/pipeline_repository/scenario_status_transitions.rs:83) | `WHERE scenario_id = $1` |
| [scenario_store.rs:157](backend/src/repositories/pipeline_repository/scenario_store.rs:157) INSERT | `name` bound as a value only |

Grep for `name = $`, `lower(name)`, `name ILIKE` across `backend/src`: **zero
hits on any scenario table.** [measured]

### 4b. Every read of `record.name` — all display-only [measured]

`scenarios.rs:124` (DTO), `scenarios.rs:237/384/418` (validate + trim on
create/update), `scenario_augmentation_read.rs:83`, `rehearsal_assembly.rs:91`
(`title:` field), `scenario_dashboard.rs:255` and `:368` (`attack:` label). None
reaches a `WHERE`, a `HashMap` key, a dedup, or a match.

### 4c. Routes and the frontend [measured]

Scenario addresses are UUID-based: `scenarioPagePath()`
([frontend/src/utils/routePaths.ts:90](frontend/src/utils/routePaths.ts:90))
composes `/cases/:slug/trial-prep/:scenarioId` with the UUID; the rehearsal
address takes the **code** (`S-1`), documented at
[routePaths.ts:113](frontend/src/utils/routePaths.ts:113). The dashboard keys
cards by `s.id` ([TrialPrepDashboardPage.tsx:187](frontend/src/pages/TrialPrepDashboardPage.tsx:187)).
`ScenarioCreateForm` navigates nowhere — `onCreated()` just closes the form and
bumps a refresh key ([TrialPrepDashboardPage.tsx:170-173](frontend/src/pages/TrialPrepDashboardPage.tsx:170)),
so Roman reached S-3 by clicking its card, i.e. by UUID. No `localStorage` key,
no query parameter, and no `find(… .name === …)` on a scenario anywhere in
`frontend/src`.

### 4d. The one genuine name-as-key in the chain [measured]

**[backend/src/bias/repository.rs:511](backend/src/bias/repository.rs:511)** —
`subjects.iter().filter(|s| s.name == configured)`, matching the configured
`CASE_DEFAULT_SUBJECT_NAME` string against graph node `name` properties. This is
**not a scenario name** — it is a *config-declared* subject name resolved once to
an id, with both the no-match and the ambiguous-match cases logged
(`repository.rs:514-530`). It is nonetheless the only name→identity hop in the
path that produced today's symptom, and it is what makes a target-less scenario
inherit S-2's subject. Listed here for completeness, not as the defect.

---

## 5. WHAT THE CORRECT BEHAVIOUR IS, PER THE RATIFIED REQUIREMENTS

`SCENARIO_FUNCTION_REQUIREMENTS_v2` **§2a** — "Codes are for humans; anchors are
for the machine; neither substitutes for the other." Measured against that, the
create and read paths are **compliant**: identity is the UUID, the human handle
is the `S-n` code, and the title is a label with no load-bearing role anywhere.

**§C2** is equally explicit that the shared pool is *by design*: "C2 items are
references to shared case evidence, keyed by anchor — the same underlying fact
can appear in many scenarios' C2 sets with a different role in each." A new
scenario over the same subject **should** see the same candidates. Nothing in
the observed behaviour contradicts the requirements at the data level.

The gap is therefore not identity and not storage — it is that **an unauthored
scenario is presented as though its full pool were already its own**, with no
surface distinguishing "148 candidates gathered over the case-default subject
because you have not named a target" from "148 candidates gathered over the
target you chose." Two operationally distinct states, one observable — the
Standing Rule 1 shape. The fallback is logged at `debug`
([scenario_gather.rs:430](backend/src/api/scenario_gather.rs:430)) and never
reaches the response body, so the UI cannot tell the human which one they are
looking at.

**Defect class:** *silent default substitution* — an absent input replaced by a
configured default with no observable at the surface that consumed it. It is
**not** the identity-by-name class (`e.speaker`, the composed-route 404s); that
class is absent from this path.

---

## 6. ONE MEASURED ASIDE (out of scope, recorded not pursued)

S-2 holds 83 fact-refs (52 included / 3 dropped / 28 undecided) but gather
reports only 46 included and 0 dropped — because the pool has shrunk from 242
ever-seen nodes to 148 live ones, and refs whose node left the pool are no
longer rendered by the pool-driven walk (`reconcile_candidates`,
[scenario_gather.rs:121](backend/src/api/scenario_gather.rs:121)). All 83 refs
do have ordinal rows, so each was in the pool when it was ruled on. [measured]
This is the known stale-pointer family, unrelated to today's question; flagged
only so the 52-vs-46 discrepancy in this report's own numbers is not read as an
error.

---

## VERDICT: MEASURED

**Answer to Roman's question — "so the scenario title is the key?"** No. The key
is the UUID, and the human handle is the `S-n` code; the title is a label that
appears in no `WHERE` clause, no route, no lookup, no dedup, and no unique
constraint anywhere in the backend or the frontend — S-3 is a genuinely
separate, genuinely empty row that shares nothing with S-2 in Postgres. What
happened when you reused the title is a coincidence of the title being reused at
the same moment as the real cause: you created S-3 without authoring its
`definition`, so its `definition` is `{}`, and the candidate workbench —
which derives its pool from the scenario's **subject**, not from the scenario —
could not parse a target, fell back to the case-default subject
(`CASE_DEFAULT_SUBJECT_NAME = "Marie Awad"` → `person-marie-awad`,
[services/scenario_subject.rs:99](backend/src/services/scenario_subject.rs:99)
via [bias/repository.rs:511](backend/src/bias/repository.rs:511)), and that is
exactly the target S-2 names explicitly — so both scenarios gather the identical
148 Evidence nodes ABOUT Marie Awad, in [api/scenario_gather.rs:303](backend/src/api/scenario_gather.rs:303).
You saw S-2's cards, unruled: 148 undecided against S-2's 46 included. Give S-3
any other title and the result is byte-identical; that is what S-1 already
demonstrates.

*No writes were made to any database, no code was changed, no branch was created,
and no scan was run in the production of this report.*
