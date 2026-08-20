-- practice_read_t1_per_part_storage: the read stops being one sentence.
--
-- Created: 2026-08-20 16:55:01
-- Target: pipeline database (colossus_legal_v2)
--
-- ## What this is for
--
-- T1 of PRACTICE_BUILD_DRILL_STAGING_v1. The read is judged against material the
-- model was never sent — her receipts, the sworn pair, what she said she would
-- point to — and it answers in one line this build truncates without saying so.
-- After T1 the model is GIVEN those things, every factual claim it makes carries
-- a citation key code validates, and the reply arrives in three parts that are
-- stored as three parts.
--
-- ## THE HARD CONSTRAINT THIS MIGRATION OBEYS
--
-- Marie and Roman are running real practice sessions on S-5 and S-6 tonight and
-- tomorrow. Their answers are not test data. So:
--
--   * every ADD COLUMN carries IF NOT EXISTS;
--   * no pre-existing column on any pre-existing answer, note, flag or
--     change-log row is read, rewritten or deleted by any statement here;
--   * every new column is NULLABLE with no DEFAULT, so the twelve answer rows
--     already on DEV keep their single sentence in `read_text` and gain eight
--     NULLs that mean "this read predates T1" — which is the honest reading, and
--     the reason no backfill is attempted. There is nothing to backfill FROM.
--
-- The one pre-existing value this migration does change is the settings row
-- `practice_read_prompt_file`, which task §2.3 instructs it to move to v3. That
-- is a parameter, not a record of something a human did.
--
-- ## ⚑ THE PROMPT FILE MUST BE ON THE MOUNT BEFORE THIS DEPLOYS
--
-- Boot calls std::process::exit(1) when `practice_read_prompt_file` names a file
-- the template directory lacks — services/settings_template_file.rs. This
-- migration points that row at practice_read_prompt_v3.md. Push the file to
-- /mnt/data/legal-docs/extraction_templates FIRST and verify its md5 inside the
-- container. A backend deployed ahead of the file does not start.
--
-- v2 stays on the mount untouched, exactly as v1 has since August, so the
-- rollback is one UPDATE of this row and no file work.

-- ─── 1 · The read, stored per part ───────────────────────────────────────────
--
-- ## Why six columns and not one JSONB blob
--
-- The blob is what this build already had: `read_text`, one string, with the
-- second line of every reply discarded at the parser and recoverable from
-- nowhere. A column per part is what makes "the model wrote a why and no
-- pointers" a different row from "the model wrote neither" — and those are
-- different facts about a witness's practice, not different renderings.
--
-- `read_text` KEEPS being written, composed from these parts, because the
-- reveal screen and the question-review page both read that column and neither
-- is touched by this task (T4 removes it).

-- The one line naming what happened. NULL on every read written before T1, and
-- on every abstain.
ALTER TABLE practice_answers ADD COLUMN IF NOT EXISTS read_call TEXT;

-- The reasoning, citing the record. NULL is LEGITIMATE and not a gap: the design
-- says a part may be omitted when there is nothing to say, so "the model wrote
-- no why" and "there is no read" must stay distinguishable — which they are,
-- because the second has no `read_call` either.
ALTER TABLE practice_answers ADD COLUMN IF NOT EXISTS read_why TEXT;

-- The pointers, ORDERED, as a JSON array of strings. An array and not three
-- columns because the count is 0-3 and carries meaning: the design defaults to
-- ONE pointer and reaches for three only when the faults are genuinely distinct,
-- so how many there were is a fact worth keeping.
ALTER TABLE practice_answers ADD COLUMN IF NOT EXISTS read_pointers JSONB;

-- The citation keys the model actually used (`R1`, `S2`, `P3`), as a JSON array.
-- Kept because a read that cannot cite cannot claim: this column is how a later
-- reader checks that the sentence Marie was shown stood on something she was
-- shown too.
ALTER TABLE practice_answers ADD COLUMN IF NOT EXISTS read_keys JSONB;

-- Why the read declined, in plain English, when it declined. Distinct from
-- `read_error`: that column is the OPERATOR's — a typed cause with a model name
-- and a byte count in it — and this is the sentence a human reads. An abstain
-- populates both.
ALTER TABLE practice_answers ADD COLUMN IF NOT EXISTS read_abstain_reason TEXT;

