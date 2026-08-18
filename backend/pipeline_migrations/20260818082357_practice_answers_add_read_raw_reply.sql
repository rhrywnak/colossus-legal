-- practice_answers_add_read_raw_reply: Practice answers add read raw reply
--
-- Created: 2026-08-18 08:23:57
-- Target: pipeline database (colossus_legal_v2)
--
-- Task CC_TASK_PRACTICE_V0_FIX_v1. The column the v0 code has named since
-- `f142bff` and the v0 migration never created.
--
-- ## What this fixes
--
-- `insert_answer` writes fourteen columns, one of them `read_raw_reply`. The
-- 20260817213319 migration creates sixteen columns, and `read_raw_reply` is not
-- among them. So on DEV every "Answer" press ended the same way:
--
--     rows_affected=0
--     column "read_raw_reply" of relation "practice_answers" does not exist
--
-- and Marie was told "Your answer was not recorded." Nothing else in v0 is
-- wrong: the read itself returned, on time and inside its cap. This one absent
-- column is the whole of the outage.
--
-- ## Why the column is wanted at all
--
-- When the parser REFUSES a reply — a paragraph, or a sentence over the word cap
-- — the screen says "no system read this time" and `read_text` stays NULL. That
-- is the right thing to show a witness and the wrong thing to leave in the
-- record: diagnosing a wave of refusals means reading what the model actually
-- wrote, and the log window rolls over. So the refused reply is kept on the row
-- that refused it. NULL on an accepted read (`read_text` is the model's own line
-- then) and on a call that never returned.
--
-- ## Why `IF NOT EXISTS`
--
-- DEV may already carry the column by hand — it was added there to unblock a
-- sitting. `ADD COLUMN IF NOT EXISTS` applies once and is a no-op the second
-- time, so the same migration is correct against a hand-patched DEV, an
-- untouched PROD, and a database built from zero.
--
-- ## Additive only
--
-- One nullable column. Nothing is altered, dropped or re-typed, and no existing
-- row changes: the rows written before this migration had no raw reply to keep,
-- and NULL is exactly what "none was kept" means.

ALTER TABLE practice_answers ADD COLUMN IF NOT EXISTS read_raw_reply TEXT NULL;

COMMENT ON COLUMN practice_answers.read_raw_reply IS
    'What the model said when the parser refused to show it. NULL on an accepted '
    'read and on a call that never returned.';
