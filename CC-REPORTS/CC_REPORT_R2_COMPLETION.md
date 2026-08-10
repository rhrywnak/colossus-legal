=== CC REPORT — CC_TASK_R2_SURFACE_BATCH_391_v1 COMPLETION — 2026-08-10 ===

# R2 .391 — a SUBSET of the batch, built and gated

**Branch:** `fix/r2-surface-391` · **Commit:** `500fde6` (amended after the gate) ·
**Base:** `main` @ `94a4166` (v2.0.0-beta.390) · **Not pushed, not tagged.**

**Read this first: this is a partial batch.** Items 1 (part), 2, 4, 5 (part) and
the named renames of item 3 shipped. Seven things did not. Each is listed in
**WHAT DID NOT SHIP** with a one-line reason, per the batch's SKIP clause — but
the honest cause for most of them is capacity in a single pass, not that the item
was unsafe. The naming work was prioritised within item 3 because that is what
tonight's test needs.

---

## BUILD AND TEST RESULTS — true numbers

| Check | Result |
|---|---|
| `cargo build --workspace` | **clean** |
| `cargo test --lib` | **1764 passed**, 0 failed, 2 ignored (1757 on main; **+7**) |
| `cargo clippy --workspace -- -D warnings` | **clean** |
| `cargo fmt --check` | **clean** |
| `npm run typecheck` | **clean** (both compiler flags still on) |
| `npx vitest run` | **902 passed**, 66 files (898 on main; **+4**) |
| `npm run build` | **clean** |
| `./scripts/check-migrations.sh` | **OK** — 75 pipeline migrations, no duplicates |

`cargo test --workspace` and `npm run lint` were **not run and cannot be**:
`backend/tests/` has not compiled since ~beta.343 (17 errors, untouched here), and
there is no lint script or eslint config in `frontend/`. Same as .390.

**The migration was executed against the live DEV schema inside a transaction and
rolled back.** Both fenced UPDATEs matched **exactly one row each**, confirming
they hit their intended targets and no other. DEV is unchanged.

> **A near-miss worth recording.** My first attempt piped the migration file
> straight into `psql` on DEV, which would have APPLIED it to the live database
> Roman is testing on — the one thing the batch's state facts forbid. The
> permission classifier blocked it. The rolled-back form is what should have been
> written first, and is what .390 used. No harm done; recorded because the next
> reader of this file should not copy the first instinct.

---

## WHAT SHIPPED

### Item 4 — ONE ATTACK BOX (Roman, today)

The editor asked the same question twice and the create route seeded both fields
from the single answer the form collects, so every UI-made scenario carried two
byte-identical texts — one rendered by no read surface, and the one the Theme
Scan judged against.

- `ScenarioIdentityModal` drops "What that is meant to imply".
- `authored_definition` seeds `attack_text` and leaves `attack_meaning` `None`.
- The scan judges against `attack_text`, falling back to `attack_meaning` **only**
  when the first is blank — which is what keeps pre-today scenarios scanning
  against the words their author actually wrote.
- The column stays. `patchFrom` carries any stored gloss through untouched, so
  nothing is destroyed.
