# CC_REPORT_DEFER_FIX_AND_FACETS — PHASE A (diagnostic)

**Task:** `CC_TASK_DEFER_FIX_AND_FACETS_v1` · **Date:** 2026-08-08
**Branch:** `fix/defer-and-facet-counts`, created off `main` @ `ccc664b` (v2.0.0-beta.385), working tree clean.
**Status: STOPPED at the Phase A gate.** No fix written. **Defect 2 is root-caused to a line. Defect 1 is NOT — and the instruction's leading hypothesis is REFUTED by measurement.** Three rulings needed — §5.

Everything marked **[measured]** was read or run in this tree today. Line numbers are as of `ccc664b`.

---

## 1 — HEADLINE

| | Root cause | Confidence |
|---|---|---|
| **Defect 2** (stale facet counts) | `applyRulingToCard` patches `status` but not `proposed`, so the browser's own Proposed count keeps counting a card it has just ruled | **Certain** [measured] — file:line below, and it explains the exact 25-vs-27 split the architect saw |
| **Defect 1** (defer silent no-op) | **Not established.** The path the instruction suspected is measurably untouched, and the pure state machine is proven correct by 102 passing tests. What remains is the one layer this repo cannot test | **Narrowed to two candidates**, one browser observation apart |

---

## 2 — DEFECT 2, ROOT-CAUSED [measured]

