-- practice_polish_wording: the five rows the practice polish needs
--
-- Created: 2026-09-15
-- Target: pipeline database
--
-- =============================================================================
-- Five rows for four changes from Marie's first week
-- =============================================================================
--
-- CC_TASK_PRACTICE_POLISH_v1, ruled by Roman on
-- PRACTICE_DEFAULTS_MOCKUP_v1_2026-09-15. Two of the four changes carry new
-- words; two carry none (a default that moves and a switch's behaviour are not
-- sentences).
--
-- ## The three analysis rows
--
-- The mockup draws a switch on the practice bar under Practice mode: a label and
-- ONE state word beside it. The word is the switch's CURRENT state, not a pair of
-- choices, so `on` and `off` are two rows rather than one template: they are read
-- separately, one at a time, and a template with a slot would make the screen
-- compose a word out of a boolean.
--
-- ## Why the state word is a row at all
--
-- It is three letters and it is the only thing on the page that says whether a
-- model will be asked to read what Marie types. A literal here would be a word no
-- operator could change on a control whose whole purpose is to withhold a model
-- call, and "off" is a word a witness has to trust.
--
-- ## The two editor rows
--
-- `practice_editor_field_receipt` labels the "Built from" box, which becomes
-- editable in this task. Every other field in that form already has its own
-- label row (`practice_editor_field_question`, `…_tactic`, `…_watch_for`,
-- `…_stronger`), so a literal here would be the one label in the stack an
-- operator could not edit.
--
-- `practice_editor_tactic_none` is the blank option of the new tactic dropdown.
-- It is NOT `practice_editor_attach_none`, whose value is the words `no receipt`:
-- that row belongs to the attach control one field along, and reusing it would
-- put "no receipt" in a dropdown about cross-examination tactics. Two controls,
-- two vocabularies, two rows — the same reasoning that keeps the side picker's
-- two options on the rows the side CARDS already own.
--
-- Idempotent: `ON CONFLICT (key) DO NOTHING`, so a re-run changes nothing and a
-- row Roman has edited is never clobbered.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    -- ── The answer-analysis switch, on the practice bar ──────────────────────
    ('practice_answer_analysis_label', 'Answer analysis', 'text',
     'Answer analysis', NULL, NULL,
     'The label beside the switch on the practice bar that decides whether a '
     'model is asked to read what Marie types. OFF is the default and off means '
     'the read is never requested — no model call at all, not a call whose '
     'result is hidden.',
     NULL, now(), 'system (seed)'),

    ('practice_answer_analysis_on', 'on', 'text', 'on', NULL, NULL,
     'The switch''s state word when the analysis is ON. A word and not a '
     'template: the screen reads whichever of the two states holds, and never '
     'composes one from a boolean.',
     NULL, now(), 'system (seed)'),

    ('practice_answer_analysis_off', 'off', 'text', 'off', NULL, NULL,
     'The switch''s state word when the analysis is OFF — the default, and the '
     'word a witness reads to know that nothing she types is being sent to a '
     'model.',
     NULL, now(), 'system (seed)'),

    -- ── The deck editor's two new words ──────────────────────────────────────
    ('practice_editor_field_receipt', 'Built from', 'text', 'Built from',
     NULL, NULL,
     'The label of the receipt box in the deck editor. It names the line the '
     'deck row prints under a question ("Built from: …"), which Chuck can now '
     'edit like every other authored field. Blank clears it.',
     NULL, now(), 'system (seed)'),

    ('practice_editor_tactic_none', 'no tactic', 'text', 'no tactic',
     NULL, NULL,
     'The blank option of the tactic dropdown in both editor forms — a cross '
     'question that carries no card. Deliberately not practice_editor_attach_none '
     '("no receipt"), which belongs to the attach control beside it.',
     NULL, now(), 'system (seed)')
ON CONFLICT (key) DO NOTHING;

-- ─── The row-count assertion (CLAUDE.md rule 25a) ─────────────────────────────
--
-- A statement matching zero rows is SILENT in Postgres, and `ON CONFLICT DO
-- NOTHING` makes that possible on purpose. A key mistyped above inserts nothing,
-- this migration reports success, and the backend then refuses to start with a
-- missing-key error naming the key and not the file that should have seeded it.
-- Assert the END state.
DO $$
DECLARE
    present INTEGER;
BEGIN
    SELECT count(*) INTO present
      FROM app_settings
     WHERE key IN (
        'practice_answer_analysis_label',
        'practice_answer_analysis_on',
        'practice_answer_analysis_off',
        'practice_editor_field_receipt',
        'practice_editor_tactic_none');

    IF present <> 5 THEN
        RAISE EXCEPTION
            'practice polish wording: expected 5 settings rows after this '
            'migration, found %. The backend declares all five keys at boot '
            '(PRACTICE_LIST_WORDING_KEYS and PRACTICE_EDITOR_WORDING_KEYS) and '
            'refuses to start when one is missing, so this migration fails here '
            'rather than at the next deploy.', present;
    END IF;
END $$;
