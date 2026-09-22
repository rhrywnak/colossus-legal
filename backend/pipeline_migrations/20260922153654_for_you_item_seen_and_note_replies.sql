-- for_you_item_seen_and_note_replies: read-state one level finer than the deck
--
-- Created: 2026-09-22 15:36:54
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_FOR_YOU_v1, layer L0 (plan §4.2, §5.3, §5.4; GO rulings Q2 blind,
-- Q4, Q6). Schema and back-fill only — no wording rows, no reader is re-pointed
-- at this table in this layer.
-- =============================================================================
--
-- ## What this replaces, and why a finer grain was needed
--
-- `practice_review_cursor` holds ONE moment per (person, deck): "I have read up
-- to here". It answers "does this deck hold anything new" and nothing else. It
-- cannot answer "WHICH question is Chuck waiting on", because a watermark has no
-- idea what it swept past — so Marie, who has answered 185 questions across 11
-- scenarios, has no way to find the one he wrote on.
--
-- This table records the same fact one level finer: one row per (person, ITEM)
-- they have seen. The deck-level count is then derived from it — the same
-- derivation the cursor already did, over a set instead of a timestamp.
--
-- ## An ITEM is a row that ALREADY exists
--
--     item = ('answer', practice_answers.id)         — the CURRENT answer of a question
--          | ('note',   practice_notes.id)           — while it stands (not struck)
--          | ('change', practice_deck_changes.id)    — kind 'reworded' or 'edited'
--
-- No new event table, and NOTHING is written when an event happens. The events
-- are already stored, already stamped and already attributed; all that was
-- missing is who has looked at them. That is the same argument
-- `practice_review_cursor`'s own header makes ("events and read-state are
-- separate"), and it is why the counts on the War Room stay derived — there is
-- no stored counter here to drift out of step with the answers.
--
-- ## Why three nullable columns and not (kind TEXT, item_id UUID)
--
-- The precedent is `practice_notes` itself (migration 20260819113610): a
-- `question_id` and an `answer_id`, exactly one of which is meant, held straight
-- by a CHECK. What that shape buys is REFERENTIAL INTEGRITY. Deleting a question
-- cascades to its answers, its notes and its changes, and their seen rows go with
-- them. A `(kind, item_id)` pair cannot cascade: the seen rows would outlive
-- their items as silent orphans that no query could ever match again — precisely
-- the class of quiet wrongness the no-silent-failures rule exists to stop.
--
-- The cost is stated honestly: every read carries three join legs rather than one.
--
-- ## practice_review_cursor is NOT dropped here (GO ruling Q6)
--
-- It stops being read one layer later, and it is dropped one RELEASE later. A
-- drop in the same migration as the back-fill would be a rollback with no way
-- back: the back-fill reads the cursor, so a revert that needed to run it again
-- would find nothing to read from. The COMMENT below records the retirement so
-- the next reader of the schema is not left guessing.

-- ─── 1 · The seen record ─────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS practice_item_seen (
    -- `AuthUser.username`, the same stable id every practice write is stamped
    -- with (`practice_sessions.user_id`, `practice_notes.author_id`,
    -- `practice_review_cursor.user_id`). Never a display name.
    user_id   TEXT        NOT NULL CHECK (btrim(user_id) <> ''),
    -- Exactly one of the three is set. CASCADE on each: when the item is gone,
    -- the record of having seen it means nothing.
    answer_id UUID        REFERENCES practice_answers(id)      ON DELETE CASCADE,
    note_id   UUID        REFERENCES practice_notes(id)        ON DELETE CASCADE,
    change_id UUID        REFERENCES practice_deck_changes(id) ON DELETE CASCADE,
    -- When this person FIRST saw it. The writes are `ON CONFLICT DO NOTHING`, so
    -- a second look never moves this — the first reading is the one worth
    -- keeping, the same argument striking a note makes about its first strike.
    seen_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- The database refuses a row naming two items, or none. `num_nonnulls` is
    -- Postgres's own counter for exactly this shape, and a CHECK here means no
    -- caller can invent a fourth kind by leaving all three NULL.
    CONSTRAINT practice_item_seen_one_target
        CHECK (num_nonnulls(answer_id, note_id, change_id) = 1)
);

