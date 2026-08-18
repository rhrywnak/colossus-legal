-- practice_session_v0: Practice session v0 — the deck, the log, and the words
--
-- Created: 2026-08-17 21:33:19
-- Target: pipeline database (colossus_legal_v2)
--
-- Task CC_TASK_PRACTICE_SESSION_V0_v1 (PRACTICE_SESSION_DESIGN_v1 §4–§5,
-- mockup v2 of 2026-08-17). Marie sits alone with the laptop, twenty minutes,
-- one accusation. She answers; the screen shows her own words back, ONE sentence
-- of system read, her three points with their receipts, the pair, the watch-for,
-- four boxes she ticks herself, and a collapsed stronger answer. At the end,
-- Chuck's sheet.
--
-- ## Why these tables live in the PIPELINE database
--
-- `scenarios`, `scenario_responses`, `response_items` and `scenario_human_facts`
-- are all here (CLAUDE.md §1, corrected 2026-06-17). A deck keyed to a scenario
-- in another database could not carry a foreign key, and the one invariant that
-- matters most — a question cannot outlive its scenario — is exactly what the FK
-- buys.
--
-- ## Additive only
--
-- Three new tables and one block of `app_settings` rows. Nothing existing is
-- altered, dropped or re-typed.

-- ─── 1 · The deck ─────────────────────────────────────────────────────────────
--
-- One row = one question Marie will be asked, with everything her reveal screen
-- needs to render WITHOUT reading the graph (design §5: the tool reads the
-- scenario record, the deck and the log — nothing else).
--
-- ## Why the reveal's text is stored on the ROW rather than derived at render
--
-- The receipt, the watch-for, the pair and the stronger answer are all authored
-- prose. Deriving them from the record at render time would mean composing a
-- witness-facing sentence out of raw evidence — which is the one thing the
-- honest-gap law forbids (REHEARSAL_VIEW_DESIGN_v2). They are written once, by a
-- human, reviewed by Roman, and then fixed (ruling R1).
CREATE TABLE practice_questions (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),

    -- The scenario this question drills. CASCADE because a deck for a deleted
    -- scenario is not history worth keeping — it is a question about an
    -- accusation that no longer exists.
    scenario_id   UUID        NOT NULL
                  REFERENCES scenarios(scenario_id) ON DELETE CASCADE,

    -- Who is asking. 'george' = cross (the attack turned into a question);
    -- 'chuck' = direct (so she can tell it in her own words). A CHECK rather than
    -- an enum for the reason the scenario status column gives: the vocabulary
    -- lives in code and a new side is a code change plus a migration, both loud.
    side          TEXT        NOT NULL CHECK (side IN ('george', 'chuck')),

    -- The question, verbatim as authored.
    text          TEXT        NOT NULL CHECK (btrim(text) <> ''),

    -- TACTIC_DECK_v1 card 1–7, or NULL when the question carries none (every
    -- Chuck question does). NULL and 'none' are NOT the same thing and only one
    -- of them is representable here on purpose: a Chuck question has no tactic,
    -- it does not have a tactic called none.
    tactic        SMALLINT    CHECK (tactic IS NULL OR tactic BETWEEN 1 AND 7),

    -- The barrage rows a braid question braids, as the sentence the screen shows
    -- ("Barrage rows 1 (S-5) · 2 (S-6) · 5 (the courts)"). NULL on every question
    -- that is not a braid.
    braid_rows    TEXT        CHECK (braid_rows IS NULL OR btrim(braid_rows) <> ''),

    -- Where the question came from: a ruled instance, a talking point, or a human
    -- typing it in. 'manual' is the honest answer when neither applies, and it is
    -- what makes `source_ref IS NULL` mean something rather than looking like a
    -- bug.
    source_kind   TEXT        NOT NULL
                  CHECK (source_kind IN ('instance', 'point', 'manual')),

    -- The graph node id of the ruled instance, or the `response_items.id` of the
    -- point. NULL for 'manual'. Not an FK: the instance half addresses Neo4j.
    source_ref    TEXT        CHECK (source_ref IS NULL OR btrim(source_ref) <> ''),

    -- The "Built from: …" line printed under the question. NULL means NO RECEIPT
    -- and the screen says so in words — it never renders a blank.
    receipt       TEXT        CHECK (receipt IS NULL OR btrim(receipt) <> ''),

    -- The gold WATCH FOR box. NULL withdraws the box entirely.
    watch_for     TEXT        CHECK (watch_for IS NULL OR btrim(watch_for) <> ''),

    -- The one-breath example inside the collapsed drawer, precomputed at seed
    -- time (design §5: preferred over a live call — cheaper, and reviewable by
    -- Roman before Marie ever sees it). NULL renders the stored "no receipt for
    -- this one" line instead.
    stronger      TEXT        CHECK (stronger IS NULL OR btrim(stronger) <> ''),

    -- "leans on point 1" — which of HER points the example is built from.
    stronger_lean TEXT        CHECK (stronger_lean IS NULL OR btrim(stronger_lean) <> ''),

    -- ## Beyond the task's column list, and why (the pair)
    --
    -- The task named the columns above. Screen S2 also renders the PAIR — what
    -- they said beside what they admitted under oath — and the design's own
    -- reading law says the tool reads the scenario record, the deck and the log,
    -- never the graph. The pair's two quotes are authored, abbreviated prose in
    -- the mockup ("…they're at each other's throats." — Phillips, hearing, p. 34),
    -- not raw node text, so there is nowhere else they can honestly come from.
    -- Both NULL together withdraws the block; the seed writes both or neither.
    pair_said     TEXT        CHECK (pair_said IS NULL OR btrim(pair_said) <> ''),
    pair_admitted TEXT        CHECK (pair_admitted IS NULL OR btrim(pair_admitted) <> ''),

    -- The order the deck is dealt in. Unique per scenario so two seeds cannot
    -- silently interleave.
    sort_order    INTEGER     NOT NULL,

    created_by    TEXT        NOT NULL,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT practice_questions_order_unique UNIQUE (scenario_id, sort_order),

    -- A 'manual' question has no ref; an 'instance' or 'point' question must have
    -- one. Stated as a constraint rather than left to the writer because a
    -- dangling source is the defect that made the scenario refs stale-pointer
    -- report (2026-08-14) — a join key that is fine and 26 refs that pointed
    -- nowhere.
    CONSTRAINT practice_questions_ref_matches_kind CHECK (
        (source_kind = 'manual' AND source_ref IS NULL)
        OR (source_kind <> 'manual' AND source_ref IS NOT NULL)
    )
);

