-- practice_one_page_l2: the list page's words, and Print answers.
--
-- Created: 2026-08-23 13:43:49
-- Target: pipeline database (colossus_legal_v2)
--
-- ## ⚑ TWO FORMAT RULES, BOTH LOAD-BEARING, BOTH LEARNED THE HARD WAY
--
-- 1. `value` and `default_value` must each be ONE quoted literal on ONE line.
--    `seeded_value_in` reads the first quoted literal after the key; adjacent
--    literal concatenation is invisible to it and pins a truncated string.
--
-- 2. The key must sit IMMEDIATELY after the opening paren — `VALUES ('key',` —
--    because the parser's marker is the two characters `('` followed by the key.
--    L1's migration was first written with the key indented on its own line:
--    perfectly good SQL, and the fixture then reported the key as seeded by NO
--    migration at all. Well-formatted SQL is what breaks this, which is why it
--    is written here rather than left to be rediscovered.
--
-- Corrections use the `SET value         = '` / `WHERE key           =` spacing
-- exactly, for the same reason: `corrected_value_in` searches for that shape.
--
-- No answer, note, flag or change-log row is read or written by this file.

-- ─────────────────────────────────────────────────────────────────────────────
-- NEW ROWS — the list page after the cuts, and the answers sheet
-- ─────────────────────────────────────────────────────────────────────────────

-- The third button in the title row. Chuck prints questions to mark up; he
-- prints answers to read. Two different sheets for two different acts.
INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_print_answers_label',
    '🖨 Print answers',
    '🖨 Print answers',
    'string',
    'The button that prints Marie''s answers for Chuck to read. Sits beside Print questions in the title row of the practice page.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_print_answers_page_title',
    'Answers — {code}',
    'Answers — {code}',
    'string',
    'The browser tab''s title on the printed-answers view. {code} is the scenario code. A person with three print tabs open needs to tell them apart from the tab strip alone — and to tell an answers tab from a questions tab.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_print_answers_howto',
    'Marie''s answers as they stand today, in the same order as the questions. The current answer only — not the earlier versions.',
    'Marie''s answers as they stand today, in the same order as the questions. The current answer only — not the earlier versions.',
    'string',
    'The line under the header on every answers sheet. Domain note: it states BOTH facts a reader needs — that these are current as of printing, and that an answer Marie has since rewritten is not what he is holding. Paper outlives the deck it came from.',
    'migration')
ON CONFLICT (key) DO NOTHING;

-- What an answers sheet says where a question has no answer behind it.
--
-- Domain note: the question still PRINTS. Omitting it would make the answers
-- sheet disagree with the questions sheet about how many questions there are,
-- and Chuck reads them side by side.
INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_print_answer_missing',
    'Not answered yet.',
    'Not answered yet.',
    'string',
    'Printed in place of an answer where Marie has not written one. The question itself still prints: omitting it would make the answers sheet disagree with the questions sheet about how many questions the deck holds.',
    'migration')
ON CONFLICT (key) DO NOTHING;

-- ── The practice bar ─────────────────────────────────────────────────────────
-- The dropdown's two options are NOT new rows: `practice_who_george_title` and
-- `practice_who_chuck_title` already hold "The defense asks" and "Chuck asks",
-- which is exactly what the bar offers. Seeding a second pair would be two
-- places to edit and one of them eventually not edited.

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_practice_mode_label',
    'Practice mode',
    'Practice mode',
    'string',
    'The label at the left of the practice bar, above the question list.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_start_practising_label',
    'Start practising',
    'Start practising',
    'string',
    'The button that begins a practice walk. Domain note: this does NOT open a sitting and writes nothing — practice mode makes no model call and no database write of any kind. It walks questions Marie has already answered so she can say them out loud and check herself.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_practice_hint',
    'One question at a time, your answer hidden until you ask for it.',
    'One question at a time, your answer hidden until you ask for it.',
    'string',
    'The one line of hint beside Start practising. Standing rule of 2026-08-19: no control on a practice page is dim and silent — a person must be able to tell what a control does before pressing it.',
    'migration')