**The line:** [cardTriage.ts:346-364](frontend/src/components/cardTriage.ts#L346) — `applyRulingToCard`, the optimistic patch the queue applies to its own copy of a card the moment a ruling is dispatched:

```ts
case "include":
  return { ...card, status: "included", defer_reason: null };
```

It maintains `status` and `defer_reason`. It does **not** clear `proposed`.

**Why that produces the split.** The Proposed facet reads [candidateFilters.ts:102-104](frontend/src/components/candidateFilters.ts#L102) — `isProposed(card) = card.proposed != null` — over `state.cards`, the queue's own optimistically-patched array. A ruled card keeps its `proposed` object, so it keeps being counted. Meanwhile Included/Excluded read `status`, which the patch DOES maintain, so those move immediately.

The heading and the collapsed summary read a different array. `ScanSection` counts [ScanSection.tsx:198](frontend/src/components/ScanSection.tsx#L198) over the **page's** cards, and the page re-reads them on every confirmed ruling ([ScenarioDetailPage.tsx:457](frontend/src/pages/ScenarioDetailPage.tsx#L457) → `refreshCards`). Those cards come back from the server with no `proposed` on a ruled card — precedence R-a — so every server-sourced number falls while the client-sourced facet does not.

Heading 25 · facet 27 is exactly this: two counts of the same field over two arrays of different freshness.

**Whose defect it is:** mine, from beta.385. `applyRulingToCard`'s own doc says it "extends [the optimistic advance] to the one field that is now visible". .385 added a **second** visible field and did not extend it. The comment was true when written and became false the day I added the facet.

**The fix is one line plus a test**, and the interesting question is only which way `proposed` should go on `undo` — see ruling R2.

---

## 3 — DEFECT 1, AS FAR AS MEASUREMENT REACHES

### 3.1 — The instruction's hypothesis is refuted [measured]

> *"per-node ruling moved to `scenario_ruling_apply` and routes were split in the .385 refactor; the defer path may have lost its reason-collection step or its request wiring in that move"*

It did not. `git diff 9f3bf1d..b9d4dde --stat` over the four files that own the defer path returns **one file**:

```
frontend/src/components/CardQueue.tsx | 47 ++++++++++++++++++++++++++++++++---
```

`cardTriage.ts` (the state machine, including all reason collection), `useQueueReducer.tsx` (the request wiring), and `RulingButtons.tsx` (the ⏸ button) are **byte-identical to beta.384**. And the 44 added lines in `CardQueue.tsx` are the `proposalSource` state, the default-filter comment, the `proposedAttribution` prop and a date helper — `git diff` of that file shows **no line inside the defer path changed**.

So whatever is wrong was wrong in .384 too, or it is in the render layer. It is not a .385 regression in the reason-collection or the request wiring.

### 3.2 — What IS proven correct [measured]

* **The reducer.** `npx vitest run cardTriage` → **102 passed**, including `D opens the prompt on an ordinary card` (asserts `mode = {kind:"deferring", draft:"", graphNodeId:"ev-1"}`) and `a defer button opens the prompt ON ITS OWN card`. The button path and the key path both reach it: [cardTriage.ts:598-612](frontend/src/components/cardTriage.ts#L598).
* **The button is live.** ⏸ is not in the disabled set — only I and E are shut on a defer-only card ([RulingButtons.tsx:255](frontend/src/components/RulingButtons.tsx#L255)) — and its `onClick` dispatches `onRule("d")` ([:281-284](frontend/src/components/RulingButtons.tsx#L281)).
* **A re-fetch cannot close the prompt.** `cards_loaded` returns `{...state, cards, index, ruled, lastRuling, notice: null}` ([cardTriage.ts:429-432](frontend/src/components/cardTriage.ts#L429)) — `mode` is preserved by omission.
* **The request shape is right end to end.** The client sends `{action, reason}` only when a reason exists ([scenarioGather.ts:178](frontend/src/services/scenarioGather.ts#L178)); `FactActionRequest` accepts exactly that ([dto/scenario_facts.rs:250](backend/src/dto/scenario_facts.rs#L250)); `record_ruling` requires a reason for defer and refuses one for anything else.
* **A failed request would be visible.** `useQueueReducer` surfaces a rejection as an on-screen alert AND re-reads the pool ([useQueueReducer.tsx:145-155](frontend/src/components/useQueueReducer.tsx#L145)). The architect saw no error, which is consistent with **no request having been made** — i.e. the human never reached the commit step.

### 3.3 — The two surviving candidates

**Candidate A — the prompt opens where nobody can see it.** The prompt renders *after* the card list ([CardQueue.tsx:509](frontend/src/components/CardQueue.tsx#L509)), and the card list is a `maxHeight: 70vh; overflowY: auto` window ([CandidateList.tsx:44-53](frontend/src/components/CandidateList.tsx#L44)). So the prompt sits below a full viewport-height block, inside a section panel that is itself `overflow: hidden` ([scenarioSectionStyles.ts:85](frontend/src/components/scenarioSectionStyles.ts#L85)). A human who clicks ⏸ on a card near the top of the window is looking ~70vh above the thing that just appeared. Every one of the architect's four observations — no dialog, no state change, no error, no visible anything — is consistent with a prompt that opened correctly and off-screen.

Against it: `deferInputRef.current?.focus()` ([CardQueue.tsx:329](frontend/src/components/CardQueue.tsx#L329)) should scroll the input into view, and the architect would have seen the page jump. Unless the focus lands somewhere the panel's clipping makes invisible, or the ref is null at that moment.

**Candidate B — the click never reaches the reducer in the browser.** Something at the DOM level (an overlay, a stopped event, a remount) swallows it. The unit tests cannot see this: they exercise the reducer directly, not a rendered tree.

**Neither can be settled from source, and that is itself the finding** — see §4.

### 3.4 — The measurement that separates them, in 30 seconds

On DEV, S-4, with devtools open: click ⏸ on any proposed card, then

1. **Ctrl-F the page for "Why defer this?"** — present ⇒ Candidate A (it opened, you could not see it). Absent ⇒ Candidate B.
2. **Network tab** — a `POST …/facts/…/action` ⇒ the commit fired and the failure is later than we think. Nothing ⇒ consistent with both.

I can take this myself with the DEV browser recipe if you log me in (you type the password; I drive from there). It is the honest next step and it is one page-load long.

---

## 4 — THE FINDING UNDER THE FINDING

**This defect class is invisible to the entire test suite, by construction.** CLAUDE.md rule 30 records that component-test infrastructure (RTL, jsdom) is deliberately not set up. So the queue's reducer has 102 tests and its RENDER has none — and a defect that lives between "the state machine is correct" and "the human sees something" cannot be caught by anything we run.

That is precisely the gap the instruction's rider names: *"a ruling click that does nothing is never acceptable UI again."* The rider's test — *a failed ruling action surfaces its failure on screen* — is writable today against `useQueueReducer`'s failure path. **A test that a ruling click PRODUCES something is not**, without either RTL or a browser check in the walk. Worth saying out loud before the fix, because the rider cannot be fully honoured by a unit test alone.

---

## 5 — RULINGS NEEDED

**R1 — is the reason prompt's PLACEMENT in scope?** If Candidate A is confirmed, the honest fix is not to nudge the existing prompt into view — it is to collect the reason **on the card the human clicked**, where every other ruling control already lives. That is a change to the card's anatomy, which your instruction calls mockup territory and tells me to stop on. So: (a) restore visibility minimally in place (scroll-into-view / a sticky footer on the queue), which I can do now and which leaves the reason collection three inches from the card it belongs to; or (b) move it onto the card, which needs the architect's markup first. *My recommendation: (a) now, and file (b) — the current placement predates the 70vh window and was never redesigned around it.*

**R2 — what happens to `proposed` on UNDO?** Clearing it on a ruling is obvious. On `u` the card returns to undecided, and the server would re-propose it (the ruling deleted no verdict, and undo removes the reference row). Options: restore the `proposed` object the card arrived with (the queue keeps it in the undo target), or leave it cleared and let the re-fetch settle it. *My recommendation: leave it cleared and let the server's word arrive — inventing a proposal client-side is the browser asserting something only precedence can decide. The count will be briefly one low, which errs toward under-claiming.*

**R3 — the rider's scope.** *"If a ruling request fails or is never sent, the card must say so."* The FAILS half exists today (`useQueueReducer` alerts and reconciles) and I would move its sentence to a stored wording row and add the behaviour test. The **never sent** half is the interesting one: nothing currently notices a dispatch that produces no effect. I can add that — a ruling that resolves to `effect: none` and no visible mode change is reportable — but it is new behaviour, not a restoration, and it touches every ruling path. Confirm you want it, or confirm the rider is satisfied by the FAILS half plus the walk.

---

**Nothing built. Nothing committed beyond this file's branch. Awaiting R1–R3, and the browser observation in §3.4 if you want me to take it.**

---

## 6 — ADDENDUM (after the architect's rulings): DEFECT 1 IS ROOT-CAUSED, AND IT IS NOT A NO-OP [measured]

The rulings arrived with a new browser measurement — *"NO defer prompt exists anywhere in the DOM … the prompt never MOUNTS"* — and instructed me to diagnose from there. Before building on it I queried the DEV database. **The premise is wrong, and the defect is the opposite of what it looks like.**

### 6.1 — The defer LANDED [measured]

`scenario_ruling_anchors` on DEV, last three days:

```
 ruled_status | rulings |            latest
--------------+---------+-------------------------------
 defer        |       1 | 2026-08-08 20:51:04.19835+00
 exclude      |       1 | 2026-08-08 19:50:42.893162+00
 include      |      30 | 2026-08-08 20:47:23.766021+00
```

That defer is **the most recent ruling of any kind on DEV** — later than the last include. Its detail:

```
ruled_at      | 2026-08-08 20:51:04.19835+00
ruled_by      | roman
scenario      | 4                       (S-4)
graph_node_id | doc-hearing-…-12-15-2009:evidence:10ad7cdb
reason        | "A scan scored this item, but it is not linked to any accusation yet, so
                 there is nothing for it to support or dispute. …"
```

And the state row it wrote:

```
 status    | defer_reason (60)                                  | has_provenance | tagged_at
-----------+-----------------------------------------------------+----------------+------------------------
 undecided | A scan scored this item, but it is not linked to a… | t              | 2026-08-08 20:51:04.20
```

**A ruling anchor, a fact-ref row, `undecided` + `defer_reason`, and `source_run_id` populated.** The .385 provenance fix works. Defer persists correctly, end to end, on beta.385.

### 6.2 — Why no prompt mounted, and it is not a bug [measured]

The recorded `defer_reason` is the SERVER-COMPOSED sentence, which `defer_reason_for` ([scenario_card.rs:344-355](backend/src/services/scenario_card.rs#L344)) emits only for `stance.is_none() && !has_human_link` — a LOCKED card. Confirmed for that node: `evidence_allegation_links` → **0 human links**, and its verdict is `relevant = t, supports, 0.50`, so it was a proposed AND locked card.

That is the `defer_required` short-circuit ([cardTriage.ts:598-604](frontend/src/components/cardTriage.ts#L598)): on a locked card, D accepts the sentence the system already wrote and commits in one press — **deliberately without a prompt**, because prompting would ask the human to retype a sentence the server composed. It is covered by a passing test (`D accepts the server's reason in one press, with no prompt`).

So "no prompt in the DOM" is **correct behaviour for this card**, not a mount failure. (Two notes on the measurement: the prompt is an `<input>`, not a `<textarea>` — [CardQueue.tsx:553](frontend/src/components/CardQueue.tsx#L553) — so a `<textarea>` search returns zero even when it IS mounted; and it would only ever mount for an UNLOCKED card.)

### 6.3 — The actual defect: an invisible SUCCESS

The ruling worked and the screen said nothing that read as success. The likely visible sequence, which fits every observation: the click commits → the page re-reads on confirmation → the fresh payload has a reference row for that node → precedence R-a drops it from the projection → **the card leaves the Proposed filter and simply vanishes from the list**, with no "Deferred" confirmation anywhere the human was looking.

**Defer's defect is therefore feedback, not function.** The rider was aimed at a failure that never happened; what actually needs saying on the card is that the ruling SUCCEEDED — and, for a locked card, that the system supplied the reason.

### 6.4 — What this does to the rulings

* **R1 (inline reason input) survives, with its scope corrected.** It is right for UNLOCKED cards, where the prompt genuinely does mount below a 70vh window. It is *irrelevant to C-13* and to every locked card, which never prompt by design. Building it will not change what the architect measured.
* **D3b ("defer must work on locked cards") is ALREADY TRUE** [measured] — it worked at 20:51:04. What is untrue is the screen. D3b becomes: a locked card's defer must SAY it happened.
* **D3a (say the condition on the card face) is unaffected and right.**
* **R2, R3 unaffected.**

**I have not built anything on the corrected picture.** R1's inline form is still buildable as ruled; the question is whether the architect wants the emphasis moved from "restore defer" to "confirm a ruling landed", since the first is measurably not broken.

=== END REPORT — VERDICT: DIAGNOSED ===
