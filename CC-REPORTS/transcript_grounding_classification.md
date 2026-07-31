# Transcript Grounding Failure Classification

**Document:** `doc-hearing-to-approve-plan-for-adminnistration-12-15-2009` (court_transcript, 37 pages, Surya OCR)
**Build under study:** v2.0.0-beta.360, branch `feature/scenario-1a`
**Report date:** 2026-07-25
**Type:** read-and-report. No code changed, no file in `backend/` or `frontend/` touched, no document
reprocessed, no LLM call issued, Re-verify & Sync not pressed. Only this report file and the
`CC-REPORTS/` directory were created.

---

## 0. Method — and why the numbers below can be trusted

Everything in this report was derived from three sources:

1. **The pipeline database** (`colossus_legal_v2` on the DEV host) — read-only `SELECT`s against
   `documents`, `document_text`, `extraction_items`, `extraction_relationships`, `extraction_runs`.
2. **The shipped matcher source** — `backend/src/api/pipeline/canonical_verifier.rs` and
   `backend/src/api/pipeline/verify.rs` at HEAD (`8b3feb4`).
3. **A line-for-line Python port of the shipped matcher**, used offline so that every failing item
   could be dissected without re-running the pipeline.

The port is not taken on faith. It was **validated by replay**: it was run over all 138 stored items
and its verdict compared to the `grounding_status` the real pipeline wrote on 2026-07-25.

```
total items: 138   mismatches: 0
  Evidence exact       37    Evidence normalized  42    Evidence not_found  44
  Party    exact        9    Party    normalized   5    Party    not_found   1
```

138/138 agreement, including which tier (`exact` vs `normalized`) fired. Every counterfactual number
in §4 is therefore a measurement of the shipped algorithm, not a model of it.

Throughout, **measured** means it came out of a query or a validated replay. **Inferred** means I am
reasoning past the evidence, and it is labelled as such.

---

## 1. Baseline — one correction before anything else

| Metric | Instruction said | **Measured** |
|---|---|---|
| Items total | 138 | **138** ✔ |
| Verified (grounded) | 94 | **93** |
| Not verified | 44 | **45** |
| Grounding rate | 68% | **67.4%** |
| Relationships in `extraction_relationships` | 297 | **389** |

The `44` is correct **for Evidence alone** — Evidence not_found is exactly 44. There is also **one
ungrounded `Party` item** (id 9016), which the UI's headline figure appears not to be counting. The
report below classifies **all 45**, with the 44 Evidence items called out separately wherever the
distinction matters.

The relationship count discrepancy (297 vs 389) is noted but not investigated — it is outside the
scope of this instruction and may be an artifact of when the UI figure was read.

**Disposition of the 45 (measured):** all 45 sit at `review_status = 'PENDING'`, `neo4j_node_id IS
NULL`. **Nothing ungrounded reached the graph.** The 93 grounded items were auto-approved
(`auto_approve_grounded: true`) and all 93 were written. That part of the system behaved correctly.

---

## 2. The causes, and how many items each accounts for

| # | Cause | Items | Evidence | Party | Fixable where |
|---|---|---|---|---|---|
| **C1** | **OCR line transposition in `document_text`** — Surya emitted short transcript lines out of reading order; the quoted words are all present but in a different sequence | **28** | 28 | 0 | **Ingest / OCR only.** Not fixable in the verifier. Not fixable in the template. |
| **C2** | **Gutter numeral absorbed by the hyphen-rejoin, which runs first** — `in-` + `17` + `formation` normalizes to `in17 formation` | **9** | 9 | 0 | **Verifier.** Two-line reorder inside `normalize_text`. |
| **C3** | **Page-pair join destroys line structure, so the boundary gutter numeral survives** — `strip_trailing_page_number` removes the printed page number but leaves the line-25 gutter numeral, which is then no longer a standalone line | **7** | 7 | 0 | **Verifier.** Normalize each page on its own line structure *before* joining. |
| **C4** | **Extracted value is not in the document in any form** — Party `"Robert Sharp"`; the transcript says `MR. JEFFREY SHARP (P53838)` / `Jeff Sharp` | **1** | 0 | 1 | **Nowhere in the matcher.** This is an extraction error and the verifier caught it correctly. |
| | **Total** | **45** | **44** | **1** | |

