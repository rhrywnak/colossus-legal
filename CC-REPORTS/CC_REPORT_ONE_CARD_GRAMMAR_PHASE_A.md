# CC_REPORT_ONE_CARD_GRAMMAR_PHASE_A — pre-coding analysis

**Task:** CC_TASK_ONE_CARD_GRAMMAR_v1 (rides as beta.387)
**Date:** 2026-08-09 · **Type:** Phase A. No code written, no file in `backend/` or
`frontend/` modified, no branch created, no migration authored.
**Sources:** ONE_CARD_GRAMMAR_DESIGN_v1.md · ONE_CARD_MOCKUP_v1_2026-08-09.html ·
SCENARIO_FACTS_REDESIGN_v1.md · SCENARIO_FUNCTION_REQUIREMENTS_v2 §2a/§2b/§2c/§2d/§7/§8/§10 ·
the repo at `482a68f` · three read-only queries against the DEV graph.

Findings are labelled **[measured]** (a query or a command produced the number) or
**[read]** (the source file says so at the cited line). Nothing below is inferred
without saying so.

---

## 1. Task Understanding

A candidate and an included fact are the same evidence item at two lifecycle
stages, and today they render as two unrelated layouts built from two unrelated
descriptors. This task makes them one card body — header chips, Q:/A: with the
anchor highlighted, pinpoint, A-codes, compressed element chips, the scan's
reason, context behind a click — with only the wrapper differing: ruling controls
on the candidate, weight/order/Remove on the fact. Around it: filter chips replace
the status dropdown, progress follows the active filter, both bars pin, and
type-ahead replaces the checkbox wall.

One write-path change is in scope (Piece 4c) and it **does not have the data the
design assumes**. That is ruling R-1 below and it is the reason this report stops.

---

## 2. Branch Verification

- Current branch: `main` **[measured]** — `git branch --show-current`
- Working tree clean: **YES** **[measured]** — `git status --porcelain` empty
- Last commit: `482a68f chore: bump version to v2.0.0-beta.386`
- Task branch `fix/one-card-grammar`: **not yet created.** It will be cut from
  `482a68f` as the first act of Phase B, not before — this report is Phase A.

**Baseline, measured now, not recalled:**

| Check | Result |
|---|---|
| `npm run typecheck` | clean **[measured]** |
| `npx vitest run` | 63 files, **847 tests, 0 failures** **[measured]** |
| `npm run lint` | **no such script.** `package.json` has `dev/build/preview/test/typecheck` only **[measured]** — CLAUDE.md §6 names `npm run lint`; it does not exist in this tree. Reported, not worked around. |
| `cargo test --workspace` | known broken on `main` (stale `tests/*.rs` since ~beta.343). The honest backend baseline is `cargo test --lib`. Not re-run in Phase A — it costs a full build and no backend file is touched until a ruling lands. |

---

## 3. What is already true (so the plan does not rebuild it)

**[read]** unless noted.

