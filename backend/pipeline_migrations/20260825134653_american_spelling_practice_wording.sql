-- american_spelling_practice_wording: British "practise" becomes American "practice"
--
-- Created: 2026-08-25
-- Target: pipeline database (colossus_legal_v2)
--
-- Ruled by Roman, 2026-08-25, from the live S-8 page: the product speaks
-- American English. Six rows carried the British spelling in text a person
-- actually reads — four in a `value` a screen prints, and three in a `meaning`
-- the Settings page prints (`SettingsPage.tsx:109` renders `setting.meaning`,
-- which is why a meaning is user-facing text and not a code comment).
--
-- A CORRECTION, not an edit. The migrations that seeded these rows are applied
-- on DEV and PROD; their files are history and are not touched. This is the
-- established shape — see 20260823101322, which corrected `practice_intro` the
-- same way.
--
-- ## ⚑ THE COLUMN ALIGNMENT BELOW IS LOAD-BEARING
--
-- `domain::wording::tests::corrected_value_in` finds a correction by searching
-- for the assignment to `value`, padded so its `=` lines up with the one on the
-- `default_value` line beneath it, and likewise for the `key` comparison in the
-- WHERE clause. Copy the shape of the statements below EXACTLY; a correction
-- written with different padding is INVISIBLE to the fixture tests, which then
-- keep reading the original INSERT, go green, and disagree with the store. That
-- is the drift those tests exist to catch, reported as a pass. The migration
-- that first documented this warning is 20260823101322.
--
-- ⚑ AND NOTE WHAT THIS PARAGRAPH CAREFULLY DOES NOT DO: quote those two literal
-- strings. `corrected_value_in` does not strip comments, so a header that spells
-- them out becomes a decoy the parser finds BEFORE the real statement. That is
-- not hypothetical — this file did exactly that on the first attempt, and the
-- fixture then reported this migration's own prose as the stored value of
-- `practice_practice_hint`. See `wording_tests.rs`'s "prose versus parser" note,
-- which predicted it.
--
-- ## ⚑ THE TWO MEANING-ONLY UPDATES COME FIRST, AND THAT ORDER MATTERS
--
-- `corrected_value_in` locates a key's WHERE clause and then scans BACKWARDS for
-- the nearest assignment to `value`. A statement that sets only `meaning` has no
-- such line of its own — so if one sat AFTER a value correction in this file, a
-- lookup for that key would walk back into the PREVIOUS statement and return a
-- value belonging to a different row. Putting the two meaning-only corrections
-- at the top means a backward scan from either of them finds nothing in this
-- file and correctly falls through to the seeded value.
--
-- ## What is deliberately NOT changed
--
-- · THE KEYS. `practice_start_practising_label` and `practice_practise_again_label`
--   keep their names. A key is an identifier, not a rendered word: it appears in
--   Rust constants, in the frontend `w("…")` calls and in the boot loader's
--   declared-key list, and renaming one is a code change across four files for
--   no reader's benefit. Roman's ruling is about what a person reads.
-- · THE DECK YAML. `backend/practice_decks/*.yaml` is Chuck-reviewed content and
--   is out of scope by the task. (Measured: it contains no British spelling
--   anyway — the census found zero.)
-- · `updated_by` is `'migration'`, the value every sibling correction uses, so
--   the change log reads consistently.

UPDATE app_settings
SET meaning       = 'The IANA zone this case''s days are counted in. It decides when "answered today" becomes "last: <date>" on a deck row, and when the unfinished-session line says "today". Case data, not a deployment value: the witness practices in the evening in Michigan, and comparing in UTC ended her day at 20:00 local. Postgres does the comparing, so any zone it knows is valid and one it does not know fails the read loudly rather than falling back to UTC.',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'practice_case_timezone';

UPDATE app_settings
SET meaning       = 'The one line of hint beside Start practicing. Standing rule of 2026-08-19: no control on a practice page is dim and silent — a person must be able to tell what a control does before pressing it.',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'practice_practice_hint';

UPDATE app_settings
SET value         = 'These are seed questions, drafted from the record. An attorney must review them before anyone practices answering.',
    default_value = 'These are seed questions, drafted from the record. An attorney must review them before anyone practices answering.',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'practice_intro';

UPDATE app_settings
SET value         = 'There is nothing to practice yet — practice walks the questions you have already answered.',
    default_value = 'There is nothing to practice yet — practice walks the questions you have already answered.',
    meaning       = 'Shown when the chosen side has no answered questions. Domain note: practicing an answer she has not written is nothing to practice, so the walk offers only answered questions, and the empty case says why rather than showing an empty screen.',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'practice_practice_none_answered';

UPDATE app_settings
SET value         = 'Practice them again',
    default_value = 'Practice them again',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'practice_practise_again_label';

UPDATE app_settings
SET value         = 'Start practicing',
    default_value = 'Start practicing',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'practice_start_practising_label';
