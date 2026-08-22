-- practice_print_questions_wording: Chuck's review sheets — every string.
--
-- Created: 2026-08-22 15:43:21
-- Target: pipeline database (colossus_legal_v2)
--
-- ## What this is for
--
-- Chuck develops questions away from his laptop and today cannot take the deck
-- with him. The print view renders up to three sheets — the defense's cross
-- questions, Chuck's directs, and his redirects each under the question it
-- repairs — with the deck key on every row, so a note he writes today still
-- points at the right question next week after the deck has been re-ordered.
--
-- ## THIS MIGRATION IS PURELY ADDITIVE
--
-- Twenty-six INSERTs and nothing else. No ALTER, no UPDATE, no DELETE, and no
-- statement naming practice_answers, practice_questions, practice_notes or any
-- other table that records something a human did. `ON CONFLICT (key) DO NOTHING`
-- so a row an operator has already re-worded survives a redeploy.
--
-- ## ⚑ practice_print_button IS NOT THIS FEATURE
--
-- That key already exists, reads 'Print Chuck''s sheet', and belongs to the
-- END-OF-SITTING sheet (consumed_by 'frontend PracticeSheet'). It is untouched
-- here. Every row below is namespaced `practice_print_questions_*` or
-- `practice_print_sheet_*` or `practice_print_howto_*` precisely so that neither
-- feature can ever be re-worded by an operator aiming at the other.

-- ## ⚑ EVERY `value` IS ONE QUOTED LITERAL, HOWEVER LONG THE LINE
--
-- `domain::wording::tests::seeded_value_in` parses a value by finding the key and
-- reading the FIRST quoted literal after it. Adjacent-literal concatenation —
-- 'a ' 'b' — is invisible to it, so a value split that way is read as its first
-- fragment and the fixture test fails with a truncated string. `meaning` may be
-- wrapped freely (nothing parses it); `value` and `default_value` may not.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES

-- ─── 1 · The control on the practice page ───────────────────────────────────

    ('practice_print_questions_label', '🖨 Print questions', 'text',
     '🖨 Print questions', NULL, NULL,
     'The control right of the scenario title that opens the print view in a new '
     'tab. Domain note: it IGNORES the "Who''s asking?" selector and always takes '
     'the whole deck, because that selector includes Mixed and Mixed is a dealing '
     'order, not a thing anyone reviews.',
     'frontend PracticeStart', NOW(), 'migration'),

    ('practice_print_questions_empty_hint', 'No questions in this deck yet.', 'text',
     'No questions in this deck yet.', NULL, NULL,
     'Why the print control is disabled when the deck has no visible questions. '
     'Standing rule of 2026-08-19: no control on a practice page may be dim and '
     'silent. Also shown when every question in the deck is hidden.',
     'frontend PracticeStart', NOW(), 'migration'),

-- ─── 2 · The print view''s own two controls ──────────────────────────────────
--
-- ## Domain note: the view does NOT print itself on load
--
-- Chuck opens the tab, READS the sheets, and then decides. A page that starts
-- printing before he has looked at it is a page he cannot review — and reviewing
-- is the whole purpose. Both controls hide themselves under @media print.

    ('practice_print_now_label', 'Print', 'text', 'Print', NULL, NULL,
     'The button at the top of the print view that actually prints. Hidden in '
     'print. Deliberately NOT fired on load: the view exists so Chuck can look '
     'first.',
     'frontend PracticePrintPage', NOW(), 'migration'),

    ('practice_print_back_label', '◂ Back to the deck', 'text',
     '◂ Back to the deck', NULL, NULL,
     'The way back from the print view. Hidden in print. The view opens in a new '
     'tab, so closing it usually suffices — this is for a person who arrives from '
     'a bookmark or a second monitor and has no tab to close.',
     'frontend PracticePrintPage', NOW(), 'migration'),

    ('practice_print_page_title', 'Questions — {code}', 'text',
     'Questions — {code}', NULL, NULL,
     'The browser tab''s title on the print view. {code} is the scenario handle '
     '(S-5). Carries no accusation text: a tab title is read over a shoulder.',
     'frontend PracticePrintPage', NOW(), 'migration'),

