-- practice_one_page_l1_answered_on: the only status a deck row will carry.
--
-- Created: 2026-08-23 12:36:57
-- Target: pipeline database (colossus_legal_v2)
--
-- ## What this is for
--
-- CC_TASK_PRACTICE_ONE_PAGE §3 strips the deck row down to the question, its
-- pills, where it was built from, and ONE fact about state: whether there is an
-- answer behind it. `Answered on 22 Aug` when there is; nothing at all when
-- there is not.
--
-- The marks it replaces — `answered today · fine`, `repeat`, `attempt 2`,
-- `skipped today`, the `review` link — are retired from the interface. Their
-- rows are NOT deleted: `practice_row_answered_today_template` and its siblings
-- still seed a block the boot loader declares, and dropping a row a running
-- build reads is how you make the backend refuse to start. They simply stop
-- being rendered.
--
-- ## Domain note: no weekday
--
-- Its siblings say `last: Wed 19 Aug`, where the weekday earns its place — that
-- line is about RECENCY and "Wed" is how a person places a sitting against their
-- own week. This line is not about recency. It answers "is there an answer
-- behind this question, and roughly when", and on every row of a list the
-- weekday is three characters of noise. The format itself is a code constant
-- (`practice_clock::DAY_MONTH_FORMAT`) and deliberately NOT a settings row: a
-- strftime string is the one kind of value this store cannot validate, and a
-- typo would print `22 %v` onto the screen with every check green.
--
-- ## ⚑ FORMAT NOTE
--
-- One INSERT, purely additive, `ON CONFLICT (key) DO NOTHING` so a re-run is a
-- no-op. `value` and `default_value` must each be ONE quoted literal on ONE
-- line: `wording::tests::seeded_value_in` reads the first quoted literal it
-- finds after the key, and adjacent-literal concatenation is invisible to it —
-- the fixture would then pin a truncated string and go green over the wrong
-- words.
--
-- And the key must sit IMMEDIATELY after the opening paren — `VALUES ('key',` —
-- because the parser's marker is the two characters `('` followed by the key.
-- This file was first written with a newline and an indent between them, which
-- is perfectly good SQL and completely invisible to the fixture: the test then
-- reported the key as seeded by NO migration at all.
--
-- No answer, note, flag or change-log row is read or written.
INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_row_answered_on_template',
    'Answered on {when}',
    'Answered on {when}',
    'string',
    'The deck row''s only status line, shown when an answer exists and withheld entirely when one does not. {when} is the day the current answer was given, in the case''s own timezone, without a weekday (e.g. "22 Aug"). Domain note: an empty status line under a question reads as a status that FAILED TO LOAD, which is why the absent case renders nothing rather than an empty string.',
    'migration'
)
ON CONFLICT (key) DO NOTHING;
