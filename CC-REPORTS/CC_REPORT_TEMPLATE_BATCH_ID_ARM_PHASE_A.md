=== CC REPORT — CC_TASK_TEMPLATE_BATCH_AND_ID_ARM_v1 — PHASE A — 2026-08-14 ===

**Branch:** `feat/template-batch-id-arm` off `origin/main` after `git fetch`.
**HEAD:** `9d50dcd chore: bump version to v2.0.0-beta.395` — the bump commit, with
`fc102da Merge branch 'fix/396-tuesday-batch'` beneath it and `backend/Cargo.toml`
reading `2.0.0-beta.395`. The Tuesday-batch code is in. Baseline confirmed; no
stale checkout.

**Nothing was built. All access was read-only `SELECT` / `MATCH`.**

---

## 1. P1a — THE COLLISION MEASUREMENT (the gating item)

Key measured exactly as specified: `doc_slug + page_number + normalized
verbatim_quote + question (when present)`, where `doc_slug` is `Document.id`
(already a slug — e.g. `doc-cfs-interrogatory-response-08-08-16`) and
normalization is trim + collapse-internal-whitespace.

| Figure | Measured |
|---|---|
| Evidence nodes | **525** (asserted 525 — **confirmed**) |
| Documents | 9 (8 carry Evidence) |
| Evidence with no `CONTAINED_IN` Document | **0** |
| Evidence with no `page_number` | **0** |
| Evidence with no `verbatim_quote` | **0** |
| Evidence with no `question` | 287 (documentary — the key's optional arm) |
| **Distinct keys** | **504** |
| Keys held by exactly 1 node | 483 |
| **Keys held by 2 nodes** | **21** (42 nodes) |
| Keys held by 3+ nodes | **0** |

**So the ×2 class is the whole of it: 21 pairs, no triples.** The key is
95.9% unique by node (483/525), 100% of collisions are pairs.

*Caveat stated honestly:* Unicode NFC could not be applied inside Cypher, so the
normalization measured is trim + whitespace-collapse only. NFC can only MERGE
further, so 504 is an upper bound on distinct keys and 21 a lower bound on
collisions. Given every collision is already an exact-text match, NFC is very
unlikely to move these numbers; the implementation will apply NFC as specified.

## 2. WHAT THE 21 COLLISIONS ACTUALLY ARE

Not distinct statements sharing a key. **The same statement extracted twice.**
Every pair shares document, page, Q-number (`paragraph`), verbatim quote and
question; 20 of 21 also share `statement_type`. The only differences are in
LLM-mooded prose — `significance` phrased two ways. Examples:

```
doc-george-phillips-response-to-discovery p16 Q108  "Yes."          admission ×2
doc-cfs-interrogatory-response-08-08-16   p8  Q28   "It was the personal
                                                     representative's belief…"  admission ×2
doc-george-phillips-response-to-discovery p21 Q72   "The materials submitted to
                                                     Judge Tighe speak for
                                                     themselves…"   referral / evasive  ← the one type disagreement
```

## 3. CURATED EXPOSURE — AND THE FINDING THAT BLOCKS 1a

Case-wide across the 7 named tables (8 columns — `scenario_human_facts` has two):

| Reference | Rows | Distinct ids |
|---|---|---|
| `scenario_candidate_ordinals.graph_node_id` | 444 | 148 |
| `scan_run_verdicts.graph_node_id` | 226 | 113 |
| `scenario_ruling_anchors.graph_node_id` | 167 | 66 |
| `evidence_allegation_link_events.graph_node_id` | 37 | 23 |
| `scenario_fact_refs.graph_node_id` | 35 | 34 |
| `scenario_human_facts.anchor_graph_node_id` | 18 | 9 |
| `evidence_allegation_links.graph_node_id` | 11 | 11 |
| `scenario_human_facts.answers_graph_node_id` | 9 | 8 |
| **TOTAL** | **947 rows** | **148 distinct Evidence ids** |

Asserted "~929" → **947 measured** today. Close, and grown since the earlier
report; use 947.

**Per-pair exposure — the decisive number:**

```
21 pairs
   7  BOTH twins carry curated rows   (3–12 rows each side)
   0  exactly ONE twin curated
  14  NEITHER twin curated
```

Zero one-sided pairs is not noise. It means the duplicates reached the ruling
queue as two cards and Roman ruled **both**.

**And the twins do not carry the SAME ruling.** Measured on the 7 live pairs:

```
…cfs…:evidence:98515eda   included/CARRIES   ┐ same scenario aecbaf77
…cfs…:evidence:f1439b2c   included/BACKUP    ┘
…phillips…:evidence:042d8287  included/CARRIES ┐ same scenario aecbaf77
…phillips…:evidence:8f261bd1  included/BACKUP  ┘
…phillips…:evidence:0fd1a748  included/CARRIES ┐ same scenario e8868d6b
…phillips…:evidence:b7f5a787  included/BACKUP  ┘
```

**The twins are therefore NOT interchangeable.** Any scheme that assigns the two
ids by position risks swapping Carries and Backup on the next extraction —
silently reweighting Roman's curation on S-5/S-6, the demo-facing scenarios.

## 4. THE DISAMBIGUATOR — PROPOSAL, AND WHY I AM STOPPING

The instruction asks for a deterministic disambiguator. The honest measured
answer first: **no content-derived disambiguator exists.** The twins differ only
in fields the spec correctly bans from the key (`significance` and friends).
Every stable field — doc, page, quote, question, `paragraph`, `statement_type` —
is identical.

Three mechanisms, for your ruling:

**(A) Persisted occurrence assignment.** Key stays `base_key`; a new table maps
`base_key → ordered list of assigned suffixes` (`…#1`, `…#2`), written at first
ingest and reused thereafter. Re-extraction matches by base key and reuses
suffixes in order; a change in occurrence count routes to P2's human queue rather
than auto-assigning. Deterministic and stable *as a scheme* — but it cannot tell
which twin is which, so on a re-extraction that returns them in the other order,
Carries and Backup swap. It survives the re-key (§1b is 1:1 from today's state)
and only becomes unsafe at the next real re-extraction of those two documents.

**(B) Refuse to auto-key any base-key group with N>1.** The 483 unique keys
re-key automatically; the 21 groups go to the human queue every time. Safe,
never silently wrong, and costs Roman 21 decisions per affected reprocess (7 of
which actually matter).

**(C) Treat the ×2 class as the data defect it is.** These are duplicate
extractions of one statement. Dedupe at extraction, and have Roman merge the 7
curated pairs by hand once — after which the key is 100% unique and the whole
question disappears. This is the only option that ends the problem rather than
managing it, and the 14 uncurated pairs cost nothing to collapse.

**My recommendation: (C) for the end state, (B) as the interim** — because (A)'s
failure mode is silent and lands on the two scenarios Chuck sees. But this is
your ruling and I have built nothing.

*Cross-reference worth noting:* the .395 matrix work already collapses this exact
class at DISPLAY time (same speaker + question + answer → one row with "×2"). So
a duplicate that survives in the graph is already being shown to Chuck as one
item. That is display-layer mercy over a data-layer defect, and it is an argument
for (C).

## 5. STANDING QUESTION — IS PER-PAGE DOCUMENT TEXT PERSISTED?

**Yes, completely.** Table `document_text` (`document_id`, `page_number`,
`text_content`) holds **160 page rows across all 9 documents, 266,076 characters**,
and coverage is exact: every document's stored page count equals its
`documents.page_count` (17/17, 32/32, 35/35, 37/37, 14/14, 10/10, 5/5, 5/5, 5/5).
Pages are 1-indexed, max 37. So a future task needing page text — re-grounding, a
quote locator, the OCR-transposition work — does **not** need a re-extraction or a
filesystem read; it is a `SELECT` away. `documents` also carries `page_count`,
`scanned_pages`, `text_pages` and `pages_needing_ocr`, so OCR state is queryable
per document. Nothing built for this; it prices the separate task as small.

## 6. WHAT PHASE A DID NOT TOUCH

P1b re-key, P2 remap script, P3 ISO paragraph, P4 document dates, P5 templates,
P6 party/alias — all unstarted, per mandatory-and-stops. No migration created, no
code written, no branch content beyond this report.

## 7. WHAT I NEED BEFORE BUILDING

1. **The disambiguator ruling** — (A), (B), (C), or your own. §4 is blocking for
   P1a and therefore for P1b.
2. If **(C)**: whose call is the merge of the 7 curated pairs, and does it happen
   before the re-key or after? It is Roman's curation either way, and it changes
   whether §1b re-keys 525 nodes or 504.
3. Confirm **947** replaces "~929" in the task's context facts, so the re-key
   count proofs are measured against the right number.

=== END REPORT — VERDICT: STOPPED ===
