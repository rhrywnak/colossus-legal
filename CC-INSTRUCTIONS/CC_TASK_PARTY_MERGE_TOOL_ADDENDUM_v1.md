# CC_TASK_PARTY_MERGE_TOOL_ADDENDUM_v1 — P7, joins the template batch

**Date:** 2026-08-14 night · **Ruled by Roman** ("proceed with the
people-merge tool"). Addendum to CC_TASK_TEMPLATE_BATCH_AND_ID_ARM_v1;
read after the task file and handoff note. **Products:** INFRA — People
totality (Roman's ruled justification for process-all) · wave
prerequisite (the human merge pass must precede the wave and currently
has NO mechanism).

## P7 · THE PARTY-MERGE ONE-SHOT TOOL

Same family and discipline as rekey_evidence (39a8ba8/08c7c2e): one-shot
Rust binary · dry-run default, --apply the only write path · per-cluster
unit of work: verify counts → repoint → verify → commit or roll the
cluster back · count proofs to tracing + report file · idempotent ·
honest exit codes on the ruled scheme (0 clean / 1 bad input / 2
connection / 3 a cluster aborted / 4 unsafe plan / 5 mid-run failure).

a. **Input = a rulings file, never a guess.** The tool executes ONLY an
   explicit human rulings file: per cluster — survivor node id · members
   to merge in · or SKIP. No fuzzy matching in the tool, no defaults,
   nothing merges that a human did not name. The two do-not-auto-merge
   clusters arrive as SKIP unless Roman's file says otherwise.
b. **Generate the rulings TEMPLATE for Roman's session** from the Phase
   A party census: the 12 clusters, each member's name, label
   (Person/Organization), statement count, and source documents — so
   Roman rules with the facts in front of him. File it in both
   CC-INSTRUCTIONS locations as PARTY_MERGE_RULINGS_TEMPLATE_v1.md.
c. **Merge mechanics:** repoint every edge and statement from merged
   nodes to the survivor · union source_documents · record merged name
   variants on the survivor (aliases property — the B2 §5 model) ·
   enumerate ALL referencing columns first (Postgres and graph) the way
   rekey pinned its eight; measure, do not assume Person ids are
   graph-only.
d. **Acceptance:** statement totals conserved per cluster (e.g., the
   TIGHE ×3 = 39+62+0 → one node with 101) · zero dangling references ·
   People page count drops by exactly the merged-member count · report
   proves each.

## PRIORITY

P7 is a WAVE PREREQUISITE: it joins P1–P4 in the never-drop set.
Drop order becomes: P6 first, then P5b, then P5a; P1–P4 + P7 never.
Saturday's human sequence depends on it: deploy → re-key dry-run/apply →
PARTY MERGE SESSION (Roman rules the 12) → Morris gate test → wave.