-- One partial UNIQUE index per leg. Partial because two of the three columns are
-- NULL on every row, and a plain UNIQUE over a NULL-bearing pair does not stop a
-- duplicate (NULLs never equal each other, so `('roman', NULL, NULL, x)` could be
-- inserted twice). These are also the indexes the reader's NOT EXISTS probe uses:
-- (user_id, leg) is exactly the lookup "has this person seen this item".
CREATE UNIQUE INDEX IF NOT EXISTS idx_practice_item_seen_answer
    ON practice_item_seen (user_id, answer_id) WHERE answer_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_practice_item_seen_note
    ON practice_item_seen (user_id, note_id)   WHERE note_id   IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_practice_item_seen_change
    ON practice_item_seen (user_id, change_id) WHERE change_id IS NOT NULL;

COMMENT ON TABLE practice_item_seen IS
    'Read-state only, one row per (person, item) they have seen. An item is an '
    'existing row: the current answer to a question, a standing note, or a '
    'reworded/edited deck change. Nothing is written when the event happens — '
    'only when somebody looks. Replaces the deck-level watermark in '
    'practice_review_cursor; every count is still derived at read time.';

-- ─── 2 · A note may ANSWER another note (plan §5.2) ──────────────────────────
-- Nullable: almost every note stands on its own. When it is set, this note is a
-- reply to that one, which is what makes an exchange readable as an exchange
-- rather than as two unrelated remarks that happen to share a question.
--
-- CASCADE for the reason the sibling columns give: a reply to a note that no
-- longer exists answers nothing. (Notes are never deleted in the application —
-- striking is the withdrawal — so this fires only when the question above them
-- goes.)
ALTER TABLE practice_notes
    ADD COLUMN IF NOT EXISTS answers_note_id UUID
    REFERENCES practice_notes(id) ON DELETE CASCADE;

COMMENT ON COLUMN practice_notes.answers_note_id IS
    'The note this one replies to, or NULL for a note that stands on its own. '
    'The reply and its parent must sit on the same question — a cross-row rule '
    'no CHECK can express, enforced by the repository on write.';

CREATE INDEX IF NOT EXISTS idx_practice_notes_answers
    ON practice_notes (answers_note_id) WHERE answers_note_id IS NOT NULL;

