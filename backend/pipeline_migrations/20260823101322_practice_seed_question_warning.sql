-- practice_seed_question_warning: the line under the scenario title becomes a warning.
--
-- Created: 2026-08-23 10:13:22
-- Target: pipeline database (colossus_legal_v2)
--
-- ## What changes, and why it is a CORRECTION rather than a re-wording
--
-- The start card's second line encouraged Marie to practise:
--
--   "Twenty minutes, one accusation, no clock, nobody watching. Answer out loud
--    first, then type it in a sentence or two. You'll see your own three points
--    after every answer."
--
-- Every question in every deck on this system is a SEED — drafted from the record
-- by the architect, never reviewed by an attorney. Encouraging a witness to
-- rehearse answers to unreviewed questions is the wrong instruction, and the line
-- that did it sat directly under the scenario's own title. It is replaced with
-- the warning, on Roman's ruling of 2026-08-22.
--
-- ## Domain note: the warning is PERMANENT
--
-- It does not disappear once a deck has been reviewed. "Reviewed" is not a state
-- this system tracks, and inventing one in order to hide a warning would be
-- trading a real safeguard for a tidier screen. If that is ever to change it is
-- ruled separately.
--
-- ## Domain note: it does NOT go on the printout
--
-- Chuck's review sheets already tell him the deck is his to mark up, and this
-- line is aimed at HIM — the attorney whose review it asks for. Printing it on
-- his own sheet would be the paper asking him to wait for himself.
--
-- ## ⚑ THE FORMAT OF THIS STATEMENT IS LOAD-BEARING
--
-- `settings_store_tests` and `wording_practice_tests` find a correction by
-- searching for `SET value         = '` and `WHERE key           =` with exactly
-- this alignment. Written any other way the correction is invisible to them and
-- they go green while the store holds something else — the drift those tests
-- exist to catch. The v1 migration that moved the read prompt carries the same
-- warning; this is that warning obeyed.
--
-- Purely additive in the sense that matters: no answer, note, flag or change-log
-- row is read or written. This edits ONE parameter row.
UPDATE app_settings
SET value         = 'These are seed questions, drafted from the record. An attorney must review them before anyone practises answering.',
    default_value = 'These are seed questions, drafted from the record. An attorney must review them before anyone practises answering.',
    meaning       = 'The line under the scenario title on the practice start card. Domain '
                    'note: this is a WARNING, not an invitation. Every deck on this system '
                    'is seeded from the record and unreviewed, and the line it replaced '
                    'encouraged a witness to rehearse answers to questions no attorney had '
                    'read. It is PERMANENT — it does not vanish once a deck is reviewed, '
                    'because "reviewed" is not a state this system tracks and inventing one '
                    'to hide a warning is the wrong trade. It does not print: Chuck''s '
                    'review sheets are aimed at the very attorney this asks for.',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'practice_intro';
