-- practice_witness_row_and_done_confirm_wording: the count's OWNER stops badging herself
--
-- Created: 2026-09-20 16:13:41
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_REVIEW_COUNTS_HONEST_v1 — the WORDING/SETTINGS migration. The junk
-- answer's data fix is a SEPARATE file; nothing here deletes anything.
-- =============================================================================
--
-- ## Five rows, two defects
--
-- 1. `practice_witness_username` — the one row that says who the War Room's
--    "new or changed" count BELONGS to. Marie's own note to Chuck badged her own
--    tile: the note LATERAL in `war_room_status::changed_counts` filtered only
--    struck notes and answer linkage, with no author leg at all, so every note
--    she wrote came back to her as work she had not read.
--
-- 2. Four sentences for a confirmation on Done reviewing. That button fired a
--    write on ONE click with no question and no undo — the same hazard class as
--    the bare `Scan again` link before 2026-09-11, and the same remedy.
--
-- ## Why the WITNESS is a settings row and not a comparison against the viewer
--
-- This build deliberately has no is-the-reader-the-witness concept (CC's
-- 2026-09-16 STOP), and the count is GLOBAL by ruling (2026-09-17): every viewer
-- sees the same number, so it cannot be filtered per viewer. What it CAN be
-- filtered by is its OWNER — the one person the number is addressed to. That
-- owner is a fact about this case, which is to say a row.
--
-- Seeded `docmarie`, which is the login Marie has signed in as since
-- 2026-09-17 (`20260917094039_reattribute_practice_sessions_to_marie.sql`).
--
-- ## Why Chuck's queue is untouched
--
-- His exclusion is its own LATERAL in `review_cursor.rs` and already drops
-- notes written by anyone on the reviewer bench. Marie is not on that bench, so
-- her notes must keep counting as work awaiting Chuck. This file adds a filter
-- to HER count only.
--
-- ## Why the confirmation has a singular AND a plural (ruled 2026-09-20)
--
-- The bar's own sentence has had both since GO v3 — "1 answers" does not ship
-- to lawyers. The confirmation is the one screen whose entire job is to be read
-- before an irreversible click, so it is the last place to print bad grammar.
-- `pickByCount` is the single reader that decides between them.
--
-- ## Idempotent
--
-- ON CONFLICT (key) DO NOTHING, and the assertion checks the END state either
-- way (CLAUDE.md rule 25a). An operator who has already pointed the witness row
-- at a different login keeps their choice.
--
-- ## Rolling this back
--
-- Delete the five rows. This build refuses to start without them, which is the
-- point: a missing witness row must not degrade to "filter nobody".