Sizing, in one sentence: **16 of the 44 ungrounded Evidence items (36%) are verifier bugs and can be
recovered by two small, generic changes; 28 (64%) are an OCR defect that no matcher change can
legitimately repair.**

---

## 3. Each cause, with the evidence

### C1 — OCR line transposition (28 items, 62% of all failures)

**What the stored text actually looks like.** Page 4, verbatim from `document_text`, line numbers
added by me:

```
  6|court while he was still alive regarding his conservator-
  7|ship. And apparently, a conservator never got appointed and
  8|4
  9|5
 10|And there was $50,000 and some kind of contention,
 11|he died.
 12|6
 13|which ended up at Catholic Family Service and is now being
```

The gutter numerals `4` and `5` are adjacent with no text between them, and the two text lines that
follow are **in the wrong order** — `he died.` belongs before `And there was $50,000…`. The same
pattern on page 7:

```
 17|THE COURT: Well, let Mr. Phillips finish his
 18|9
 19|Okay?
 20|explanation, then I'll have you talk.
```

`Okay?` is the end of the Court's sentence; Surya placed it in the middle.

**Decisive test.** I took page 7 exactly as stored, moved that single `Okay?` line back into reading
order, changed nothing else, and re-ran the *shipped* matcher:

```
item 8995, page 7
quote (normalized): well, let mr. phillips finish his explanation, then i'll have you talk. okay?
  grounds against page as stored : False
  moved stored line 19 ('Okay?') to position 21
  grounds after the move         : True
```

One displaced line is the entire difference between grounded and not-grounded.

**Scale of the defect (measured).** In a correctly ordered line-numbered transcript the stored line
stream alternates numeral, text, numeral, text. Across the 37 pages there are 2,014 non-empty lines
and exactly 962 standalone gutter/page numerals (26 per page, every page). Two numerals appear
back-to-back — meaning the text line that belonged between them was displaced — **106 times**,
roughly 2.9 per page. (Pages 1–2 are the caption/appearance pages and have a genuinely different
layout; their counts are less meaningful.)

**Why it hits long quotes and spares short ones (measured).** A quote fails as soon as it spans one
displaced line, so failure probability rises with quote length:

| Evidence quote length (words) | items | grounded | rate |
|---|---|---|---|
| 1–20 | 53 | 47 | **88%** |
| 21–50 | 32 | 23 | **71%** |
| 51–100 | 14 | 6 | **42%** |
| 101+ | 24 | 3 | **12%** |

Median length of a grounded Evidence quote: **17 words**. Median length of an ungrounded one:
**78.5 words**. This is the same "short verifies, long fails" signature that motivated commit
`8140815` — but this time the mechanism is transposition, not interleaving.

