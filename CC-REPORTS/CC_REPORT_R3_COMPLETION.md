=== CC REPORT — CC_TASK_R3_REHEARSAL_PAGE_392_v1 COMPLETION — 2026-08-10 ===

# R3 .392 — the rehearsal page becomes the prep page

**Branch:** `fix/r3-rehearsal-392` · **Commits:** `ead74f2` (payload), `1bbf354`
(page, amended after the gate) · **Base:** `main` @ `cb0cf8c` (.391 merged) ·
**Not pushed, not tagged. .392 rides the same deploy as .391.**

---

## BUILD AND TEST RESULTS — true numbers

| Check | Result |
|---|---|
| `cargo build --workspace` | **clean** |
| `cargo test --lib` | **1793 passed**, 0 failed, 2 ignored (1771 at .391; **+22**) |
| `cargo clippy --workspace -- -D warnings` | **clean** |
| `cargo fmt --check` | **clean** |
| `npm run typecheck` | **clean** (both compiler flags still on) |
| `npx vitest run` | **900 passed**, 65 files |
| `npm run build` | **clean** |
| `./scripts/check-migrations.sh` | **OK** — 76 pipeline migrations, no duplicates |

`cargo test --workspace` and `npm run lint` remain unrunnable for the standing
reasons (`backend/tests/` has not compiled since ~beta.343; there is no lint
script or eslint config). Untouched by this build.

**The migration was executed against the live DEV schema inside a transaction and
rolled back** — `INSERT 0 16`, then `ROLLBACK`. DEV is unchanged.

---

## PHASE A — the three findings, and what the rulings changed

**1. Forum metadata does not exist.** [measured] `Document` nodes carry six
properties — `doc_type`, `id`, `ingested_at`, `source_document_id`, `status`,
`title`. None is a forum, and `doc_type` does not stand in for one: `court_ruling`
covers **both** the Judge Tighe probate opinion (Apr 2012) and the Court of
Appeals ruling (Jan 2012) — precisely the two the forum-wins rule exists to
separate.

Date-only assignment was **rejected**: it tags the appeal "Probate", which is a
known-wrong chip on the page Marie preps from. The approved fix ships as
`rehearsal_phase_document_forums`, a `document-id=Phase` map Roman edits in
Settings with no build. A document not in the map falls through to the date rule
rather than being guessed at.

**Only one document is mapped today** — the Court of Appeals ruling. It is the
only one whose forum I could establish as fact, and it is the one the rule exists
for. Adding the rest is a row edit.

**2. Dates are partial and sparse, and that is fine as-is.** [measured] 228 of
525 evidence nodes (43%) carry a date via `coalesce(event_date, sent_date,
statement_date)`; formats run from `"2005"` to `"2015-10"`. **No precision column
and no date migration were needed** — ISO prefixes sort as time, and `display_date`
already renders partial values raw by deliberate design.

The other side of it: **57% of evidence has no date**, so the chronology has a
real "No date yet" tail, sorted last.

**3. My own near-miss, recorded.** My first query tested for a stored
`occurred_on` property, found zero across every label, and I nearly reported that
dates do not exist at all. They do — `occurred_on` is a derived alias over three
columns. Caught and corrected before it reached a ruling.

---

## WHAT SHIPPED

### The payload (`ead74f2`)

- **Chronology.** Instances sort oldest-first with undated last, on the STORED
  date string. Formatting first and sorting after would give `"1 Dec 2009"` before
  `"15 Nov 2009"` — the defect `rehearsal_timeline` documents. This sort is what
  let the separate TIMELINE section be removed.
- **A plain-words count line, grammatical at ONE.** Seven rows for one sentence:
  .391 deferred the general plural system and this page *opens* with the sentence.
  The date span is omitted, not invented, when nothing is dated; a single shared
  date gets its own clause rather than "from December 2009 through December 2009".
- **A phase on every instance**, forum-then-date, with the undated label as the
  honest third answer.
- **"5 of 5 answered" / "3 of 5 answered — 2 to prepare"**, the second clause
  absent when nothing is outstanding.
- **The identity line's direction word, the verbatim `attack_text`, and the
  bears-on A-codes.**

**16 wording rows, one migration.**

### The page (`1bbf354`)

Read-only. Removed: every edit control, `rehearsalEdits.ts` and its test, the
authorship lines, the "gaps" header counts, the WHAT THIS IS heading, the separate
TIMELINE section, and every collapsible section — a fold on a surface a witness
works from is a place for something to be missed.

New: `PrepTopBlock` (identity line → big unlabelled theme → them-tinted attack →
folded verbatim attack with A-chips beside the control → count line) and
`PrepInstanceCard` (who · date · kind · phase · quote · source, with the paired
answer **inside the same card** behind a green edge, or the loud gap instead).
Phase chips filter; only phases actually present are offered.

**Motivation does not render** — ratified §10 exclusion, re-confirmed today.

---

## WHAT DID NOT SHIP

