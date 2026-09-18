-- include_picker_wording: the six sentences the Include picker speaks
--
-- Created: 2026-09-17
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_INCLUDE_PICKER_v1 — the Include button asks its two questions
-- =============================================================================
--
-- ## The defect these rows serve
--
-- Since FACT_CARD_v2 §2, including a fact has written the Proof Matrix link as
-- well as the ruling (ruling R32), so `POST …/facts/:id/action` refuses an
-- include that does not name the accusation it bears on and which way it cuts:
--
--     "Including a fact needs the accusation it bears on and which way it cuts."
--
-- The browser has never sent either field — `applyFactAction` composed
-- `{action}` and nothing else — so EVERY Include in the triage queue has
-- returned 400. Roman hit it on PROD S-13 (C-129, C-127).
--
-- The fix is to ask. Clicking Include now opens a row under the card: the
-- accusation it goes under, and whether it helps us or helps them. These are the
-- words on that row.
--
-- ## Why "Helps us" and not "supports"
--
-- The graph's own tokens are `supports` and `rebuts` (`domain::fact_card`, 802
-- and 142 edges measured) and they are NOT renamed here — a prettier synonym in
-- the store would leave the wire, the drafted cards on disk and this row saying
-- three different things. What a curator reads is the question they are actually
-- answering: does this fact help us or help them? The mapping between the two
-- vocabularies lives in ONE place, the picker, beside a comment that says so.
--
-- ## Why a card with no bears-on gets a prompt rather than a default
--
-- `card_include_picker_choose_prompt` is the empty option on a card the
-- extraction linked to nothing. Defaulting such a card to the first accusation
-- in a scenario-wide list would file a fact under an accusation nobody chose,
-- and it would do it on exactly the cards nobody has looked at yet.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('card_include_picker_label', 'Include under:', 'text', 'Include under:',
     NULL, NULL,
     'The Include picker''s opening words, before the accusation select. It names '
     'what the select is FOR: the fact is being filed under an accusation, not '
     'merely marked as kept.',
     'components::IncludePickerRow', now(), 'system (seed)'),

    ('card_include_picker_choose_prompt', 'Choose an accusation…', 'text',
     'Choose an accusation…',
     NULL, NULL,
     'The empty option, shown on a card the extraction linked to no accusation. '
     'Such a card opens with NOTHING selected and Save refused until a human '
     'picks one — a default here would file a fact under an accusation nobody '
     'chose, on exactly the cards nobody has read yet.',
     'components::IncludePickerRow', now(), 'system (seed)'),

    ('card_include_picker_helps_us_label', 'Helps us', 'text', 'Helps us',
     NULL, NULL,
     'The supports half of the stance control, in the words a curator is actually '
     'thinking in. The WIRE token stays `supports` — that is the graph''s own '
     'vocabulary (r.stance) and renaming it would break the drafted cards on '
     'disk. This row is the human half of that mapping.',
     'components::IncludePickerRow', now(), 'system (seed)'),

    ('card_include_picker_helps_them_label', 'Helps them', 'text', 'Helps them',
     NULL, NULL,
     'The rebuts half. Same rule as its sibling: the wire token stays `rebuts`, '
     'which is the word the graph carries and the word Job B wrote into the 59 '
     'drafted cards.',
     'components::IncludePickerRow', now(), 'system (seed)'),

    ('card_include_picker_save_label', 'Save', 'text', 'Save',
     NULL, NULL,
     'Commits the accusation and the stance — the one control on this row that '
     'sends anything.',
     'components::IncludePickerRow', now(), 'system (seed)'),

    ('card_include_picker_cancel_label', 'Cancel', 'text', 'Cancel',
     NULL, NULL,
     'Closes the row and sends nothing. A chosen absence, distinct from a save '
     'that failed: the card keeps whatever receipt it already carried.',
     'components::IncludePickerRow', now(), 'system (seed)')
ON CONFLICT (key) DO NOTHING;

-- ─── The END-state assertion (CLAUDE.md rule 25a) ────────────────────────────
--
-- A statement matching zero rows is SILENT in Postgres. Here that would mean a
-- backend that refuses to boot on the next deploy — `CARD_GRAMMAR_WORDING_KEYS`
-- enumerates all six and `build_card_grammar_wording` names the missing one —
-- so the failure is brought forward to the migration that was supposed to
-- prevent it.
DO $$
DECLARE
    present INTEGER;
BEGIN
    SELECT count(*) INTO present
      FROM app_settings
     WHERE key IN (
        'card_include_picker_label',
        'card_include_picker_choose_prompt',
        'card_include_picker_helps_us_label',
        'card_include_picker_helps_them_label',
        'card_include_picker_save_label',
        'card_include_picker_cancel_label');

    IF present <> 6 THEN
        RAISE EXCEPTION
            'include picker wording: expected 6 settings rows after this '
            'migration, found %. The backend declares every card_include_picker_* '
            'key at boot and refuses to start without them, so the Include '
            'button would go on returning 400 with no row to explain it.', present;
    END IF;
END $$;
