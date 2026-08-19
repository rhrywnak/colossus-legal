-- practice_v1_part_b_deck_editor_notes_and_review: Practice v1 Part B — Chuck
-- edits the deck, everybody writes notes, and an answer becomes reviewable
--
-- Created: 2026-08-19 11:36:10
-- Target: pipeline database (colossus_legal_v2)
--
-- Task CC_TASK_PRACTICE_V1_CHUCK_REVIEW_v1 §B1–B5 against
-- PRACTICE_MOCKUP_v4_2026-08-19.html, with Roman's three amendments
-- (CC_TASK_PRACTICE_V1_PART_B_GO_v1). Chuck critiques the deck on Thursday
-- morning and may not be available next week, so the parts that let him CHANGE
-- it — and the parts that let Marie see what changed and why — are what this
-- migration makes representable.
--
-- ## Additive only
--
-- Two new tables, four new columns, one backfill, and a block of `app_settings`
-- rows. Nothing existing is dropped or re-typed. Every `ADD COLUMN` carries
-- `IF NOT EXISTS`; every `CREATE TABLE` carries it too; the one constraint that
-- is replaced is dropped-if-exists first.


-- ─── 1 · A question can be HIDDEN, and never deleted ─────────────────────────
--
-- ## Why hidden and not deleted
--
-- `practice_answers.question_id` is `ON DELETE RESTRICT`, and Chuck's sheet is
-- the record of what Marie was actually asked. A question she has answered
-- cannot be removed without taking the meaning of her answer with it. So the
-- deck editor HIDES: the row vanishes from Marie's list and from every queue,
-- past sheets are untouched, and the decision has an author and a time.
--
-- Two columns rather than a boolean, for the reason the flag columns give: "who
-- decided this, and when" must survive the log window, and a lone boolean cannot
-- answer either question.
ALTER TABLE practice_questions ADD COLUMN IF NOT EXISTS hidden_at TIMESTAMPTZ NULL;
ALTER TABLE practice_questions ADD COLUMN IF NOT EXISTS hidden_by TEXT        NULL;

ALTER TABLE practice_questions DROP CONSTRAINT IF EXISTS practice_questions_hidden_pair;
ALTER TABLE practice_questions ADD  CONSTRAINT practice_questions_hidden_pair
    CHECK ((hidden_at IS NULL) = (hidden_by IS NULL));

COMMENT ON COLUMN practice_questions.hidden_at IS
    'When the deck editor hid this question. NULL = live. A hidden question '
    'leaves Marie''s list and every queue; it is never deleted, and every sheet '
    'that already printed it still reads.';


-- ─── 1b · `draft_by` — a column Part A wrote and never created ───────────────
--
-- ## A defect, caught by the guard that exists for it
--
-- Part A taught the deck FILE a `draft_by:` field, taught `seed_rows.rs` to
-- INSERT and UPDATE it, and never added the column. Nothing noticed: the SQL is
-- a `&str`, so it is not a build error; the disk/code guard covered
-- `practice.rs` and `practice_flow.rs` but NOT `seed_rows.rs`, so it was not a
-- test failure either. It would have been a runtime `column "draft_by" does not
-- exist` on the first `--update --apply` against DEV — on the morning Chuck
-- reads the deck.
--
-- Part B widened the guard to every practice repository file, and the guard
-- named it immediately. The column is created here, and `seed_rows.rs` is now
-- inside the cover.
ALTER TABLE practice_questions ADD COLUMN IF NOT EXISTS draft_by TEXT NULL;

ALTER TABLE practice_questions DROP CONSTRAINT IF EXISTS practice_questions_draft_by_nonblank;
ALTER TABLE practice_questions ADD  CONSTRAINT practice_questions_draft_by_nonblank
    CHECK (draft_by IS NULL OR btrim(draft_by) <> '');

COMMENT ON COLUMN practice_questions.draft_by IS
    'Who drafted this question when nobody has reviewed it yet (architect). The '
    'deck editor shows a draft badge on such a row until it is edited. NULL on '
    'every question a human has settled.';