-- ─── 3 · The three sheet titles ─────────────────────────────────────────────

    ('practice_print_sheet_cross_title', 'The defense asks', 'text',
     'The defense asks', NULL, NULL,
     'Sheet 1''s title — every cross question, in deck order.',
     'frontend PrintSheets', NOW(), 'migration'),

    ('practice_print_sheet_direct_title', 'Chuck asks', 'text',
     'Chuck asks', NULL, NULL,
     'Sheet 2''s title — every direct question, in deck order.',
     'frontend PrintSheets', NOW(), 'migration'),

    ('practice_print_sheet_redirect_title', 'Chuck, after the defense', 'text',
     'Chuck, after the defense', NULL, NULL,
     'Sheet 3''s title — every redirect, each under the defense question it '
     'follows.',
     'frontend PrintSheets', NOW(), 'migration'),

    ('practice_print_sheet_subtitle_template', '{code} · “{title}”', 'text',
     '{code} · “{title}”', NULL, NULL,
     'The line under a sheet''s title, naming the scenario. {code} is S-5; '
     '{title} is the accusation, in quotes because it is a claim somebody made '
     'and not this build''s own words.',
     'frontend PrintSheets', NOW(), 'migration'),

    ('practice_print_sheet_redirect_subtitle', 'the redirects — each follows one of the defense''s questions', 'text',
     'the redirects — each follows one of the defense''s questions', NULL, NULL,
     'Sheet 3''s subtitle, replacing the accusation. Domain note: it must NOT say '
     '"one for each question the defense asks" — that is false whenever the counts '
     'differ, and on S-7 they do (6 cross, 2 redirects). Roman''s correction of '
     '2026-08-22.',
     'frontend PrintSheets', NOW(), 'migration'),

-- ─── 4 · The header meta block ──────────────────────────────────────────────

    ('practice_print_printed_template', 'printed {when}', 'text',
     'printed {when}', NULL, NULL,
     'When this copy came off the printer. Distinct from the deck''s own date '
     'below it: paper outlives the deck it was taken from, and a sheet that says '
     'only one of the two cannot tell a reader which.',
     'frontend PrintSheets', NOW(), 'migration'),

    ('practice_print_deck_as_of_template', 'deck as of {date} · {n} of {m} questions', 'text',
     'deck as of {date} · {n} of {m} questions', NULL, NULL,
     'The deck''s own last change, and two counts. Domain note: {n} is THIS '
     'SHEET''S count and {m} is the WHOLE DECK — Roman''s ruling of 2026-08-22. '
     'Both, because a sheet showing only its own count cannot tell Chuck how much '
     'of the deck he is not holding. Hidden questions are in neither number.',
     'frontend PrintSheets', NOW(), 'migration'),

-- ─── 5 · The how-to line, one per sheet ─────────────────────────────────────

    ('practice_print_howto_cross', 'In the order the defense would ask them at trial — the facts first, the conclusion last. Mark anything up. To enter your changes: Trial Prep → {code} → Practice → Edit the deck. The code in the blue box is the question''s permanent name; it does not change when the deck is re-ordered.', 'text',
     'In the order the defense would ask them at trial — the facts first, the conclusion last. Mark anything up. To enter your changes: Trial Prep → {code} → Practice → Edit the deck. The code in the blue box is the question''s permanent name; it does not change when the deck is re-ordered.',
     NULL, NULL,
     'Sheet 1''s instruction. It carries the route back into the app, because the '
     'whole value of the paper is that a note written on it can be acted on later.',
     'frontend PrintSheets', NOW(), 'migration'),

    ('practice_print_howto_direct', 'Your direct — foundation first, then her three points. Each says which point it rests on.', 'text',
     'Your direct — foundation first, then her three points. Each says which point it rests on.', NULL, NULL,
     'Sheet 2''s instruction.',
     'frontend PrintSheets', NOW(), 'migration'),

    ('practice_print_howto_redirect', 'Each one repairs one defense question, so the question it follows is printed above it — a redirect read on its own means nothing.', 'text',
     'Each one repairs one defense question, so the question it follows is printed above it — a redirect read on its own means nothing.', NULL, NULL,
     'Sheet 3''s instruction. Domain note: it carries NO COUNT. The mockup said '
     '"All five are drafts", which is wrong twice — the count varies by deck (S-7 '
     'has two) and draftness is a separate fact. Roman''s correction, 2026-08-22.',
     'frontend PrintSheets', NOW(), 'migration'),

    ('practice_print_howto_redirect_drafts', 'These are drafts, written for you to rewrite.', 'text',
     'These are drafts, written for you to rewrite.', NULL, NULL,
     'Added to sheet 3''s instruction ONLY when at least one redirect on it '
     'carries draft_by. Its own row rather than part of the line above so it can '
     'be withheld: on S-7 no redirect is marked a draft, and a sheet claiming '
     'draftness its own rows do not show would be the paper contradicting itself.',
     'frontend PrintSheets', NOW(), 'migration'),

