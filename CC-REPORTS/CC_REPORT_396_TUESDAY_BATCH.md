=== CC REPORT — CC_TASK_396_TUESDAY_MATRIX_AND_RULED_FIXES_v1 — COMPLETION — 2026-08-13 ===

**Branch:** `fix/396-tuesday-batch` off `d64514c` · **Commit:** the single commit at the head of that branch — gated as `65efdfa`, then amended to fold in the test-auditor fix (pinning the final hash in this file is self-defeating: writing it changes it) · one migration
`20260813152536_tuesday_batch_396_…` · not pushed, not tagged, no version bump.

## SHIPPED / DROPPED — one line each

| Item | Result |
|---|---|
| **P1a** strong headline + raw small print | SHIPPED — the matrix column leads with the strong count; `· {n} approved` beside it from a stored template |
| **P1b** drill-down ranked, strength chips | SHIPPED — strongest→hedged→other→unmapped, chip per row from stored rows, `Strongest first` note above the list |
| **P1c** duplicate collapse, ×N | SHIPPED — with a **ruled amendment** (see below); counts agree at chart, depth line and detail because one function produces all three |
| **P2a** multi-link clause deleted | SHIPPED — panel stays after the first link; one new wording row for the already-linked state |
| **P2b** 👤 marker muted | SHIPPED — own span, `--text-muted` |
| **P2c** human-fact completeness | **DROPPED** — capacity, on the task's own drop order; its columns and rows were removed from the migration rather than shipped ahead of their code |
| **P3a** the `-HIGH` suffix | **OUT** — ruled out in Phase A; it is Roman's own Settings edit (`updated_by=roman`, 2026-08-05), he retypes it |
| **P3b** F2 / the R2 §3 rows | SHIPPED — answer: **never migrated**, not bypassed; subtitle + `Draft` tile from rows, pattern chip dead |

## THE TWO PHASE-A MEASUREMENTS THAT CHANGED THE BUILD

**1. The tier key had to be a PAIR.** On DEV, `evidence_strength =
'sworn_party_admission'` appears under BOTH `statement_type = 'admission'` (21
items) and `'partial_admission'` (12). Strength alone cannot separate a firm
admission from a hedged one — the exact distinction the headline exists to make.
All six measured pairs are mapped as ruled; the map is three `app_settings` rows,
not code.

**2. The drafted collapse rule over-collapsed.** "Same speaker + same normalized
quote" applied to DEV merges George Phillips' three `"yes."` answers to three
different interrogatories into one row — deleting two real sworn admissions from
the number Chuck reads. Ruled amendment built: Q/A items key on speaker +
question + answer; documentary evidence keeps speaker + quote. Both behaviours are
pinned by name in `matrix_strength_tests.rs` and again at the drill-down layer in
`element_detail_repository_tests.rs`, because the question column has to survive
the Cypher, the fold and the adapter to do its job.

## EXPECTED NUMBERS ON DEV (measured 2026-08-13, read-only)

Count I: **1.1 → 2 · 2 approved** · **1.2 → 15 · 26 approved** · **1.3 → 1 · 2**.
Count IV: **4.1 → 9 · 12** · **4.2 → 1 · 5**. Count II elements are mostly `1 · 2`,
with 2.2 at `1 · 6` and 2.9 at `1 · 4`; 2.6, 2.11 and 3.4 read `0 · 1` — a real and
important state (one item corroborates, none of it is undisputable).

None of the 16 new keys exists on DEV today, so `ON CONFLICT DO NOTHING` cannot
silently keep an old value.

## WALK CHECKLIST (architect, against .394)

1. **Proof Matrix → Count I.** Third column heads "Strong support". Row 1.2 reads
   `15` with `· 26 approved` small print beside it. Hover the `15` — the stored
   hint says what strong means.
2. **Expand 1.2.** Header row right side reads "Strongest first". Supporting items
   under each allegation are ranked; each carries a chip ("Their own words" /
   "Qualified" / "Our sworn word"), and an unmapped pair carries none.
3. **Find a collapsed row.** It shows once with `×2` beside its chip. Confirm the
   three separate "Yes." admissions are still three separate rows.
4. **Count II element 2.6** reads `0 · 1 approved`, not a dash — the "nothing here
   is undisputable" state must be visible.
5. **Scenario page, a card that already holds a link.** The link control is still
   offered, above the chips, saying "This card is already linked…". Add a second
   accusation; the first survives.
6. **Any card with a human link.** The 👤 is muted, the accusation name is not.
7. **Trial Prep.** Subtitle reads "The attacks and what we answer them with —
   built by you, gathered by the system, rehearsed by Marie." Third tile reads
   "Draft". No "pattern analysis pending" chip on any card.
8. **Settings page.** All 16 new rows are listed and editable; changing
   `matrix_tier_strong_pairs` and reloading the matrix moves the headline number.

## GATE (four agents, once, against `65efdfa`)

| Agent | Verdict |
|---|---|
| rules-enforcer | **PASS** — 0 violations introduced |
| architecture-reviewer | **PASS** — 0 introduced across 59 files |
| observability-checker | **PASS** |
| test-auditor | **FAIL → fixed → PASS** |

**The test-auditor finding, and what it cost.** `evidence_tier.rs` documented that a
pair listed under two tiers takes the WEAKER one — the safe reading, so a Settings
typo cannot promote a pair into the headline — and nothing tested it. Two tests
now pin it from both ends: `from_entries` guarantees last-group-wins, and
`a_pair_seeded_under_two_tiers_ranks_as_the_weaker_one` drives the real boot path
(`build_settings` over a fixture with the pair duplicated) to prove the groups are
supplied strongest-first. Nothing but three hand-written lines in
`build_evidence_tier_map` makes that true, so the order was flipped to confirm the
test fails — it did, with `left: Some(Strong)`, the dangerous outcome — and
reverted.

The observability agent also noted that the `// best-effort:` comment on the
unknown-tier branch in `matrixStrength.ts` borrowed a Rule 1 carve-out scoped to
`localStorage` preferences. Behaviour was right, the label overclaimed; the comment
now argues its own case.

**Pre-existing, flagged and NOT fixed here** (group B, for the tracker): four column
labels in `proofMatrixColumns.ts` are still literals (this diff extracted the fifth
into a row) · `ElementDetailRepoError::NotFound`'s Display has no test though it is
trivially constructable · two `// best-effort:` blocks in
`element_detail_repository.rs` open three lines above their `.ok()` rather than
immediately above · two clippy warnings in `scenario_code.rs` and
`scenario_dashboard.rs`.

## VERIFICATION (full suites, once, at the gate)

`cargo build` ok · `cargo test --lib` **1867 passed / 0 failed / 2 ignored** ·
`cargo clippy --lib --bins -- -D warnings` clean · `cargo fmt --check` clean ·
`cargo check --bins` ok · `npm run typecheck` clean · `npm test` **979 passed / 0
failed** · `npm run build` ok · `check-migrations.sh` clean.

`backend/tests/*.rs` remain broken from ~beta.343 (stale `AppState` fields),
untouched by this batch — `cargo test --lib` is the honest baseline.

## STATE FACTS

DEV = .394 live (`d64514c` merged) · S-5 (5 Carries/16 Backup) and S-6 (4 Carries/
7 Backup) weighted 2026-08-13 — **no scenario row was read or written by this
batch**. All DEV access during the build was read-only `SELECT` / `MATCH`.

=== END REPORT — VERDICT: PASS ===
