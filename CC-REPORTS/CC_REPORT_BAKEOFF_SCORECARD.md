=== CC REPORT ===

# CC_REPORT_BAKEOFF_SCORECARD — Opus 4.8 vs Opus 5 on S-4

**Task:** CC_TASK_BAKEOFF_SCORECARD_READONLY_v1 · **Date:** 2026-08-09 · **READ-ONLY.**
Every number below is **[measured]** by `SELECT` against DEV `colossus_legal_v2`.
No writes, no DDL, no temp tables, no builds, no branches. Neo4j was not needed.

**Scenario:** S-4 `0e5c40af-f400-458d-aaf9-ca7e92f50cab` — "Marie Refused to divide property amicably"
**Runs:** Opus 4.8 `08baef15` (Aug 7, 9:37 PM ET) · Opus 5 `2c7b7d87` (Aug 9, 4:43 PM ET)

---

## THE FIVE LINES

1. **Against your ledger, 4.8 caught 20 of 20 includes; Opus 5 caught 17 of 20 — and lost all three to CRASHES, not to judgment.** Opus 5 did not mark a single one of your includes "not relevant".
2. **Head to head on the 104 candidates both judged successfully, they agree on 102 — 98.1%.** Two disagreements, both 4.8-says-relevant / Opus-5-says-not. Zero the other way.
3. **The 7 failures are TRUNCATION, not content declines.** Six replies were cut off mid-sentence inside the JSON, one returned no text at all. `max_tokens: 512` is being shared with Opus 5's adaptive thinking.
4. **Six of the seven had already decided "relevant: true" in the text that survived** — including C-74. The verdicts existed; the budget ran out before `confidence` was written.
5. **The two judges did not run the same prompt** — Opus 5 ran `theme_scan_prompt_v3.md`, 4.8 ran `v2`. Model and prompt both changed between these runs.

---

## 0. The ledger, pulled first [measured]

| Bucket | n | Codes |
|---|---|---|
| `included` | **20** | C-7, 9, 11, 13, 14, 40, 41, 45, 46, 48, 51, 73, 89, 91, 117, 128, 130, 139, 147, and C-8 |
| `dropped` | **1** | C-5 |
| `deferred` (`undecided` + a defer reason) | **3** | C-3, C-42, C-43 |
| `undecided`, no defer reason | 0 | — |