- The precondition error now names the box a human can see ("has no attack text …
  write what they claim on the scenario's identity") rather than a jsonb key.

The selection was extracted as a pure `judging_criteria` with **four tests**,
including the one both reviewers asked for: that `attack_text` wins even when a
legacy gloss is stored beside it. Reversing that order would silently re-point
every legacy scan at a field its author never filled in.

### Item 3 — the named renames (the part that shipped)

**Nine settings rows** now carry the header's vocabulary, and the read-only block
and the editor render the **same row** — not two rows seeded alike, which is a
pair free to drift the first time either is edited.

| Column | Was (block / editor) | Now, both |
|---|---|---|
| `attack_text` | "The attack — what they claim" / "What they say — quote it if you can" | **"The attack — what they claim, in their words"** |
| `theme_statement` | "Our theme — one sentence" / "Our answer, in one sentence" | **"Our theme — one sentence"** |
| `motivation` | "Their motivation" / "What they want the jury to believe" | **"Their motivation — what they want the jury to believe"** |
| `anchor_allegation_ids` | "Bears on" / "Complaint paragraphs this touches" | **"Bears on"** |

Plus the four stated absences and the theme's helper line. Every one is a row
value, so all of them retune from the Settings page with no build.

### Item 2 — one scrolling page

| Change | Site |
|---|---|
| the queue's `maxHeight: 70vh; overflowY: auto` **dies** | `CandidateList.tsx` |
| the facts table's `60vh` **dies too** | `WorkingView.tsx` |
| the mount-time `scrollIntoView` no longer moves the document on arrival | `CandidateList.tsx` |
| the ruling bar's sticky offset moves from the dead 70vh box to the document, clear of the 56px header | `RulingButtons.tsx` + a new exported `APP_HEADER_HEIGHT_PX` |
| `sectionPanelStyle`'s `overflow: hidden` → `overflowX: hidden` | `scenarioSectionStyles.ts` |
| the four past-tense doc comments corrected | 4 files |

**Why the second scroller had to go too**, though the batch named only the first:
the acceptance is one continuous scroll from the header to the watch-list, and
the facts table sits between them. Leaving 60vh would have moved the jam down the
page rather than removing it.

The `overflow: hidden` change is the one to review hardest: an `overflow: hidden`
ancestor becomes the sticky containing block, so a non-scrolling one makes every
descendant's `position: sticky` inert. `ScenarioIdentityBlock` had already
overridden the clip wholesale for its pencil, which was standing evidence it was
broader than any caller wanted. One axis keeps the corner-clipping the mockup
asked for.

### Item 1 — the queue frame (the part that shipped)

- Default filter: proposed → **included** → full_pool. **Ruling R5's comment is
  rewritten** to record what now governs and why, rather than left arguing the
  opposite of the code.
- The progress line measures the **proposed bucket** — everything the scans put
  forward, plus anything acted on — so it no longer moves when a chip is clicked.
  **Include, Exclude and Defer all count as addressed**; the deferred-not-ruled
  comment is likewise rewritten. On S-5's live data this is why it reads 22.
- It moved onto the chip row, right of the chips.
- **Three teaching lines died**: the Keys legend, the Move legend, and "Next up:".
  The letters are printed on the buttons that do the same thing. **The keys still
  work** — only the text went.
- The ⓘ popup gained outside-click (`mousedown`, so it clears before the click
  lands underneath) and Escape. It always toggled; what it lacked was any other
  way out, and it sat over the ruling acknowledgment.

### Item 5 — stragglers

- **10a (value half)**: `link_cut_supports_label` "It supports us  !!!" → "Supports us".
- **10b**: `allegationLabel` emits `A-41` instead of `¶41`, so the identity chips
  and the card link chips finally call one allegation one thing.
- **10e**: `theme_scan_default_model` is a settings row read **beneath** the env
  var — env var → row → chat default. The previous third step was list order.
  Extracted as a pure `scan_default_model` with three tests. **No Ansible change
  rides with this**; retiring the env var stays filed.

---

## WHAT DID NOT SHIP — seven items, each with its reason

1. **The bulk ~101-literal → row migration (item 3's main body).** Only the nine
   named identity rows landed. *Reason: capacity in one pass — ~101 rows × six
   touch points each, and the naming was the part tonight needs.*
2. **Plural-aware templates.** "Said 1 times" still reads that way. *Reason: it is
   a renderer change on both sides plus a row-shape change across every
   count-bearing row; not safely done in the time left.*
3. **Queue heading naming the active filter ("Included — 21").** *Reason: the
   heading lives in `ScanSection` and the active filter lives in `CardQueue`;
   joining them means lifting the queue's most-used state. The chip row already
   shows the active filter and its count, so the information is on screen.*
4. **Collapsed scan summary "19 proposed · all ruled".** Still reads "0 proposed"
   once everything is ruled. *Reason: needs two new rows plus a new datum
   (`scan_runs.relevant_count`) threaded to the panel.*
5. **10a's render half** — the linked allegation's text still appears three times
   per card. *Reason: the row value was the cheap half; deduplicating the renders
   is a card-layout change.*
6. **10f — the locked-card explanation is still per-card**, not hoisted. *Reason:
   moving it changes what `RulingButtons` may assume about its context, and Q4
   already reversed this once.*
7. **Trial Prep subtitle / "Draft" tile / "pattern analysis pending" chip.**
   *Reason: part of item 3's bulk, above.*

**Also not done, and deliberately:** the `.390` row
`scenario_identity_meaning_needs_attack_text` still says "the words you just
typed", which is now wrong — that box is gone, so the refusal is reachable only on
a legacy row. It was left alone because the disk/code fixture test pins values
against their INSERT, and an UPDATE would have put the database and that fixture
into silent disagreement. **Roman can retune it from the Settings page in ten
seconds.**

---

## THE GATE — all four agents

| Agent | Verdict | Findings in this diff | Action |
|---|---|---|---|
| `rules-enforcer` | **PASS** | none | — |
| `architecture-reviewer` | REVIEW | 2 | both fixed |
| `observability-checker` | REVIEW | 2 | both fixed |
| `test-auditor` | **FAIL** | 4 | all four fixed |

**Both reviewers independently found the same highest-severity item**, which is
worth noting: the scan's fallback had two operationally distinct outcomes and one
observable (none). A scan is one billed LLM call per candidate, the definition is
mutable and the run is not — so an operator asking "what was this run judging?"
six weeks later had the verdicts, the model and the count, and no way to recover
the criteria. Fixed with a `tracing::info!` naming the source field before any
call goes out, and the legacy path names itself out loud.

Everything else fixed in the amend:

- **`ValidatedScan.attack_meaning` / `PreparedScan.attack_meaning` renamed to
  `scan_criteria`.** After the ruling that name was a lie in the normal case — the
  value comes from `attack_text` — and a downstream reader would have believed
  they were reading a gloss.
- The three-tier model resolution logs which step answered.
- **The stale frontend fixture** (`cardFixtures.ts` still carried
  `{ruled} of {total} {filter} ruled`). `fillSlots` leaves an unknown slot
  verbatim, so a future test would have produced `"3 of 5 {filter} ruled"` and
  passed. Corrected, with a render-site pin.
- **The backend card-grammar fixture had the same problem, one layer deeper.** Its
  disk/code test compared against the original INSERT and could not see a later
  correction, so the fixture would have gone on agreeing with history while the
  database held something else. The test now follows corrections, the UPDATEs were
  reformatted to the house convention that reader parses, and the `{filter}` slot
  requirement was retired with the rule it named.
- `defaultFilters`' new middle branch and the three-tier resolution each gained
  tests; the criteria selection was extracted to make it testable at all.

---

## DEPLOYMENT IMPACT

- **Migration:** 1 (`20260810114629_…`), pipeline DB. **10 new rows** (9 identity
  + 1 scan default) and **2 fenced corrections**.
- **New env vars:** none. **No Ansible change owed by this build.**
- **Deploy ordering:** all 10 keys are declared to the boot loader, and a declared
  key with no row **refuses start**. The Migrator handles this on a normal deploy.
  Rolling back to .390 leaves 10 unread rows and two corrected values — harmless.
- **Container rebuild:** both.
- **API:** no endpoints added or changed. `POST /cases/:slug/scenarios` now stores
  `attack_meaning` absent rather than duplicated — the only wire-visible change.
- **Scenario rows:** untouched, as required. S-5 stays Ready.

---

## WALK CHECKLIST — one pass

1. **Scenario page opens at the header**, not at Scan & candidates.
2. **One continuous wheel scroll** from the header to the watch-list. No jam over
   the queue, none over the facts table.
3. **Ruling bar pins** while ruling, below the app header, not behind it.
4. **Identity block and Edit modal use the SAME four names** — "The attack — what
   they claim, in their words", "Our theme — one sentence", "Their motivation — …",
   "Bears on".
5. **The modal has ONE attack box.** "What that is meant to imply" is gone.
6. **Queue frame is two lines**: heading, then chips + the count on the right. No
   Keys line, no Move line, no "Next up:". Press **I / E / D / U** — they still rule.
7. **The count reads "N of M addressed"** and does **not change** when you click
   between chips. On S-5: **22 of 22**.
8. **ⓘ on Full pool** — click away, and it closes. Escape closes it.
9. **Bears-on chips read A-nn**, not ¶nn.
10. **A scenario with proposals cleared opens on Included**, not the full pool.

**Probes for the runbook:**

| Probe | Expected |
|---|---|
| `SELECT count(*) FROM app_settings WHERE key LIKE 'scenario_identity_%'` | **14** (5 pre-existing + 9 new) |
| `SELECT value FROM app_settings WHERE key='card_filter_progress_template'` | `{ruled} of {total} addressed` |
| `SELECT value FROM app_settings WHERE key='link_cut_supports_label'` | `Supports us` |
| `SELECT value FROM app_settings WHERE key='theme_scan_default_model'` | `claude-opus-5` |
| create a scenario, then read its `definition` | `attack_text` set, **`attack_meaning` absent** |
| backend log at scan start | `theme scan: judging criteria resolved` with `criteria_source="attack_text"` |
| backend boot log | no wording-key refusal; scenario-authoring count **25** |

=== END REPORT — VERDICT: PARTIAL ===