-- ─── 6 · Row-level strings ──────────────────────────────────────────────────

    ('practice_print_after_template', 'After the defense asks {key}: {question}', 'text',
     'After the defense asks {key}: {question}', NULL, NULL,
     'The quoted antecedent above a redirect. {key} is the defense question''s '
     'deck key (g4); {question} is its text. The judgement Chuck is making is '
     'whether this repairs that, and he cannot make it from the redirect alone.',
     'frontend PrintSheets', NOW(), 'migration'),

    ('practice_print_after_missing', 'The defense question this one repairs is no longer in the deck.', 'text',
     'The defense question this one repairs is no longer in the deck.', NULL, NULL,
     'Stands in when a redirect''s follows_key names no question in this scenario. '
     'Domain note: follows_key is a KEY, not a foreign key, so nothing in the '
     'database stops the question it names being hidden or removed. A named '
     'absence, never an empty quote box — a blank there reads as a redirect that '
     'repairs nothing.',
     'frontend PrintSheets', NOW(), 'migration'),

-- ─── 7 · The footer ─────────────────────────────────────────────────────────

    ('practice_print_footer_template', '{code} · {sheet} · {n} questions', 'text',
     '{code} · {sheet} · {n} questions', NULL, NULL,
     'The foot of each sheet. {sheet} is that sheet''s own title; {n} is its own '
     'count, so a sheet separated from the others still says what it is.',
     'frontend PrintSheets', NOW(), 'migration'),

    ('practice_print_sheet_number_template', 'sheet {n} of {m}', 'text',
     'sheet {n} of {m}', NULL, NULL,
     'Which SHEET this is, and how many were printed. Domain note: SHEETS, NOT '
     'PAGES — Roman''s correction of 2026-08-22. A sheet with enough questions '
     'runs onto a second and third piece of paper (S-7 has eight directs), and '
     '"page 2 of 3" on both halves of one sheet is a lie the browser cannot fix. '
     'Physical pagination is the browser''s; this number is the document''s.',
     'frontend PrintSheets', NOW(), 'migration'),

-- ─── 8 · What is ABSENT, said in words ──────────────────────────────────────
--
-- ## Why five rows for one sentence
--
-- The line has six possible shapes — any one or any two of the three kinds may
-- be absent; all three absent is the empty deck, where the control is disabled
-- instead. Five fragments compose all six. Six whole sentences would be six rows
-- that drift out of agreement with one another.
--
-- ## ⚑ THE JOINER CANNOT CARRY ITS OWN SPACE
--
-- The settings store TRIMS every value, so ', and ' is stored as ', and' and the
-- renderer supplies the joining space. A joiner written with a trailing space
-- here would arrive without it and print "no redirects, andno questions".

    ('practice_print_missing_prefix', 'This deck has', 'text',
     'This deck has', NULL, NULL,
     'Opens the line on the last sheet naming what the deck does not yet contain. '
     'Most decks are partial; this is the normal case, not an edge case.',
     'frontend PrintSheets', NOW(), 'migration'),

    ('practice_print_missing_cross', 'no questions from the defense yet', 'text',
     'no questions from the defense yet', NULL, NULL,
     'The fragment naming an absent cross sheet.',
     'frontend PrintSheets', NOW(), 'migration'),

    ('practice_print_missing_direct', 'no questions from Chuck yet', 'text',
     'no questions from Chuck yet', NULL, NULL,
     'The fragment naming an absent direct sheet.',
     'frontend PrintSheets', NOW(), 'migration'),

    ('practice_print_missing_redirect', 'no redirects', 'text',
     'no redirects', NULL, NULL,
     'The fragment naming an absent redirect sheet.',
     'frontend PrintSheets', NOW(), 'migration'),

    ('practice_print_missing_joiner', ', and', 'text',
     ', and', NULL, NULL,
     'Joins two absent-kind fragments. Stored WITHOUT a trailing space — the store '
     'trims, and the renderer supplies the space. See this section''s header.',
     'frontend PrintSheets', NOW(), 'migration'),

    ('practice_print_hidden_template', '{n} questions are hidden and are not shown.', 'text',
     '{n} questions are hidden and are not shown.', NULL, NULL,
     'On the last sheet when the deck holds hidden questions. Domain note: hidden '
     'questions do not print and are in no count (Roman, 2026-08-22) — but their '
     'EXISTENCE is said, because a Chuck who does not know one is hidden will '
     'rewrite a question the deck already has.',
     'frontend PrintSheets', NOW(), 'migration')

-- A row an operator has already re-worded is never overwritten by a redeploy.
ON CONFLICT (key) DO NOTHING;
