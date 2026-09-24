-- witness_starting_line_and_name_joiner: the witness starts at zero, and the
-- reviewers' names are joined by a word rather than by a dot
--
-- Created: 2026-09-23 21:39:50
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_FOR_YOU_POLISH_v1 §2 and §3 (GO rulings 2 and 4 of 2026-09-23).
-- =============================================================================
--
-- ## §2 — why the witness needs a starting line at all
--
-- Her "For you" list is derived, not stored: it is every item on her side that
-- she has no `practice_item_seen` row for. On the day this feature reaches her
-- that is the whole case — every note a reviewer has ever written on her
-- answers, and every question that has ever been reworded. She has answered 185
-- questions across 11 scenarios, and a list that opens with a hundred unread
-- rows is not an inbox, it is a wall. The badge would say the same number on
-- every page of the app, and it would never go down, because nothing she does
-- today clears what was written in August.
--
-- So this marks everything that already exists as seen FOR HER, once, at the
-- moment it runs. From then on her list holds only what arrives after go-live,
-- which is the only list she can actually work through.
--
-- ## Whose items — the audience table, copied from the code that reads it
--
-- `WaitingSide::Witness` in `repositories::pipeline_repository::waiting_items`
-- defines her side in SQL as:
--
--     a note whose author is on practice_reviewer_usernames   → waits for her
--     a reworded or edited question (field 'text')            → waits for her
--
-- Everything else — an answer, a note by anybody who is not a listed reviewer —
-- waits for the REVIEWERS, and none of it is touched here. That is the point of
-- the task: the review backlog is real work that somebody still owes, while her
-- backlog is an artefact of the feature arriving late.
--
-- ## Deliberately a SUPERSET of that table, and why
--
-- Two widenings, both chosen rather than stumbled into:
--
--   * every question change is marked, not only the `reworded`/`edited` kinds
--     on field `text` that count as waiting today. If that definition ever
--     widens — and it has already moved once, on 2026-09-22 — a narrower
--     baseline would let 2026's changes reappear in her list years later. The
--     starting line has to hold against the reading of "waiting" changing.
--   * a struck (withdrawn) note is marked too, although it is filtered out of
--     the list while it stands struck. Same argument, one line shorter.
--
-- Marking an item seen says only "she is not being nagged about this". It hides
-- nothing: every note stays on its question and in the notes panel, exactly
-- where it was.
--
-- ## The cut-off is the item's own stamp against now() (GO ruling 4)
--
-- ⚑ The write and its assertion live in ONE `DO` block, holding the instant in a
-- variable, and that is not tidiness. `now()` is the TRANSACTION's timestamp:
-- sqlx runs a migration inside one transaction, so two separate blocks would
-- agree there — but a hand-applied proof (`psql -f`, which is how this file is
-- mutation-proved) runs every statement in a transaction of its own, and the
-- two blocks then read two different instants. Measured on 2026-09-23: with the
-- assertion in its own block, a deliberately broken write that swept the
-- REVIEWERS' backlog along with hers passed, because the guard was looking for
-- rows stamped with an instant that no longer matched. An assertion that cannot
-- fail is worse than none — it is a green light with nothing behind it.
--
-- ## A blank witness row is SAID OUT LOUD, not passed over (GO ruling 4)
--
-- If `practice_witness_username` is missing or blank there is nobody to mark
-- anything for, and this writes nothing. That is the correct behaviour and it
-- is also indistinguishable, in a migration log, from a join that silently
-- matched nothing — which is the failure this project calls silent. So the
-- blank case RAISEs a NOTICE naming the row and stating the consequence, and
-- the boot log carries it.
--
-- ## Undo (there is no down-migration in this project)
--
-- Every row written here carries one timestamp — the `stamp` variable below,
-- read once — so the exact reverse is:
--
--     DELETE FROM practice_item_seen
--      WHERE user_id = <the witness> AND seen_at = <that timestamp>;
--
-- The joiner's previous value was a bare '·' in both `value` and
-- `default_value`.

-- ─── 1 · The witness's starting line, and the two assertions that prove it ──
--
-- The END-state assertions (CLAUDE.md rule 25a) are (a) and (b) at the foot of
-- this block. An INSERT … SELECT that matched nothing is SILENT in Postgres,
-- and this one is a join across three tables and a settings row: a wrong key
-- writes nothing and leaves a store that looks untouched. On PROD that means
-- Marie opens her list to the whole backlog — the exact outcome this exists to
-- prevent. Both are stated as PROPERTIES rather than as row counts, because a
-- count would have to be edited every time DEV gains a note.
DO $$
DECLARE
    witness   TEXT;
    -- One instant, read once, used by the write and by both assertions.
    stamp     TIMESTAMPTZ := now();
    notes     BIGINT := 0;
    changes   BIGINT := 0;
    unseen    INTEGER;
    reviewers INTEGER;
