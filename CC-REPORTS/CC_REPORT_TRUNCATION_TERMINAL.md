=== CC REPORT — TRUNCATION_TERMINAL_CLASSIFICATION — COMPLETION — 2026-08-27 ===

**Branch:** `fix/truncation-terminal-failure` off `b5dc630` (main) · **Commit:** `8c4976a`, single commit · not merged, not pushed, not tagged, no version bump · no migration.

## WHAT WAS WRONG

The truncation gate shipped in `v2.0.0-beta.412` (Rider 2) detects `stop_reason == max_tokens` and fails the call. It reports through `PipelineError::LlmProvider`, which `classify_llm_extract_error` folded into `LlmExtractError::LlmCallFailed` — the **retryable** bucket. Restate therefore re-ran a truncated extraction with byte-identical parameters: same document, same prompt, same `max_tokens`, same model.

That retry cannot succeed. The ceiling that cut the response off is still the ceiling on the retry, so every attempt is cut off in the same place. The backoff spent real money and wall-clock arriving at the identical failure, and the operator was told `Will retry.` about a retry that could never help.

## WHAT SHIPPED

**1. Truncation is now a terminal, non-retryable step failure.**

New typed variant `LlmExtractError::ResponseTruncated`, and a terminal arm in `classify_llm_extract_error`. The step fails once, immediately.

**2. The message is kept whole; only the retry framing is dropped.**

The gate's text already names everything the operator needs, so it passes through unchanged. What an operator now reads in Execution History (captured from a real test run, not paraphrased):

> `Terminal error [500]: step_llm_extract_pass1: document 'doc-motion': LLM response truncated: LLM provider error: model claude-opus-4-6 stopped at the max_tokens ceiling: the response was TRUNCATED, not completed. Produced 32000 output tokens against a configured cap of 32000. The extraction is discarded rather than parsed, because a truncated response repairs into plausible JSON and would otherwise be stored as a complete result. Raise max_tokens for this document type in its profile YAML and re-run. No retry — a truncated response is deterministic, and re-running against the same cap produces the same truncation.`

`Will retry.` is absent from this arm and still present on the genuinely retryable arm. The doc id leads rather than trails, because the gate's message is a paragraph ending in its own remedy — appending `for 'doc-x'` after `…and re-run.` read as a fragment.

**3. How the signal travels — the one design decision worth your review.**

`colossus_extract::PipelineError` lives in the **colossus-rs** repo, so this repo cannot give truncation its own variant there; the gate has to report home inside `LlmProvider(String)`. That enum's own doc comment argues — correctly — that matching on error *message* strings is fragile.

So the string match happens **exactly once**, in `truncation::is_truncation_failure`, against a `TRUNCATION_SIGNATURE` constant that `truncation_message` is *built from* — one definition, not two hand-kept copies. It narrows on the `LlmProvider` variant **first**, so a document whose own text quotes that sentence cannot be mistaken for a truncated call, and a `RateLimited` can never be swallowed by it. Callers then convert immediately to the typed variant via the shared `LlmExtractError::from_provider_failure` constructor, so **nothing downstream of that point ever inspects a message again**.

Both passes route through that single constructor. A source-level test pins this, because a split where pass 1 classifies correctly and pass 2 does not is invisible to any test of the classifier alone — the classifier would still be right.

**4. `motion_v5_3.yaml`: `max_tokens` 32000 → 64000.**

Same ruling as appellate (R-3, 2026-08-25), applied the same way — a one-line change, no comment, mirroring `e7b0fc7`. Checked before raising, per the `constrain` REJECTS-rather-than-clamps lesson: the profile's model is `claude-opus-4-6`, the *same* model `appellate_brief_v5_3.yaml` already runs at 64000. The cap is proven accepted, not assumed.

## FILES CHANGED — 7, all in the approved scope

| File | Change |
|---|---|
| `backend/src/pipeline/truncation.rs` | `TRUNCATION_SIGNATURE` const, `is_truncation_failure()`, message built from the const, module doc for the terminal ruling |
| `backend/src/pipeline/truncation_tests.rs` | +5 tests |
| `backend/src/pipeline/steps/llm_extract.rs` | `ResponseTruncated` variant, `from_provider_failure()` constructor, pass-1 call site |
| `backend/src/pipeline/steps/llm_extract_pass2.rs` | pass-2 call site (1 line) |
| `backend/src/pipeline/workflow_steps/llm_extract.rs` | terminal classification arm |
| `backend/src/pipeline/workflow_steps/llm_extract_tests.rs` | +2 tests |
| `backend/profiles/motion_v5_3.yaml` | `max_tokens: 32000` → `64000` |

## GATES

| Gate | Result |
|---|---|
| `cargo test --lib` | **2636 passed / 0 failed / 3 ignored** (7 new) |
| `cargo fmt --check` | clean |
| `cargo check` | clean |
| `npm run typecheck` | clean (no frontend files touched) |
| `cargo clippy --lib --tests -- -D warnings` | 13 warnings — **every one pre-existing on main at an identical line**, none in this diff. Verified individually, not assumed |
| `Task(rules-enforcer)` | **PASS** — 0 violations introduced |
| `Task(architecture-reviewer)` | **PASS** |
| `Task(test-auditor)` | **PASS** — 0 further tests required |
| `Task(observability-checker)` | **PASS** |

**Test baseline note.** `cargo test --workspace` remains broken on main — the `tests/*.rs` integration target has stale `AppState` fields (`theme_scan_provider`) and a stale `ScanRunStart` initializer, unrelated to this diff and untouched by it. The honest baseline is `cargo test --lib`, and that is what is reported above.

**Gate-agent triage.** All three of `steps/llm_extract.rs` (1757), `steps/llm_extract_pass2.rs` (761) and `workflow_steps/llm_extract.rs` (333) were **already over the Rule 17 300-line budget on main**. This diff adds 15 / 0 / 6 lines to those pre-existing violations and creates no new one; `truncation.rs` is 37. The agents were asked to separate introduced from pre-existing findings and did.

## THREE THINGS FOUND, NOT FIXED — your call

**1. `backend/Cargo.lock` is stale on main.** It records `2.0.0-beta.402` while `Cargo.toml` says `2.0.0-beta.413`, so *any* cargo invocation rewrites that line. **Deliberately left out of this commit** — version numbers are yours, and a lockfile version bump inside a fix commit would read as CC bumping a version. It will keep reappearing as a dirty file until someone commits it.

**2. The chunked extraction path tolerates a truncated chunk.** In `run_chunked_extraction`, a failed LLM call increments `chunks_failed` and the run continues, so a truncated chunk yields a *partial* extraction that still COMPLETEs. Only if **every** chunk fails does the run fail — and it then fails as the untyped string `"All N chunks failed extraction"`, which `classify_dyn_llm_error` cannot downcast and classifies **retryable**. Out of scope here, and it does not affect the profiles in question (`motion_v5_3` and `appellate_brief_v5_3` are both `chunking_mode: full`, which is the single-call path this fix covers). Worth a separate ruling if any chunked profile matters.

**3. Theme Scan needs no change.** `theme_scan_judge::outcome_from_result` records a per-item failure carrying the gate's message verbatim and does not retry, so a truncated verdict already fails once with a clear reason. Confirmed by reading, not assumed — the known 512-cap truncation there behaves correctly under this design.

## NOT DONE, BY INSTRUCTION

Not merged. Not pushed. No version bump, no tag. `backend/twin_merge_human_queue.txt` was untracked before this session and remains untracked and uncommitted.

=== END REPORT — VERDICT: PASS ===