ON CONFLICT (key) DO NOTHING;

-- ── Delete, and the undo that replaces a confirm dialog ──────────────────────

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_row_delete_label',
    'Delete',
    'Delete',
    'string',
    'The control that removes a question from the deck, on the row and on the question page. Domain note: the LABEL is Delete and the mechanism underneath is the existing hide, unchanged — so a question Marie has answered can never be orphaned from her answers. The user''s contract is "I will not see this again" and that is what is kept.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_row_deleted_notice',
    'Question deleted.',
    'Question deleted.',
    'string',
    'The line that replaces a row after it is deleted, carrying the undo beside it. Domain note: this exists INSTEAD of a confirm dialog. A dialog costs a step every time to guard against the rare case; an undo costs nothing in the normal case and still covers the misclick.',
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_row_undo_label',
    'Undo',
    'Undo',
    'string',
    'The control beside "Question deleted." that puts the question back. It lives until the page is left or reloaded; there is no restore path beyond it, deliberately, because a second one would be a state nobody ruled on.',
    'migration')
ON CONFLICT (key) DO NOTHING;

-- The footnote under the list, explaining the one status a row carries.
INSERT INTO app_settings (key, value, default_value, kind, meaning, updated_by)
VALUES ('practice_deck_status_footnote',
    'No date means not answered yet. That is the only status a row carries.',
    'No date means not answered yet. That is the only status a row carries.',
    'string',
    'The small line under the question list. It exists because the row''s marks were removed: a reader who remembers "answered today · repeat · attempt 2" needs to be told once that their absence is not a fault.',
    'migration')
ON CONFLICT (key) DO NOTHING;

-- ─────────────────────────────────────────────────────────────────────────────
-- CORRECTIONS — three rows whose words name things that no longer exist
-- ─────────────────────────────────────────────────────────────────────────────

-- "George's side" goes. It is a human name on a screen that already calls him
-- "the defense" two inches above, and Roman's ruling of 2026-08-23 is that the
-- two contradicted each other. `{n}` — the deck total — goes with it: the two
-- side counts already add up to it, and a third number is a third thing to read.
UPDATE app_settings
SET value         = '· {george} from the defense · {chuck} from Chuck',
    default_value = '· {george} from the defense · {chuck} from Chuck',
    meaning       = 'The count beside the question-list heading. {george} and {chuck} are the two sides'' counts. Domain note: the human name is deliberately absent — the page calls that side "the defense" everywhere else, and a screen that used both made a reader wonder whether they were two different things.',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'practice_deck_count_template';

-- The button gains its mark, matching the two print buttons beside it. Chuck
-- could not find this control when it was a text link.
UPDATE app_settings
SET value         = '✎ Edit the deck',
    default_value = '✎ Edit the deck',
    meaning       = 'The button that turns the deck editor on. Domain note: this was a TEXT LINK until 2026-08-23 and Chuck could not find it — which put Edit, the reorder arrows and Hide all behind the least discoverable thing on the page.',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'practice_editor_switch_label';

-- The how-to loses its last sentence. Question codes are gone from the printed
-- sheets, so a sentence explaining the blue box describes a box that is not
-- there — the worst kind of stale instruction, because it reads as correct.
UPDATE app_settings
SET value         = 'In the order the defense would ask them at trial — the facts first, the conclusion last. Mark anything up. To enter your changes: Trial Prep → {code} → Practice → Edit the deck.',
    default_value = 'In the order the defense would ask them at trial — the facts first, the conclusion last. Mark anything up. To enter your changes: Trial Prep → {code} → Practice → Edit the deck.',
    meaning       = 'The how-to line on the printed cross sheet. Domain note: the sentence about "the code in the blue box" was removed on 2026-08-23 when question codes left both the screen and the paper. An instruction naming a thing that is not on the page reads as correct and is not.',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'practice_print_howto_cross';
