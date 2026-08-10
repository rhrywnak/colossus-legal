=== CC REPORT — CC_TASK_R1_SURFACE_REPAIR_BATCH_v1 PHASE A — 2026-08-10 ===

# R1 surface repair — Phase A: change sites, measurements, sizing

**Branch:** `main` · **Last commit:** `7527c4d` (v2.0.0-beta.389) · **Tree:** clean
apart from the untracked `CC-REPORTS/CC_REPORT_SCENARIO_SURFACE_CODE_AUDIT.md`
from the audit. [measured: `git branch --show-current`, `git status --short`]

**Read-only discipline.** No code written, no branch cut, no migration created.
Live DEV work was strictly `SELECT` against `colossus_legal_v2` on 10.10.100.200
— no DDL, no temp tables, no writes. One local read-only typecheck run
(`npx tsc --noEmit` with flags) for the Piece 2 measurement; no build artefacts.

---

## THE FOUR REQUESTED MEASUREMENTS, UP FRONT

### M1 — `noUnusedLocals` cost, and the thing it does not catch (Piece 2)

[measured — `npx tsc --noEmit` with each flag set, from `frontend/`]

| Flags | Errors | Files |
|---|---|---|
| current `tsconfig.json` (`strict`, neither flag) | 0 | — |
| `--noUnusedLocals` | **21** | 15 (2 are `__tests__`) |
| `--noUnusedLocals --noUnusedParameters` | **28** | 19 |

**The finding that changes Piece 2's premise:** `noUnusedLocals` **does not
catch defect 7.** `onFactsChanged` at `ThemeScanPanel.tsx:147` is a destructured
*parameter* binding, and it appears only under `--noUnusedParameters`:

```
ThemeScanPanel.tsx(146,3): TS6133: 'scenarioTitle' is declared but never read.
ThemeScanPanel.tsx(147,3): TS6133: 'onFactsChanged' is declared but never read.
```

Under `noUnusedLocals` alone the same file reports only `writeCollapsed` (:115)
and `collapsed` (:199) — two unrelated dead locals. So the flag the instruction
names would have shipped .389 green with the dead wire intact. If a compiler
guard against this defect class is wanted, **`noUnusedParameters` is the flag
that buys it**, and it is the cheaper of the two to adopt: it adds 7 findings
over the 21.

The 21 `noUnusedLocals` findings, all `TS6133`/`TS6196`, all one-line deletions:

`CardQueue.tsx:96,114` · `CaseHealthViews.tsx:17` · `EvidenceCardParts.tsx:25` ·
`factRowStyles.ts:56,89` · `pipeline/ConfigurationPanel.tsx:33` ·
`pipeline/ProcessingPanel.tsx:186` · `ScanSection.tsx:105` ·
`ScenarioFactsSection.tsx:50` · `ThemeScanPanel.tsx:115,199` ·
`TrialPrepViews.tsx:21,23` · `WorkingView.tsx:30,33,39` · `useSchema.ts:45` ·
`trialPrepHelpers.ts:19` · plus two in test files
(`candidateWorkbench.test.ts:35`, `trialPrepHelpers.test.ts:38`).

The 7 additional `noUnusedParameters` findings: `CandidateCard.tsx:229,230` ·
`FactStack.tsx:74` · `pipeline/PeopleLinksPanel.tsx:39` ·
`RulingButtons.tsx:253` · `ThemeScanPanel.tsx:146,147`.

**Assessment: cheap, and it rides — but as BOTH flags, not one.** 28 deletions
across 19 files, zero of them behavioural. Two are in `WorkingView.tsx` and
`TrialPrepViews.tsx`, which is where the dead `ScenarioSummary` type and the
unused `statusMeta` import sit — worth a glance during Piece 4, since Piece 4
touches `statusMeta`'s vocabulary.

### M2 — DEV `scenario_responses` row counts (Piece 6)

[measured — live DEV, `colossus_legal_v2`]

| Fact | Value |
|---|---|
| total `scenario_responses` rows, whole DB | **1** |
| distinct `scenario_id` | **1** |
| scenarios with more than one row | **0** |
| the owner | S-2 (`code_ordinal = 2`, "Refused to divide property amicably") |
| that row | `status = draft`, `origin = human`, created 2026-08-04, **1** `response_items` row |

**`UNIQUE (scenario_id)` is safe to add.** The constraint would be satisfied by
the current data with room to spare — three of the four scenarios have no
response row at all.

Two observations the constraint does not cover:

- S-5 — the keeper — has **no** `scenario_responses` row, so it has no talking
  points. Both read paths return `Ok(vec![])` before `.first()` is ever reached
  (`rehearsal_assembly.rs:256`, `scenario_augmentation.rs:461`), which is why
  defect 11 has never fired in practice. The constraint is prophylactic, and
  correctly so.
- The single row's `status` is `'draft'`, and `rehearsal_assembly.rs:243-246`
  documents that it deliberately does **not** filter on that column. Unchanged
  by this piece; noted so the constraint is not mistaken for a behaviour change.

### M3 — the layout mechanism (Piece 8)

[measured, except where marked] The construction, outermost first:

| Layer | Site | Property |
|---|---|---|
| document | `styles/index.css:1-6` | `html, body { height: 100% }` — the document is the outer scrollport |
| app shell | `App.tsx:59` | `minHeight: 100vh`, no overflow |
| app header | `Header.tsx:63-67` | `position: sticky; top: 0; height: 56px; zIndex: 100` |
| `<main>` | `App.tsx:62` | `maxWidth: 1080px; margin: 0 auto; padding: 0 2rem` |
| page container | `ScenarioDetailPage.tsx:97-106` | `padding: 28px 40px 80px`, **`marginLeft/Right: -2rem`** — negates the shell's inset |
| section panel | `scenarioSectionStyles.ts:79-86` | **`overflow: hidden`** |
| queue list | `CandidateList.tsx:47-57` | **`maxHeight: 70vh; overflowY: auto`** — a second scrollport |
| rulebar | `RulingButtons.tsx:278` | `position: sticky; top: 0` |

**Four symptoms, three named mechanisms and one that needs the browser.**

**(a) "Outer scroll jams at MacBook-class height" — mechanism named.**
`CandidateList.tsx:48` is a live `maxHeight: 70vh` with `overflowY: auto`. On a
~900px-tall window that is a 630px inner scrollport sitting in the middle of the
page, and the queue is the tallest thing on it. A wheel event over the queue
scrolls the **inner** region; the document only resumes once the inner region
reaches its end (scroll chaining). Because the queue holds 148 cards on S-5, the
inner end is effectively never reached, so the outer page never moves.
*[measured construction; [inferred] that this is what Roman felt as "jams" —
the arithmetic follows, but I have not reproduced it in a browser.]*

Note four doc comments in this family (`CardQueue.tsx:551,564`,
`CandidateCard.tsx:90,208`, `RulingButtons.tsx:262`) describe the 70vh window in
the **past tense**, as a thing that used to be there. It is still there. Whoever
reads those comments while fixing this will be told the opposite of the truth —
worth correcting in the same pass whichever way the piece is ruled.

**(b) "Page opens scrolled to Scan & candidates, identity header off-screen" —
mechanism named.** `CandidateList.tsx:154-158`:

```ts
useEffect(() => {
  selectedRef.current?.scrollIntoView({ block: "nearest" });
}, [selectedId]);
```

This effect runs on **first render** with a selection already made, not only on
subsequent moves. `scrollIntoView` walks and adjusts **every** scrollable
ancestor, including the document — `block: "nearest"` limits how far each one
moves, not how many of them move. The queue sits below the identity block and
the scan card, so on arrival the document scrolls down to bring the first
selected card into view, taking the header off-screen. The comment on the line
("scrolls only when the card is actually out of view, so a selection that is
already visible does not jerk the list") is true of the *inner* region and
silent about the *document*, which is the one that moves.
*[measured mechanism; [inferred] that this is the observed cause — the effect
demonstrably fires on mount, and no other auto-scroll or autofocus exists on this
page (`autoFocus` appears only in `ScenarioIdentityModal.tsx:322` and
`QuestionLine.tsx:140`, both modal-gated).]*

**(c) "Rulebar / facts-header sticky pinning inconsistent" — mechanism named,
and it is two different things.**
- The **rulebar** (`RulingButtons.tsx:278`, `top: 0`) is calibrated to the 70vh
  region — its own comment at `:262` says so — and inside that region it works.
  What it cannot do is pin against the *document*, because its scrollport is the
  inner one. So it pins while you scroll the queue and does nothing while you
  scroll the page: two behaviours from one control, which is exactly "inconsistent."
- The **facts header** has no sticky rule at all (`grep` for
  `position: "sticky"` across `ScenarioFactsSection.tsx`, `ScanSection.tsx`,
  `CardQueue.tsx`, `ScenarioDetailPage.tsx` returns nothing outside
  `RulingButtons` and `Header`). If Roman saw it fail to pin, it is because it
  was never asked to. Additionally, `sectionPanelStyle`'s `overflow: hidden`
  (`scenarioSectionStyles.ts:85`) would defeat any sticky descendant added there
  — an `overflow: hidden` ancestor becomes the sticky containing block, and a
  non-scrolling one means the element never appears to pin.
  `ScenarioIdentityBlock.tsx:49` already overrides that clip to `visible` for an
  unrelated reason (the absolutely-positioned pencil), which is standing evidence
  the clip is over-broad.

**(d) "A dead band grows above the sticky nav as inner containers scroll (the
shell translates)" — NOT named from code. [inferred, low confidence]** No
`transform`, `filter`, `will-change`, or `contain` exists anywhere in the
frontend (`grep` returns only `text-transform` in `tokens.css:291,309`), so the
classic sticky-breaking causes are absent. My best code-level hypothesis is the
page container's `marginLeft/Right: -2rem` (`ScenarioDetailPage.tsx:99-106`)
against `<main>`'s `maxWidth: 1080px`: at MacBook-class widths the negative
margin can push content past the viewport edge, and horizontal overflow on the
document changes how the sticky header and the visual viewport interact.
**This one wants a browser measurement before any CSS moves**, which is what the
instruction asks Phase A to provide and which I cannot honestly provide from the
source alone. The architect's own walk covers the same surface; I recommend the
dead band be characterised there (scroll position vs. band height at 1080×900)
and the ruling made against that data.

