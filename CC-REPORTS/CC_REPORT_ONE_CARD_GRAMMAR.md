# CC_REPORT_ONE_CARD_GRAMMAR — completion, beta.387

**Task:** CC_TASK_ONE_CARD_GRAMMAR_v1 as amended by
CC_TASK_ONE_CARD_GRAMMAR_PHASE_B_AUTHORIZATION_v1 (rulings R1–R8).
**Branch:** `fix/one-card-grammar`, cut from `main` at `482a68f`.
**Date:** 2026-08-09 · **Scope built:** Pieces 1, 2, 3, 4a, 4b, 5, 7, 8 + R2 + R3
+ R5. **No ruling-write change** — Piece 4c and Piece 6 ride .388 per R7.

All numbers below are **[measured]** by the command named beside them. Nothing is
recalled.

---

## 1. Commits

| SHA | What |
|---|---|
| `b295a0c` | Phase A pre-coding analysis (the report file) |
| `b6808fa` | **Move-only** — the candidate card's body split from its ruling wrapper (R8) |
| `bba6bc9` | Backend — the wording block, the A- codes, the reason that survives include |
| HEAD | Frontend — one evidence card under both wrappers (this commit; it carries this report, so it cannot name its own SHA) |

The move-only commit is separate and reviewable apart, per R8. Not one rendered
pixel changed in it; the behaviour diff that follows reads as behaviour rather
than as a re-indent.

**62 files changed, 5920 insertions, 1910 deletions** **[measured]** —
`git diff --stat 482a68f..HEAD`.

---

## 2. Verification

| Check | Result |
|---|---|
| `npm run typecheck` | **clean** **[measured]** |
| `npx vitest run` | **64 files, 873 tests, 0 failures** **[measured]** — baseline 63/847 |
| `npm run build` | **clean** **[measured]** (the pre-existing >500 kB chunk warning is unchanged) |
| `cargo check --bins` | **clean** **[measured]** |
| `cargo test --lib` | **1737 passed, 0 failed, 2 ignored** **[measured]** |
| `cargo clippy --workspace -- -D warnings` | **clean** **[measured]** |
| `cargo fmt --check` | **clean** **[measured]** |
| `./scripts/check-migrations.sh` | **OK — 71 pipeline migrations, no duplicates** **[measured]** |
| Route-link guard | **19 tests pass** **[measured]** — `npx vitest run src/utils/__tests__` |
| `npm run lint` | **does not exist.** `package.json` has dev/build/preview/test/typecheck only **[measured]**. Per the authorization, typecheck + vitest + the gate are the bar; no lint script was invented. |

**Tests added:** 14 Rust `#[test]`, 45 frontend `it(` **[measured]**. Backend
count moved 1723 → 1737, frontend 847 → 873.

---

## 3. What was built, ruling by ruling

### R6 + Piece 8 — the eighth wording block

`backend/src/domain/wording_card_grammar.rs`: **32 keys**, plus two §2b
thresholds (`card_question_truncate_chars` = 110,
`card_element_chips_visible_k` = 2). The migration
`20260809121531_one_card_grammar_wording_and_settings.sql` seeds **34 rows**
**[measured]** and ships in the same commit as the declaration, because
`build_all_wording` refuses to boot on a declared key with no row.

`wording_card_grammar_tests.rs` reads that migration off disk and asserts every
declared key is seeded **with the value this build expects** (Rule 21), that no
key collides with a sibling block's, and that every shipped template still
carries the slot its caller fills.

**Two compiled-in strings died** (Standing Rule 2):

- `SYSTEM_AUTHORSHIP_LABEL = "System"` — `services/scenario_human_links.rs:139`.
- `attribution: "extracted"` — the `CardSpeaker` literal in
  `services/scenario_card.rs`.

### R2 — "System" is the question-authorship badge, not a speaker

Confirmed re-scope. **[measured, DEV graph, 2026-08-09]** every `STATED_BY` actor
name: George Phillips 158 · Catholic Family Services 107 · Tighe 62 · Marie Awad
54 · William B. Murphy 39 · Karen A. Tighe 39 · Sabrina Morris 27 · Jeffrey
Humphrey 26 · Mike 2. **No "System".** The speaker chip never said it and could
not.