-- ─── 2 · The answer keeps the question AS ASKED ──────────────────────────────
--
-- ## The defect this closes, before it can happen
--
-- Chuck re-words a question on Thursday. Marie answered the OLD wording on
-- Tuesday. Until now `sheet_rows` joined `practice_questions.text` live, so
-- Wednesday's printed sheet would silently re-write itself to show her Tuesday
-- answer under a question she was never asked — which is the one thing a record
-- of testimony may not do.
--
-- So the answer row keeps its own copy, written at answer time. The join stays
-- for everything the deck still owns (the side, the tactic, the braid); the TEXT
-- comes from the answer.
--
-- The backfill uses the question's CURRENT text, which is the best available
-- statement for every row written before this column existed — and it is exact
-- for all of them, because nothing could edit a question's text until now.
ALTER TABLE practice_answers ADD COLUMN IF NOT EXISTS question_text TEXT NULL;

UPDATE practice_answers a
   SET question_text = q.text
  FROM practice_questions q
 WHERE q.id = a.question_id
   AND a.question_text IS NULL;

COMMENT ON COLUMN practice_answers.question_text IS
    'The question AS ASKED, copied at answer time. Chuck''s sheet and the review '
    'page print this, never the deck''s current text: an answer is a moment, and '
    'a later edit must not re-write what she was asked.';


