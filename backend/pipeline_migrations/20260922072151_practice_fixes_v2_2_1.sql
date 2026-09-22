-- practice_fixes_v2_2_1: four fixes from the DEV walk of v2.2.0
--
-- Created: 2026-09-22 07:21:51
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_PRACTICE_FIXES_v2.2.1, ruled by CC_TASK_PRACTICE_FIXES_v2.2.1_GO.
-- Four new wording rows and the read prompt's move to v5.
-- =============================================================================
--
-- ## ⚑ THE PROMPT FILE MUST BE ON BOTH MOUNTS BEFORE THIS DEPLOYS
--
-- Boot calls std::process::exit(1) when `practice_read_prompt_file` names a file
-- the template directory lacks — services/settings_template_file.rs. This
-- migration points that row at practice_read_prompt_v5.md. Push the file to
-- /mnt/data/legal-docs/extraction_templates on DEV AND on PROD FIRST, mode 644
-- (`scripts/push-templates.sh` and `scripts/push-templates.sh PROD` both chmod
-- it), and verify its md5 inside the container. A backend deployed ahead of the
-- file does not start — which is exactly how v2.2.0's DEV deploy refused boot.
--
-- v4 stays on the mounts untouched, exactly as v1–v3 have, so the rollback of
-- the prompt half is one UPDATE of this row back to practice_read_prompt_v4.md
-- and no file work. The v4 file has no re-request corrections section; the read
-- then re-requests plainly, as it did before this migration.
--
-- ## What this migration does NOT touch
--
-- No stored read. Old answers keep the `read_text`, `read_call`, `read_why` and
-- `read_pointers` they were written with — including any internal key in the
-- prose, and the old "I can't read this one." line on an old failure. Only
-- reads made after this deploys follow v5 and the failure line below.

-- ─── 1 · Wording ─────────────────────────────────────────────────────────────
-- Idempotent: ON CONFLICT (key) DO NOTHING, so a re-run changes nothing and a row
-- somebody edited is never clobbered.
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('practice_chat_close_label', 'Close discussion', 'text', 'Close discussion', NULL, NULL,
     'The close (×) button''s accessible name and tooltip, in the side panel''s header after the expand button. Shuts the panel and drops ?discuss from the address. Full screen keeps Back and Collapse instead.',
     NULL, now(), 'migration'),

    ('practice_row_answer_saved_template', 'Your answer — saved {when}', 'text', 'Your answer — saved {when}', NULL, NULL,
     'The label over the answer box once an answer is saved. {when} is the day and time of the CURRENT saved answer in the case''s own timezone (e.g. "Wed 21 Sep · 10:54 pm"). With no saved answer the label is practice_answer_label unchanged. Domain note: pressing Answer on unchanged text writes no version, so the time stays the original one.',
     NULL, now(), 'migration'),

    ('practice_row_answer_saved_off_line', 'Saved. Answer analysis is off, so no read was requested.', 'text', 'Saved. Answer analysis is off, so no read was requested.', NULL, NULL,
     'The one line under the question page''s buttons after Answer is pressed with Answer analysis switched OFF. Decided by the server, which is what knows no model was asked. With analysis on, the read itself is the confirmation and this line never shows.',
     NULL, now(), 'migration'),

    ('practice_read_failed_line', 'Your answer is saved. The quick read didn''t come through this time — press Answer to try again, or use Discuss this answer to talk it through.', 'text', 'Your answer is saved. The quick read didn''t come through this time — press Answer to try again, or use Discuss this answer to talk it through.', NULL, NULL,
     'The ONE line the witness reads whenever the read fails for a system reason: the read could not be set up, the model could not be reached, an input did not load, or the reply was unusable twice (unparseable, empty, citing an unknown key, or naming internal keys in its prose). Domain note: no technical reason ever reaches her (Roman, 2026-09-21); the specific cause stays in read_error / read_abstain_reason and the logs. A MODEL decline is different and is not this line: it shows practice_read_abstain_line followed by the model''s own sentence about her answer.',
     NULL, now(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ─── 2 · The read's prompt moves to v5 (v4 stays on disk) ────────────────────
--
-- v5 names points, receipts and sworn statements by what they say — never by key
-- — in `call`, `why` and `pointers`, and carries the one-sentence corrections the
-- re-request sends. Pointing this row back at v4 is the whole rollback of the
-- prompt half.
UPDATE app_settings
SET value         = 'practice_read_prompt_v5.md',
    default_value = 'practice_read_prompt_v5.md',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'practice_read_prompt_file';

-- ─── 3 · The END-state assertion (CLAUDE.md rule 25a) ────────────────────────
DO $$
DECLARE
    present        INTEGER;
    prompt         TEXT;
    prompt_default TEXT;
BEGIN
    SELECT count(*) INTO present
      FROM app_settings
     WHERE key IN (
        'practice_chat_close_label',
        'practice_row_answer_saved_template',
        'practice_row_answer_saved_off_line',
        'practice_read_failed_line');
    IF present <> 4 THEN
        RAISE EXCEPTION
            'practice fixes v2.2.1: expected 4 new settings rows, found %. The backend '
            'declares every one at boot and refuses to start without it.', present;
    END IF;

    SELECT value, default_value INTO prompt, prompt_default
      FROM app_settings WHERE key = 'practice_read_prompt_file';
    IF prompt IS DISTINCT FROM 'practice_read_prompt_v5.md'
       OR prompt_default IS DISTINCT FROM 'practice_read_prompt_v5.md' THEN
        RAISE EXCEPTION
            'practice fixes v2.2.1: practice_read_prompt_file is % (default %), '
            'expected practice_read_prompt_v5.md for both', prompt, prompt_default;
    END IF;
END $$;