Built: the question folds to one line (Piece 2a), the **speaker chip leads the
card**, and the badge is a stored row saying where the question's TEXT came from.
`system_never_renders_as_a_speaker` re-scoped exactly as ruled — the badge never
occupies the speaker slot, and the speaker renders above the Q:.

"Responding party" is out of scope as ruled: no such property exists to render.

### R3 — the scan's reason survives include

The reason **left `CardProposal` and became `ScenarioCard.scan_reason`** — a
property of the card, not of being proposed. That is the whole fix: precedence
law R-a drops every node with a reference row before the projection groups, so
the field could not survive where it was.

For a ruled card it is recovered at read time by
`list_ruled_card_reasons` (`scan_run_projection.rs`), joining the reference row's
`source_run_id` to that run's verdict — keyed on the **pair**, never the node
alone, so a superseded run's sentence cannot decorate a later ruling. Two SQL
fences assert both properties. Derive-on-read; no migration, no write, R-a
untouched.

A ruling with no provenance serves no reason **and is never queried for one**.

### §2a extended — allegations are A-nn

`domain::scenario_code::allegation_code`, beside `S-` and `C-`. Both composition
sites (`scenario_card.rs`, `scenario_link_options.rs`) call it, so the rename was
one edit rather than two that could disagree. `allegation_code` takes `&str`
because a complaint paragraph is TEXT and the corpus carries values like `72(a)`
— parsing to an integer would point the reader at the wrong paragraph.

The **eight surfaces still rendering ¶** are the ones Phase A filed as
out-of-scope; the list is in §4 of the Phase A report and is unchanged.

### R5 + Piece 1 — five chips

"Rulable now" and "All" retired. `full_pool` replaces `all` in name because the
old word read as a to-do list. Default = Proposed when a run projects, **Full
pool** otherwise. A locked card rides **inside** Proposed, stating its condition
on its own face with the type-ahead ready — which is what let the facet go.

The pool-wide **"23 of 148 ruled" bar and "125 remaining" died** (Piece 1c).
Progress now follows the active filter and names it. "N of M linked" left the
frame (Piece 1d).

### Piece 2 — the one card body

`evidenceCardModel.evidenceCardView` is **one builder over one `ScenarioCard`**,
and both wrappers call it. `WorkingRow` now carries its payload rather than a
thinner projection of it — that projection is what had been discarding the
speaker's provenance, every element chip, the count chip, the anchor highlight,
the context, the band and the reason.

`oneCardGrammar.test.tsx` renders **both wrappers** with
`renderToStaticMarkup` and asserts thirteen named fields appear on each, with the
fact row built through the production `includedRows` rather than by hand.

### Pieces 3, 4a, 4b, 5, 7

- Both bars pin inside their existing scroll regions (70vh / 60vh) — asserted on
  the markup, since a missing sticky rule looks like nothing at all.
- The 120-checkbox wall became a type-ahead. The cut is still asked for, because
  the column is NOT NULL and inventing one would be a claim the human never made.
  A landed link acknowledges itself **on the card**, since the control unmounts.
- The cycling star became a labelled picker; every weight change acknowledges
  itself with undo; **a demotion no longer scroll-jumps** —
  `splitBackground(rows, pending)` holds the row in place while its sentence is
  on screen, which is the mechanism Phase A proposed and is purely testable.
- "Clear my order" left the card for one confirmed "Reset order" in the section
  header. It loops the existing per-fact route over PLACED facts only and reports
  **what actually cleared**, not what was attempted.
- Chips filter their list, matched on the raw payload value; the facts search box
  matches speaker and kind too.

---

## 4. Rule 17

The shared-body extraction is the remedy R8 named, and it worked:

| File | Before | After |
|---|---|---|
| `CandidateCard.tsx` | 342 | **269** |
| `WorkingView.tsx` | 296 | **266** (FactStack extracted) |
| `CardQueue.tsx` | 315 | **287** (useCardsPayload, QueueNotices extracted) |
| `ScenarioFactsSection.tsx` | 218 | **256** (FactsResetOrder extracted) |

Every new file is under the limit. **[measured]** by the §8-equivalent sweep over
non-comment lines.

**Two files remain over, and neither was made worse:**

| File | On `main` | Now | My diff |
|---|---|---|---|
| `components/cardTriage.ts` | 330 | 330 | doc comment only — **no size change** |
| `pages/ScenarioDetailPage.tsx` | 358 | 359 | **+1 line** (the `options={linkOptions}` prop) |

