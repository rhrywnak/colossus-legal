-- seed_opus_5_temperature_mode_and_failed_honesty_wording
--
-- Created: 2026-08-09 15:36:30
-- Target: pipeline database (colossus_legal_v2)
--
-- Task CC_TASK_MODEL_PARAMS_AND_FAILED_HONESTY_v1, as ruled by the architect
-- in CC_TASK_MODEL_PARAMS_PHASE_B_AUTHORIZATION_v1 (rulings R2, R4, R5).
--
-- ## The incident, in one paragraph
--
-- Scan run 6a9fad89 (2026-08-09 14:57, `claude-opus-5`, scenario S-4) attempted
-- 104 judge calls. Every one returned HTTP 400 "temperature is deprecated for
-- this model", inside five seconds. The run recorded `completed`, the screen
-- read "Complete · 104 judged · 0 relevant", and — because only the latest
-- COMPLETED run projects — it took the projecting slot from the Opus 4.8 run
-- before it and projected nothing. Thirty proposals left the queue and nothing
-- said why.
--
-- This file carries the three data changes that follow from it. The code changes
-- (the resolver's unsafe default, the FAILED status predicate, the reconciliation
-- checks) ride the same commit.
--
-- ## 1. The row that caused it
--
-- `claude-opus-5` was the ONLY Anthropic row in the registry with no
-- `temperature_mode` recorded — every sibling that needs one already says
-- `omit`. NULL used to resolve to "send 0.0"; as of this build it resolves to
-- "send nothing", so the 400s stop either way. The row is set explicitly anyway,
-- because a model's capability being RECORDED and a model's capability being
-- INFERRED FROM AN ABSENCE are different states, and only the first survives the
-- next person reading the registry.
--
-- Guarded on `IS NULL`: if the value was already set — by the interim UPDATE
-- Roman may have applied by hand, or by an operator on the new admin form — the
-- human's value stands. Same respect for a human edit that every
-- `ON CONFLICT (key) DO NOTHING` below gives.
--
-- ## 2. Five new SCAN rows + one correction (ruling R4)
--
-- The report had no word for a failed call: `ScanConservation` carried no
-- `failed` field, so the tiles and the conservation sentence — both built from
-- that block — could not show one. The five rows are the failed tile's caption,
-- the clause the sentence splices in, the two status pills, and the collapsed
-- card's line for a dead run. The correction adds the `{failed}` SLOT to the
-- conservation template; it renders as nothing at all on a clean run, which is
-- why the clause carries its own separator.
--
-- ## 3. Seven new MODELS-ADMIN rows (ruling R5)
--
-- There was no screen on which to record what a model does with `temperature`.
-- These are that screen's words — asked as "does this model accept a
-- temperature?" rather than as the `zero-ok` / `omit` tokens the column stores,
-- because the operator is answering the first question and not the second.
--
-- ## The deploy ordering hazard
--
-- All twelve new keys are declared to the boot loader (`SCAN_WORDING_KEYS`,
-- `MODEL_PARAMS_WORDING_KEYS`), and a declared key with no row makes the backend
-- REFUSE TO START — there are no compiled-in defaults to serve with (v2 §2b).
-- The runtime Migrator applies this at backend boot, before the settings load,
-- so a normal deploy orders itself. A rollback to an older image is safe (extra
-- rows are ignored); a roll-FORWARD without this file is not.
--
-- Forward-only, idempotent, no down migration — the house rule for every seed.

-- ─── 1. The model row that 400'd 104 times ───────────────────────────────────

UPDATE llm_models
   SET temperature_mode = 'omit'
 WHERE id = 'claude-opus-5'
   AND temperature_mode IS NULL;

