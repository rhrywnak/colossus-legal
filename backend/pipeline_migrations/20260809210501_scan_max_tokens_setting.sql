-- scan_max_tokens_setting: the judge's token budget leaves the code
--
-- Created: 2026-08-09 21:05:01
-- Target: pipeline database (colossus_legal_v2)
--
-- Task CC_TASK_SCAN_MAX_TOKENS_SETTINGS_v1, as ruled in
-- CC_TASK_SCAN_MAX_TOKENS_PHASE_B_AUTHORIZATION_v1 (rulings R1, R2, R6).
--
-- ## What was measured, and why a constant is becoming a row
--
-- The judge's per-candidate budget was `THEME_SCAN_MAX_TOKENS: u32 = 512`,
-- compiled in on the argument that a verdict is a tiny four-key JSON object and
-- its budget is a protocol shape rather than a deployment knob. That was correct
-- while the model's OUTPUT was the verdict. Claude Opus 5 runs adaptive thinking
-- by default and `max_tokens` caps thinking and answer TOGETHER.
--
-- S-4 run 2c7b7d87 (2026-08-09, CC_REPORT_BAKEOFF_SCORECARD.md): 7 of 104 judged
-- groups failed. Six replies were cut off mid-word inside the `reason` string —
-- every one of them while writing `"relevant": true` — and the seventh emitted no
-- text block at all, having spent the whole budget thinking. The counter-intuitive
-- tell: the FAILED replies were shorter (101-328 chars) than the successful ones
-- (377 average), because the loss happened upstream of the text.
--
-- ## The bounds (ruling R1)
--
-- min 256: above the largest observed successful reply (575 chars, roughly 150
-- tokens) with room for thinking, so a value that cannot produce a verdict at all
-- cannot be typed.
--
-- max 64000: the LOWEST `max_output_tokens` on any Anthropic row in the registry
-- (measured: opus-5 / 4-8 / 4-6 and sonnet-5 / 4-6 all 64000; opus-4-7 128000).
-- Above that, `constrain` would silently clamp and the stored number would stop
-- describing what is sent. The bound keeps the row honest about its own effect.
--
-- ## A CONSEQUENCE FOR THE vLLM MODELS — read this before running a Qwen scan
--
-- The two vLLM rows carry `max_output_tokens = 2048`. `constrain` is
-- CLAMP-BY-ERROR, not clamp: `max_tokens > ceiling` returns
-- `LlmConfigError::MaxTokensExceedsCeiling` and the scan REFUSES TO START, naming
-- the model, the request and the ceiling. It does not quietly proceed at 2048.
--
-- So with this row at 8192, a scan judged by either Qwen model fails at parameter
-- resolution — where the old compiled-in 512 sailed under the ceiling and worked.
-- The failure is loud, named and recoverable (lower the row on the Settings page,
-- or raise the model's `max_output_tokens` if the server really allows more), and
-- that is exactly the kind of decision this row now exists to let an operator
-- make without a rebuild. But it IS a behaviour change for the local models and
-- nobody should meet it by surprise.
--
-- Recorded here rather than in the row's `meaning`: the meaning is read by
-- somebody choosing a number, and this is a fact about two specific registry rows
-- that will stop being true the moment either one is edited.
--
-- ## consumed_by (ruling R2)
--
-- NULL — read on every scan, so it sorts into the LIVE group on the Settings page
-- rather than the dormant one.
--
-- ## The deploy ordering hazard
--
-- `theme_scan_max_tokens` is declared to the boot loader (`REQUIRED_KEYS`), and a
-- declared key with no row makes the backend REFUSE TO START — there are no
-- compiled-in defaults left to serve with (v2 §2b), which is the entire point of
-- this change. The runtime Migrator applies this at backend boot, before the
-- settings load, so a normal deploy orders itself. A rollback to an older image is
-- safe (an extra row is ignored); a roll-FORWARD without this file is not.
--
-- Forward-only, idempotent, no down migration — the house rule for every seed.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    -- Key and VALUE stay on ONE line: the disk/code consistency test
    -- (`settings_store_tests::the_fixtures_carry_the_values_the_migration_actually_seeds`)
    -- reads the seeded value straight out of this file by matching `('key', '`,
    -- and a line break between them leaves it finding nothing at all.
    ('theme_scan_max_tokens', '8192',
     'count',
     '8192',
     256, 64000,
     'The most the judge may write per candidate, thinking included. Too small '
     'cuts verdicts off mid-sentence — on 2026-08-09 a budget of 512 killed 7 of '
     '104 verdicts on a model that thinks before it answers, and the scan reported '
     'them as failures rather than as findings. A model that does not think needs '
     'far less; there is no cost to the headroom, because only what is actually '
     'written is billed.',
     NULL, now(), 'system (seed)')

ON CONFLICT (key) DO NOTHING;
