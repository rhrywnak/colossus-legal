# PARTY_MERGE_RULINGS_TEMPLATE_v1 — the merge session's worksheet

**Written:** 2026-08-16, task P7 (CC_TASK_PARTY_MERGE_TOOL_ADDENDUM_v1).
**For:** Roman's party-merge session, Saturday step 5 — after the re-key
applies, before the Morris gate test.
**Census below measured live on DEV 2026-08-15, read-only.**

---

## 1 · WHAT YOU ARE DOING, AND WHAT THE TOOL WILL NOT DO

The graph holds **54 `Person` + 8 `Organization` nodes**, and only 8 of the
Persons carry any statement at all. Several of them are the same human being
under two spellings — the worst is the judge, whose 101 sworn statements are
split across `Karen A. Tighe` (39, from the hearing transcript) and `Tighe`
(62, from the opinion).

Ingest will not fix this and should not: it auto-merges only exact and
normalized name matches, because a false merge silently attributes one person's
sworn statements to another. Everything fuzzier is demoted to a new node. That
policy is right, and its consequence is that **the merge pass must happen
before the wave**, by hand, once.

`merge_parties` executes **only what this file says**. No fuzzy matching in the
execution path, no default survivor, nothing merges that you did not name. The
two do-not-auto-merge clusters arrive as `SKIP` and stay that way unless you
change them.

---

## 2 · GENERATING YOUR OWN COPY (recommended)

The census in §5 is a snapshot. Regenerate it against the live graph before the
session so the numbers are today's:

```bash
cd ~/Projects/colossus-legal/backend && cargo run --bin merge_parties -- --emit-template ~/party_merge_rulings.txt
```

That reads the graph and writes nothing to it. Every block it generates says
`SKIP`, so a file handed straight back merges nothing at all.

---

## 3 · THE FORMAT

Four keywords. No punctuation, no indentation rules, no quoting.

```text
# Anything after a hash is a comment. Blank lines are ignored.

CLUSTER Tighe — the judge, split across the transcript and the opinion
SURVIVOR person-karen-a-tighe
MERGE person-tighe
END

CLUSTER Humphrey
SKIP "Jeff" could equally be Jeff Sharp; not merging on a first name
END
```

A block is `CLUSTER <label>`, then **either** one `SURVIVOR` line and one or
more `MERGE` lines, **or** exactly one `SKIP` line with a reason, then `END`.

Rules the parser enforces, each with a line number in its error:

- a `SKIP` needs a reason — it is the record of why a cluster stayed split, and
  without it the next session re-derives that "Jeff" is ambiguous;
- a `SURVIVOR` with no `MERGE` lines is refused (did you mean `SKIP`?);
- `MERGE` lines with no `SURVIVOR` are refused;
- a node named in two blocks is refused, by name and both line numbers;
- a survivor listed as its own member is refused;
- an unknown keyword is refused rather than ignored — a silently skipped line is
  a decision that did not happen;
- an empty file is refused, so an unedited template cannot read as "merge
  nothing".

**Which node should survive?** The one whose id you want to keep living in the
graph — usually the fuller, more canonical name. The merged names are recorded
as `aliases` on the survivor, so "Tighe" stays findable after the merge.

---

## 4 · RUNNING IT

```bash
# 1. Dry run. Writes nothing. Check the numbers.
cargo run --bin merge_parties -- --rulings ~/party_merge_rulings.txt

# 2. Confirm database backups exist (pg_dump + Neo4j). This tool DELETES nodes.

# 3. Apply.
cargo run --bin merge_parties -- --rulings ~/party_merge_rulings.txt --apply
```

**What to check on the dry run:** "Nodes merging in" must equal the number of
`MERGE` lines you wrote, and each cluster's line must show the statement total
you expect after the merge (Tighe: 101).

**Exit codes.** `0` clean · `1` bad rulings file or unwritable report · `2`
connection failure · `3` a cluster was rolled back on a count mismatch — STOP
and read the report · `4` the rulings do not match the live graph, nothing was
written · `5` failure part-way through.