BEGIN
    SELECT btrim(value) INTO witness
      FROM app_settings
     WHERE key = 'practice_witness_username' AND btrim(value) <> '';

    IF witness IS NULL THEN
        RAISE NOTICE
            'for you: practice_witness_username is blank or missing, so no '
            'starting line was written for the witness. Nothing is broken, but '
            'whoever is named in that row later will open "For you" with every '
            'note and every question change in the case unread. Set the witness '
            'on the Settings page before she signs in.';
        RETURN;
    END IF;

    -- Notes written by a listed reviewer. `author_id` is a login, never a
    -- display name, and the reviewer list is stored comma-separated — the same
    -- shape `waiting_items` binds and the L0 back-fill unnested.
    INSERT INTO practice_item_seen (user_id, note_id, seen_at)
    SELECT witness, n.id, stamp
      FROM practice_notes n
     WHERE n.created_at <= stamp
       AND n.author_id IS NOT NULL
       AND n.author_id IN (
            SELECT btrim(name)
              FROM app_settings, LATERAL unnest(string_to_array(value, ',')) AS name
             WHERE key = 'practice_reviewer_usernames' AND btrim(name) <> '')
    -- Re-runnable: sqlx applies a file once, but the mutation proof re-applies
    -- it by hand and must not die on the second pass.
    ON CONFLICT DO NOTHING;
    GET DIAGNOSTICS notes = ROW_COUNT;

    INSERT INTO practice_item_seen (user_id, change_id, seen_at)
    SELECT witness, d.id, stamp
      FROM practice_deck_changes d
     WHERE d.changed_at <= stamp
    ON CONFLICT DO NOTHING;
    GET DIAGNOSTICS changes = ROW_COUNT;

    RAISE NOTICE
        'for you: the witness (%) starts at zero — % note(s) and % question '
        'change(s) marked as already seen for her. Her list from here holds '
        'only what arrives after this migration.', witness, notes, changes;

    -- (a) The property this block exists to produce: not one item on HER side,
    -- stamped at or before `stamp`, is still unseen for her. A wrong join key
    -- would write nothing, and this is what catches it.
    WITH reviewer_logins AS (
            SELECT btrim(name) AS login
              FROM app_settings, LATERAL unnest(string_to_array(value, ',')) AS name
             WHERE key = 'practice_reviewer_usernames' AND btrim(name) <> ''),
         hers AS (
            SELECT n.id
              FROM practice_notes n
             WHERE n.created_at <= stamp
               AND n.author_id IS NOT NULL
               AND n.author_id IN (SELECT login FROM reviewer_logins)
            UNION ALL
            SELECT d.id
              FROM practice_deck_changes d
             WHERE d.changed_at <= stamp)
    SELECT count(*) INTO unseen
      FROM hers i
     WHERE NOT EXISTS (SELECT 1 FROM practice_item_seen v
                        WHERE v.user_id = witness
                          AND (v.note_id = i.id OR v.change_id = i.id));
    IF unseen <> 0 THEN
        -- The witness is named here as well as in the NOTICE above: an
        -- exception line is what gets pasted into a ticket, on its own, hours
        -- later, and "the witness" is not an answer to "which account?".
        RAISE EXCEPTION
            'for you: % item(s) on the witness''s (%) side are still unread for '
            'her after her starting line was written — the baseline did not '
            'land, and she would open her list to the whole backlog',
            unseen, witness;
    END IF;

    -- (b) And the REVIEWERS were not swept along with her. Their backlog is
    -- work somebody still owes; a baseline that cleared it too would look like
    -- a success here and lose real work silently. Nobody but the witness may
    -- hold a seen row carrying this block's own stamp.
    SELECT count(*) INTO reviewers
      FROM practice_item_seen v
     WHERE v.seen_at = stamp
       AND v.user_id <> witness;
    IF reviewers <> 0 THEN
        RAISE EXCEPTION
            'for you: this migration wrote % seen row(s) for somebody other '
            'than the witness — the reviewers'' backlog must not be touched',
            reviewers;
    END IF;
END $$;

-- ─── 2 · "Chuck and Roman", not "Chuck · Roman" (§3) ─────────────────────────
--
-- The joiner sits between two reviewers' names wherever a screen prints the
-- bench — the subtitle of her "For you" list, the review pill, the owner chip.
-- A dot between two people's names reads as a list of tags; the sentence it
-- sits in is about two human beings, and it should say so.
--
-- Stored WITHOUT its spaces: `settings_store::text_of` trims every value, and
-- `war_room_progress::reviewer_display_line` supplies a space on each side. A
-- value of ' and ' would arrive as 'and' anyway; a value of 'and' is the honest
-- statement of what is stored.
--
-- Fresh-database reproduction (Law 20): the seeding migration
-- 20260919100133_review_page_reviewer_list_and_wording.sql writes '·' and sorts
-- earlier, so a database built from zero runs that one and then this one, and
-- ends on 'and'.
UPDATE app_settings
   SET value         = 'and',
       default_value = 'and',
       meaning       = 'Between two reviewers'' names wherever a screen prints the bench — the subtitle of the "For you" list, the review pill, the owner chip. Stored WITHOUT its spaces: the store trims every value, so the server supplies the space on each side. It is a word rather than a dot because the sentence it sits in is about people, and a dot between two names reads as a list of tags.',
       updated_at    = now(),
       updated_by    = 'migration'
 WHERE key           = 'practice_review_name_joiner';

-- ─── 3 · The joiner's END-state assertion (CLAUDE.md rule 25a) ──────────────
--
-- An UPDATE whose WHERE matched nothing is silent too, and a mistyped key would
-- leave the bench printing two names joined by a dot under a build that no
-- longer means it. `default_value` is checked beside `value` so that "reset to
-- default" on the Settings page cannot quietly bring the dot back.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM app_settings
                    WHERE key = 'practice_review_name_joiner'
                      AND value = 'and' AND default_value = 'and') THEN
        RAISE EXCEPTION
            'for you: practice_review_name_joiner is not "and" in both value '
            'and default_value — the bench would still print two names joined '
            'by a dot';
    END IF;
END $$;