CREATE INDEX idx_practice_questions_scenario
    ON practice_questions (scenario_id, sort_order);

COMMENT ON TABLE practice_questions IS
    'The per-scenario practice deck (PRACTICE_SESSION_DESIGN_v1 §5). Seeded once '
    'per scenario, human-edited, then fixed. Every row carries everything its '
    'reveal screen renders, so the drill reads no graph.';

-- ─── 2 · The session ──────────────────────────────────────────────────────────
CREATE TABLE practice_sessions (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),

    scenario_id UUID        NOT NULL
                REFERENCES scenarios(scenario_id) ON DELETE CASCADE,

    -- Which deck she chose to run: George's side, Chuck's, or both.
    who         TEXT        NOT NULL CHECK (who IN ('george', 'chuck', 'mixed')),

    started_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- NULL while the session is open. A session she walked away from and a
    -- session she finished are different states and stay different: the "last
    -- session" line reads only ENDED ones.
    ended_at    TIMESTAMPTZ
);

CREATE INDEX idx_practice_sessions_scenario
    ON practice_sessions (scenario_id, started_at DESC);

COMMENT ON TABLE practice_sessions IS
    'One sitting. FRE 612: nothing here is printed for the witness — the only '
    'print is Chuck''s sheet, until Chuck rules.';