-- ─── 3 · Carrying the watermark forward (plan §5.4) ──────────────────────────
--
-- Without a back-fill, on the day this ships every person sees every item ever
-- written as unread — 158 questions' worth on DEV and more on PROD. The rule is
-- "nothing that was already read reappears", so the existing watermarks are
-- converted into seen rows before anybody looks.
--
-- ## WHOSE rows are written, and why more than one person's
--
-- Since 2026-09-22 (CC_TASK_REVIEW_PERMISSION_v1, R1) the cursor is SHARED: any
-- permitted press cleared the deck's count for everyone. Marking only the person
-- who pressed would therefore make items reappear for the others — the exact
-- failure this back-fill exists to prevent. So the set is everybody who ever
-- pressed, plus everybody on today's reviewer list.
--
-- Residual, stated rather than hidden: an administrator who is neither listed nor
-- ever pressed sees that deck's pre-watermark backlog once. Nobody on DEV is in
-- that position (measured 2026-09-22: the only cursor rows are roman's).
--
-- ## One statement, three legs
--
-- The three item sources are folded into one `items` CTE and the leg is chosen
-- by CASE at INSERT time, rather than three near-identical statements that could
-- drift apart. The CHECK on the table guarantees exactly one of the three CASEs
-- produced a value — if `kind` ever held a fourth string, every column would be
-- NULL and the row would be REFUSED rather than quietly written.
WITH watermarks AS (
        SELECT scenario_id, MAX(looked_at) AS looked_at
          FROM practice_review_cursor
         GROUP BY scenario_id),
     people AS (
        SELECT DISTINCT user_id FROM practice_review_cursor
        UNION
        SELECT btrim(name)
          FROM app_settings, LATERAL unnest(string_to_array(value, ',')) AS name
         WHERE key = 'practice_reviewer_usernames' AND btrim(name) <> ''),
     items AS (
        SELECT 'answer'::text AS kind, a.id, s.scenario_id, a.answered_at AS at
          FROM practice_answers a JOIN practice_sessions s ON s.id = a.session_id
        UNION ALL
        SELECT 'note', n.id, n.scenario_id, n.created_at FROM practice_notes n
        UNION ALL
        SELECT 'change', d.id, d.scenario_id, d.changed_at FROM practice_deck_changes d)
INSERT INTO practice_item_seen (user_id, answer_id, note_id, change_id, seen_at)
SELECT p.user_id,
       CASE WHEN i.kind = 'answer' THEN i.id END,
       CASE WHEN i.kind = 'note'   THEN i.id END,
       CASE WHEN i.kind = 'change' THEN i.id END,
       m.looked_at
  FROM watermarks m
  JOIN items i ON i.scenario_id = m.scenario_id AND i.at <= m.looked_at
  CROSS JOIN people p
ON CONFLICT DO NOTHING;

-- ─── 4 · The witness's side has never had a watermark ────────────────────────
--
-- Her "new or changed" count compares each question against HER OWN last answer
-- in that scenario (`war_room_status::changed_counts`) — stored state she never
-- had. So her back-fill uses that same predicate, once, as a starting line: every
-- item older than her most recent answer in a scenario is marked seen. Her count
-- on the morning of the deploy is then the number she was shown the evening
-- before, rather than every question in the case.
--
-- The witness is a settings row, never a name in this file. If that row is
-- missing or blank, `witness_marks` is empty, this statement writes nothing, and
-- §6(b) still passes — she simply starts with everything unread, which is the
-- honest reading of a store that does not say who she is.
WITH witness AS (
        SELECT btrim(value) AS user_id FROM app_settings
         WHERE key = 'practice_witness_username' AND btrim(value) <> ''),
     witness_marks AS (
        SELECT w.user_id, s.scenario_id, MAX(a.answered_at) AS answered_at
          FROM witness w
          JOIN practice_sessions s ON s.user_id = w.user_id
          JOIN practice_answers a ON a.session_id = s.id
         GROUP BY w.user_id, s.scenario_id),
     items AS (
        SELECT 'answer'::text AS kind, a.id, s.scenario_id, a.answered_at AS at
          FROM practice_answers a JOIN practice_sessions s ON s.id = a.session_id
        UNION ALL
        SELECT 'note', n.id, n.scenario_id, n.created_at FROM practice_notes n
        UNION ALL
        SELECT 'change', d.id, d.scenario_id, d.changed_at FROM practice_deck_changes d)
INSERT INTO practice_item_seen (user_id, answer_id, note_id, change_id, seen_at)
SELECT w.user_id,
       CASE WHEN i.kind = 'answer' THEN i.id END,
       CASE WHEN i.kind = 'note'   THEN i.id END,
       CASE WHEN i.kind = 'change' THEN i.id END,
       w.answered_at
  FROM witness_marks w
  JOIN items i ON i.scenario_id = w.scenario_id AND i.at <= w.answered_at
ON CONFLICT DO NOTHING;

-- ─── 5 · Notes written before attribution existed (GO ruling Q2, BLIND) ──────
--
-- `practice_notes.author_id` arrived on 2026-08-19 (migration 20260819135156).
-- A note older than that carries no author id at all, and an item with no author
-- has no side: it cannot be said to wait for the reviewers or for the witness,
-- because nobody can tell who it came from. Left alone it would wait for the
-- reviewers for ever, and nothing could clear it.
--
-- Ruled blind on 2026-09-22, before PROD was read: such a note is marked seen for
-- everyone this store knows about. It stays exactly where it is — visible on the
-- question, visible in the notes panel, never hidden — it simply stops nagging.
--
-- "Everyone this store knows about" is the union of five sets: everybody who has
-- pressed Done, everybody on the reviewer list, the witness, everybody who has
-- ever answered, and everybody who has ever written a note. A person outside all
-- five has never touched this case's practice. Residual, stated: somebody who
-- signs in for the first time AFTER this migration is not in the union and would
-- see those notes once.
WITH known_people AS (
        SELECT DISTINCT user_id FROM practice_review_cursor
        UNION
        SELECT btrim(name)
          FROM app_settings, LATERAL unnest(string_to_array(value, ',')) AS name
         WHERE key = 'practice_reviewer_usernames' AND btrim(name) <> ''
        UNION
        SELECT btrim(value) FROM app_settings
         WHERE key = 'practice_witness_username' AND btrim(value) <> ''
        UNION
        SELECT user_id FROM practice_sessions
         WHERE user_id IS NOT NULL AND btrim(user_id) <> ''
        UNION
        SELECT author_id FROM practice_notes
         WHERE author_id IS NOT NULL AND btrim(author_id) <> '')
INSERT INTO practice_item_seen (user_id, note_id, seen_at)
SELECT k.user_id, n.id, n.created_at
  FROM known_people k
  JOIN practice_notes n ON n.author_id IS NULL
ON CONFLICT DO NOTHING;

-- ─── 6 · The END-state assertion (CLAUDE.md rule 25a) ────────────────────────
--
-- An INSERT that matched zero rows is silent in Postgres. Every statement above
-- is an INSERT … SELECT over joins, and any one of them could match nothing: a
-- mistyped column would not compile, but a join landing on the wrong key simply
-- writes nothing and leaves the store looking untouched. So the END state is
-- asserted directly, as a PROPERTY rather than as a row count — a count would
-- have to be edited every time DEV gains a note.
DO $$
DECLARE
    missing      INTEGER;
    leftovers    INTEGER;
    witness_rows INTEGER;
BEGIN
    -- (a) The shape exists. `CREATE … IF NOT EXISTS` against a pre-existing table
    -- of the same name and a DIFFERENT shape is silent, which is how this could
    -- be wrong with nothing raising.
    IF to_regclass('public.practice_item_seen') IS NULL THEN
        RAISE EXCEPTION 'for you: practice_item_seen was not created';
    END IF;
    IF (SELECT count(*) FROM pg_indexes
         WHERE tablename = 'practice_item_seen'
           AND indexname IN ('idx_practice_item_seen_answer',
                             'idx_practice_item_seen_note',
                             'idx_practice_item_seen_change')) <> 3 THEN
        RAISE EXCEPTION 'for you: the three per-leg unique indexes are not all present';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                    WHERE table_name = 'practice_notes'
                      AND column_name = 'answers_note_id') THEN
        RAISE EXCEPTION 'for you: practice_notes.answers_note_id was not added';
    END IF;

    -- (b) The watermark back-fill landed: not one item at or before a watermark
    -- is still unseen for a person that watermark covered. This is the property
    -- §3 exists to produce, and the one a wrong join key would silently fail.
    WITH watermarks AS (
            SELECT scenario_id, MAX(looked_at) AS looked_at
              FROM practice_review_cursor GROUP BY scenario_id),
         people AS (
            SELECT DISTINCT user_id FROM practice_review_cursor
            UNION
            SELECT btrim(name)
              FROM app_settings, LATERAL unnest(string_to_array(value, ',')) AS name
             WHERE key = 'practice_reviewer_usernames' AND btrim(name) <> ''),
         items AS (
            SELECT a.id, s.scenario_id, a.answered_at AS at
              FROM practice_answers a JOIN practice_sessions s ON s.id = a.session_id
            UNION ALL
            SELECT n.id, n.scenario_id, n.created_at FROM practice_notes n
            UNION ALL
            SELECT d.id, d.scenario_id, d.changed_at FROM practice_deck_changes d)
    SELECT count(*) INTO missing
      FROM watermarks m
      JOIN items i ON i.scenario_id = m.scenario_id AND i.at <= m.looked_at
      CROSS JOIN people p
     WHERE NOT EXISTS (SELECT 1 FROM practice_item_seen v
                        WHERE v.user_id = p.user_id
                          AND (v.answer_id = i.id OR v.note_id = i.id
                               OR v.change_id = i.id));
    IF missing <> 0 THEN
        RAISE EXCEPTION
            'for you: % item(s) at or before a watermark are still unseen for the '
            'person that watermark covered — the back-fill did not land', missing;
    END IF;

    -- (d) The WITNESS's starting line landed too. §3's and §5's INSERTs each
    -- have an assertion above; §4's had none, and a silent zero-write from it
    -- looked exactly like the legitimate "no witness is configured" case. Two
    -- states, one observable — which is the failure this project calls silent.
    -- So the check is in two parts: the property §4 produces, and, when she IS
    -- configured and has answered anything, the flat fact that she came out of
    -- this migration with at least one seen row.
    WITH witness AS (
            SELECT btrim(value) AS user_id FROM app_settings
             WHERE key = 'practice_witness_username' AND btrim(value) <> ''),
         witness_marks AS (
            SELECT w.user_id, s.scenario_id, MAX(a.answered_at) AS answered_at
              FROM witness w
              JOIN practice_sessions s ON s.user_id = w.user_id
              JOIN practice_answers a ON a.session_id = s.id
             GROUP BY w.user_id, s.scenario_id),
         items AS (
            SELECT a.id, s.scenario_id, a.answered_at AS at
              FROM practice_answers a JOIN practice_sessions s ON s.id = a.session_id
            UNION ALL
            SELECT n.id, n.scenario_id, n.created_at FROM practice_notes n
            UNION ALL
            SELECT d.id, d.scenario_id, d.changed_at FROM practice_deck_changes d)
    SELECT count(*) INTO missing
      FROM witness_marks w
      JOIN items i ON i.scenario_id = w.scenario_id AND i.at <= w.answered_at
     WHERE NOT EXISTS (SELECT 1 FROM practice_item_seen v
                        WHERE v.user_id = w.user_id
                          AND (v.answer_id = i.id OR v.note_id = i.id
                               OR v.change_id = i.id));
    IF missing <> 0 THEN
        RAISE EXCEPTION
            'for you: % item(s) at or before the witness''s own last answer are '
            'still unseen for her — her starting line did not land', missing;
    END IF;

    SELECT count(*) INTO witness_rows
      FROM app_settings s
      JOIN practice_sessions ps ON ps.user_id = btrim(s.value)
      JOIN practice_answers a ON a.session_id = ps.id
     WHERE s.key = 'practice_witness_username' AND btrim(s.value) <> '';
    IF witness_rows > 0
       AND NOT EXISTS (SELECT 1 FROM practice_item_seen v
                        WHERE v.user_id = (SELECT btrim(value) FROM app_settings
                                            WHERE key = 'practice_witness_username')) THEN
        RAISE EXCEPTION
            'for you: the witness is configured and has % answer(s), yet she came '
            'out of this migration with no seen row at all', witness_rows;
    END IF;

    -- (c) And the ruled-blind sweep landed: no author-less note is left unseen
    -- for anybody this store knows about.
    WITH known_people AS (
            SELECT DISTINCT user_id FROM practice_review_cursor
            UNION
            SELECT btrim(name)
              FROM app_settings, LATERAL unnest(string_to_array(value, ',')) AS name
             WHERE key = 'practice_reviewer_usernames' AND btrim(name) <> ''
            UNION
            SELECT btrim(value) FROM app_settings
             WHERE key = 'practice_witness_username' AND btrim(value) <> ''
            UNION
            SELECT user_id FROM practice_sessions
             WHERE user_id IS NOT NULL AND btrim(user_id) <> ''
            UNION
            SELECT author_id FROM practice_notes
             WHERE author_id IS NOT NULL AND btrim(author_id) <> '')
    SELECT count(*) INTO leftovers
      FROM known_people k
      JOIN practice_notes n ON n.author_id IS NULL
     WHERE NOT EXISTS (SELECT 1 FROM practice_item_seen v
                        WHERE v.user_id = k.user_id AND v.note_id = n.id);
    IF leftovers <> 0 THEN
        RAISE EXCEPTION
            'for you: % author-less note(s) are still unseen for somebody this '
            'store knows about', leftovers;
    END IF;
END $$;

-- ─── 7 · The watermark's retirement, recorded in the schema (ruling Q6) ──────
COMMENT ON TABLE practice_review_cursor IS
    'RETIRED 2026-09-22 (CC_TASK_FOR_YOU_v1). Kept, unread, so this release stays '
    'revertable and the back-fill in migration 20260922153654 stays re-runnable; '
    'dropped in a later release. Replaced by practice_item_seen, which records the '
    'same fact one level finer: one row per (person, item) seen, instead of one '
    'moment per (person, deck).';