R8 expected the shared-body seam to reduce `cardTriage.ts`. It did not, and the
reason is structural rather than an omission: that file is the reducer — what a
key does — and contains no card-rendering code for the extraction to take. I
moved `progress()` out of it, found it is the reducer's own state derivation that
twelve reducer tests depend on, and put it back with its justification rewritten.
**Reducing `cardTriage.ts` needs its own seam and is not this task's.** Reported,
not hidden.

---

## 5. Deviations from the mockup and the instruction — all deliberate, all stated

1. **Chips are in the shared BODY on both wrappers, not in the fact card's bar.**
   The mockup's `.factbar` puts speaker/kind/cut chips in the fact header; the
   DESIGN (§3.1) says they are "first class, visible, **same position on every
   card**". Those conflict. I followed the design, because a chip row that lives
   in the bar on one wrapper and in the body on the other is exactly the split
   this task exists to end. The fact bar carries drag + code + weight.

2. **The type-ahead is not PRE-FILLED from the document's bears-on edges.**
   R1's display half describes a pre-filled type-ahead, but the control only
   renders on a **locked** card — one with no extraction edge at all — so there is
   nothing to pre-fill it from. Pre-filling would mean showing the control on
   already-linked cards, which is new surface area belonging with Piece 4c. It
   rides .388 with the write it serves. **No wording row was seeded for it**, so
   nothing on screen promises it.

3. **No "Include accepts it" wording anywhere.** R1 defers the write to .388, and
   a card saying Include would accept a link when it would not is a lie on
   screen. The `card_proposed_cut_template` key Phase A sketched was **not**
   created.

---

## 6. Owed follow-ups — found during implementation, not in the Phase A plan

1. **`components/cardRows.ts` is orphaned in production.** Nothing imports it
   but `__tests__/cardTriage.test.ts` **[measured]**. Its role passed to
   `evidenceCardModel`. It still holds `REQUIRED_CARD_ELEMENTS` /
   `missingElements` — **the §7 completeness contract** — which now guards a
   descriptor nothing renders. Recommendation: port that contract onto
   `EvidenceCardView` and retire the file. I did not do it here because retiring a
   §7 contract test is a ruling, not a cleanup.

2. **Eleven wording rows are no longer rendered** **[measured]**: the link
   panel's vocabulary (`link_panel_intro`, `link_allegations_heading`,
   `link_show_all_label`, `link_filter_placeholder`, `link_no_match_notice`,
   `link_empty_options_notice`, `link_save_label`, `link_cancel_label`,
   `link_missing_cut_refusal`) plus `fact_background_move_notice` and
   `fact_unplace_label`. They are still DECLARED, so boot still requires them.
   Harmless today; they should be retired from `domain::wording` with a migration
   when someone is next in that file.

3. **`ScenarioCardsResponse.link_progress` is no longer read** by any surface
   (Piece 1d moved linking status onto the cards). The field is still served.

4. **`LinkToAccusationPanel.tsx` was deleted** — fully superseded by the
   type-ahead and imported by nothing. That is the one piece of orphan removal I
   did take, because it carried no contract.

5. **No frontend formatter is configured.** `npx prettier --check` reports style
   issues in **94 files**, most of them untouched by this task **[measured]** — so
   prettier is not the repo's enforced style and I did not reformat 94 files.

6. **Ansible:** no new env vars. The outstanding `THEME_SCAN_MODEL` /
   `THEME_SCAN_CONCURRENCY` debt from D2b is unrelated and still owed.

---

## 7. Deployment

- **Migration:** one, pipeline DB, additive-only (34 `app_settings` rows, no
  schema change). Rollback is code-only: beta.386 runs unchanged against a
  migrated database.
- **Container rebuild:** BOTH — frontend for the card, backend for the wording
  block, the settings fields, the A- codes and the R3 read.
- **Env vars / Ansible / Traefik:** none.

## 8. The five behavioural probes for BUILD_RUNBOOK_BETA_v1 Step 6

1. The queue opens on **Proposed** with chips, no dropdown.
2. A card leads with the **speaker chip** and a one-line **Q:**.
3. An included fact still shows the **scan's reason**.
4. The **ruling bar stays pinned** while the card scrolls, MacBook-sized window.
5. A **locked card inside Proposed** states its condition and its type-ahead
   lists allegations as **A-codes**.