-- Which prompt produced this read. NULL on every read written before T1.
--
-- ## Domain note: this is load-bearing, not bookkeeping
--
-- Without it, "Read my answer" on unchanged text keys on the TEXT alone — and an
-- answer Marie is happy with could then never receive a post-T1 read, freezing
-- in place a read produced by the very defect T1 exists to fix. The rule that
-- consumes this ("a re-read is a no-op only when the text AND the version both
-- match") ships in T3 with the control that needs it; the stamp ships here,
-- because a read written tomorrow needs to be identifiable next week.
ALTER TABLE practice_answers ADD COLUMN IF NOT EXISTS read_version TEXT;

-- How many times the model was asked, for THIS answer.
--
-- ## Domain note: without this the token count cannot be read
--
-- T1 accumulates tokens across every attempt, because that is what the answer
-- cost — but a row reading 4,200 input tokens is uninterpretable if nobody can
-- tell whether it was one expensive call or two ordinary ones. It also closes the
-- gap the payload audit named on 2026-08-20: `read_ms` has always measured the
-- whole retry loop with no attempt counter beside it, so a rate-limited call that
-- slept and retried looked simply slow.
--
-- NULL on every read written before T1, and on a stored line that called nothing.
ALTER TABLE practice_answers ADD COLUMN IF NOT EXISTS read_attempts SMALLINT;

-- Which parts came back over their ceilings, and what the ceilings WERE.
--
-- ## Domain note: the ceiling in effect AT THE TIME
--
-- The four per-part ceilings are settings rows and an operator will move them.
-- Recording only that a part was long would leave a later reader unable to say
-- whether a stored `read_call` was over the limit that was actually in force when
-- it was judged. So each entry carries `part`, `words` and `limit` together.
--
-- NULL when nothing overran, which is the ordinary case — this column exists so a
-- WAVE of overruns is visible in the permanent record rather than only in a log
-- window that has since rolled over. That wave is how an operator learns a model
-- changed, or that a ceiling is set wrong.
ALTER TABLE practice_answers ADD COLUMN IF NOT EXISTS read_overruns JSONB;

COMMENT ON COLUMN practice_answers.read_attempts IS
    'How many model calls this answer cost. NULL before T1 and on a stored line.';
COMMENT ON COLUMN practice_answers.read_overruns IS
    'JSON array of {part, words, limit} for parts stored over their ceiling. NULL when none.';
COMMENT ON COLUMN practice_answers.read_call IS
    'The read''s one-line call. NULL on a pre-T1 read and on an abstain.';
COMMENT ON COLUMN practice_answers.read_why IS
    'The read''s reasoning. NULL is legitimate — a part may be omitted.';
COMMENT ON COLUMN practice_answers.read_pointers IS
    'Ordered JSON array of pointers, 0-3. The design defaults to one.';
COMMENT ON COLUMN practice_answers.read_keys IS
    'JSON array of the citation keys used (R1/S2/P3), validated in code against the keys sent.';
COMMENT ON COLUMN practice_answers.read_abstain_reason IS
    'Why the read declined, in plain English. read_error carries the operator''s half.';
COMMENT ON COLUMN practice_answers.read_version IS
    'The prompt file that produced this read. Pre-T1 reads have none.';

-- ─── 2 · The per-part ceilings ───────────────────────────────────────────────
--
-- ## Why these are rows and not constants
--
-- The same lesson the theme scan learned on 2026-08-09, when a compiled-in 512
-- truncated 7 of 104 verdicts and nobody could change it without a build. These
-- three decide what a witness is shown; Roman will want to move them the first
-- evening he watches Marie use this.
--
-- ## Domain note: these are CEILINGS, NOT TARGETS
--
-- A part may be a single clause. `why` and `pointers` may be empty when there is
-- nothing to say. And — the inversion this task ships — a part OVER its ceiling
-- is no longer discarded: the read is re-requested once, and a second overrun is
-- stored and shown as returned, with the part and the count logged. One word
-- over the cap used to mean Marie saw nothing at all.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('practice_read_max_words_call', '12', 'count', '12', 3, 40,
     'The most words the read''s CALL may use — the one line naming what '
     'happened. Domain note: a ceiling, not a target; a call may be three words. '
     'An overrun re-requests once and is then stored as returned, never '
     'truncated and never discarded.',
     'services::practice_read_parse', NOW(), 'migration'),

    ('practice_read_max_words_why', '55', 'count', '55', 10, 150,
     'The most words the read''s WHY may use — the reasoning, citing the record. '
     'May be empty when there is nothing to say beyond the call.',
     'services::practice_read_parse', NOW(), 'migration'),

    ('practice_read_max_words_pointer', '20', 'count', '20', 5, 60,
     'The most words ONE pointer may use. A pointer names the move and never '
     'supplies the words — the cap is part of what keeps it from becoming a '
     'sentence Marie could speak verbatim.',
     'services::practice_read_parse', NOW(), 'migration'),

    ('practice_read_max_pointers', '3', 'count', '3', 0, 5,
     'The most pointers one read may carry. Domain note: the design DEFAULTS to '
     'one — coaching that names one thing is acted on, coaching that names three '
     'is skimmed — and reaches for more only when the faults are genuinely '
     'distinct. This is the hard ceiling, not the expectation.',
     'services::practice_read_parse', NOW(), 'migration'),

-- ─── 3 · The two sentences Marie reads when there is no read ─────────────────
--
-- ## Why these are wording rows and the abstain CAUSES are not
--
-- Marie reads these two, so they are stored — the same rule that puts
-- `practice_read_unavailable` and `skipped_answer_text` in this table. The
-- specific cause of an abstain ("her points could not be loaded") goes to
-- `read_abstain_reason` as a code-owned diagnostic, for the reason the skip
-- marker gives in api/practice_answers.rs: a value composed by this build FROM A
-- FAILURE IT OBSERVED must not be something an operator can edit after the fact.
-- T4 renders that column and may promote the causes to rows then.

    ('practice_read_abstain_line', 'I can''t read this one.', 'text',
     'I can''t read this one.', NULL, NULL,
     'What the read says when it declines rather than guesses — a reply it could '
     'not parse, a citation it was not given, a call that failed, or an input '
     'that failed to load. Domain note: this is NOT practice_read_unavailable. '
     'That line stands in when no read was attempted or none arrived; this one '
     'is the read SPEAKING, saying it will not judge on what it has. The model''s '
     'own reason follows it when the model was the one to decline.',
     'services::practice_read', NOW(), 'migration'),

    ('practice_read_dont_recall_line', 'Fine. "I don''t recall" is a complete answer.', 'text',
     'Fine. "I don''t recall" is a complete answer.', NULL, NULL,
     'The stored read for the one-click "I don''t recall." control. Domain note: '
     'no model is called for it. The button sends a sentence this system wrote, '
     'and paying a model to judge our own words produced a sentence about a '
     'sentence — at full token cost, on a control Marie presses without typing. '
     'It begins with practice_read_fine_token so the rail goes green, which is '
     'the correct verdict: "I don''t recall" is complete when it is true.',
     'services::practice_read', NOW(), 'migration')

-- A row an operator has already corrected is never overwritten by a redeploy.
ON CONFLICT (key) DO NOTHING;

-- ─── 4 · The prompt row moves to v3 ──────────────────────────────────────────
--
-- ## Why this one UPDATE is not a violation of the hard constraint
--
-- The constraint protects RECORDS OF WHAT A HUMAN DID — answers, notes, flags,
-- change-log rows. This is a parameter naming which instructions the model is
-- given, and task §2.3 instructs this migration to move it. v2 is left on disk
-- and in the repo, so the rollback is this statement with 'v2' in it.
--
-- `default_value` moves with `value`: leaving the default at v2 would make the
-- Settings page's "reset to default" a silent downgrade to the prompt this task
-- exists to replace.
-- ## The FORMAT of this statement is load-bearing
--
-- `settings_store_tests` and `wording_practice_report_tests` both find a
-- correction by searching for `SET value         = '` and `WHERE key           =`
-- with exactly this alignment. Written any other way the correction is invisible
-- to them and they go green while the store holds something else — which is the
-- exact drift those tests exist to catch. The v1 migration that moved this row to
-- v2 carries the same warning; this is that warning obeyed.
UPDATE app_settings
SET value         = 'practice_read_prompt_v3.md',
    default_value = 'practice_read_prompt_v3.md',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'practice_read_prompt_file';