### M4 — wording-row count and migration shape (Piece 9)

**The store.** `app_settings` in `colossus_legal_v2`: `key, value, value_kind,
default_value, min_value, max_value, meaning, consumed_by, updated_at,
updated_by`. **252 rows today** [measured]. Namespaces: `rehearsal` 61,
`card` 43, `accusation` 27, `link` 23, `fact` 22, `scan` 16, `scenario` 14,
`talking` 14, `watch` 10, `model` 7, `queue` 5, `theme` 4, and six singletons.

**The migration shape** is settled house pattern — `20260807155419_scenario_
authoring_wording.sql` is the model: one `INSERT INTO app_settings (…) VALUES …
ON CONFLICT (key) DO NOTHING`, forward-only, no down migration, each row
carrying `value`, `default_value`, `meaning` and `consumed_by`. Created only via
`./scripts/new-migration.sh pipeline "<description>"` (CLAUDE.md rule 25).

**The per-row cost is six touch points, not one.** From
`domain/wording_scenario_authoring.rs:104-176`, each key needs: (1) the seed row,
(2) a `pub(crate) const KEY_…`, (3) an entry in the `…_WORDING_KEYS` slice,
(4) a struct field, (5) a `read(KEY_…)?` line in the builder, (6) a DTO field —
then, on the frontend, a TypeScript interface field and prop threading to the
component. **This is what makes Piece 9 the heaviest item in the batch**, not the
migration.

**The deploy hazard, verbatim from the code** (`wording_scenario_authoring.rs:
102-103, 120-121`): *"Renaming one is a migration, and until it runs the boot
loader refuses to start"* — a declared key with no row **refuses boot**. So the
migration must be applied by or with the image that declares the keys. It is
applied at backend boot by the runtime Migrator, so a normal deploy is safe; a
rollback to .389 with .390's rows present is also safe (extra rows are ignored).
The dangerous direction is a .390 image reaching a database whose migration did
not run.

**Count.** Hand-counted from the audit's §4.1–4.4 inventory, counting every
distinct user-visible string including `title`, `aria-label` and `placeholder`
attributes and error sentences:

| Surface | Strings | Notes |
|---|---|---|
| §4.1 scenario detail (header, status control, identity block, facts/orphan sections, page-level notices, delete dialog) | **≈45** | includes the 3-value status vocabulary at `trialPrepHelpers.ts:102-106` |
| §4.2 queue frame (ScanSection, queueRegion, CardQueue, ScanHistoryTable, themeScanFormat, ThemeScanPanel errors) | **≈24** | **minus 3** killed by Piece 7d |
| §4.3 create form | **≈17** | 7 of its strings are already rows |
| §4.4 identity modal | **≈21** | 5 already rows |
| **Total to migrate** | **≈104**, less the 3 Piece 7d kills → **≈101** | |

The instruction's "~90" is the right order of magnitude; the gap is the nine
`ThemeScanPanel` error sentences and the attribute-borne strings (`title`,
`aria-label`), which are user-visible and which I have counted. If the architect
rules attributes out of scope, the number falls to ≈85. **This is a ruling I need
before building** — see the open list.

§4.5 (rehearsal page) and §4.6 (pairing panel) stay as they are: three
documented un-storable strings and zero literals respectively.

---

## PER-PIECE CHANGE SITES

### Piece 1 — the rehearsal link and page (defects 1–5)