1. **Talking-point exhibits are not proposed.** `rehearsal_assembly.rs` still sets
   `exhibit: None` unconditionally, so every point renders its "No exhibit yet"
   notice rather than the exhibit its pairing implies. The points themselves
   render; only the "Backed by:" line is a stated absence. *Reason: capacity — it
   needs a pairing→document read this build did not have room for.*
2. **The two .391 leftovers did not fold in** (queue heading naming the active
   filter; collapsed scan summary's run story). *Reason: capacity; the page itself
   took the budget.* Per the addendum, one line each — this is it.
3. The other five .391 leftovers were held out of scope by the addendum and are
   untouched.

---

## THE GATE — all four agents

| Agent | Verdict | Findings | Action |
|---|---|---|---|
| `rules-enforcer` | **FAIL** | 2 module-size, both introduced here | both fixed |
| `architecture-reviewer` | REVIEW | 3 | all fixed |
| `observability-checker` | **FAIL** | 1 | fixed |
| `test-auditor` | **FAIL** | 10 | all addressed |

**Both reviewers independently found the same highest-severity defect**, which is
worth recording because it is the one I would least have wanted to ship: the two
malformed-settings fallbacks in `phase_of` returned the undated label **silently**.
On a record where 57% of evidence genuinely has no date, a mis-edited settings row
would have made every dated card read "No date yet" with nothing anywhere pointing
at the cause. Both arms now warn, naming which key is malformed and what it costs.

Also fixed:

- **`direction_label` silently rendered the defense label for any unexpected
  token** — stating the wrong side of the argument on the page where being on the
  wrong side matters most. Now an explicit three-arm match with a warn, and the
  two schema tokens are named constants.
- **Two module-size violations, both mine.** `rehearsal_render.rs` went 287 → 378;
  `wording_rehearsal.rs` 243 → 307. Split into `rehearsal_instances.rs` (the row
  builders and the chronology comparator), `rehearsal_count.rs` (the two composed
  count lines), and `rehearsal_shape.rs` (`SectionState`). Now **176 / 160 / 68 /
  77 / 288** — every module under 300. My first attempt at the wording split was
  too broad and took the builder with it; reverted and redone narrowly.
- **The chronology comparator was extracted** so its tests exercise the real rule
  rather than restating it — my first three tests re-implemented the comparator,
  which is exactly the "test that proves nothing" this repo rejects.
- **22 new backend tests** covering the count lines (singular/plural, range, single
  date, no dates), the answered line (both forms), the phase rule (forum-wins, the
  appeal specifically, boundaries, year-only dates, both malformed-row fallbacks),
  and the assembly helpers. **Frontend structure pins** for read-only and the phase
  filter, asserting on imports rather than prose — the same trap as .390.

---

## DEPLOYMENT IMPACT

- **Migration:** 1 (`20260810134706_…`), pipeline DB, **16 new rows**.
- **New env vars:** none. **No Ansible change owed.**
- **Deploy ordering:** all 16 keys are declared to the boot loader; a declared key
  with no row **refuses start**. The Migrator handles this on a normal deploy.
- **Container rebuild:** both. **API:** no endpoints added or changed.
- **Scenario rows untouched** — S-5 stays Ready, as required.
- **One deploy carries .391 + .392.**

---

## WALK CHECKLIST — `/rehearsal/S-5`

1. **Theme big and unlabelled**, directly under the small `S-5 · name ·
   direction` line.
2. **The attack, them-tinted**, in plain words.
3. **"▸ The attack in full"** opens the verbatim paragraph; **A-nn chips** sit
   beside the control.
4. **The count line reads grammatically** — check it at whatever count S-5 has.
   If S-5 has one marked instance it must say "1 time", not "1 times".
5. **Cards are chronological**, oldest first, undated last.
6. **Every card carries the answer inside it** behind a green edge — or the loud
   "no answer prepared" gap.
7. **The section count** reads "N of M answered", with "— K to prepare" only when
   work remains.
8. **Phase chips filter**; tapping the active one clears. Expect "No date yet" as
   a chip if S-5's instances are undated — that is the 57% tail, not a fault.
9. **Zero edit controls. Zero authorship lines. No TIMELINE section.**
10. **Points render with "No exhibit yet"** — the known gap above.
11. **S-2 unaffected.**

**Probes for the runbook:**

| Probe | Expected |
|---|---|
| `SELECT count(*) FROM app_settings WHERE key LIKE 'rehearsal_count%' OR key LIKE 'rehearsal_phase%' OR key LIKE 'rehearsal_answered%' OR key LIKE 'rehearsal_direction%' OR key LIKE 'rehearsal_attack%'` | **16** |
| `GET /api/cases/<slug>/rehearsal` | each scenario carries `direction_label`, `attack_text`, `bears_on`; each instance a `phase`; the accusation a `plain_count_line` and an `answered_line` |
| backend boot log | no wording-key refusal; rehearsal count **58** |
| backend log on a rehearsal load | no `phase … list is malformed` warning |

**The question that decides it:** could Marie prep from this page without being
taught? The one place I would watch for a "no" is point 10 — a column of "No
exhibit yet" under her three points is the only part of this page that still asks
her to remember something the system knows.

=== END REPORT — VERDICT: PARTIAL ===