The task anticipated ≈21 includes. The measured figure is **20** (C-73 is among them, so today's ruling is in). Nothing else on S-4 has been ruled — in particular **C-74 is not in the ledger at all**.

---

## 1. The ruled-agreement table [measured]

### Opus 4.8 — `08baef15`

| Ledger bucket | relevant (with role) | not relevant | failed | outside judged set |
|---|---|---|---|---|
| included (20) | **20** | 0 | 0 | 0 |
| dropped (1) | 0 | **1** ✓ | 0 | 0 |
| deferred (3) | 3 | 0 | 0 | 0 |

### Opus 5 — `2c7b7d87`

| Ledger bucket | relevant (with role) | not relevant | failed | outside judged set |
|---|---|---|---|---|
| included (20) | **17** | **0** | **3** (C-8, C-48, C-51) | 0 |
| dropped (1) | 0 | **1** ✓ | 0 | 0 |
| deferred (3) | 0 | 1 (C-3) | 2 (C-42, C-43 — one folded group) | 0 |

**No ruled row fell outside either run's judged set.** The pre-filter and the fold removed nothing you have ruled on — the denominators are directly comparable for every row in the ledger.

**The three includes Opus 5 "lost" are all failures.** It never disagreed with you about an include. Roles differ in six cases (C-7 rebuts/contradicts, C-13, C-40, C-73 supports/corroborates, C-130 corroborates/supports) but relevance never does.

---

## 2. The disagreement list [measured]

On the 104 candidates **both** judged without error: 18 both-relevant, 84 both-not, **2 disagreements**, 0 where Opus 5 said relevant and 4.8 did not.

### C-3 — deferred in your ledger

> **4.8** · relevant, supports, 0.50 — "unknown speaker: at the plan-for-administration hearing, speaker asks whether the personal representative could accommodate sentimental items rather than requiring family to buy back their father's belongings, which bears on the disputed plan to auction the estate's personal property; **it lends context but does not confirm the specific refusal-to-cooperate claim.**"

> **Opus 5** · not relevant, 0.60 — "unknown speaker: an inquiry at the plan-approval hearing asking whether the personal representative would accommodate sentimental items rather than requiring heirs to buy them; **it asserts nothing about Marie refusing to divide property amicably** or about that claim being used to justify the auction, so it does not bear on the accusation's factual claim."

Both read the quote identically. 4.8 admits in its own reason that it does not confirm the claim and marks it relevant anyway; Opus 5 draws the line there. You deferred it.

### C-113 — not in your ledger

> **4.8** · relevant, **contradicts**, 0.50 — "party: Marie Awad's letter lists specific personal property items and asserts she gave much of it to her father, reflecting her engagement in dividing/claiming property rather than a flat refusal to cooperate, cutting against the claim she refused to work anything out."

> **Opus 5** · not relevant, 0.72 — "party: a bare list item enumerating furniture, appliances, a piano and ambulatory aids; **on its own text it asserts nothing** about a proposal to divide, cooperation with the sisters, or any refusal."

The two are describing different things: 4.8 reads a letter that "asserts she gave much of it to her father"; Opus 5 reads "a bare list item". Worth an eyeball at the actual card — one of them is reading context the other is not.

---

## 3. The 7 failures [measured]

Eight verdict rows, seven judged groups — C-42 and C-43 are byte-identical twins folded into one call and share one error, which is the fan-out working as designed.

| Cause | groups | codes |
|---|---|---|
| Reply truncated mid-JSON → `missing field 'confidence'` | **6** | C-8, C-42+C-43, C-51, C-74, C-119, C-129 |
| No text block at all (`only tool_use / reasoning / image blocks`) | **1** | C-48 |

**These are truncation, not content declines, and not oversized inputs.** The evidence:

- **Not one of the seven raw replies terminates.** All end mid-word inside the `reason` string; none ends in `}`. The parser reports "missing field `confidence`" because it hits end-of-input, which is what a cut-off reply looks like from inside serde.
- **They are SHORTER than the successful ones, not longer** — 101 to 328 characters against a successful-reply average of 377 and max of 575. So they did not exhaust the budget with output text.
- **`max_tokens` is 512 on both runs** [measured, `resolved_params`]. Claude Opus 5 runs **adaptive thinking by default**, and `max_tokens` caps thinking and answer *together*. The thinking consumed the budget; the text block was cut wherever it happened to be. C-48 is the same failure at its limit — thinking took everything and no text block was emitted.
- **This is exactly the risk named in the Phase A analysis for beta.388** (§6.1, ruling R6: "no blind cap change; the first Opus 5 re-run is the probe"). This run is that probe, and it came back positive.

**What the seven had already decided**, verbatim from the surviving text:

| Code | Ledger | Surviving reply |
|---|---|---|
| C-8 | included | `{"relevant": true, "proposed_role": "rebuts", "reason": "party: CFS's own sworn interrogatory response` |
| C-42/43 | deferred | `{"relevant": true, "proposed_role": "supports", "reason": "party/counsel: Phillips' own discovery answer describes ongoing \"allegations by` |
| C-51 | included | `{"relevant": true, "proposed_role": "supports", "reason": "party: Phillips's own discovery admission (\"That would be correct\") confirms that the estate's personal property went to auction and that buying` |
| C-74 | — | `{"relevant": true, "proposed_role": "supports", "reason": "party: Phillips admits authoring character` |
| C-119 | — | `{"relevant": false, "proposed_role": "supports", "reason": "court finding … it says nothing about dividing the father's personal property, her sis` |
| C-129 | — | `{"relevant": true, "proposed_role": "supports", "reason": "court finding: Judge Murphy adopts the appellate court's own conclusion rejecting Awad's challenge that the auction was 'unnecessary or ill-conceived'` |
| C-48 | included | *(no text at all)* |

Read against the ledger: **C-8 and C-51 were both heading for "relevant"** — two of the three includes Opus 5 "lost" were not lost to judgment. C-48 is genuinely unknown. Had the six parsed, Opus 5's relevant count would have been 21 groups rather than 16, and its include recall 19 of 20 with one unknown.

---

## 4. The C-74 question — the hypothesis is falsified [measured]

**Opus 5 did not reject C-74. It marked it relevant and was cut off.**

> **Opus 5** — `{"relevant": true, "proposed_role": "supports", "reason": "party: Phillips admits authoring character` → **failed**, truncated before `confidence`.

> **4.8** · relevant, supports, **0.55** — "party: Phillips admits characterizing Marie as unable to get along with her own attorneys — his own words reflecting the broader theme that Marie could not cooperate, which is the rhetorical basis the accusation describes, **though it concerns her attorneys rather than the property-division dispute with her sisters.**"

**Ledger status: not ruled.** C-74 has no row in `scenario_fact_refs` for S-4.

The architect's hypothesis was that Opus 5 rejected C-74 as off-accusation (attorney-smear, not property-division). It did not — as far as its reply got, it agreed with 4.8. The off-accusation concern is real and is in the record, but it is **4.8's own reason** that raises it, in its final clause, while marking the card relevant at 0.55. Both judges pointed the same way here; only one of them finished the sentence. The call on whether attorney-smear material belongs in S-4 is yours, and neither judge has argued against it.

---

## 5. Pool-shape note — the denominators [measured]

| | 4.8 | Opus 5 |
|---|---|---|
| pool read | 148 | 148 |
| excluded — statement type (`referral`) | — | 19 |
| excluded — too short (<60 chars) | — | 16 |
| duplicates folded | — | 9 |
| **judged (groups)** | **148** | **104** |
| verdict rows written | 148 | 113 |

The 4.8 run predates Tier 2 and stored no conservation block; its `candidates_read` and `candidates_total` are both 148, so it pre-filtered and folded nothing.

**Only two candidates 4.8 marked relevant were never judged by Opus 5:**

| Code | 4.8 verdict | Ledger | 4.8's reason |
|---|---|---|---|
| C-109 | relevant, supports, 0.55 | not in ledger | "court finding: Judge Tighe's terse conclusion that there is no agreement adopts the impasse premise underlying…" |
| C-110 | relevant, supports, 0.55 | not in ledger | "court finding: Judge Tighe's remark that these parties will not agree on anything characterizes the family as…" |

Both have **zero verdict rows** in the Opus 5 run, which means they were **pre-filtered, not folded** — a folded member still gets its own verdict row. Neither is in your ledger, so the pre-filter cost the agreement comparison nothing that has been ruled. The remaining 33 dropped candidates were all 4.8-not-relevant.

---

## 6. Two things the task did not ask for, and you should know

1. **The prompts differ.** Opus 5 ran `theme_scan_prompt_v3.md`; 4.8 ran `theme_scan_prompt_v2.md` [measured, `resolved_params`]. This is a two-variable comparison, not a clean model bake-off. v3's successful replies are also longer on average (377 vs 303 chars), which makes the shared `max_tokens` budget tighter for exactly the model that also spends it on thinking.
2. **The economics.** Opus 5: 129s, 359,932 in / 26,239 out, **$2.46**. Opus 4.8: 108s, 517,035 in / 15,463 out, **$8.92**. The input-token gap is mostly the 44 fewer candidates; the cost gap is larger than that alone explains.

Both runs recorded `temperature: null` — the beta.388 resolver fix is live on DEV and the parameter is no longer being sent.

---

## 7. Closing note

Strictly `SELECT` statements against `colossus_legal_v2` on `10.10.100.200`, run through
`sudo podman exec … psql`. No writes, no DDL, no temp tables, no schema changes, and no
Neo4j query was required. `scripts/scan-scorecard.sql` was the starting instrument; it was
read and extended in-session, not modified on disk.

**No recommendation line, per the task.** The numbers are above; the judge is yours.

=== END REPORT — VERDICT: COMPLETE ===