-- ─── 3 · The answers (the log Chuck''s sheet is rendered from) ────────────────
CREATE TABLE practice_answers (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),

    session_id   UUID        NOT NULL
                 REFERENCES practice_sessions(id) ON DELETE CASCADE,

    -- RESTRICT, not CASCADE: a deck row cannot be deleted out from under an
    -- answer that quotes it. Chuck's sheet is the record of what she was asked,
    -- and a question that vanishes takes the row's meaning with it.
    question_id  UUID        NOT NULL
                 REFERENCES practice_questions(id) ON DELETE RESTRICT,

    -- Her words, exactly as typed. Never trimmed to nothing: the stored
    -- "(nothing typed)" sentence is what an empty box records, so a blank answer
    -- and an unanswered question stay different rows.
    answer_text  TEXT        NOT NULL,

    -- She pressed "I don't recall." rather than typing. A separate column from
    -- the text because the two are different acts even when the text matches.
    dont_recall  BOOLEAN     NOT NULL DEFAULT FALSE,

    -- The one sentence the model returned. NULL means THE CALL FAILED and the
    -- screen says "no system read this time" — it is never an empty string, and
    -- never a substitute sentence composed here.
    read_text    TEXT,

    -- TRUE = "Fine.", FALSE = it named a tactic she fell for, NULL = no read.
    -- Three states, three values, no collapsing (Standing Rule 1).
    read_ok      BOOLEAN,

    -- Why the read is absent, when it is. NULL when a read arrived. Log-side
    -- honesty: the screen shows one fixed line, this column says which failure.
    read_error   TEXT,

    -- The call's cost and latency, for the answer row (task §5: "Log every call
    -- (tokens, ms) in the answer row"). NULL when no call was made or the
    -- provider reported none.
    read_input_tokens  INTEGER,
    read_output_tokens INTEGER,
    read_ms            INTEGER,
    read_model         TEXT,

    -- The four boxes, as she ticked them: {"only_asked":bool,
    -- "accepted_premise":bool, "explained_unasked":bool, "guessed":bool}.
    -- jsonb rather than four columns because they are ONE act of self-grading
    -- and the design may add a fifth box; a jsonb row does not need a migration
    -- for that, and nothing joins on them.
    self_check   JSONB       NOT NULL,

    -- She opened "Show a stronger answer ▸". Chuck's sheet shows it, which is
    -- the whole point (ruling R3) — not to grade her, but so he knows where the
    -- help was needed.
    help_opened  BOOLEAN     NOT NULL DEFAULT FALSE,

    -- 'fine' or 'repeat'. 'repeat' is what "Ask me this one again later" writes,
    -- and it is what re-queues the question in the same session.
    mark         TEXT        NOT NULL CHECK (mark IN ('fine', 'repeat')),

    answered_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_practice_answers_session
    ON practice_answers (session_id, answered_at);

COMMENT ON TABLE practice_answers IS
    'One answered question. Chuck''s sheet is rendered from this table and '
    'nothing else; it is the only thing that leaves the screen.';