-- ─── 3 · Every change to the deck, with an author ────────────────────────────
--
-- ## Why this is a table and not `updated_at` / `updated_by` on the row
--
-- Marie needs to be told WHAT changed since her last sitting, in plain words,
-- and a pair of columns on the question can only say that something did. It also
-- cannot hold two changes to one row on two days, which is exactly what a week
-- of Chuck's review looks like.
--
-- ## Why `before` and `after` are TEXT and nullable
--
-- A re-wording has both. A move has an order on each side. A hide has neither —
-- the change IS the act. Forcing a value for the cases that have none would mean
-- writing something nobody said.
CREATE TABLE IF NOT EXISTS practice_deck_changes (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    scenario_id UUID        NOT NULL
                REFERENCES scenarios(scenario_id) ON DELETE CASCADE,
    -- CASCADE: a change describing a question that no longer exists describes
    -- nothing. (A question with answers cannot be deleted at all — see the
    -- RESTRICT on `practice_answers` — so this only ever fires for a row nobody
    -- has been asked.)
    question_id UUID        NOT NULL
                REFERENCES practice_questions(id) ON DELETE CASCADE,
    -- What kind of act this was. A CHECK rather than an enum, for the reason
    -- every sibling column gives: the vocabulary lives in code, and a new kind
    -- is a code change plus a migration, both loud.
    change_kind TEXT        NOT NULL
                CHECK (change_kind IN ('added', 'reworded', 'edited', 'moved', 'hidden', 'unhidden')),
    -- Which field an `edited` change touched. NULL on the kinds that are not
    -- about a field: added, moved, hidden, unhidden.
    field       TEXT        CHECK (field IS NULL OR btrim(field) <> ''),
    -- What it was, and what it became. See the header for why both are NULL-able.
    before_value TEXT,
    after_value  TEXT,
    -- Chuck or Roman. There is ONE login, so this is what the editor picked in
    -- "Editing as" — stored on every change rather than derived, because the
    -- answer to "who changed this" must survive the log window.
    changed_by  TEXT        NOT NULL CHECK (btrim(changed_by) <> ''),
    changed_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_practice_deck_changes_scenario
    ON practice_deck_changes (scenario_id, changed_at DESC);

COMMENT ON TABLE practice_deck_changes IS
    'One edit to one question. Read two ways: newest-first for Marie''s "changed '
    'since your last sitting" box, and per-day for the footer of Chuck''s sheet.';


-- ─── 4 · Notes — one mechanism, three places ─────────────────────────────────
--
-- Scenario-level (`question_id IS NULL`), question-level (`question_id` set,
-- `answer_id` NULL), and on one attempt (`answer_id` set). One table, because
-- they are the same act — somebody writing a sentence to the other two — and
-- three tables would be three places to keep the striking rule in step.
--
-- ## Nothing is deleted; a note is STRUCK
--
-- Roman's ruling, and the same one that governs answers and questions. A struck
-- note stays visible, struck through, with who struck it and when. A note
-- somebody could delete is a note nobody can rely on having read.
--
-- ## Why the author is a stored VOCABULARY and not a CHECK
--
-- The three names are case-specific data about real people, and Rule 2 keeps
-- those out of code. `practice_note_authors` below holds them the way
-- `practice_tactic_names` holds the seven cards; the API refuses an author the
-- store does not list, and adding a fourth is a Settings edit.
CREATE TABLE IF NOT EXISTS practice_notes (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    scenario_id UUID        NOT NULL
                REFERENCES scenarios(scenario_id) ON DELETE CASCADE,
    -- NULL = a note about the scenario, shown on the start card.
    question_id UUID        REFERENCES practice_questions(id) ON DELETE CASCADE,
    -- NULL = a note about the question rather than about one attempt at it.
    answer_id   UUID        REFERENCES practice_answers(id) ON DELETE CASCADE,
    author      TEXT        NOT NULL CHECK (btrim(author) <> ''),
    text        TEXT        NOT NULL CHECK (btrim(text) <> ''),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- NULL while the note stands. Struck notes are still rendered.
    struck_at   TIMESTAMPTZ,
    struck_by   TEXT,
    CONSTRAINT practice_notes_struck_pair
        CHECK ((struck_at IS NULL) = (struck_by IS NULL)),
    -- A note on an ATTEMPT is a note on that attempt's question. Without this a
    -- row could name an answer and no question, and the question panel would
    -- silently not show it.
    CONSTRAINT practice_notes_answer_needs_question
        CHECK (answer_id IS NULL OR question_id IS NOT NULL)
);

CREATE INDEX IF NOT EXISTS idx_practice_notes_scenario
    ON practice_notes (scenario_id, created_at);
CREATE INDEX IF NOT EXISTS idx_practice_notes_question
    ON practice_notes (question_id, created_at);

COMMENT ON TABLE practice_notes IS
    'Chuck, Marie and Roman writing to each other about a scenario, a question, '
    'or one attempt. Never deleted; struck notes stay visible. Chuck''s printed '
    'sheet does NOT carry them (task B4).';


-- ─── 5 · The answer box says what v4 says ────────────────────────────────────
--
-- Mockup v4 changed the placeholder, and it is not cosmetic: it is the same
-- ruling prompt v2 carries. "One or two sentences" told her to be brief on a
-- REDIRECT, where length is the right answer, and the box was the first thing
-- she read.
--
-- The alignment below is what `domain::wording::tests`' `corrected_value_in`
-- parses. Written any other way the correction is invisible to the fixture
-- tests, and they go green while the store holds something else.
UPDATE app_settings
SET value         = 'Say it out loud first, then type it — as long as it needs to be.',
    default_value = 'Say it out loud first, then type it — as long as it needs to be.',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'practice_answer_placeholder';


-- ─── 6 · The words ───────────────────────────────────────────────────────────
--
-- Everything Part B adds that a human reads. Not one of them is written in JSX
-- or in Rust: Chuck reads these on Thursday and Roman will want their tone
-- changed after watching him use it, which is a Settings edit and a restart.
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    -- ── Who may sign a note or an edit ───────────────────────────────────────
    ('practice_note_authors', 'Chuck,Marie,Roman', 'text', 'Chuck,Marie,Roman',
     NULL, NULL,
     'Who may sign a note. A comma-separated VOCABULARY, read the way '
     'practice_tactic_names is — case-specific data about real people, which is '
     'exactly what Rule 2 keeps out of code. The API refuses an author this row '
     'does not list.',
     'services::settings_practice', NOW(), 'migration'),
    ('practice_editor_authors', 'Chuck,Roman', 'text', 'Chuck,Roman', NULL, NULL,
     'Who may be picked in "Editing as". A SHORTER list than the note authors '
     'and deliberately so: Marie answers the deck, she does not edit it.',
     'services::settings_practice', NOW(), 'migration'),

    -- ── S0 · the deck editor ─────────────────────────────────────────────────
    ('practice_editor_switch_label', 'Edit the deck', 'text', 'Edit the deck',
     NULL, NULL,
     'Turns the question list into the editor. Domain note: Marie never presses '
     'this — there is one login, and "Editing as" is the honest substitute for '
     'the account separation this build does not have.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_done_label', 'Done editing', 'text', 'Done editing',
     NULL, NULL, 'The same switch, once the editor is open.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_as_label', 'Editing as', 'text', 'Editing as', NULL, NULL,
     'Labels the Chuck/Roman picker. Required before the first change of a '
     'page-load, and stored on every change.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_as_unset', 'Who is editing?', 'text', 'Who is editing?',
     NULL, NULL,
     'The picker''s empty option. A change signed by nobody is a change nobody '
     'can ask about, so the controls stay disabled until this is answered.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_edit_label', 'Edit', 'text', 'Edit', NULL, NULL,
     'Opens the inline fields on one row.', 'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_hide_label', 'Hide', 'text', 'Hide', NULL, NULL,
     'Takes a question out of Marie''s list and every queue. Never a delete.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_unhide_label', 'Unhide', 'text', 'Unhide', NULL, NULL,
     'Puts it back.', 'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_hidden_badge', 'hidden', 'text', 'hidden', NULL, NULL,
     'The grey badge on a hidden row. Only the editor sees the row at all.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_up_label', 'Move up', 'text', 'Move up', NULL, NULL,
     'The ▲ arrow''s accessible name. The glyph is drawn by the stylesheet: it '
     'is decoration, and a screen reader announcing "black up-pointing triangle" '
     'is not a control anybody can use.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_down_label', 'Move down', 'text', 'Move down', NULL, NULL,
     'The ▼ arrow''s accessible name.', 'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_save_label', 'Save', 'text', 'Save', NULL, NULL,
     'Writes the row''s edited fields.', 'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_cancel_label', 'Cancel', 'text', 'Cancel', NULL, NULL,
     'Closes the fields without writing.', 'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_saved_hint_template',
     'Saved as a change by {who} — Marie sees a changed badge on this row.',
     'text',
     'Saved as a change by {who} — Marie sees a changed badge on this row.',
     NULL, NULL,
     'Under the editor''s Save, so the person editing knows the edit is visible '
     'to Marie and signed.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_field_question', 'Question', 'text', 'Question', NULL, NULL,
     'The editor''s text field label.', 'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_field_tactic', 'Tactic', 'text', 'Tactic', NULL, NULL,
     'The editor''s tactic field label. George''s rows only — a Chuck question '
     'has no trap in it.', 'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_field_follows', 'Follows (George question)', 'text',
     'Follows (George question)', NULL, NULL,
     'The editor''s follows field label. Redirect rows only.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_field_watch_for', 'Watch for', 'text', 'Watch for',
     NULL, NULL, 'The editor''s watch-for field label.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_field_stronger', 'Stronger answer', 'text',
     'Stronger answer', NULL, NULL, 'The editor''s stronger-answer field label.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_field_side', 'Side', 'text', 'Side', NULL, NULL,
     'The add form''s side picker label.', 'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_field_attach', 'Attach to', 'text', 'Attach to', NULL, NULL,
     'The add form''s source picker label. What a question is attached to is '
     'where its receipt, its pair and its source line come from.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_side_cross', 'George''s side (cross)', 'text',
     'George''s side (cross)', NULL, NULL, 'The add form''s first side choice.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_side_direct', 'Chuck (direct)', 'text', 'Chuck (direct)',
     NULL, NULL, 'The add form''s second side choice.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_side_redirect',
     'Chuck (redirect — follows a George question)', 'text',
     'Chuck (redirect — follows a George question)', NULL, NULL,
     'The add form''s third side choice. It names what a redirect IS, because '
     'the word alone does not.', 'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_attach_none', 'no receipt', 'text', 'no receipt', NULL, NULL,
     'The add form''s honest option for a question that traces to nothing. The '
     'screen then says "no receipt" in words rather than showing a blank source '
     'line.', 'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_attach_instance_template', 'instance {n} — {text}', 'text',
     'instance {n} — {text}', NULL, NULL,
     'One ruled accusation instance in the add form''s picker. {text} is the '
     'instance''s own source line where the deck carries one.',
     'services::practice_editor', NOW(), 'migration'),
    ('practice_editor_attach_point_template', 'point {n} — {text}', 'text',
     'point {n} — {text}', NULL, NULL,
     'One talking point in the add form''s picker.',
     'services::practice_editor', NOW(), 'migration'),
    ('practice_editor_add_label', '+ Add a question', 'text', '+ Add a question',
     NULL, NULL, 'Opens the add form.', 'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_add_heading', 'Add a question', 'text', 'Add a question',
     NULL, NULL, 'The add form''s heading.', 'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_add_button', 'Add', 'text', 'Add', NULL, NULL,
     'Writes the new question.', 'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_add_hint',
     'A new question shows a changed badge to Marie until she has answered it once.',
     'text',
     'A new question shows a changed badge to Marie until she has answered it once.',
     NULL, NULL, 'Under the add form''s button.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_question_placeholder',
     'One sentence, in the voice of the side asking it.', 'text',
     'One sentence, in the voice of the side asking it.', NULL, NULL,
     'The add form''s textarea placeholder.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_editor_failed',
     'That change was not saved. Nothing on the deck has moved; try again.',
     'text',
     'That change was not saved. Nothing on the deck has moved; try again.',
     NULL, NULL,
     'What the editor shows when a write fails. It says the deck is UNCHANGED, '
     'because an editor who believes an edit landed when it did not will not '
     'make it again.', 'frontend PracticeDeckList', NOW(), 'migration'),

    -- ── S0 · what changed since she was last here ────────────────────────────
    ('practice_changed_heading_template',
     'Changed since your last sitting: {n} questions — {who}, {when}', 'text',
     'Changed since your last sitting: {n} questions — {who}, {when}', NULL, NULL,
     'The blue box above the deck. {who} and {when} name the NEWEST editor and '
     'day, not every one — a witness needs to know whether to re-read, not an '
     'audit trail.', 'services::practice_changes', NOW(), 'migration'),
    ('practice_changed_notes_template', '{n} new notes — {who}', 'text',
     '{n} new notes — {who}', NULL, NULL,
     'Appended to the changed line when notes have arrived since her last '
     'sitting.', 'services::practice_changes', NOW(), 'migration'),
    ('practice_changed_summary', 'what changed', 'text', 'what changed',
     NULL, NULL, 'The fold that opens the list of changes in plain words.',
     'frontend PracticeChanged', NOW(), 'migration'),
    ('practice_change_added_template', 'new: Q{n} ({side})', 'text',
     'new: Q{n} ({side})', NULL, NULL,
     'One added question, in the list. {n} is its PRINTED position in the deck, '
     'which is the number beside it on screen.',
     'services::practice_changes', NOW(), 'migration'),
    ('practice_change_reworded_template', 'Q{n} re-worded', 'text',
     'Q{n} re-worded', NULL, NULL, 'One re-worded question, in the list.',
     'services::practice_changes', NOW(), 'migration'),
    ('practice_change_edited_template', 'Q{n} — {field} changed', 'text',
     'Q{n} — {field} changed', NULL, NULL,
     'One question whose watch-for, stronger answer, tactic or follows changed.',
     'services::practice_changes', NOW(), 'migration'),
    ('practice_change_moved_template', 'Q{n} moved', 'text', 'Q{n} moved',
     NULL, NULL, 'One re-ordered question, in the list.',
     'services::practice_changes', NOW(), 'migration'),
    ('practice_change_hidden_template', 'Q{n} hidden', 'text', 'Q{n} hidden',
     NULL, NULL, 'One hidden question, in the list. Domain note: it is listed '
     'even though she can no longer see the row — "a question you were going to '
     'be asked is gone" is exactly what she needs told.',
     'services::practice_changes', NOW(), 'migration'),
    ('practice_change_unhidden_template', 'Q{n} put back', 'text', 'Q{n} put back',
     NULL, NULL, 'One un-hidden question, in the list.',
     'services::practice_changes', NOW(), 'migration'),
    ('practice_badge_changed', 'changed', 'text', 'changed', NULL, NULL,
     'The badge on a row that has changed since her last sitting. It stays until '
     'she has answered that question once AFTER the change.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_badge_draft', 'draft — Chuck to edit', 'text',
     'draft — Chuck to edit', NULL, NULL,
     'The badge on a question the architect drafted and nobody has reviewed. It '
     'names who is expected to act, which "draft" alone does not.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_sheet_changes_heading', 'Changed today', 'text', 'Changed today',
     NULL, NULL,
     'The heading of the change list at the foot of Chuck''s sheet — what was '
     'edited on the day of this sitting, and by whom.',
     'frontend PracticeSheet', NOW(), 'migration'),
    ('practice_sheet_change_item_template', '{what} — {who}', 'text',
     '{what} — {who}', NULL, NULL,
     'One change, as the sheet prints it.',
     'services::practice_changes', NOW(), 'migration'),

    -- ── Notes ────────────────────────────────────────────────────────────────
    ('practice_notes_heading_template', 'Notes ({n})', 'text', 'Notes ({n})',
     NULL, NULL,
     'The collapsed panel''s header (Roman, 2026-08-19). {n} counts the UNSTRUCK '
     'notes: a struck note is still there to read, but it is not something '
     'waiting for her. The disclosure arrow is drawn by the control, not stored '
     'here — see practice_stronger_summary for what a second arrow looks like.',
     'frontend PracticeNotes', NOW(), 'migration'),
    ('practice_notes_scenario_title', 'Notes on this scenario', 'text',
     'Notes on this scenario', NULL, NULL,
     'The scenario panel''s title, once open.', 'frontend PracticeNotes', NOW(), 'migration'),
    ('practice_notes_question_title', 'Notes on this question', 'text',
     'Notes on this question', NULL, NULL,
     'The review page''s panel title, once open.', 'frontend PracticeNotes', NOW(), 'migration'),
    ('practice_notes_hint',
     'Chuck, Marie and Roman see all of these. Nothing is deleted; a note can be struck.',
     'text',
     'Chuck, Marie and Roman see all of these. Nothing is deleted; a note can be struck.',
     NULL, NULL,
     'Beside the panel title. It states the two facts that decide whether '
     'somebody writes honestly: who reads it, and that it cannot be made to '
     'disappear.', 'frontend PracticeNotes', NOW(), 'migration'),
    ('practice_notes_placeholder', 'Add a note…', 'text', 'Add a note…',
     NULL, NULL, 'The scenario and question panels'' input placeholder.',
     'frontend PracticeNotes', NOW(), 'migration'),
    ('practice_notes_attempt_placeholder', 'Add a note on this attempt…', 'text',
     'Add a note on this attempt…', NULL, NULL,
     'The per-attempt input placeholder. A different sentence because it is a '
     'different subject: this note is about ONE answer, not about the question.',
     'frontend PracticeReview', NOW(), 'migration'),
    ('practice_notes_save_label', 'Save', 'text', 'Save', NULL, NULL,
     'Writes the note.', 'frontend PracticeNotes', NOW(), 'migration'),
    ('practice_notes_strike_label', 'Strike', 'text', 'Strike', NULL, NULL,
     'Strikes a note through. Never a delete — the note stays readable and says '
     'who struck it.', 'frontend PracticeNotes', NOW(), 'migration'),
    ('practice_notes_struck_template', 'struck {when}', 'text', 'struck {when}',
     NULL, NULL, 'Appended under the author of a struck note.',
     'services::practice_notes', NOW(), 'migration'),
    ('practice_notes_empty', 'No notes on this yet.', 'text',
     'No notes on this yet.', NULL, NULL,
     'What the opened panel says when there are none. A named absence, never a '
     'blank panel — an empty box reads as a list that failed to load.',
     'frontend PracticeNotes', NOW(), 'migration'),
    ('practice_notes_failed',
     'That note was not saved. Nothing was written; try again.', 'text',
     'That note was not saved. Nothing was written; try again.', NULL, NULL,
     'What the panel shows when a write fails.',
     'frontend PracticeNotes', NOW(), 'migration'),
    ('practice_notes_author_unset', 'Who is writing?', 'text', 'Who is writing?',
     NULL, NULL,
     'The author picker''s empty option. An unsigned note is one nobody can '
     'answer, so Save stays disabled until this is chosen.',
     'frontend PracticeNotes', NOW(), 'migration'),

    -- ── The review page ──────────────────────────────────────────────────────
    ('practice_row_review_link', 'review', 'text', 'review', NULL, NULL,
     'The link at the end of an answered row''s status, opening the review page.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_review_progress_template', 'Question {n} · review', 'text',
     'Question {n} · review', NULL, NULL,
     'The review page''s progress line. {n} is the question''s printed position '
     'in the deck.', 'services::practice_review', NOW(), 'migration'),
    ('practice_review_attempts_kicker', 'Your attempts — newest first', 'text',
     'Your attempts — newest first', NULL, NULL,
     'Over the stack of attempts. The ORDER is stated because it is the opposite '
     'of the sheet''s, and a reader who assumes oldest-first reads her worst '
     'answer as her latest.', 'frontend PracticeReview', NOW(), 'migration'),
    ('practice_review_attempt_template', 'attempt {n} · {when}', 'text',
     'attempt {n} · {when}', NULL, NULL,
     'One attempt''s heading. {n} counts from her FIRST attempt, so attempt 1 is '
     'always the same row however the list is sorted.',
     'services::practice_review', NOW(), 'migration'),
    ('practice_review_detail_template', 'help: {help} · boxes: {boxes}', 'text',
     'help: {help} · boxes: {boxes}', NULL, NULL,
     'The grey line under one attempt. What she pointed to is prefixed '
     'separately (practice_points_to_reveal_prefix) so an attempt that named '
     'nothing renders no empty clause.',
     'services::practice_review', NOW(), 'migration'),
    ('practice_review_boxes_none', 'none ticked', 'text', 'none ticked',
     NULL, NULL,
     'Stands where the self-check boxes go when she ticked none. A named absence '
     'rather than an empty clause — and NOT a fault: ticking nothing is a '
     'legitimate reading of her own answer.',
     'services::practice_review', NOW(), 'migration'),
    ('practice_review_no_attempts',
     'You have not answered this question yet.', 'text',
     'You have not answered this question yet.', NULL, NULL,
     'What the review page says when it is reached for a question with no '
     'answers — by a typed address, since no row offers the link. The study '
     'material below it still stands.',
     'frontend PracticeReview', NOW(), 'migration'),
    ('practice_review_practice_again', 'Practice this one again ▸', 'text',
     'Practice this one again ▸', NULL, NULL,
     'Opens a one-question sitting from the review page. The ▸ is part of the '
     'label here and not a disclosure marker: this is a link to another screen, '
     'and nothing else draws an arrow beside it.',
     'frontend PracticeReview', NOW(), 'migration'),
    ('practice_review_stronger_heading', 'A stronger answer', 'text',
     'A stronger answer', NULL, NULL,
     'The drawer''s heading on the review page, where it is OPEN by default. A '
     'different sentence from the drill''s "Show a stronger answer", because '
     'here it is not something to show — it is already showing.',
     'frontend PracticeReview', NOW(), 'migration');