**Attribution per item (measured).** For each residual failure I decomposed the quote into maximal
runs that *are* present in the document, then asked whether each run boundary lands on a stored OCR
line boundary. Of 133 internal run boundaries examined, **114 (85%) fall exactly on a stored line
boundary.** Word-level corruption would scatter boundaries mid-line; whole-line reordering puts them
where they are. The remaining 19 are an artifact of the greedy decomposition (a maximal run can
overshoot into the displaced line's text), not evidence of a second mechanism.

**Three items look at first like something worse, and are not.** 8999, 9014 and 9039 report a token
absent from the document (`business.`, `for--as--ashe's`). In each case a transposed line landed
*inside* a hyphenated word split:

```
p15|18|contentious, wants to bid it up, well, that's not our busi-
p15|19|9
p15|20|They can bid it up.        <- displaced
p15|21|10
p15|22|ness.
```

`busi-` and `ness.` are four lines apart. Same cause, C1.

**Where the fix has to go.** Extraction and verification read the *identical* text
(`backend/src/pipeline/steps/llm_extract.rs:409` and `backend/src/pipeline/steps/verify.rs:250` both
call `get_document_text`); there is no vision/image path anywhere in the pipeline. The OCR call
(`backend/src/api/pipeline/ocr.rs:230-321`) posts the PDF to the external Surya service and sends
**no layout or reading-order parameter**; `p.text` is stored as returned. Nothing in the Rust code
re-sorts Surya's lines. The Surya service itself is not in any repo on this machine — it runs from
`/home/roman/Projects/local-ocr/surya_ocr_service.py` on the GPU host.

So C1 is fixable in exactly two places, neither of them the verifier:
- in the Surya service, by using bounding-box geometry to sort lines into true reading order; or
- in `extract_text.rs`, by a post-OCR reordering pass — which would need the bounding boxes Surya
  currently does not return (`SuryaPageResult` carries only `page_number`, `text`, `line_count`,
  `confidence`).

### C2 — the gutter strip is defeated by the hyphen-rejoin that runs before it (9 items)

`normalize_text` runs `rejoin_hyphenated_breaks` (step 2) *before* `strip_gutter_line_numbers`
(step 2b) — `canonical_verifier.rs:279` and `:287`. When a line ends in a hyphenated word break and
the next line is a gutter numeral, the rejoin fires on the numeral:

```
p5|35|Service got appointed. We asked some of the people for in-
p5|36|17
p5|37|formation like tax returns, checking accounts, and so forth.
```

The rejoin sees `-`, then `\n`, then `1` — a digit, which `is_alphanumeric()` accepts — and glues
them: `in17`. That is no longer a bare numeral line, so the gutter strip cannot remove it, and the
page normalizes to `…people for in17 formation like…` while the quote normalizes to `…people for
information like…`. The failure is manufactured by the normalizer itself.

Five such sites exist in this document (`for in-`, `and con-`, `what every-`, `so that every-`,
`is unin-`), and nine extracted quotes span one of them.

**Fix:** run `strip_gutter_line_numbers` *before* `rejoin_hyphenated_breaks`. Then the numeral line
is gone when the rejoin runs, `in-` meets `formation`, and the word reassembles correctly.
**Measured effect: 93 → 102 grounded (67% → 74%).**

### C3 — the page-pair join is normalized after concatenation, so boundary numerals survive (7 items)

Every page ends with the line-25 gutter numeral followed by the printed page number:

```
p20|…|MR. SHARP: And then we did submit a letter asking
p20|…|25
p20|…|20                      <- printed page number
p21|  1|for the information that we supplied to the personal repre-
```

`strip_trailing_page_number` removes only the **last** numeric line (`20`), leaving `25` at the end.
The pair path then builds `combined = left + " " + right` and normalizes the *result*
(`canonical_verifier.rs:220-226`). By that point `25` is no longer on a line of its own — it sits at
the head of a line that continues with page 21's first words — so the gutter strip cannot see it.
The boundary reads `…a letter asking 25 for the information…`; the quote reads `…a letter asking for
the information…`.

**Fix:** normalize each page on its own line structure and *then* join —
`normalize(left) + " " + normalize(right)` instead of `normalize(left + " " + right)`.
**Measured effect: 102 → 109 grounded (74% → 79%).**

I also tested whether widening the window to three pages helps. **It does not — zero additional
items.** The gain is entirely the boundary-normalization fix, not the window size. Worth recording,
because "widen the window" is the intuitive fix and it is the wrong one.

### C4 — Party `"Robert Sharp"` is not in the document (1 item)

The transcript names the attorney `MR. JEFFREY SHARP (P53838)` (p1) and `Jeff Sharp on behalf of
Maria Awad` (p3). The token `robert` does not occur anywhere in the 37 pages in any normalized form.
The extraction invented a first name.

**The verifier did the right thing.** No matcher change should ever ground this, and none of the
proposed fixes does.

---

## 4. Counterfactual measurements

Each variant is the shipped matcher with one change, replayed over all 138 items:

| Variant | Change | Grounded | Rate |
|---|---|---|---|
| **V0** | shipped beta.360 (validated: reproduces all 138 stored verdicts) | 93 | 67% |
| **V1** | V0 + gutter strip moved *before* the hyphen rejoin (C2) | **102** | **74%** |
| **V1b** | V1 + per-page normalization before the page-pair join (C3) | **109** | **79%** |
| **V1c** | V1b + three-page window | 109 | 79% (no gain) |

Residual after V1b: **29 items — 28 C1 + 1 C4.**

For sizing only, I also measured how many residual failures are pure reorderings of text that *is*
in the document: **28 of 29**. Only the Party name (C4) contains text that genuinely is not there.
This is a diagnostic, **not a proposed matcher** — see §5.

---

## 5. Reusability verdict

The instruction's standard: *does the fix work for every document type with zero code changes, or is
it transcript-specific? A transcript-specific matcher is a defect, not a fix.*

**C2 fix (reorder the two normalization steps) — PASSES.**
It introduces no new rule and no new vocabulary; it changes the order of two existing steps. The
existing safety property is preserved exactly: `strip_gutter_line_numbers` is the identity function
on any text with no standalone 1–2 digit line, so on a document with no gutter numerals the two
orderings are *provably* byte-identical. The six already-verified non-transcript documents cannot
move. Zero config, zero document-type knowledge, zero case-specific data. `colossus-ai` could take
this unchanged.

**C3 fix (normalize each page before joining) — PASSES.**
Purely structural: it preserves line structure across a page boundary so the existing
page-number/gutter strips can do the job they were already written to do. It contains no transcript
concept whatsoever, and it improves any document whose paragraphs cross a page boundary — complaints
and briefs included. Zero config. Reusable as-is.

**Fuzzy / order-insensitive matching for C1 — FAILS, and should not be built.**
The only way to "fix" C1 in the verifier is to stop requiring that the quote be a contiguous
substring of the document — token-set coverage, bag-of-words, Levenshtein over a window, or
segment-and-rejoin. Every one of those grounds text that is not in the document *in the order
quoted*. For a litigation-support tool, grounding **is** the assertion "this string appears in this
document at this page." A matcher that grounds a reordering is not a lenient matcher; it is a
matcher that lies, and it lies in the one direction that matters — it would let a genuinely
misquoted or model-recomposed passage pass as verified. It would also be undetectable downstream,
because the DB stores only `exact`/`normalized`/`not_found`.

**Restricting any such leniency to `court_transcript` — FAILS on its own terms.**
It would put a document-type name into shared matcher code, which Rule 2 forbids outright, and it
would still be the lying matcher above, just scoped.

**Verdict: C1 is not a verifier problem and must not be solved in the verifier.** It is an OCR
reading-order defect. Fix it where it is.

---

## 6. The three hypotheses, adjudicated

**Hypothesis 1 — "Flagged quotes are not paraphrases; the matcher is at fault, not the template."**
**CONFIRMED as to paraphrase; only partly right about the matcher.**
Measured: of the 44 ungrounded Evidence quotes, **44 reproduce document text word-for-word**,
including disfluencies (`these--these`, `uh`, `--`) and OCR hyphenation (`conservator-\nship`,
`busi-\nness.`). Zero are paraphrases. But the conclusion "therefore the matcher" holds for only 16
of the 44; the other 28 are an OCR defect upstream of both the matcher and the template.

There is a subtlety worth its own line. Pass-1 reads the *same scrambled text* the verifier matches
against (`llm_extract.rs:409`). So on the C1 items the model was handed out-of-order lines and
**silently reassembled them into the correct reading order** while otherwise copying verbatim —
right editorially, fatal for grounding. It does this inconsistently: items 9064 and 9112 preserve the
scrambled order faithfully, which is why they ground once C3 is fixed. *(The mechanism claim here is
measured — the quotes' word order versus the stored line order. Why the model is inconsistent is
inferred.)*

**Hypothesis 2 — "A flagged quote reads 'Emil Awad' where the page reads 'Mail Awad'; the model
silently corrected OCR, so no normalization can rescue those items."**
**REFUTED.**
`Mail Awad` does not appear anywhere in the stored text. `Emil Awad` appears four times (p3 ×3,
p4 ×1), including `the estate of Emil Awad, A-w-a-d.` Items 8989 and 8990 do quote "Emil Awad" and
do fail — but the string is present and their failure is C1. Across all 45 failures exactly **one**
token is absent from the document in any normalized form, and it is not `Emil`: it is `robert`
(§C4). The "silent OCR correction" failure class exists in this document with a population of zero.

*Caveat, stated plainly:* I compared against the stored OCR text, which is what the verifier matches.
I did not open the scan image. If the paper page does read "Mail", then Surya already normalized it
to "Emil" during OCR, which changes nothing about grounding.

**Hypothesis 3 — "The gutter number usually survives as an *inline* token on the same line as the
speech, so the strip added in `8140815` is a no-op for multi-line quotes."**
**REFUTED as stated — and the profile comment it rests on is wrong for this document.**
Measured over all 2,014 non-empty lines: **962 standalone 1–2 digit lines, and 0 lines matching
`<digits><space><CAPITAL>`.** In this OCR the gutter numerals are *exclusively* on their own lines.
The comment at `backend/profiles/court_transcript_v5_3.yaml:80` — "the gutter number surviving as an
inline token (it usually does)" — does not describe what Surya produced here.

Consequently `8140815` is emphatically **not** a no-op: it is what moved grounding from 53% to 68%,
and the replay confirms the strip is load-bearing on 962 lines.

But the hypothesis was reaching for something real. There *is* an inline gutter token in this
document — `in17`, `con18`, `every20` — and it is **created by `normalize_text` itself** via the
rejoin-before-strip ordering. That is C2, 9 items. The intuition was right; the location was wrong.

---

## 7. Complete classification — all 45 items

Every not-verified item, none sampled away. `pg` is the item's own `page_number` property.

| Cause | item_id | pg | words | speaker (as extracted) | label |
|---|---|---|---|---|---|
| C1 | 8989 | 3 | 123 | George Phillips | Phillips describes estate background and assets |
| C1 | 8990 | 4 | 198 | George Phillips | Phillips describes personal property and house transfers |
| C1 | 8991 | 5 | 122 | George Phillips | Phillips characterizes daughters as contentious and at each other's throats |
| C1 | 8992 | 5 | 225 | George Phillips | Phillips describes phone-call demands and refusal to talk to represented party |
| C1 | 8993 | 6 | 43 | George Phillips | Phillips characterizes Marie's submission as irrelevant |
| C1 | 8994 | 7 | 58 | Jeffrey Sharp | **Sharp objects that Marie has been singled out** |
| C1 | 8995 | 7 | 13 | Karen A. Tighe | Court directs Phillips to finish before Sharp |
| C1 | 8996 | 7 | 179 | George Phillips | **Phillips characterizes Marie's material as shrill, contentious, accusatory, and pathetic** |
| C1 | 8997 | 8 | 133 | George Phillips | Phillips requests distribution of all filed materials |
| C1 | 8998 | 8 | 128 | George Phillips | Phillips characterizes Camille and Nadia's debt-pursuit demand as a dumb request |
| C1 | 8999 | 9 | 211 | George Phillips | Phillips proposes auction of personal property and discovery period |
| C1 | 9000 | 10 | 177 | George Phillips | Phillips proposes deferring claims until claims period closes |
| C1 | 9001 | 11 | 51 | Karen A. Tighe | Court restates Phillips's request for authority to dispose of personal property |
| C1 | 9003 | 12 | 232 | George Phillips | Phillips explains dispute-resolution mechanism and requests approval of plan |
| C1 | 9004 | 13 | 203 | George Phillips | Phillips objects to unreasonable ultimatum demands |
| C1 | 9005 | 13 | 13 | Karen A. Tighe | Court confirms estate consists of cash account and personal property |
| C1 | 9007 | 14 | 36 | George Phillips | Phillips describes photographing property and mailing to parties |
| C1 | 9009 | 14 | 54 | George Phillips | Phillips requests authority to abandon issues and proposes auction |
| C1 | 9010 | 14 | 20 | Karen A. Tighe | Court restates auction mechanism as taking items |
| C1 | 9014 | 15 | 77 | George Phillips | Phillips describes competitive bidding at the auction |
| C1 | 9023 | 17 | 78 | Karen A. Tighe | Court explains rationale for appointing Catholic Family Services |
| C1 | 9039 | 21 | 30 | *Robert* Sharp | Counsel argues issues are for the parties to work out |
| C1 | 9046 | 22 | 124 | George Phillips | Counsel proposes retaining DVDs as evidence |
| C1 | 9066 | 26 | 31 | George Phillips | Counsel agrees to send claims and await revised claim |
| C1 | 9076 | 28 | 106 | *Robert* Sharp | Counsel requests final distribution hearing in March or April |
| C1 | 9092 | 32 | 109 | Nadia Awad | Nadia Awad describes father's intended gift of shop to nephew |
| C1 | 9103 | 33 | 11 | Karen A. Tighe | Court notes uncertainty about location of the clothing |
| C1 | 9111 | 35 | 13 | Karen A. Tighe | Court states characterizations will not be in the order |
| C2 | 9002 | 11 | 168 | George Phillips | Phillips requests orders on distribution and delivery of estate property |
| C2 | 9020 | 16 | 209 | *Robert* Sharp | **Counsel objects to "pathetic"/"hostage" characterizations and alleges selective singling out** |
| C2 | 9021 | 17 | 104 | *Robert* Sharp | Counsel argues PR not working for all parties; disputes bankruptcy discharge claim |
| C2 | 9022 | 17 | 55 | Karen A. Tighe | Court limits hearing to the auction proposal and defers claims disputes |
| C2 | 9027 | 18 | 110 | *Robert* Sharp | Counsel concedes no qualms with personal property being taken away |
| C2 | 9028 | 19 | 188 | *Robert* Sharp | Counsel requests pictures; objects to charging heirs for valueless remembrances |
| C2 | 9110 | 35 | 27 | Karen A. Tighe | Court comments that it lets Phillips's remarks "wash over" |
| C2 | 9114 | 36 | 79 | George Phillips | Counsel clarifies Marie Awad's claim excluded from release |
| C2 | 9115 | 37 | 46 | Karen A. Tighe | Court confirms understanding and restates hearing date and conditions |
| C3 | 9029 | 19 | 66 | *Robert* Sharp | Counsel agrees to two months discovery conditioned on documentation |
| C3 | 9034 | 20 | 30 | *Robert* Sharp | Counsel requests submitted information be returned and not released |
| C3 | 9040 | 21 | 47 | Karen A. Tighe | Court states you cannot unring a bell; asks whether PR has divulged |
| C3 | 9047 | 22 | 124 | George Phillips | Counsel proposes keeping DVDs as valuable evidence |
| C3 | 9050 | 23 | 113 | George Phillips | **Counsel says Sharp objected to singling out his client's claim** |
| C3 | 9064 | 25 | 31 | *Robert* Sharp | Sharp confirms no other claim has been served on the other parties |
| C3 | 9112 | 35 | 20 | Karen A. Tighe | Court comments Sharp's client not the only one not making it easy for the PR |
| **C4** | **9016** | — | 2 | — | **Party `"Robert Sharp"` — name absent from the document** |

The instruction's claim that the ungrounded set contains the highest-value material is **confirmed**:
the "singled out" objection appears twice (8994, and Phillips's acknowledgement of it at 9050),
Phillips's characterizations of the plaintiff are 8991 / 8993 / 8996 / 8998, Sharp's rebuttal to the
"pathetic"/"hostage-video" framing is 9020, and the bench colloquy is 8995 / 9022 / 9040 / 9110 /
9111 / 9112.

---

## 8. Findings not anticipated by the instruction

**8.1 — A wrong attorney name is already in the graph.** (Highest-severity finding in this report.)
The extraction produced **two Party nodes for one person**: `Jeffrey Sharp` (8980, grounded) and
`Robert Sharp` (9016, not_found). Worse, the invented name propagated into the `speaker` property of
Evidence items — measured across all 138 items: `Robert Sharp` ×20, `Jeffrey Sharp` ×1. Of those 20,
**11 grounded, were auto-approved, and were written to Neo4j**:

```
 3  Robert Sharp  exact       approved  written
 8  Robert Sharp  normalized  approved  written
 9  Robert Sharp  not_found   PENDING   not written
 1  Jeffrey Sharp not_found   PENDING   not written
```

The mechanism is structural, not incidental: grounding verifies `verbatim_quote` and nothing else.
Every derived property — `speaker`, `represents`, `speaker_role`, `attribution` — is unverified, and
`auto_approve_grounded: true` sends the whole item to the graph on the strength of the quote alone. A
correct quote with a fabricated speaker is indistinguishable from a fully correct item at every gate
it passes. The 68% grounding number says nothing about those fields.

**8.2 — `verification_reason` is NULL on all 45 failures.** Measured. The verifier writes
`not_found` with no reason (`verify.rs:219-222` passes `None`). Nothing in the database distinguishes
C1 from C2 from C4 — the four causes in this report, with wildly different owners and fixes, are one
undifferentiated status. Reconstructing them required a Python port of the matcher and a direct DB
session. Against Rule 1 ("every operationally distinct state must produce a different observable"),
this is a gap: the column exists and is already used for `derived_invalid`, but the grounding path
never populates it.

**8.3 — The profile's turn estimate is off by ~40%, which is why the run produced 6 chunks.**
Measured: 231 lines match the profile's `boundary_pattern`, so at `units_per_chunk: 40` the splitter
produced 6 chunks. The comment at `court_transcript_v5_3.yaml:87` predicts "~300-400 turns … roughly
8-10 chunks". Not a defect — the run succeeded, 6/6 chunks — but the comment is stale and it is the
same comment block that carries the incorrect "inline token" claim refuted in §6.

**8.4 — `PIPELINE_OCR_ENGINE` is resolved but never dispatched on.** Found while tracing the OCR
path: the value is read and reported in the step summary (`extract_text.rs:403`, `:821`) but nothing
branches on it, and `PIPELINE_OCR_DPI` / `_LANG` / `_OEM` feed only dead tesseract helpers. An
operator setting these would see them echoed back and assume they took effect. Out of scope here;
logged so it is not lost.

---

## 9. Recommendations — not implemented, per the stop gate

Listed in the order I would do them.

1. **Fix C2 and C3 in `canonical_verifier.rs` (one instruction, small).** Measured recovery: 16
   items, 93 → 109, 67% → 79%. Both are generic, both pass the reusability checkpoint, and both are
   provable no-ops for the six already-verified documents. The C3 change should carry a regression
   test with a real page-boundary fixture (gutter numeral + printed page number at the tail), and the
   C2 change should carry the `in-`/`17`/`formation` case, because both are invisible to the current
   test suite — every existing test constructs its fixtures by hand and none reproduces the two
   artifacts stacked.
2. **Populate `verification_reason` on `not_found`.** Even a coarse discriminator ("no run longer
   than N words matched" vs "matched in fragments — possible reading-order defect") would have made
   this report a query instead of an investigation, and would let the next transcript be triaged from
   the UI.
3. **Take C1 to the OCR service.** The decision is whether Surya can return bounding boxes and be
   made to sort by geometry, or whether `extract_text.rs` gets a reordering pass. Either way it needs
   `SuryaPageResult` to carry more than `text`. Until this lands, **every line-numbered transcript in
   the corpus will ground at roughly this rate** — the defect is in the OCR of this document class,
   not in this document.
4. **Treat 8.1 as its own instruction.** Two questions, neither answered here: how does a fabricated
   speaker name get corrected in items already written to Neo4j, and should `auto_approve_grounded`
   continue to promote items whose non-quote properties have never been checked against anything.

**Do not** widen the page window (measured: zero gain), and **do not** loosen the matcher to tolerate
reordering — §5.

---

## Appendix — reproduction

Investigation tooling lives in the session scratchpad, outside the repo, and is read-only:

| File | Purpose |
|---|---|
| `q.py` | read-only DB helper; reads the DSN from `backend/.env` so no credential is ever on a command line |
| `matcher.py` | line-for-line Python port of `canonical_verifier.rs` at beta.360 |
| `replay.py` | validates the port — 138/138 agreement with stored `grounding_status` |
| `diagnose.py` | decomposes each failing quote and prints the divergence points |
| `counterfactual.py` / `refine.py` | the V0 / V1 / V1b / V1c measurements in §4 |
| `prove_transposition.py` | the run-boundary-vs-line-boundary test in §C1 |
| `reorder_demo.py` | the single-line-move demonstration for item 8995 |
| `transpose_scan.py`, `boundary.py`, `lenstat.py`, `sharp2.py` | the counts in §3, §6 and §8 |

No script writes to the database, and nothing was run against PROD.