| Design claim | Reality in the tree |
|---|---|
| "The question is already on the wire" | Yes. `CardQuote.question` (`services/scenarioCards.ts:77`) and `WorkingRow.question` (`components/factsTable.ts:201`). Both wrappers already render it. |
| Kind is served on the candidate payload (Piece 7's open question) | **Yes — no fallback needed.** `api/scenario_cards.rs:110` reads `all_evidence_about_subject`, which projects `e.statement_type` (`bias/queries.rs:221`). It reaches `ScenarioCard.statement_kind` and `WorkingRow.statementKind`. **[measured]** 19 distinct values, 543 classified Evidence nodes on DEV — `factual_assertion` 89, `court_finding` 74, `referral` 71, `evasive` 53, `admission` 43, `partial_admission` 40, `attorney_argument` 38, … |
| Speaker is served | Yes, `CardSpeaker.name` from the `STATED_BY` edge (`services/scenario_card.rs:418`), and on the fact row as `WorkingRow.speaker`. |
| Elements + count exist | Yes, on the **candidate** only: `CardBearsOn { accusation, elements, count }` (`services/scenario_card.rs:237`). The fact row throws all three away — `includedRows` keeps `b.accusation` and drops `elements` and `count` (`components/factsTable.ts:181`). |
| Anchor highlight | Candidate only. `CandidateCard.tsx:486` wraps the quote in `<mark>`. The fact card renders `row.text` as plain body text (`FactRowParts.tsx:233`). Confirms the C-91 finding. |
| Queue scroll region | `CandidateList.tsx:47` — `maxHeight: 70vh; overflowY: auto`. Facts list — `WorkingView.tsx:67` — `maxHeight: 60vh; overflowY: auto`. **Both already have a scroll container, so `position: sticky` on the bars is a styling change, not a restructure.** Piece 3 is the cheapest piece in the task. |
| Two descriptors, not one | `cardRows(card)` (candidate, `components/cardRows.ts:71`) and `sectionsFor(row)` (fact, `components/factsTable.ts:115`). They share no type. This is the defect Piece 2 exists to remove. |

---

## 4. The four findings that need a ruling

### R-1 — Piece 4c: **the scan does not propose a link. It proposes a cut.**

**[read]** `scan_run_verdicts` carries exactly four judged columns:

```
relevant BOOLEAN · proposed_role TEXT · confidence REAL · reason TEXT
```

(`pipeline_migrations/20260715121130_create_scan_runs_and_verdicts.sql:118-151`;
the read is `RELEVANT_VERDICTS_SQL`,
`repositories/pipeline_repository/scan_run_projection.rs:100`, and the row struct
`RelevantVerdictRow` at `:88` has no fifth field.)

**There is no allegation id anywhere in a verdict.** `CardProposal` on the wire is
`{role_label, reason, duplicate_count, duplicate_label}` — no allegation
(`services/scenarioCards.ts:170`). So the mockup's *"Scan proposes the link:
supports A-55 — Include accepts it"* has no stored value behind "A-55".

The only allegation the card holds is `bears_on` — the **extraction's** graph
edges, not the scan's judgment. And a card with a non-empty `bears_on` is already
rulable, so it is not the locked card the type-ahead was designed for. Concretely:

- Card **with** `bears_on` → not locked; the "proposed link" would be the
  extraction's allegation wearing the scan's cut.
- Card **without** `bears_on` → locked (`defer_required`), and the scan proposes
  nothing to accept. **[measured, 2026-08-04, quoted in the migration header]** 104
  of S-2's 148 candidates carry no extraction edge at all.

**Three options. I recommend (b).**

| | What Include would write | Cost | Honesty |
|---|---|---|---|
| **(a) As designed** | Compose the proposed link as `bears_on[0].accusation` + `proposed_role` as the cut; Include writes a `evidence_allegation_links` row alongside the ruling. | ~1 route change + card composition | **Weak.** The card would say "the scan proposes this link" about an allegation the scan never saw. It attributes an extraction fact to the judge. |
| **(b) Say what is actually proposed** *(recommended)* | The card renders "Scan proposes: **supports** — Include accepts it", cut only, and the type-ahead sets the *allegation*. Include commits ruling + link in one action whenever an allegation is present (proposed-by-extraction or typed by the human), with the cut taken from `proposed_role`. | Same route change, honest sentence | Strong. Nothing is attributed to the scan that it did not say. Roman's one-action goal is met. |
| **(c) Defer 4c** | Include writes only the ruling, as today; the type-ahead ships read/write but stays a separate save. | Zero backend change; Piece 4 becomes frontend-only | Loses the "no save-the-link-first step" win, which was the point. |

**If (a) or (b): the exact write path, named as the instruction requires.**

`POST /cases/:slug/scenarios/:scenario_id/facts/:graph_node_id/action` →
`api::scenario_facts::apply_fact_action` (`api/scenario_facts.rs:397`). It already:
resolves the proposal server-side (`resolve_proposed_ruling`, `:423`), takes
`source_run_id` from it (`:424`), expands the twin set (`:425-428`), and calls
`rule_one` per target (`:437-451`). The accepted link would ride as **one added
optional field on `FactActionRequest`** (`dto/scenario_facts.rs:250`) —
`accepted_link: Option<{allegation_id, cut}>` — written inside the same handler
through the existing `evidence_allegation_links` writer, once per ruling (not once
per twin: a link is case-wide, `pipeline_migrations/20260804132730:21`).

`source_run_id` discipline is **unchanged**: it stays server-derived at
`:424`. The client never supplies it, and the accepted link is a *different* field
that does not touch it. That is the one property the design's "provenance
discipline unchanged" clause is asking for, and it holds.

**Nothing here is touched until Roman rules.**

### R-2 — Piece 2e: the scan's reason is deleted on include **by construction**, and restoring it is a backend read change

**[read]** Precedence law R-a: `project()` drops every node that has a
`scenario_fact_refs` row *before grouping* —
`services/scenario_card_projection.rs:120`, `.filter(|v| !ruled.contains(...))`.
`CardProposal` is therefore absent the instant a card is included, and
`card.proposed.reason` — the only place the reason lives on the wire — goes with
it. The frontend cannot fix this; there is nothing to render.

Two ways to give the reason back. **I recommend (i).**

- **(i) Read-time join, no migration.** An included card's ref row already carries
  `source_run_id` (`ScenarioFactRefRecord`,
  `repositories/pipeline_repository/scenario_store.rs:477`, written since
  2026-08-08). One extra `SELECT reason FROM scan_run_verdicts WHERE run_id = $1
  AND graph_node_id = ANY($2)` over the included set, folded into the card
  assembler, restores the reason on every fact that came from a proposal. Facts
  included before `source_run_id` was written carry NULL and honestly show no
  reason — a real, distinguishable state, not a gap.
- **(ii) Copy the reason onto the ref row at ruling time.** Needs a migration and
  a write change, and duplicates a value that is already stored — the thing the
  projection design deliberately avoided.

Either way this is **backend work inside a piece the instruction bills as
"frontend"**. Flagged, not smuggled.

### R-3 — Piece 2b: **"System" is not a speaker, and never was**

**[measured]** Every `STATED_BY` actor name on the DEV graph:

```
George Phillips 158 · Catholic Family Services 107 · Tighe 62 · Marie Awad 54
William B. Murphy 39 · Karen A. Tighe 39 · Sabrina Morris 27
Jeffrey Humphrey 26 · Mike 2
```

Nine names. **No "System".** The speaker chip cannot render it today.

**[read]** What Roman saw is the **question-authorship badge**:
`SYSTEM_AUTHORSHIP_LABEL = "System"` (`services/scenario_human_links.rs:139`),
served as `question_authorship.label` and rendered by `QuestionLine.tsx:200` as
"⚙ System" — sitting directly under a seven-line interrogatory question, in the
exact position a reader takes for the attribution of the text above it. The design
brief read the screen correctly and named the wrong field.

This is better news than the design assumed. The fix is three small things, none
of which needs data we do not have:

1. Collapse the question to one line (Piece 2a) — the badge stops sitting under a
   wall of text.
2. Move the speaker chip **above** the Q: line, first-class (Piece 2b) — the card
   answers "who said this" before it says anything else.
3. Retire the compiled-in `"System"` for a wording row saying what it means
   (it is the machine's transcription of the question, not a speaker). **That
   const is a standing Rule-2 violation in its own right** and this task is the
   right place to kill it.

The instruction's fallback — *"where nothing honest exists, the chip says 'speaker
not extracted'"* — is still needed, for genuinely speaker-less documentary
evidence (`CardSpeaker.name` is `None` there by design,
`services/scenario_card.rs:420`). Today that chip is simply absent. It becomes a
wording row.

**Ruling asked:** confirm the target is the authorship badge + speaker-chip
promotion, and that "the document's responding party" is **dropped** from scope —
there is no responding-party property on Document to render, and inventing one
would be the guess §2b forbids.

### R-4 — Piece 2c: the A- code law touches **two** composition sites, and one more surface than the instruction expects

**[read]** `¶` is composed at runtime in exactly two places, both `format!("¶{paragraph} — {body}")`:

- `services/scenario_card.rs:193` — feeds `bears_on[].accusation` **and** the
  stance line `"This supports ¶41 — …"`.
- `services/scenario_link_options.rs:49` — feeds `AllegationOption.label`, the
  type-ahead's own list, **and** `CardHumanLink.label`, which is quoted verbatim
  into the composed sentence `"You linked this to ¶41 — … · they'll use it
  against us"` (`services/scenario_human_links.rs`).

Changing both to `A-{paragraph}` converts every surface this task touches in one
edit each. **The third surface is `human_link_summary`** — a pre-composed sentence
that will start reading "You linked this to A-41 — …" automatically, which is
correct and worth knowing before it surprises anyone.

**Surfaces still rendering `¶` after this task — filed, not scope** (per the
instruction's "Phase A reports any OTHER surface still rendering ¶"):

| Surface | Site |
|---|---|
| Allegations page | `pages/AllegationsPage.tsx:254` |
| Graph page | `pages/GraphPage.tsx:318` |
| Element ↔ allegation list | `components/ElementAllegationList.tsx:162` |
| Element detail sort control | `components/ElementDetailContent.tsx:364` |
| Proof review matrix | `components/ProofReviewViews.tsx:206` |
| Scenario identity modal | `components/ScenarioIdentityModal.tsx:177` |
| Shared label helper | `components/allegationLabel.ts:48` |
| Node-type display helper | `components/nodeTypeDisplay.ts:62` |

Eight surfaces, all outside the queue and the facts list. The global sweep is
explicitly out of scope; this is the list for whoever picks it up.

---

## 5. Sizing — and the split I recommend

**Today is a ruling day, and this does not fit one clean build.**

**[measured]** Non-comment line counts on the files this task must change
(CLAUDE.md Rule 17 limit: 300):

```
342  components/CandidateCard.tsx     ← ALREADY OVER
330  components/cardTriage.ts         ← ALREADY OVER
315  components/CardQueue.tsx         ← ALREADY OVER
296  components/WorkingView.tsx       ← one edit from over
218  components/ScenarioFactsSection.tsx
190  components/FactRowParts.tsx
...
```

Three of the five files at the centre of this task are already past the limit
before a line is added. That is a pre-existing overage this task must not worsen —
and, usefully, the shared-card-body extraction *is* the remedy: pulling the body
out of `CandidateCard` shrinks it **and** is the thing the fact wrapper needs.

**Recommended split — the design's own §8 seam, which is also the instruction's:**

| Build | Pieces | Why the seam is here |
|---|---|---|
| **beta.387** | 1, 2, 3, 4, 5, 7, 8 | All frontend + the two `¶`→`A-` edits + one wording/settings migration + (pending R-1/R-2) two small backend reads/writes. Ends at a working screen: one card everywhere, chips, pinned bars, type-ahead, labelled weight. |
| **beta.388** | 6 (the marked line) | The only piece with a **schema** change, a new selection interaction, and a new write route. Bundling it means the .387 build cannot be verified until the migration is right. |

This is exactly the split the instruction pre-authorises ("Pieces 1–5 ride .387;
Piece 6 rides .388"), with 7 and 8 riding .387 because they are not separable from
the card and the wording they belong to.

**Second-line contingency:** if R-1 lands as option (a) or (b), Piece 4c adds a
DTO field, a handler branch and its tests to a build that is already the largest
of the three. If Roman wants .387 smaller, **Piece 4c is the clean thing to hold
back** — 4a/4b (the type-ahead replacing the checkbox wall) ship without it and
are the bigger usability win.

---

## 6. Files to Modify (Pieces 1–5, 7, 8 — the .387 build)

| File | Changes | Est. lines |
|---|---|---|
| `frontend/src/components/CandidateFilterBar.tsx` | Dropdown → five filter chips + ⓘ popup; "Clear filters" and the "Filtered: n of n" line die (1a, 1b) | −60 / +50 |
| `frontend/src/components/candidateFilters.ts` | Facet set → Proposed/Deferred/Included/Excluded/Full pool; per-filter progress derivation; `defaultFilters` re-based (see §11 Q2) | ±80 |
| `frontend/src/components/CardQueue.tsx` | Wire chips; progress line follows the filter; `link_progress` leaves the frame (1c, 1d) | −40 / +25 |
| `frontend/src/components/queueRegion.ts` | `progressFromCards` → filter-scoped | ±25 |
| `frontend/src/components/cardTriage.ts` | `progress()` → filter-scoped (`:826`); no other change — this file is over the limit and gets smaller or stays flat | ±15 |
| `frontend/src/components/cardRows.ts` | Becomes the **shared** descriptor over the new view-model; A-codes; bears-on compression; elements/count/reason/context rows | +90 |
| `frontend/src/components/CandidateCard.tsx` | Body moves out to the shared component; keeps the ruling wrapper, receipt, defer form. **Ends well under 300.** | −150 |
| `frontend/src/components/QuestionLine.tsx` | Collapse to one line + unfold (truncation length from settings); authorship label from wording, not the const | ±45 |
| `frontend/src/components/FactRow.tsx` | Adopts the shared body; header row becomes sticky | ±50 |
| `frontend/src/components/FactRowParts.tsx` | `TierControl` star-cycling → labelled picker; `HeaderRow` gains chips | ±70 |
| `frontend/src/components/factsTable.ts` | `WorkingRow` gains elements, count, reason, anchor quote, context, cut, confidence; `splitBackground` honours a pending-demotion set (the no-scroll-jump mechanism, §8) | +70 |
| `frontend/src/components/ScenarioFactsSection.tsx` | "Reset order" moves into the section header with confirm + acknowledgment (5b) | ±40 |
| `frontend/src/components/WorkingView.tsx` | Reset-order wiring; demotion without reflow | ±35 |
| `frontend/src/components/candidateCardStyles.ts` · `factRowStyles.ts` | Sticky bar styles; the second `mark` style | +30 |
| `frontend/src/services/evidenceLinks.ts` | ~14 new `LinkPanelWording` keys (§8 below) | +40 |
| `frontend/src/services/scenarioCards.ts` | Payload fields added by R-2 / R-1, if ruled in | +15 |
| `backend/src/services/scenario_card.rs` | `¶`→`A-`; elements/reason/context/anchor served on **included** cards too | ±40 |
| `backend/src/services/scenario_link_options.rs` | `¶`→`A-` | ±3 |
| `backend/src/domain/wording.rs` (or a new block — see §11 Q4) | Declare the new keys | +40 |
| `backend/src/domain/settings.rs` | Two thresholds: Q truncation length, element-compress K | +12 |

## 7. Files to Create

| File | Purpose | Est. lines |
|---|---|---|
| `frontend/src/components/EvidenceCardBody.tsx` | **The one card body**, rendered identically by both wrappers | ~180 |
| `frontend/src/components/evidenceCardModel.ts` | The pure view-model both wrappers build, and the pure `evidenceCardRows` over it — this is what makes "same payload fields" a real test rather than a hope (Rule 30) | ~140 |
| `frontend/src/components/AllegationTypeahead.tsx` | Piece 4a — replaces `LinkToAccusationPanel`'s checkbox wall | ~140 |
| `frontend/src/components/QueueFilterChips.tsx` | Piece 1a/1b — chips + ⓘ popup (reuses `InfoPopup.tsx`) | ~90 |
| `frontend/src/components/WeightPicker.tsx` | Piece 5a — the labelled three-tier picker | ~80 |
| `frontend/src/components/__tests__/evidenceCardModel.test.ts` | The shared-body contract tests | ~200 |
| `frontend/src/components/__tests__/queueFilters.test.ts` (extend existing) | Filter chips + per-filter progress | ~90 |
| `backend/pipeline_migrations/<ts>_one_card_grammar_wording.sql` | Wording rows + two settings rows, seeded **in the same migration that the code declaring them ships in** | ~120 |

`LinkToAccusationPanel.tsx` and `cardLinking.ts` are **superseded, not deleted**, in
.387 — the panel's save path is what the type-ahead reuses. Deletion is a follow-up
once the type-ahead has been verified on DEV.

---

## 8. Migration ↔ struct side-by-side (standing rule)

**.387 migration is wording + settings only — no table, no column.**

| Migration row (`app_settings`) | Declared in | Field | Kind |
|---|---|---|---|
| `card_question_truncate_chars` | `domain/settings.rs` | `card_question_truncate_chars: usize` | number |
| `card_element_chips_visible_k` | `domain/settings.rs` | `card_element_chips_visible_k: usize` | number |
| `queue_filter_proposed_label` | `domain/wording.rs` | `queue_filter_proposed_label` | text |
| `queue_filter_deferred_label` | ″ | ″ | text |
| `queue_filter_included_label` | ″ | ″ | text |
| `queue_filter_excluded_label` | ″ | ″ | text |
| `queue_filter_full_pool_label` | ″ | ″ | text |
| `queue_full_pool_explainer` | ″ | ″ | text (the ⓘ popup) |
| `queue_filter_progress_template` | ″ | ″ | text, carries `{ruled}` `{total}` `{filter}` |
| `card_question_expand_label` | ″ | ″ | text |
| `card_question_machine_authorship_label` | ″ | ″ | text (**retires the `"System"` const**) |
| `card_speaker_not_extracted_label` | ″ | ″ | text |
| `card_elements_more_template` | ″ | ″ | text, carries `{count}` |
| `card_context_toggle_label` | ″ | ″ | text |
| `card_proposed_cut_template` | ″ | ″ | text (R-1 (b) only) |
| `fact_weight_picker_label` | ″ | ″ | text |
| `fact_weight_changed_template` | ″ | ″ | text, carries `{code}` `{tier}` |
| `fact_reset_order_label` / `_confirm` / `_done` | ″ | ″ | text ×3 |

**Boot law honoured:** a key declared in `domain/wording.rs` with no row is a boot
refusal (`build_all_wording` → `require`, `services/settings_wording.rs:70`). The
migration therefore **ships in the same commit** as the declaration, never after.

**Template-fill trap, from the record:** the backend's `render` takes **unbraced**
keys (`verb`), while every frontend fill takes braced (`{when}`). A wrong key ships
the raw token to screen. Every template above is filled **frontend-side** except
`card_proposed_cut_template`, which is composed server-side and must use the
unbraced form.

**Piece 6 (.388) migration, previewed:**

| Column on `scenario_fact_refs` | Type | Struct field on `ScenarioFactRefRecord` |
|---|---|---|
| `marked_line` | `TEXT NULL` | `pub marked_line: Option<String>` |
| `marked_line_updated_by` | `TEXT NULL` | `pub marked_line_updated_by: Option<String>` |
| `marked_line_updated_at` | `TIMESTAMPTZ NULL` | `pub marked_line_updated_at: Option<DateTime<Utc>>` |

No `NUMERIC` (the beta.364 lesson). The column must also be added to
`SCENARIO_FACT_REF_COLUMNS` (`scenario_store.rs:581`) or the read silently omits it.

---

## 9. Tests to write — behaviour only

All pure-module tests; there is no RTL/jsdom in this tree (Rule 30). The
`evidenceCardModel` seam is what makes the first four testable at all.

| Test | Module | What it asserts |
|---|---|---|
| `a_candidate_and_a_fact_serve_the_same_card_payload_fields` | `evidenceCardModel` | Both builders produce the same field set from equivalent payloads |
| `the_question_renders_collapsed_and_never_leads_the_card` | `evidenceCardModel` | Speaker/kind precede Q:; Q: is truncated to the settings length |
| `the_anchor_quote_is_highlighted_on_both_wrappers` | `evidenceCardModel` | The anchor segment is marked in both view-models |
| `system_never_renders_as_a_speaker` | `evidenceCardModel` | The authorship label can never occupy the speaker slot |
| `allegations_display_as_a_codes_on_touched_surfaces` | `scenario_card` + `scenario_link_options` (Rust) | Composed labels start `A-`, never `¶` |
| `element_chips_compress_beyond_k_and_expand_on_demand` | `evidenceCardModel` | First K visible, remainder behind `+N more` |
| `the_scans_reason_survives_include` | `scenario_card` (Rust) | An included card with a `source_run_id` carries its reason (R-2 (i)) |
| `include_accepts_the_proposed_link_in_one_action` | `scenario_facts` (Rust) | One request writes ruling + link (R-1) |
| `overriding_the_proposed_link_before_include_writes_the_override` | ″ | The human's allegation wins over the proposal |
| `linking_a_locked_card_wakes_ruling_and_acknowledges` | `cardLinking` | `defer_required` clears; a receipt is produced |
| `a_weight_change_acknowledges_and_never_scroll_jumps` | `factsTable` | **Mechanism:** `splitBackground(rows, pendingDemotions)` keeps a just-demoted row in `shown` until its acknowledgment is dismissed — so the row does not leave the list under the cursor. Purely testable; no DOM. |
| `progress_counts_follow_the_active_filter` | `candidateFilters` | Progress is over the filtered set, never the pool |
| `marked_line_persists_and_never_touches_the_anchor` | .388 | — |
| `marking_never_triggers_regathering` | .388 | — |

No shape-pinning. No test restates code.

---

## 10. Standing Rule Compliance

- **No silent failures.** Every new write (weight, reset order, accepted link,
  marked line) follows the established shape in `ScenarioFactsSection.tsx:176-256`:
  call → re-read → `.catch` filling a stored failure template. The type-ahead's
  refusals reuse the stored sentences the panel already carries, so the browser
  pre-check and the server 400 cannot say two different things (R4 precedent).
- **No hardcoded values.** Two new thresholds and ~18 wording rows, all from the
  store. The task **removes** one existing violation: the `"System"` const at
  `services/scenario_human_links.rs:139`.
- **Tutorial comments.** Every new module gets a `## Why:` header; the shared
  view-model gets a `## TS Learning:` note on the discriminated union that keeps
  the two wrappers' inputs apart while producing one output.
- **Rule 17.** Three target files are already over. Net effect of this plan is
  `CandidateCard.tsx` **342 → ~190**; `CardQueue.tsx` and `cardTriage.ts` stay flat
  or shrink. No new file is planned above 200 lines. This will be re-measured
  before commit with the same command used above.

---

## 11. Deployment Impact

- New env vars: **None.** (No Ansible template change owed. The outstanding
  `THEME_SCAN_MODEL` / `THEME_SCAN_CONCURRENCY` debt from D2b is unrelated and
  still owed.)
- Migrations: **one** (.387, wording + settings, pipeline DB `colossus_legal_v2`).
  A second in .388 (marked line). Both via `./scripts/new-migration.sh`.
- Container rebuild: **both** — the frontend for the card, the backend for the two
  `¶` sites, the wording block and the settings fields.
- Traefik / auth: **None.**
- Rollback: revert the branch merge and roll back to beta.386. The .387 migration
  is additive-only (new rows in `app_settings`, no schema change), so beta.386 runs
  unchanged against a migrated database — the rollback is code-only.

## 12. Verification Plan

- Local: `npm run typecheck` · `npx vitest run` · `cargo check --bins` ·
  `cargo clippy --workspace -- -D warnings` · `cargo fmt --check` ·
  `cargo test --lib` · route-link guard test · Rule-17 size sweep.
- Gate: §11 agents on the final SHA; targeted re-pass on any amend. (Triaged
  mine-vs-pre-existing — these agents audit whole files, and three of them are
  already over the limit.)
- Report: `CC-REPORTS/CC_REPORT_ONE_CARD_GRAMMAR.md`, `[measured]` labels.
- DEV, MacBook-sized window, S-4: the five acceptance walks named in the
  instruction's VERIFICATION block.
- **STOP after the gate.** Roman runs BUILD_RUNBOOK_BETA_v1 for beta.387.

---

## 13. Open Questions for Roman

**Q1 — R-1.** The scan proposes a **cut**, not a link. Option (a) as-designed,
**(b) say what is actually proposed** *(recommended)*, or (c) defer 4c?
**Nothing is written to `apply_fact_action` until this is answered.**

**Q2 — the filter set.** Piece 1a names exactly five chips. That retires
**"Rulable now"** and **"All"** as named facets — but `defaultFilters`
(`candidateFilters.ts:246`) opens the queue on `rulable` when nothing is proposed,
and "Full pool" is not the same thing. Should "Rulable now" survive as a sixth
chip, or should the no-proposals default become **Proposed → Deferred → Full
pool**? I recommend the latter: five chips, and the default falls through them in
order.

**Q3 — R-2.** Restore the reason by read-time join (recommended, no migration) or
by copying it onto the ref row (migration)?

**Q4 — wording block.** `domain/wording.rs` is the curation block and is at its
Rule-17 ceiling. New block `domain/wording_card_grammar.rs` (an eighth), or extend
an existing one? I recommend the eighth — the precedent is already six deep and
the seam is "one surface's words".

**Q5 — R-3.** Confirm the "System" fix targets the **question-authorship badge**,
and that "the document's responding party" leaves scope — no such property exists
to render, and inventing one is the guess §2b forbids.

**Q6 — the split.** Confirm **.387 = Pieces 1–5, 7, 8** and **.388 = Piece 6**;
and say whether Piece 4c should also be held to .388 if you want .387 smaller.

---

**Read-only note:** this report ran three `MATCH` queries against the DEV Neo4j
instance and no writes of any kind. No temp tables, no DDL, no document
reprocessed, no LLM call issued. Nothing in `backend/` or `frontend/` was
modified, and no branch was created.

**STOP. Awaiting rulings on Q1–Q6 before any code.**