**Acceptance, which the tool proves itself:** statements conserved per cluster,
the referencing-row counts matching, and the People page dropping by exactly the
merged-member count. A cluster that loses a statement is rolled back whole.

---

## 5 · THE CENSUS — measured on DEV 2026-08-15

Statement counts are incoming `STATED_BY` from `Evidence`. **`Judge Tighe` and
`The Court` are recorded as ALIASES on existing nodes, not as separate nodes** —
the "TIGHE ×3" from the 2026-08-13 census is ×2 nodes today, and 39 + 62 = 101
still holds.

| # | Cluster | Nodes (statements) | Note |
|---|---|---|---|
| 1 | **Tighe** | `person-karen-a-tighe` (39) · `person-tighe` (62) | The worst one. One judge, 101 statements split. Aliases already include "Judge Tighe", "the Court", "THE COURT". |
| 2 | Morris | `person-sabrina-morris` (27) · any bare `Sabrina` | Near-certain; the affidavit is hers. Aliases include "Sabrena". |
| 3 | **Humphrey** | `person-jeffrey-humphrey` (26) · any bare `Jeff` | **Do-not-auto-merge.** "Jeff" could equally be Jeff Sharp. |
| 4 | **Sharp** | `Jeffrey Sharp` · `Jeff Sharp` · `Sharp` · `Shaw` (0 each) | **Do-not-auto-merge.** `Shaw` is a probable OCR variant of `Sharp` — your call. |
| 5 | Camille | `Camille Handley` · `Camille Hanley` · `person-camille` (0 each) | OCR n/d confusion plus a bare first name. |
| 6 | James H. | `James Handley` · `James Hanley` (0 each) | Same n/d confusion. |
| 7 | Awad | `Emil Awad` · `Emil Elias Awad` (0 each) | Middle name. |
| 8 | Buk | `Doug Buk` · `Douglas Buk` (0 each) | Diminutive. |
| 9 | Armaly | `Dr. Armaly` · `Dr. Mike Armaly` · `person-mike` (2) | `Mike`'s aliases read "cousin Mike", "the heirs' cousin" — probably NOT Dr Armaly. Your call. |
| 10 | Dalek | `Gerald Dalek` · `Mr. Dalek` (0 each) | Honorific. |
| 11 | Wurdock | `Ms. Wurdock` · `Wurdock` (0 each) | Honorific. |
| 12 | Gerardin | `Judith Gerardin` · `Judy` (0 each) | Diminutive; weaker. |
| 13 | **CFS** | `org-catholic-family-services` (107) · `org-catholic-family-service` (0) | Singular/plural — the defendant, split in two. |

Two more worth a look while you are in there, found in the live census and not
in the 2026-08-13 list:

- `person-william-b-murphy` (39) carries the alias **"the Court"**, exactly as
  `person-karen-a-tighe` does. Two different judges answering to one alias is not
  a merge — it is a reason not to merge on that alias.
- `org-archdiocese-of-detroit` has no `party_name` and no `aliases` at all;
  nothing to merge, but worth knowing it is thinner than its neighbours.

---

## 6 · A STARTING FILE

Copy this, edit it, run it. **As written it merges nothing** — every block is a
`SKIP` until you change it. The one block filled in is the example, and even that
is commented out.

```text
# PARTY MERGE RULINGS — Roman, 2026-08-__

# CLUSTER Tighe — one judge, 101 statements split across two nodes
# SURVIVOR person-karen-a-tighe
# MERGE person-tighe
# END

CLUSTER Tighe
SKIP not yet ruled
END

CLUSTER CFS — singular/plural, the defendant
SKIP not yet ruled
END

CLUSTER Humphrey
SKIP "Jeff" could equally be Jeff Sharp
END

CLUSTER Sharp
SKIP Shaw may be an OCR variant of Sharp; not ruled
END
```

---

## 7 · WHERE THIS SITS IN SATURDAY

deploy → re-key dry-run → re-key `--apply` → twin merge →
**party merge session (this file)** → Morris gate test → wave.

The party merge is a **wave prerequisite**. The wave multiplies duplicate
clusters; running it before this pass makes the People page worse, not better.