-- ─── 2. + 3. The twelve new stored strings ───────────────────────────────────

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    -- ── The scan report learns the word "failed" ─────────────────────────────
    --
    -- Spliced into the conservation sentence ONLY when the count is nonzero, so
    -- a clean run never reads "· 0 failed" — a zero there teaches the eye to skip
    -- the term, and the one run where it matters is the run where it stops being
    -- zero. It carries its own separator because only the clause knows whether it
    -- is present; a separator left in the parent template would strand a "· " on
    -- every clean run.
    ('scan_conservation_failed_clause_template',
     '· {failed} failed',
     'text',
     '· {failed} failed',
     NULL, NULL,
     'The clause the scan''s reconciliation sentence splices in when judged calls '
     'failed. Must keep {failed}. Shown only when the count is nonzero, and it '
     'carries its own separator — the store trims a stored string, so the single '
     'space that joins it to the preceding term is added by the renderer.',
     NULL, now(), 'system (seed)'),

    ('scan_report_tile_failed', 'failed', 'text', 'failed',
     NULL, NULL,
     'Caption of the scan report''s failed tile: judged calls that came back with '
     'no verdict. Shown only when the count is nonzero — a permanent zero tile '
     'reads as decoration.',
     NULL, now(), 'system (seed)'),

    -- ── The two pills ────────────────────────────────────────────────────────
    --
    -- "Complete" was a literal compiled into ThemeScanPanel. It becomes a row now
    -- because it acquired a sibling it can be WRONG about: a pill that can read
    -- either word is a pill that has to be able to read the other one.
    ('scan_status_complete_label', 'Complete', 'text', 'Complete',
     NULL, NULL,
     'The pill on a scan run whose judged calls came back. Sits beside the failed '
     'pill; the two should read as opposites at a glance.',
     NULL, now(), 'system (seed)'),

    ('scan_status_failed_label', 'Failed', 'text', 'Failed',
     NULL, NULL,
     'The pill on a scan run whose every judged call failed. Such a run does not '
     'project candidates and does not supersede the previous run''s projection.',
     NULL, now(), 'system (seed)'),

    ('scan_card_collapsed_failed_template',
     'Last scan {when} · {model} · Failed — {count} calls errored',
     'text',
     'Last scan {when} · {model} · Failed — {count} calls errored',
     NULL, NULL,
     'The one line the collapsed scan card shows when the latest run FAILED. Must '
     'keep {when}, {model} and {count}. This is the line Roman read on 2026-08-09 '
     'as "0 proposed" about a run that never judged anything.',
     NULL, now(), 'system (seed)'),

    -- ── Admin → Models: what this model does with `temperature` ──────────────
    ('model_temperature_mode_label', 'Temperature parameter', 'text',
     'Temperature parameter',
     NULL, NULL,
     'Label above the temperature-capability dropdown on Admin → Models.',
     NULL, now(), 'system (seed)'),

    -- The VALUE and DEFAULT_VALUE literals stay on ONE line each, deliberately:
    -- the disk/code consistency test (`wording_model_params_tests`) reads the
    -- seeded string straight out of this file, and SQL's adjacent-literal
    -- concatenation would leave it comparing half a sentence. The `meaning`
    -- column is not read by that test and may still wrap.
    ('model_temperature_mode_help',
     'What this model does with a temperature setting. This is not a preference: a model that has retired the parameter rejects every call that sends one.',
     'text',
     'What this model does with a temperature setting. This is not a preference: a model that has retired the parameter rejects every call that sends one.',
     NULL, NULL,
     'The sentence under the temperature dropdown on Admin → Models. Its job is to '
     'stop the control reading as a tuning knob — the wrong value here does not '
     'produce different answers, it produces no answers.',
     NULL, now(), 'system (seed)'),

    ('model_temperature_mode_omit_label',
     'Send no temperature — this model rejects it',
     'text',
     'Send no temperature — this model rejects it',
     NULL, NULL,
     'The dropdown option for a model that has retired the parameter (stored as '
     'the token "omit"). Every current Claude model is this.',
     NULL, now(), 'system (seed)'),

    ('model_temperature_mode_zero_ok_label',
     'Send an exact temperature',
     'text',
     'Send an exact temperature',
     NULL, NULL,
     'The dropdown option for a model that accepts a temperature (stored as the '
     'token "zero-ok"). Choosing it makes the numeric value below editable.',
     NULL, now(), 'system (seed)'),

    ('model_temperature_mode_unset_label',
     'Not recorded yet — nothing is sent',
     'text',
     'Not recorded yet — nothing is sent',
     NULL, NULL,
     'How an unrecorded row reads before anybody chooses. Shown, never offered: a '
     'mode can be recorded and cannot be un-recorded from this form, because '
     '"nobody has said" is a gap to close rather than a setting to pick. Since '
     '2026-08-09 an unrecorded mode behaves exactly like "omit", and the sentence '
     'says so rather than leaving the reader to guess what is going out meanwhile.',
     NULL, now(), 'system (seed)'),

    ('model_temperature_value_label', 'Temperature value', 'text',
     'Temperature value',
     NULL, NULL,
     'Label above the numeric temperature field on Admin → Models.',
     NULL, now(), 'system (seed)'),

    ('model_temperature_value_disabled_help',
     'Available once this model is set to send a temperature.',
     'text',
     'Available once this model is set to send a temperature.',
     NULL, NULL,
     'Why the numeric temperature field is unavailable while the selected mode '
     'sends no temperature at all. Without it the greyed field reads as broken.',
     NULL, now(), 'system (seed)')

ON CONFLICT (key) DO NOTHING;

-- ─── The correction: the conservation sentence gains its {failed} slot ────────
--
-- Guarded on the exact seeded value, exactly as the scan-to-ruling migration
-- guarded its own correction: if Roman has edited this sentence on the Settings
-- page, his words stand and this statement touches nothing.

UPDATE app_settings
   SET value = '{pool} gathered · {collapsed} duplicates folded · {excluded} set aside before judging · {judged} judged{failed} · {relevant} relevant',
       default_value = '{pool} gathered · {collapsed} duplicates folded · {excluded} set aside before judging · {judged} judged{failed} · {relevant} relevant',
       meaning = 'The scan''s reconciliation sentence, composed at read time from a run''s frozen counts. Must keep {pool}, {collapsed}, {excluded}, {judged}, {failed} and {relevant} — a sentence claiming to reconcile with a term missing is worse than no sentence, because it looks checked. {failed} is a SLOT the failed clause lands in and renders as nothing on a clean run.',
       updated_at = now(),
       updated_by = 'system (failed-honesty correction)'
 WHERE key = 'scan_conservation_line_template'
   AND value = '{pool} gathered · {collapsed} duplicates folded · {excluded} set aside before judging · {judged} judged · {relevant} relevant';