-- ─── 1 · The witness ─────────────────────────────────────────────────────────

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('practice_witness_username', 'docmarie', 'text', 'docmarie', NULL, NULL,
     'The signed-in username (Authentik) of the witness the War Room''s "new or '
     'changed" count is addressed to. Notes she writes herself do not badge her '
     'own tile — a note is a message to somebody else, and a count that included '
     'her own is a count of work nobody is waiting on. Question EDITS still count '
     'whoever made them, including hers. This does NOT filter by viewer: the count '
     'is one global number (ruled 2026-09-17) and this row names its OWNER.',
     'api::war_room_progress, services::war_room_progress::practice_witness, '
     'repositories::war_room_status::changed_counts', now(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ─── 2 · The Done reviewing confirmation ─────────────────────────────────────
--
-- Roman's ruling of 2026-09-20: a confirmation QUESTION is answered "Yes",
-- never restated as "Done". The affirmative label is not the button's label.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('practice_deck_review_confirm_template', 'Mark all {count} answers in {code} as reviewed?', 'text', 'Mark all {count} answers in {code} as reviewed?', NULL, NULL,
     'The question asked when Done reviewing is pressed, for a count that is not '
     '1. {count} is the number the bar is showing and {code} is the deck''s '
     'S-code — both are named because the press moves a SHARED mark for the whole '
     'reviewer bench and cannot be undone. Both placeholders are required.',
     'frontend PracticeReviewBar', now(), 'migration'),
    ('practice_deck_review_confirm_one', 'Mark the {count} answer in {code} as reviewed?', 'text', 'Mark the {count} answer in {code} as reviewed?', NULL, NULL,
     'The same question when exactly one answer waits. A separate row rather than '
     'a clever plural rule, for the reason countWording.ts gives: English takes '
     'the singular at 1 and the plural everywhere else, 0 included. Both '
     'placeholders are required here too.',
     'frontend PracticeReviewBar', now(), 'migration'),
    ('practice_deck_review_confirm_yes_label', 'Yes', 'text', 'Yes', NULL, NULL,
     'The affirmative on the Done reviewing confirmation, and the ONLY control '
     'in the application that moves the review mark. A question is answered Yes '
     '(ruled 2026-09-20) — restating the verb here would ask the reader to '
     'confirm a sentence they have already read.',
     'frontend PracticeReviewBar', now(), 'migration'),
    ('practice_deck_review_confirm_cancel_label', 'Cancel', 'text', 'Cancel', NULL, NULL,
     'The retreat on the Done reviewing confirmation. Sends nothing, moves '
     'nothing, and leaves the count exactly as it stands. Escape does the same.',
     'frontend PracticeReviewBar', now(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ─── The END-state assertion (CLAUDE.md rule 25a) ────────────────────────────
--
-- Four silent failures are possible and Postgres reports none of them: an
-- INSERT that collided and did nothing; a blank value; a confirmation template
-- that lost a placeholder and would print a raw brace to Chuck's screen; and a
-- witness row naming a login no session has ever used, which would filter
-- nobody and leave the defect exactly where it was.

DO $$
DECLARE
    present   INTEGER;
    witness   TEXT;
    template  TEXT;
    key_name  TEXT;
    sightings INTEGER;
BEGIN
    SELECT count(*) INTO present
      FROM app_settings
     WHERE key IN ('practice_witness_username',
                   'practice_deck_review_confirm_template',
                   'practice_deck_review_confirm_one',
                   'practice_deck_review_confirm_yes_label',
                   'practice_deck_review_confirm_cancel_label');
    IF present <> 5 THEN
        RAISE EXCEPTION
            'review counts honest: expected 5 new settings rows, found %. The '
            'backend declares every one at boot and refuses to start without it.',
            present;
    END IF;

    SELECT value INTO witness FROM app_settings WHERE key = 'practice_witness_username';
    IF witness IS NULL OR btrim(witness) = '' THEN
        RAISE EXCEPTION
            'review counts honest: practice_witness_username is missing or blank. '
            'A blank witness filters nobody and the tile goes on badging her.';
    END IF;

    -- BOTH confirmation sentences, because the singular is the one a reader
    -- meets least often and so the one a typo survives longest in.
    FOREACH key_name IN ARRAY ARRAY['practice_deck_review_confirm_template',
                                    'practice_deck_review_confirm_one'] LOOP
        SELECT value INTO template FROM app_settings WHERE key = key_name;
        IF position('{count}' in template) = 0 OR position('{code}' in template) = 0 THEN
            RAISE EXCEPTION
                'review counts honest: % reads %, which is missing {count} or '
                '{code}. A confirmation that does not name what it is about is '
                'not a confirmation.', key_name, template;
        END IF;
    END LOOP;

    -- A WARNING, never a refusal. On a fresh database nobody has practised yet
    -- and zero sightings is correct; on a store with history, zero means the
    -- login is wrong and the filter would do nothing. The deploy proceeds and
    -- the log says so, because refusing here would block a legitimate new
    -- environment.
    SELECT count(*) INTO sightings
      FROM practice_sessions WHERE user_id = witness;
    IF sightings = 0 THEN
        RAISE WARNING
            'review counts honest: practice_witness_username names %, and no '
            'practice_sessions row carries that login. On a fresh store that is '
            'expected. On a store with history it means the row is wrong and her '
            'own notes will go on badging her — check it on the Settings page.',
            witness;
    END IF;
END $$;
