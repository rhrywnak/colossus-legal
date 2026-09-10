-- Migration: qwen27b scan model
-- Created: 2026-09-10
-- Task: CC_TASK_QWEN27B_SCAN_MODEL_v1
--
-- Make the local Qwen 27B selectable as the Theme Scan judge.
--
-- ## Why: a $0 judge
--
-- Every scan judged so far has been billed — the nine runs in `scan_runs` total
-- $39.71265500, and four scenarios (S-1, S-12, S-13, S-14) have never been
-- scanned at all. A self-hosted judge makes a full-recall scan of a 466-card
-- pool cost nothing, which is what turns "scan the ones we can afford" into
-- "scan all of them".
--
-- ## The id is COPIED, not typed
--
-- Read from gpu1 on 2026-09-10, `GET http://10.10.0.99:8001/v1/models`, the
-- whole `data[0]` as served:
--
--   {"id":"unsloth/Qwen3.8-27B-NVFP4","object":"model","created":1789051906,
--    "owned_by":"vllm","root":"unsloth/Qwen3.8-27B-NVFP4","parent":null,
--    "max_model_len":32768, ...}
--
-- `max_context_tokens 32768` below is that response's own `max_model_len`, not a
-- number chosen here.
--
-- ## Modelled on the 32B row
--
-- `20260715230427_scan_eligible_column_and_model_refresh.sql:94-101`, same column
-- list and the same `ON CONFLICT (id) DO NOTHING`, with four deliberate
-- differences, each recorded below rather than left for a reader to spot.
--
-- ### 1. `billing_class = 'local'` — REQUIRED, and easy to miss
--
-- The column arrived AFTER the 32B row (20260802134438) as
-- `NOT NULL DEFAULT 'billed'`, and its backfill was a one-time
-- `UPDATE ... WHERE provider = 'vllm'` over the rows that existed that day. A
-- row inserted later does NOT get that treatment: omit this and the new row
-- defaults to `'billed'`, so `api::chat_models::entry_for` labels a free
-- self-hosted model "Qwen3.8 27B (NVFP4, local) (API — billed)" and `local_first`
-- sorts it below the metered Claude rows. The provider does not imply the class
-- (that migration's own §2 says so); it is a value a human sets, and this is it.
--
-- ### 2. The cost columns are 0, not NULL
--
-- The 32B row leaves them NULL. The billing_class migration is explicit that
-- "a NULL cost means unknown, never free" — but this model IS free, and 0 is the
-- true number. The consequence is in the ledger: `compute_cost`
-- (`pipeline/steps/llm_extract.rs:1647`) returns `None` when either column is
-- NULL and `Some(0.0)` when both are 0, so a run judged by this model records
-- `cost_usd = 0.0000` rather than a blank that reads as "we never worked it out".
--
-- ### 3. `max_output_tokens` is deliberately LEFT UNSET — read this before editing
--
-- The 14B and 32B rows carry `max_output_tokens = 2048` (recorded in
-- CC_REPORT_DOC_INTAKE_PREFLIGHT_v1.md §B1; note that value is NOT set by any
-- migration in this directory, so a database built from these files alone has it
-- NULL on those rows too). `domain::llm_params::constrain` is clamp-by-error:
--
--     if let Some(ceiling) = c.max_output_tokens {
--         if resolved.max_tokens > ceiling { return Err(MaxTokensExceedsCeiling) }
--     }
--
-- The `theme_scan_max_tokens` settings row is seeded at 8192
-- (20260809210501:78), so on a row with a 2048 ceiling the scan REFUSES TO START
-- — that migration's own "A CONSEQUENCE FOR THE vLLM MODELS" section spells it
-- out. Leaving this column NULL means no ceiling is enforced and the 8192 request
-- passes, which is the difference between a judge that can run and one that
-- cannot.
--
-- NULL here is "not recorded", not "unlimited", and it has one other effect worth
-- knowing: `pipeline/providers.rs:123-126` falls back to `FALLBACK_MAX_TOKENS`
-- (8000) for the provider's construction-time default. Both numbers sit far
-- inside the 32768 the server reports.
--
-- If a future operator sets a ceiling on this row, they must set it at or above
-- `theme_scan_max_tokens` or the scan stops starting — loudly, naming the model,
-- the request and the ceiling, but it stops.
--
-- ### 4. `timeout_secs 300`, above the 32B row's 180
--
-- 27B NVFP4 is a larger model than the 32B AWQ is quantised to run as, on the
-- same single GPU, and the judge writes a paragraph of reasoning per candidate.
-- 180s is the 32B's measured-enough value; this is not that model. 300 is chosen
-- to fail on a HUNG server rather than on a slow one — a timeout that fires on
-- ordinary latency turns one slow card into a failed run.
--
-- `max_concurrency 2` matches the 32B row: one vLLM model is loaded at a time
-- (Roman's ruling, 2026-09-03), and a 27B on one GPU is not more parallel than a
-- 32B was.
--
-- ## This row does NOT assume the model is loaded
--
-- Per the same 09-03 ruling, gpu1 serves one model at a time. Nothing here
-- pre-supposes this one is the loaded one: `services::vllm_model_gate` polls
-- `/v1/models` before every scan and refuses, named and loudly, when the selected
-- id is not being served. The row makes the model SELECTABLE; the gate decides
-- whether a given run may proceed.
--
-- Idempotent: `ON CONFLICT (id) DO NOTHING`, the same guard the 14B and 32B rows
-- use, so a re-run changes nothing and a hand-edited row is never clobbered.

INSERT INTO llm_models (
    id, display_name, provider, api_endpoint,
    max_context_tokens, cost_per_input_token, cost_per_output_token,
    is_active, scan_eligible, billing_class,
    temperature_mode, structured_output_mode, timeout_secs, max_concurrency)
VALUES
    ('unsloth/Qwen3.8-27B-NVFP4', 'Qwen3.8 27B (NVFP4, local)', 'vllm',
     'http://10.10.0.99:8001',
     32768, 0, 0,
     true, true, 'local',
     'zero-ok', 'guided', 300, 2)
ON CONFLICT (id) DO NOTHING;

-- The visible record, in the shape 20260802134438 §3 established: a migration
-- that adds a row should say what the table now holds, so the next reader checks
-- it against reality instead of trusting a comment.
--
-- Asserted rather than printed (CLAUDE.md 25a): a statement matching zero rows is
-- silent in Postgres, and an INSERT that quietly did nothing would leave the
-- picker unchanged with no sign that this file had run.
DO $$
DECLARE
    row_count    INTEGER;
    local_count  INTEGER;
BEGIN
    SELECT count(*) INTO row_count
      FROM llm_models
     WHERE id = 'unsloth/Qwen3.8-27B-NVFP4'
       AND is_active = true
       AND scan_eligible = true
       AND billing_class = 'local';

    IF row_count <> 1 THEN
        RAISE EXCEPTION
            'qwen27b_scan_model: expected exactly 1 active, scan-eligible, local '
            'row for unsloth/Qwen3.8-27B-NVFP4 after this migration, found %. '
            'If the row pre-existed with different values, ON CONFLICT DO NOTHING '
            'left them alone — inspect it and correct it by hand rather than '
            're-running this file.', row_count;
    END IF;

    SELECT count(*) INTO local_count
      FROM llm_models
     WHERE billing_class = 'local' AND scan_eligible = true AND is_active = true;

    RAISE NOTICE 'qwen27b_scan_model: % local scan-eligible model(s) now offered', local_count;
END $$;