| Sub | Site | Change |
|---|---|---|
| 1a | `ScenarioHeaderTiers.tsx:40,227` | import + call `rehearsalScenarioPath(slug, code)`; `code` is already a prop (`:139`) |
| 1b | `ScenarioHeaderTiers.tsx:136-151,226-232` | new `status`-gated render; `status` already a prop (`:142`); **new wording prop** |
| 1c | `RehearsalPage.tsx:298,318-322,338-351` | make the notice and the blocks mutually exclusive |
| 1d | — | no change (front door keeps today's behaviour) |
| 1e | `pages/__tests__/rehearsalAddress.test.ts` | new assertion on the header's composed arguments |

**1b's open question — where the wording row rides.** `ScenarioHeaderTiers`
receives no wording today. The natural carrier is `ScenarioAuthoringWording` →
`identity_wording` on the augmentation payload (`dto/scenario_authoring_wording.rs`,
already consumed by the identity modal), threaded from
`ScenarioDetailPage.tsx:407-420` as a new prop. **Consequence to rule on:** the
augmentation read is one of the four gated reads, and when it fails
`augmentation` is `null` (`ScenarioDetailPage.tsx:127,425-428`). Per the
honest-gap law a control with no words does not render — so on an augmentation
failure the rehearsal link would **disappear** rather than render unlabelled.
I believe that is correct and consistent with `AccusationSection`'s
`if (!panel) return null`, but it is a visible behaviour change on a failure path
and should be ruled, not assumed.

**1c's shape.** `notReady` (`:298`) currently requires `code !== undefined`.
Making the two branches exclusive is a small edit; what it must NOT do is
suppress the blocks when the URL carries **no** code (that is 1d's carousel).
So the condition is: render the notice instead of the blocks **iff a code was
given and did not resolve**.

**1e's assertion, concretely.** The existing guard (`rehearsalAddress.test.ts:
60-81`) reads source text; the same technique extends: assert
`ScenarioHeaderTiers.tsx` contains `rehearsalScenarioPath(slug, code)` and does
**not** contain `rehearsalPath(slug)`. That is a source-text pin, not a
behavioural one — honest about what it buys, and the same shape as the four
assertions already in that file.

### Piece 2 — the dead refresh wire (defects 7–9)

Two call sites, both inside `ThemeScanPanel`: the completion branch
(`:274-287`, beside the existing `refreshRuns()` at `:281`) and the delete
handler (`:403-420`, beside `refreshRuns()` at `:411`). The prop is already
declared, destructured and passed (`:140,147`; `ScanSection.tsx:120,162,289`;
`ScenarioDetailPage.tsx:454`), so **the fix is two calls and a corrected doc
comment** — the comment at `:136-139` still claims a merge path that left this
component (`:551-553`).

The three-key ruling path (`ScenarioDetailPage.tsx:177-225`) is untouched, as
instructed.

**One thing to be careful of:** `onFactsChanged` is wired to `refresh`, which
bumps `pageRefreshKey`, which also feeds `externalRefresh` into `ScanSection`
(`:450`) and therefore reloads the queue's pool and dispatches `cards_loaded`.
On run **completion** that is correct and wanted. On run **deletion** it is a
heavier reload than the piece needs — it will re-run all four reads and, per the
page's own comment at `:218-220`, `cards_loaded` "keeps the human where they were,
clamped to the new length." Deleting a run does not change pool membership, so
the clamp is harmless. Stating it because the comment block at `:177-225` warns
loudly against exactly this kind of over-refresh, and a reviewer will ask.

### Piece 3 — the last-run line (defect 10)

`themeScanFormat.ts:72-95` (`lastRunSummary`) and its caller
`ThemeScanPanel.tsx:515`. The status-aware selection to adopt already exists
eleven lines away at `ThemeScanPanel.tsx:444-451` (`latestCompleted` /
`latestSettled` / `latestFailed`).

**Live confirmation that the defect is currently latent, not active**
[measured]: every scenario's newest run on DEV is `completed`.

| Scenario | newest run status | `candidates_total` | started |
|---|---|---|---|
| S-2 | completed | 148 | 2026-07-29 |
| S-4 | completed | 104 | 2026-08-09 |
| S-5 | completed | 104 | 2026-08-10 |

Run totals by status: S-2 4 completed, S-4 2 completed, S-5 1 completed — **zero
failed or running rows anywhere on DEV**. So the wrong line cannot be
photographed today; the defect is structural and reproduces the moment a scan
fails. Worth knowing for the walk: **Piece 3 has no visible acceptance on
current DEV data** unless a scan is deliberately failed.

### Piece 4 — status honesty at create (defects 12–13)

| Site | Change |
|---|---|
| `ScenarioCreateForm.tsx:51` | `STATUS_OPTIONS` dies |
| `ScenarioCreateForm.tsx:82,216-232,166` | the `<select>` and its state die; creation lands `draft` |
| `api/scenarios.rs:77-85` | `ALLOWED_STATUSES` narrows |
| `trialPrepHelpers.ts:99-107` | `statusMeta`'s `needs_evidence` arm |

**Live precondition [measured]:** DEV holds `draft` 3, `ready` 1, and **zero**
`needs_evidence` rows. The retirement is safe on data.

**Two things to rule on.**

1. **The DB `CHECK` constraint stays.** `scenarios.status` carries
   `CHECK (status IN ('draft','needs_evidence','ready'))` from the 1.1 migration.
   Narrowing `ALLOWED_STATUSES` in Rust closes the write path; it does not
   change the constraint. Dropping the value from the CHECK is a separate
   migration and a harder one (Postgres requires a constraint swap). My reading
   of "task 3.6 folded in" is that the *write path* retires and the column
   vocabulary waits — but the instruction's wording ("retires from the write
   path") supports that reading, so I am flagging rather than assuming.
2. **`statusMeta`'s third arm.** If `needs_evidence` can no longer be written but
   the CHECK still permits it, the read path must keep an arm for it — otherwise
   a hand-written row renders as nothing. I would keep `statusMeta`'s three
   labels and remove only the write-side offer. That contradicts "the vocabulary
   retires" read maximally, so: **ruling wanted.**

### Piece 5 — the identity modal stops losing text (defects 16, 20)

| Sub | Site |
|---|---|
| 5a | `scenarioIdentity.ts:115-147` (`patchFrom`), `:149-162` (`targetWouldBeLost`), `:164-179` (`canSave`) |
| 5b | `ScenarioIdentityModal.tsx:244-260` |

**5a's shape is cleaner than "generalize the guard".** The predicate today asks
"is a target set with no attack text?". Generalised it asks "does the draft carry
any definition content that `patchFrom` would drop?" — i.e. `attackText` blank
**and** (`attackMeaning` or `target` non-blank). One predicate, renamed, plus a
wording row for the refusal (the existing one,
`scenario_identity_target_needs_attack_text`, names the target specifically and
would be wrong for a dropped meaning). **So 5a needs one new wording row** —
folding it into Piece 9's migration is the obvious move.

`ScenarioIdentityModal.tsx:273-276` reads `wording?.target_needs_attack_text`;
that call site changes with the predicate.

### Piece 6 — talking points on a guaranteed row (defect 11)

- New migration via `./scripts/new-migration.sh pipeline "unique scenario response per scenario"`:
  `ALTER TABLE scenario_responses ADD CONSTRAINT … UNIQUE (scenario_id)`.
  The existing `idx_scenario_responses_scenario` (migration
  `20260626135022…sql:43`) becomes redundant once a UNIQUE index exists — worth
  dropping in the same migration, or worth deliberately keeping; either is
  defensible, so it is a small ruling.
- `services/rehearsal_assembly.rs:252-258` gains the `responses.len() > 1`
  warning that `scenario_augmentation.rs:452-462` already carries.
- The third `.first()` — `scenario_augmentation.rs:330`, the talking-point
  **write** — has no warning either. The instruction names only the rehearsal
  read; I would give all three the same guard for one rule in one shape, and
  will unless told otherwise.

**Migration-chain note.** A known standing issue: the migration directory mixes
8-digit and 14-digit prefixes, and 8-digit prefixes sort first, so the chain does
not apply cleanly from zero. Adding migrations in Pieces 6 and 9 does not worsen
it (both apply incrementally on DEV and PROD), but a fresh environment still
cannot be built from the chain. Out of scope here; flagged so it is not
discovered during a .390 rollback.

### Piece 7 — the queue frame

| Sub | Site | Note |
|---|---|---|
| 7a | `candidateFilters.ts:234-254` (`defaultFilters`) | **overturns ruling R5** — see below |
| 7b | `queueRegion.ts:178-212`; `ScanSection.tsx:317-357` | heading composition |
| 7c | `themeScanFormat.ts:155-181`; `ThemeScanPanel.tsx:459-466` | needs a new datum — see below |
| 7d | `ScanSection.tsx:370-380` (Keys line) · `CardQueue.tsx:476-490` (Move line, Next up) · `queueRegion.ts:244-256` (`nextUpHint`) · `CandidateFilterBar.tsx:104-107,168-179` (progress moves onto the chip row) | five lines → two |
| 7e | `candidateFilters.ts:329-352` (`filterProgress`) | **overturns a documented position** — see below |
| 7f | `CandidateFilterBar.tsx:129,150-163` | popup dismiss |

**7a overturns ruling R5, in writing.** `candidateFilters.ts:243-251` is a
doc-comment arguing *for* `full_pool` as the fallback — "opening on a slice would
mean the first thing a human sees is a filtered view they did not choose and
cannot see the edges of." Piece 7a reverses that to `included`. The code change
is one line (`counts.proposed > 0 ? proposed : included`); **the comment must be
rewritten to say what now governs and why R5 was superseded**, or the next reader
finds the file arguing against itself.

**7a's edge case needs a ruling.** On a scenario with 0 proposed **and** 0
included — a never-scanned scenario, or one scanned with everything excluded —
defaulting to `included` opens an empty list. The instruction's carve-out is
"Full pool is never the default *on a scanned scenario*", which implies
never-scanned keeps `full_pool`; the never-scanned case is already special-cased
one layer up (`ScanSection.tsx:257` forces the region closed behind the raw-pool
opt-in). My reading: `proposed > 0 → proposed`, else `included > 0 → included`,
else `full_pool`. **Confirm.**

**7c needs a datum the payload no longer carries.** `api/scenario_cards.rs:
250-259` sets `proposal_source` to `None` the moment `proposed_count` hits 0 —
deliberately, with a comment. So when every proposal is ruled, `proposalSource`
is `null`, `ThemeScanPanel.tsx:465` passes `null`, and
`themeScanFormat.ts:180` renders `String(null ?? 0)` = `"0"`. **That is exactly
7c's "0 proposed" after a full ruling, and the mechanism is by-design, not a
bug** — `themeScanFormat.ts:150-153` argues for the zero explicitly. The fix
needs the run's **original** proposal size, which is already on the wire:
`scan_runs.relevant_count`, selected by `LIST_SCAN_RUNS_SQL`
(`scan_runs.rs:561-566`) and present on the `runs` the panel holds. So 7c is:
compose from `relevant_count` (how many the run put forward) plus the queue's
remaining count, with **two template forms** — "19 proposed · all ruled" and
"19 proposed · N waiting". **Two new wording rows**, and another doc comment that
must be rewritten rather than left contradicting the code.

**7e — the arithmetic checks out against live data.** [measured]
Defer is stored as `status = 'undecided'` **with** `defer_reason` set — there is
no `deferred` status (`cardTriage.ts:387-395`, `candidateFilters.ts:43-53`,
`dto/scenario_card.rs:18`). Live counts:

| Scenario | included | dropped | deferred (undecided + reason) | truly undecided |
|---|---|---|---|---|
| S-2 | 52 | 3 | 10 | 18 |
| S-4 | 20 | 1 | 3 | 0 |
| **S-5** | **21** | **0** | **1** | **0** |

So on S-5 the full-pool line moves from **21 of 148 ruled** to **22 of 148
ruled** — precisely the number the instruction predicts. 7e is implementable and
verifiable on the walk. The change is `filterProgress`
(`candidateFilters.ts:343-352`) counting `deferred` alongside `included` and
`excluded`; and, again, `:339-341` is a doc comment arguing the opposite
("A DEFERRED card is deliberately not counted as ruled") that must be rewritten.

One honest caveat for the architect: DEV's 14 deferred rows carry only **two
distinct** `defer_reason` values, and both read as system-composed lock
explanations ("A scan scored this item, but it is not linked to any accusation
yet… It can only be deferred for now"). So these are humans clicking Defer on a
card that offered nothing else, not humans parking a card they judged. Counting
them as ruled is still right — the human acted — but "22 of 148 ruled" on S-5
means 21 decisions and 1 acknowledged lock.

**7f — the popup does toggle; what it lacks is a way out.**
`CandidateFilterBar.tsx:157` is `onClick={() => setExplaining(!explaining)}`, so
the ⓘ itself closes it. There is **no outside-click handler and no Escape
handler**, and the popup is `position: absolute; zIndex: 60` (`:87-102`) — so it
sits over whatever renders below, which is the ruling acknowledgment Roman
described. The fix is a dismiss-on-outside-click + Escape, not a repair to the
toggle.

### Piece 8 — see M3. Named mechanisms for (a), (b), (c); (d) wants the browser.

### Piece 9 — see M4. ≈101 rows, six touch points each.

### Piece 10 — stragglers

**10a — this is a wording-row EDIT, not new code.** [measured] The string is
already stored: `link_cut_supports_label` = `"It supports us  !!!"` (two spaces
before the `!!!`). A sane sibling already exists: `link_cut_supports_phrase` =
`"it supports us"`. **This collides with the batch's own rule** — "No renames in
this build — wording rows carry their CURRENT text; the naming unification lands
later as row edits." Changing this value *is* a row edit. **Ruling wanted:**
either 10a is an exception (it is a typo, not a name), or it waits with the
rest. My read is that `!!!` is a defect rather than a name and should go, but the
rule as written says otherwise.

The "renders three times" half is separate and is a code change; the render sites
are `HumanLinkSection.tsx:84-111` (chip + `cut_label` + unlink) and the card's own
link block. I could not isolate a third render from source alone — **this one
needs the browser or a screenshot reference**, and I have not confirmed the
triple. Flagged as unmeasured.

**10b — one-line, and the A-code machinery already exists.** [measured]
`domain/scenario_code.rs:105,122` defines `allegation_code()` → `A-45`, and the
backend already uses it for card link chips (`scenario_card.rs:215`) and the link
options catalogue (`scenario_link_options.rs:55`). The identity block's chips do
**not** go through it: they use the frontend composer
`components/allegationLabel.ts:47-50`, which emits `¶${paragraph}`. Two routes:
(i) change that one line to `A-${paragraph}` — cheap, and preserves the module's
stated property that it "composes no vocabulary of its own, only a ¶ and a dash";
(ii) serve a code from `GET /api/allegations`, which is the case-wide generic
read shared with the Allegations page and would change that page too. **(i) is
what I would build**; (ii) is the architecturally purer one and is more work than
this batch has room for. Note the change also reaches the modal's picker labels,
since both call the same composer — the instruction says the picker's *mechanics*
stay, and the tokens changing is exactly what 10b asks for, so this is consistent.

**10c** — `ScanHistoryTable.tsx:235-255` adopts `ScenarioDeleteConfirm`
(`components/ScenarioDeleteConfirm.tsx`, already used at
`ScenarioDetailPage.tsx:561-574` and on the dashboard). The message stays
`wording.delete_confirm_template` via `fillRun`. The row currently owns the
confirm and the panel owns the call (`ThemeScanPanel.tsx:403-420`); the dialog
state will have to move up to the panel, matching how the dashboard puts the
dialog on the page rather than the card (`TrialPrepDashboardPage.tsx:143-146`).

**10d** — `TrialPrepViews.tsx:162-180` (the dashed tile) is removed; the one
create control at `TrialPrepDashboardPage.tsx:214` stays with its current name.
Note `TrialPrepViews.tsx` also holds two of the `noUnusedLocals` findings
(`:21,23`), so this file is touched twice — do both in one edit.

**10e — bigger than "a new settings row".** [measured] The scan default is not
list order today: `api/chat_models.rs:209-213` reads
`state.config.theme_scan_model` (the **`THEME_SCAN_MODEL` env var**), falling
back to `state.default_chat_model`. `catalog.models[0]?.model_id`
(`ThemeScanPanel.tsx:238`) is only the **third** fallback, reached when the
configured default is not scan-eligible — which the handler's own doc comment
at `:181-184` describes as the expected case. So 10e means **migrating
`THEME_SCAN_MODEL` from an env var to an `app_settings` row**: a Rule-2 change
that touches `config.rs`, the Ansible template, and the deploy. It also
intersects standing debt — `THEME_SCAN_MODEL` and `THEME_SCAN_CONCURRENCY` were
added by task D2b and **still owe an entry in the colossus-ansible template**
(a separate repo, and CLAUDE.md rule 3 forbids commingling). **Ruling wanted on
scope**: settings row replacing the env var (clean, with an Ansible follow-on),
or settings row as a *fallback beneath* the env var (additive, no deploy change).

**10f** — `RulingButtons.tsx:346-358` renders the locked explanation on **every**
defer-only card. S-5 has 126 not-ruled cards out of a 148 pool [measured: 148
pool, 21 included, 1 deferred], and essentially all of them are unlinked and
therefore defer-only, which is the "~120 times". The second paragraph is the link
panel's own copy (`RulingButtons.tsx:339-345` describes the interplay). Hoisting
one notice above the queue changes what `RulingButtons` may assume about its own
context, and its doc comment at `:329-345` records that Q4 already removed this
sentence once and it was put back **deliberately** because "a condition most
humans never find is also the PROMISE that Defer will work on this card." So 10f
re-opens a decided question in the opposite direction — hence the instruction
marking it for Roman's confirmation. Sites: `RulingButtons.tsx:346-358` (per-card
short line), `CardQueue.tsx` (new pool-level notice near `QueueNotices`), and one
or two new wording rows.

---

## SIZING VERDICT

**This is two builds, and I do not think Piece 8 is the right seam.**

Rough weight, by touch points rather than by line count:

| Piece | Weight | Why |
|---|---|---|
| 1 | medium | 3 files + 1 test + 1 wording row; the wording carrier needs a new prop path |
| 2 | **small** | 2 call sites + a comment |
| 3 | small | 1 function + 1 caller |
| 4 | small–medium | 4 sites, 1 open ruling |
| 5 | small | 2 files + 1 wording row |
| 6 | small | 1 migration + 2-3 warnings |
| 7 | **large** | 6 files, 3 doc-comment reversals, 2 new wording rows, 1 new derivation (7c) |
| 8 | medium, **and blocked** | mechanism (d) unmeasured; CSS in a nested-scroll construction is the highest-regression-risk work in the batch |
| 9 | **largest** | ≈101 rows × 6 touch points, plus frontend threading through 5 components |
| 10 | medium | six unrelated items, two of which (10e, 10a) have scope questions |

**Recommended split, for the architect to rule on:**

- **.390 — the correctness batch:** Pieces 1, 2, 3, 4, 5, 6, 10c, 10d, plus the
  two compiler flags. Every one of these is a defect with a named mechanism and a
  small blast radius. All of Roman's round-trip symptoms and the stale-banner
  symptom die here. This is a comfortable single build with a real walk.
- **.391 — the surface batch:** Pieces 7 and 9 together, plus 10a/10b/10e/10f.
  They belong together: Piece 7d *deletes* strings Piece 9 would otherwise
  migrate, Piece 7b/7c *creates* rows Piece 9's migration should carry, and
  10a/10e are wording/settings rows in the same migration. Splitting 7 from 9
  means writing one migration, then editing it — or worse, two.
- **.392 or later — Piece 8**, once the dead band is characterised in the
  browser. It is genuinely independent, it is the only piece with no measured
  mechanism for one of its four symptoms, and it is the one most likely to
  produce a regression that a walk catches late.

If Roman wants Piece 8 in .390 as the instruction's seam implies, I would still
lift Piece 9 out — **Piece 9 alone is larger than Pieces 1–6 combined**, and it
is the one whose failure mode is a backend that refuses to boot.

---

## OPEN RULINGS — nothing is built until these land

**Named in the instruction:**

1. **1d** — the no-code front door keeps today's Ready carousel. *(Roman)*
2. **4** — create always lands `draft`. *(Roman)*
3. **7e** — "ruled" = Include + Exclude + Defer; S-5 reads 22 of 148. *(Roman)*
4. **10f** — locked-card explanation hoists to one pool-level notice. *(Roman)*

**Found in Phase A, and each changes what gets built:**

5. **Piece 2 flag choice.** `noUnusedLocals` does **not** catch the dead prop;
   `noUnusedParameters` does. Adopt both (28 deletions), or neither, or only the
   one that buys the guard?
6. **Piece 4 scope.** Does `needs_evidence` retire from the write path only
   (keep the DB CHECK and `statusMeta`'s third arm), or from the read path too?
7. **Piece 9 scope.** Do `title` / `aria-label` / `placeholder` attribute strings
   and error sentences count as wording rows? ≈101 with them, ≈85 without.
8. **Piece 1b failure behaviour.** On an augmentation-read failure the rehearsal
   link would have no words and therefore not render. Correct, or should it fall
   back to rendering unlabelled-but-present?
9. **Piece 7a fallback.** With 0 proposed and 0 included, does the queue open on
   `full_pool`?
10. **Piece 7c datum.** Confirm composing from `scan_runs.relevant_count` plus
    the remaining count, with two template forms.
11. **10a.** Editing `link_cut_supports_label` is a wording-row value edit, which
    this batch's own rule defers to the naming ratification. Exception, or wait?
12. **10e.** Settings row **replacing** `THEME_SCAN_MODEL` (clean; owes a
    colossus-ansible follow-on in a separate repo), or **beneath** it (additive,
    no deploy change)?
13. **Piece 8 (d).** The dead band has no code-level mechanism. Characterise it
    in the architect's browser walk before any CSS moves?
14. **Piece 6 index.** Drop the now-redundant `idx_scenario_responses_scenario`
    when the UNIQUE constraint lands, or keep it?

**Also worth knowing before the walk is planned:** Piece 3 has **no visible
acceptance on current DEV data** — every scenario's newest run is `completed`,
so the wrong "last run" line cannot be photographed without deliberately failing
a scan.

=== END REPORT — VERDICT: STOPPED ===