-- ─── 4 · The words the practice surface speaks ────────────────────────────────
--
-- Task §8, the wording law: every string in the mockup that is not DATA lives in
-- the store, not in JSX or a Rust literal. That is the same rule the scan, the
-- rehearsal page, the accusation section and the matrix already answer to, and
-- the reason is unchanged (v2 §2b): a sentence a human reads is configuration.
--
-- ## Why some sentences arrive in three rows
--
-- The mockup italicises one word inside three sentences ("what was the
-- *question*?", "an example of *how*", "the ones marked *repeat*"). A row
-- carrying `<i>` would put markup in the store and HTML in a witness surface; a
-- row carrying the whole sentence would lose the emphasis. So those three arrive
-- as prefix · emphasis · suffix and the component supplies the tag. This is the
-- narrow version of the same split the .392 count line made for its clauses.
--
-- Note every value is stored TRIMMED (the store trims): a template cannot carry
-- a leading space, and the renderer supplies the joining one.
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    -- ── S0 · start ────────────────────────────────────────────────────────────
    ('practice_kicker', 'Practice session', 'text', 'Practice session', NULL, NULL,
     'The eyebrow over the scenario title on the practice start screen.',
     'frontend PracticeStart', NOW(), 'migration'),
    ('practice_intro',
     'Twenty minutes, one accusation, no clock, nobody watching. Answer out loud first, then type it in a sentence or two. You''ll see your own three points after every answer.',
     'text',
     'Twenty minutes, one accusation, no clock, nobody watching. Answer out loud first, then type it in a sentence or two. You''ll see your own three points after every answer.',
     NULL, NULL,
     'The one paragraph under the title on the start screen. It sets the terms of '
     'the session — no clock, nobody watching — which is the whole difference '
     'between a drill and a test.',
     'frontend PracticeStart', NOW(), 'migration'),
    ('practice_who_heading', 'Who''s asking?', 'text', 'Who''s asking?', NULL, NULL,
     'Heading over the three side choices.', 'frontend PracticeStart', NOW(), 'migration'),
    ('practice_who_george_title', 'George''s side (cross)', 'text', 'George''s side (cross)',
     NULL, NULL, 'The cross-examination choice.', 'frontend PracticeStart', NOW(), 'migration'),
    ('practice_who_george_detail',
     'Questions built from what they actually said in the record — the attack, turned into a question.',
     'text',
     'Questions built from what they actually said in the record — the attack, turned into a question.',
     NULL, NULL, 'What the cross choice contains.', 'frontend PracticeStart', NOW(), 'migration'),
    ('practice_who_chuck_title', 'Chuck (direct)', 'text', 'Chuck (direct)', NULL, NULL,
     'The direct-examination choice.', 'frontend PracticeStart', NOW(), 'migration'),
    ('practice_who_chuck_detail',
     'The questions Chuck asks so you can tell it in your own words.', 'text',
     'The questions Chuck asks so you can tell it in your own words.', NULL, NULL,
     'What the direct choice contains.', 'frontend PracticeStart', NOW(), 'migration'),
    ('practice_who_mixed_title', 'Mixed', 'text', 'Mixed', NULL, NULL,
     'The both-sides choice.', 'frontend PracticeStart', NOW(), 'migration'),
    ('practice_who_mixed_detail',
     'Both, in no fixed order — closest to the real day.', 'text',
     'Both, in no fixed order — closest to the real day.', NULL, NULL,
     'What the mixed choice contains.', 'frontend PracticeStart', NOW(), 'migration'),
    ('practice_how_many_heading', 'How many questions?', 'text', 'How many questions?',
     NULL, NULL,
     'Heading over the count pills. Only 5 is live in v0; the others render dimmed '
     'exactly as the mockup does, which is honest about what this build offers.',
     'frontend PracticeStart', NOW(), 'migration'),
    ('practice_count_all_template', 'all {n}', 'text', 'all {n}', NULL, NULL,
     'The third count pill. {n} is the deck''s own size, filled in the browser '
     'from the questions it was served. The mockup wrote "all 12" against a '
     'twelve-question deck; S-5''s is ten, and a pill naming a number no deck has '
     'is the kind of small wrongness a witness stops trusting a screen over.',
     'frontend PracticeStart', NOW(), 'migration'),
    ('practice_start_label', 'Start', 'text', 'Start', NULL, NULL,
     'The control that opens a session.', 'frontend PracticeStart', NOW(), 'migration'),
    ('practice_always_label', 'ALWAYS', 'text', 'ALWAYS', NULL, NULL,
     'The bold word opening the standing card.', 'frontend PracticeAlways', NOW(), 'migration'),
    ('practice_always_line',
     'Tell the truth · Answer only what''s asked · "I don''t recall" is fine if it''s true · Don''t guess · Pause before every answer — the pause is yours by right.',
     'text',
     'Tell the truth · Answer only what''s asked · "I don''t recall" is fine if it''s true · Don''t guess · Pause before every answer — the pause is yours by right.',
     NULL, NULL,
     'The five rules that never move. Domain note: this card is also an INPUT to '
     'the one-sentence read — the model is told to judge against it — so editing '
     'this row changes what the system says, not only what the screen shows.',
     'frontend PracticeAlways · services::practice_read', NOW(), 'migration'),
    ('practice_last_session_template',
     'Last session: {when} · {count} questions · {repeat} to repeat', 'text',
     'Last session: {when} · {count} questions · {repeat} to repeat', NULL, NULL,
     'The line beside Start, composed from the LOG''s most recently ENDED session '
     'for this scenario. {when}, {count} and {repeat} are filled server-side.',
     'services::practice_page', NOW(), 'migration'),
    ('practice_no_last_session', 'No session on this one yet.', 'text',
     'No session on this one yet.', NULL, NULL,
     'What stands where the last-session line goes when the log holds no ended '
     'session for this scenario. A named absence, never a blank.',
     'services::practice_page', NOW(), 'migration'),

    -- ── S1 · the question ─────────────────────────────────────────────────────
    ('practice_progress_template', 'Question {n} of {total}', 'text',
     'Question {n} of {total}', NULL, NULL,
     'The progress line on the question and reveal screens.',
     'frontend PracticeQuestion', NOW(), 'migration'),
    ('practice_pill_george', 'George''s side', 'text', 'George''s side', NULL, NULL,
     'The pill on a cross question.', 'frontend PracticeQuestion', NOW(), 'migration'),
    ('practice_pill_chuck', 'Chuck', 'text', 'Chuck', NULL, NULL,
     'The pill on a direct question.', 'frontend PracticeQuestion', NOW(), 'migration'),
    ('practice_pill_braid', 'George''s side · a braid', 'text', 'George''s side · a braid',
     NULL, NULL,
     'The pill on a compound question that braids several barrage rows.',
     'frontend PracticeQuestion', NOW(), 'migration'),
    ('practice_answer_label', 'Your answer', 'text', 'Your answer', NULL, NULL,
     'The bold half of the answer prompt.', 'frontend PracticeQuestion', NOW(), 'migration'),
    ('practice_answer_hint', '— say it out loud, then type it.', 'text',
     '— say it out loud, then type it.', NULL, NULL,
     'The rest of the answer prompt. Say-then-type is the whole method (design '
     '§1); speech-to-text is deliberately not in this build.',
     'frontend PracticeQuestion', NOW(), 'migration'),
    ('practice_answer_placeholder',
     'One or two sentences. Stop when you''ve answered the question that was asked.',
     'text',
     'One or two sentences. Stop when you''ve answered the question that was asked.',
     NULL, NULL, 'The textarea placeholder.', 'frontend PracticeQuestion', NOW(), 'migration'),
    ('practice_answer_button', 'Answer', 'text', 'Answer', NULL, NULL,
     'Submits the typed answer.', 'frontend PracticeQuestion', NOW(), 'migration'),
    ('practice_dont_recall_button', '"I don''t recall."', 'text', '"I don''t recall."',
     NULL, NULL,
     'The control that answers without typing. Domain note: it is a control and '
     'not a hint because "I don''t recall" being a COMPLETE answer is the single '
     'hardest thing for a witness to believe.',
     'frontend PracticeQuestion', NOW(), 'migration'),
    ('practice_dont_recall_text', 'I don''t recall.', 'text', 'I don''t recall.',
     NULL, NULL,
     'What the control types into the box and stores as her answer — without the '
     'quotation marks the button wears.',
     'frontend PracticeQuestion', NOW(), 'migration'),
    ('practice_pause_button', 'Pause — take a breath', 'text', 'Pause — take a breath',
     NULL, NULL, 'Shows the pause note.', 'frontend PracticeQuestion', NOW(), 'migration'),
    ('practice_pause_note_prefix',
     'Good. The pause is yours. Nobody on the stand is timing you. Now: what was the',
     'text',
     'Good. The pause is yours. Nobody on the stand is timing you. Now: what was the',
     NULL, NULL,
     'The pause note up to its emphasised word. Split so the store carries no '
     'markup — see this migration''s header.',
     'frontend PracticeQuestion', NOW(), 'migration'),
    ('practice_pause_note_emphasis', 'question?', 'text', 'question?', NULL, NULL,
     'The italicised close of the pause note.', 'frontend PracticeQuestion', NOW(), 'migration'),

    -- ── S2 · the reveal ───────────────────────────────────────────────────────
    ('practice_what_you_said_kicker', 'What you said', 'text', 'What you said', NULL, NULL,
     'Over her own answer, quoted back.', 'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_read_tag', 'system read', 'text', 'system read', NULL, NULL,
     'The tag that marks the one sentence as the machine''s, not Chuck''s.',
     'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_read_footnote',
     'one sentence, against your points, the watch-for and the ALWAYS card. It names the tactic. The boxes below are yours.',
     'text',
     'one sentence, against your points, the watch-for and the ALWAYS card. It names the tactic. The boxes below are yours.',
     NULL, NULL,
     'What the read is and what it is not. The last clause is the one that '
     'matters: the boxes are hers and cannot be wrong about her.',
     'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_read_unavailable', 'no system read this time', 'text',
     'no system read this time', NULL, NULL,
     'Stands in the read''s place when the call failed. Domain note: the boxes, '
     'the points, the pair and the watch-for all still stand — the session is not '
     'degraded by a model being down, it just says one less thing.',
     'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_points_kicker', 'Your points — in your own words', 'text',
     'Your points — in your own words', NULL, NULL,
     'Over the three talking points, which are read live from the scenario record.',
     'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_receipt_prefix', 'Backed by:', 'text', 'Backed by:', NULL, NULL,
     'Opens a point''s receipt line. The renderer supplies the following space.',
     'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_point_no_receipt', 'No receipt recorded for this point.', 'text',
     'No receipt recorded for this point.', NULL, NULL,
     'What a point with no paired exhibit says. The honest-gap law: a named '
     'absence, never a blank line under the point.',
     'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_pair_kicker', 'Where the question came from — and their own sworn answer',
     'text', 'Where the question came from — and their own sworn answer', NULL, NULL,
     'Over the two-column pair.', 'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_pair_said_label', 'What they said', 'text', 'What they said', NULL, NULL,
     'Left column of the pair.', 'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_pair_admitted_label', 'What they admitted under oath', 'text',
     'What they admitted under oath', NULL, NULL,
     'Right column of the pair — the half that wins the point.',
     'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_check_kicker', 'Check yourself', 'text', 'Check yourself', NULL, NULL,
     'Over the four boxes.', 'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_check_only_asked', 'I answered only the question that was asked', 'text',
     'I answered only the question that was asked', NULL, NULL,
     'Self-check box 1.', 'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_check_accepted_premise', 'I accepted a word or premise I shouldn''t have',
     'text', 'I accepted a word or premise I shouldn''t have', NULL, NULL,
     'Self-check box 2 — the false premise, card 4.', 'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_check_explained_unasked', 'I explained something nobody asked about', 'text',
     'I explained something nobody asked about', NULL, NULL,
     'Self-check box 3.', 'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_check_guessed', 'I guessed at a date, a number, or a name', 'text',
     'I guessed at a date, a number, or a name', NULL, NULL,
     'Self-check box 4.', 'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_stronger_summary', 'Show a stronger answer ▸', 'text',
     'Show a stronger answer ▸', NULL, NULL,
     'The collapsed drawer''s own label. Opening it is recorded on the answer row '
     'and printed on Chuck''s sheet (ruling R3).',
     'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_stronger_note_prefix', 'An example of', 'text', 'An example of', NULL, NULL,
     'The not-a-script line, up to its emphasised word.',
     'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_stronger_note_emphasis', 'how', 'text', 'how', NULL, NULL,
     'The italicised word in the not-a-script line. ABA Formal Op. 508: themes and '
     'key points, never a word-for-word script.',
     'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_stronger_note_suffix',
     ', built only from your own points — not a script. Say it your way.', 'text',
     ', built only from your own points — not a script. Say it your way.', NULL, NULL,
     'The rest of the not-a-script line. It opens with its own comma because the '
     'emphasised word precedes it with no space.',
     'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_stronger_no_receipt', 'No receipt for this one — that''s a Chuck question.',
     'text', 'No receipt for this one — that''s a Chuck question.', NULL, NULL,
     'What the drawer says when no point of hers answers the question.',
     'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_mark_not_recorded',
     'That did not save. Your answer is recorded; the mark for it is not — press the button again.',
     'text',
     'That did not save. Your answer is recorded; the mark for it is not — press the button again.',
     NULL, NULL,
     'What the reveal says when the write settling an answer — her four boxes and '
     'fine/repeat — failed. Domain note: it names what IS safe first, because the '
     'sentence a witness needs most in that moment is that her typed answer did '
     'not disappear. She stays on this screen and the button still works.',
     'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_help_not_recorded',
     'Chuck''s sheet will not show that you opened this — the note did not save.',
     'text',
     'Chuck''s sheet will not show that you opened this — the note did not save.',
     NULL, NULL,
     'What the reveal says when the write recording an opened drawer failed. '
     'Domain note: it is said in terms of the CONSEQUENCE, not the mechanism — '
     'Marie can do nothing about a failed POST, and what actually changed is that '
     'one cell on Chuck''s sheet will be wrong. Her answer, her boxes and her mark '
     'are unaffected, and the sentence says nothing to suggest otherwise.',
     'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_next_button', 'Got it — next question', 'text', 'Got it — next question',
     NULL, NULL, 'Marks the answer fine and advances.', 'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_again_button', 'Ask me this one again later', 'text',
     'Ask me this one again later', NULL, NULL,
     'Marks the answer repeat and re-queues the question in this same session.',
     'frontend PracticeReveal', NOW(), 'migration'),

    -- ── S3 · Chuck''s sheet ───────────────────────────────────────────────────
    ('practice_sheet_kicker_template', 'Session done · {code} · {when}', 'text',
     'Session done · {code} · {when}', NULL, NULL,
     'The sheet''s eyebrow, composed server-side.', 'services::practice_sheet', NOW(), 'migration'),
    ('practice_sheet_heading_template', '{count} questions. {repeat}', 'text',
     '{count} questions. {repeat}', NULL, NULL,
     'The sheet''s heading. {repeat} arrives as a whole clause so the zero case '
     'reads as a sentence rather than "0 to repeat".',
     'services::practice_sheet', NOW(), 'migration'),
    ('practice_sheet_repeat_clause_template', '{n} to repeat.', 'text', '{n} to repeat.',
     NULL, NULL, 'The repeat clause when there is something to repeat.',
     'services::practice_sheet', NOW(), 'migration'),
    ('practice_sheet_nothing_to_repeat', 'Nothing to repeat.', 'text', 'Nothing to repeat.',
     NULL, NULL, 'The repeat clause when there is not.',
     'services::practice_sheet', NOW(), 'migration'),
    ('practice_sheet_sub_prefix',
     'This is the sheet Chuck sees. Your words, as you typed them; the ones marked',
     'text',
     'This is the sheet Chuck sees. Your words, as you typed them; the ones marked',
     NULL, NULL, 'The sheet''s sub-line, up to the emphasised mark.',
     'frontend PracticeSheet', NOW(), 'migration'),
    ('practice_sheet_sub_suffix', 'are where he''ll run the real mock cross.', 'text',
     'are where he''ll run the real mock cross.', NULL, NULL,
     'The rest of the sheet''s sub-line.', 'frontend PracticeSheet', NOW(), 'migration'),
    ('practice_sheet_col_number', '#', 'text', '#', NULL, NULL,
     'Sheet column 1.', 'frontend PracticeSheet', NOW(), 'migration'),
    ('practice_sheet_col_from', 'From', 'text', 'From', NULL, NULL,
     'Sheet column 2.', 'frontend PracticeSheet', NOW(), 'migration'),
    ('practice_sheet_col_tactic', 'Tactic', 'text', 'Tactic', NULL, NULL,
     'Sheet column 3.', 'frontend PracticeSheet', NOW(), 'migration'),
    ('practice_sheet_col_question', 'Question', 'text', 'Question', NULL, NULL,
     'Sheet column 4.', 'frontend PracticeSheet', NOW(), 'migration'),
    ('practice_sheet_col_answer', 'Your answer', 'text', 'Your answer', NULL, NULL,
     'Sheet column 5 — verbatim, never summarised.', 'frontend PracticeSheet', NOW(), 'migration'),
    ('practice_sheet_col_mark', 'Mark', 'text', 'Mark', NULL, NULL,
     'Sheet column 6.', 'frontend PracticeSheet', NOW(), 'migration'),
    ('practice_sheet_col_help', 'Help', 'text', 'Help', NULL, NULL,
     'Sheet column 7 — whether she opened the drawer.', 'frontend PracticeSheet', NOW(), 'migration'),
    ('practice_sheet_from_george', 'George', 'text', 'George', NULL, NULL,
     'The From cell on a cross question.', 'services::practice_sheet', NOW(), 'migration'),
    ('practice_sheet_from_george_braid', 'George · braid', 'text', 'George · braid',
     NULL, NULL, 'The From cell on a braid.', 'services::practice_sheet', NOW(), 'migration'),
    ('practice_sheet_from_chuck', 'Chuck', 'text', 'Chuck', NULL, NULL,
     'The From cell on a direct question.', 'services::practice_sheet', NOW(), 'migration'),
    ('practice_mark_fine', 'fine', 'text', 'fine', NULL, NULL,
     'The Mark cell when nothing needs repeating.', 'services::practice_sheet', NOW(), 'migration'),
    ('practice_mark_repeat', 'repeat', 'text', 'repeat', NULL, NULL,
     'The Mark cell when it does — and the word emphasised in the sub-line.',
     'services::practice_sheet', NOW(), 'migration'),
    ('practice_help_opened', 'opened', 'text', 'opened', NULL, NULL,
     'The Help cell when she opened the drawer.', 'services::practice_sheet', NOW(), 'migration'),
    ('practice_help_none', '—', 'text', '—', NULL, NULL,
     'The Help cell when she did not.', 'services::practice_sheet', NOW(), 'migration'),
    ('practice_tactic_none', '—', 'text', '—', NULL, NULL,
     'The Tactic cell on a question that carries none.', 'services::practice_sheet', NOW(), 'migration'),
    ('practice_sheet_again_button', 'Practice again', 'text', 'Practice again',
     NULL, NULL, 'Returns to the start screen.', 'frontend PracticeSheet', NOW(), 'migration'),
    ('practice_print_button', 'Print Chuck''s sheet', 'text', 'Print Chuck''s sheet',
     NULL, NULL,
     'The ONLY print in this tool. FRE/MRE 612: nothing is printed for the witness '
     'until Chuck rules.', 'frontend PracticeSheet', NOW(), 'migration'),
    ('practice_homelab_line', 'Nothing here leaves the homelab. Chuck gets the printed sheet.',
     'text', 'Nothing here leaves the homelab. Chuck gets the printed sheet.', NULL, NULL,
     'The closing line of the sheet.', 'frontend PracticeSheet', NOW(), 'migration'),

    -- ── The gaps and the way in ───────────────────────────────────────────────
    ('practice_empty_deck', 'no practice deck yet — seed it', 'text',
     'no practice deck yet — seed it', NULL, NULL,
     'What the practice page says for a scenario whose deck is empty. Domain note: '
     'this is what S-6 shows today, and it must not read as a failure — it is an '
     'accurate statement about a deck nobody has seeded.',
     'frontend PracticePage', NOW(), 'migration'),
    -- The one practice string that does NOT belong to the practice block: it is
    -- spoken by the SCENARIO page, which is the surface that renders the control,
    -- and it rides the identity wording that page already fetches. Putting it in
    -- the drill's block would have made the scenario page fetch a whole deck
    -- payload to learn one word.
    ('scenario_practice_link_label', 'Practice ▸', 'text', 'Practice ▸', NULL, NULL,
     'The control on the scenario page that opens Marie''s drill. Sits beside '
     '"Rehearsal view →" and, unlike it, is never inert: the drill is where a '
     'deck is found to be missing, so gating it on Ready would hide the only '
     'screen that can report that.',
     'frontend ScenarioHeaderTiers', NOW(), 'migration'),
    ('practice_load_failed', 'The practice deck could not be loaded.', 'text',
     'The practice deck could not be loaded.', NULL, NULL,
     'What the page shows when the deck request fails. Distinct from an empty '
     'deck, which is not a failure at all.',
     'frontend PracticePage', NOW(), 'migration'),
    ('practice_answer_failed', 'Your answer was not recorded. Nothing was saved — try Answer again.',
     'text', 'Your answer was not recorded. Nothing was saved — try Answer again.',
     NULL, NULL,
     'What the question screen shows when the answer POST fails. It says the write '
     'did not happen, because a witness who believes an answer was logged when it '
     'was not is the worst outcome this screen has.',
     'frontend PracticeQuestion', NOW(), 'migration'),

    ('practice_tactic_braid_suffix', '· braid', 'text', '· braid', NULL, NULL,
     'Appended to the tactic tag on a question that braids several barrage rows, '
     'so the tag reads "compound · braid". A row of its own because the WORD is '
     'the reader''s only clue that this question is answered by naming strands '
     'rather than by answering it.',
     'services::practice_page', NOW(), 'migration'),

    -- ── The tactic vocabulary ────────────────────────────────────────────────
    -- The value stays on ONE line with its key: `settings_store_tests`'s reader
    -- matches `('key', '` and a wrapped value would read as un-seeded.
    ('practice_tactic_names', 'broad generalization,half-truth,character jab,false premise,compound,authority borrow,echo', 'text',
     'broad generalization,half-truth,character jab,false premise,compound,authority borrow,echo',
     NULL, NULL,
     'The seven TACTIC_DECK_v1 cards, in card order 1–7. A comma-separated ROW '
     'rather than seven compiled-in names for the reason the configuration law '
     'gives about domain vocabulary: another Colossus project drilling a '
     'different kind of witness has a different deck, and this build must not '
     'contain this one. The deck stores the NUMBER; this row turns it into the '
     'word on the tag.',
     'services::practice_page', NOW(), 'migration'),

    -- ── The read''s parameters ────────────────────────────────────────────────
    ('practice_read_prompt_file', 'practice_read_prompt_v1.md', 'text',
     'practice_read_prompt_v1.md', NULL, NULL,
     'The file, in the extraction-template directory, holding the read''s system '
     'prompt. A FILE and not a row because the prompt is a page of instructions '
     'with the seven tactic counters in it — the same reason theme_scan_prompt_file '
     'is a file. The write path refuses a name that does not resolve.',
     'services::practice_read', NOW(), 'migration'),
    ('practice_read_model', 'claude-opus-5', 'text', 'claude-opus-5', NULL, NULL,
     'Which llm_models row judges the answer. Must name an ACTIVE row or the read '
     'fails loudly and the screen says so.',
     'services::practice_read', NOW(), 'migration'),
    ('practice_read_max_words', '25', 'count', '25', 5, 60,
     'The most words a read may use when it names a tactic. Domain note: this is '
     'not a display cap — a reply above it is REFUSED, and the screen says "no '
     'system read this time", because half a sentence about testimony can invert '
     'its meaning. A row rather than a constant for the reason the theme scan''s '
     'token budget is one: the number that is right depends on the model, and '
     'Roman will want to move it the first evening he watches Marie use this.',
     'services::practice_read_parse', NOW(), 'migration'),
    ('practice_read_max_words_after_fine', '6', 'count', '6', 0, 30,
     'The most words that may follow the OK word. "Fine." plus a speech is still '
     'a speech; this is the cap that keeps the friendly arm as short as the '
     'critical one.',
     'services::practice_read_parse', NOW(), 'migration'),
    ('practice_read_fine_token', 'Fine.', 'text', 'Fine.', NULL, NULL,
     'The exact word the model must produce for "nothing wrong with that answer". '
     'Domain note: this row and the prompt file are COUPLED — the prompt teaches '
     'the model to write it and this teaches the parser to recognise it, and an '
     'operator who edits one must edit the other or every read comes back marked '
     'as a fault. A row precisely so that both halves are editable without a '
     'rebuild, in the same place, by the same person.',
     'services::practice_read_parse', NOW(), 'migration'),
    ('practice_read_max_tokens', '1024', 'count', '1024', 64, 8192,
     'The read''s output cap. Domain note: the reply is ONE sentence of at most '
     '25 words, so 1024 is not the sentence''s budget — it is headroom for a '
     'model that thinks before it answers, which is what truncated 7 of 104 theme '
     'scan verdicts on 2026-08-09 at a cap of 512. It stays under every active '
     'model''s ceiling, including the two 2048-token vLLM rows, because constrain '
     'REFUSES a cap above the ceiling rather than clamping it.',
     'services::practice_read', NOW(), 'migration');