---

## 9. The §11 gate

All four agents run against the branch. Findings, and what happened to each:

| Agent | Verdict | Findings |
|---|---|---|
| **observability-checker** | **PASS** | None. It verified the R3 read's degrade-with-`warn` is named and justified, and that all four write paths (weight, order reset, links, type-ahead) surface refusals in stored wording. |
| **test-auditor** | FAIL → **fixed** | One in-diff gap: `filterRows` gained speaker and kind to its haystack (Piece 7) with no test covering it. Three tests added — speaker match, kind match, and that a row with neither does not match everything through `?? ""`. |
| **architecture-reviewer** | FAIL → **fixed** | Two findings, both real (below). |
| **rules-enforcer** | FAIL → **fixed** | Two in-diff silent catches (below). Everything else passed across 59 files: no `.unwrap()`/`.expect()` outside tests, every `Deserialize` struct carries `deny_unknown_fields`, all fetches use `authFetch`, no hex literals in style contexts, no bare Cypher relationship names, and the three code prefixes read as structural format identifiers rather than deployment config. |

**architecture-reviewer finding 1 (in diff) — fixed.** The `warn` for a failed
`list_ruled_card_reasons` carried ~18 literal spaces mid-sentence: my edit had
joined two source lines without a `\` continuation, so the whitespace was in the
emitted message, not just the source. Both that agent and observability-checker
saw it independently. The line is now continued properly AND carries a
what-to-do — it names `scan_run_verdicts` in the pipeline database as the place
to look, which the original stated the effect of without stating the recovery.

**architecture-reviewer finding 2 (pre-existing, in a diff file) — fixed.**
`scenario_card_assembly::sort_by_code` read a candidate's ordinal back out of its
code by stripping its own private `"C-"` literal, while
`domain::scenario_code::CANDIDATE_CODE_PREFIX` held the authoritative one. A
rename would have left the sort finding no prefix, treating every code as
un-parseable, and putting the whole pool behind the un-numbered candidates —
silently, because "no ordinal" is a legitimate state. The constant is now
`pub(crate)` and the sort reads it. Taken rather than filed because I had just
added `allegation_code` beside it, and leaving one of three handles duplicated
would have been the worst of both.

**rules-enforcer findings (both in diff) — fixed.** Two `.catch` handlers in
`WorkingView` that cleared the optimistic weight notice and said nothing:
`onSetTier(...).catch(() => setWeightNotice(null))` and, on the undo path,
`.catch(() => {})`.

The agent's reading is right and the mitigation I relied on was invisible at the
site. `ScenarioFactsSection.changeTier` fills the stored
`fact_tier_save_failed_template` into the section's error banner and RE-THROWS
precisely so the caller can retract the notice — the rejection arriving in these
handlers is the retraction TRIGGER, not a second thing to report. But a reader at
the catch could not know that, and the best-effort carve-out is scoped to
cosmetic browser-storage writes, which a refused weight on real evidence is not.

Both now carry the chain in a comment and a `console.warn`, so the flow is
legible from a console alone. The retraction itself is load-bearing and stays: an
earlier version left the notice standing, so a refused write rendered the error
banner AND "C-91 now reads Background" at once — two contradictory messages, one
of them a lie about where a fact went.

**Targeted re-pass — PASS.** The four agents audited the pre-fix SHA, so
rules-enforcer was re-run over the six files that changed after it
(`WorkingView.tsx`, `ScenarioFactsSection.tsx`, `factsTable.test.ts`,
`scenario_cards.rs`, `scenario_code.rs`, `scenario_card_assembly.rs`). It
confirms both catch blocks are resolved and reports no new violation:
**PASS — all 6 modified files comply**.

**One preemptive fix, not from an agent.** `resetOrder` had a bare
`if (!options) return;` on a branch the UI cannot reach. It now warns and shows a
refusal: unreachable-through-the-UI is not the same as unreachable, and a
confirm dialog followed by silence is the shape Standing Rule 1 forbids.

---

**STOPPED after the gate, as instructed.** Roman runs BUILD_RUNBOOK_BETA_v1 for
beta.387. No version bump, no tag, no deploy performed. The Phase A report has
been copied to `~/Documents/colossus-legal/CC-REPORTS/`.
