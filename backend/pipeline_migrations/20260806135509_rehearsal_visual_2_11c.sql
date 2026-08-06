-- rehearsal_visual_2_11c: authorship on the two sentences, and the words the
-- rebuilt rehearsal page speaks (task 2.11 C, Phase B)
--
-- Created: 2026-08-06 13:55:09
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- Rules on CC_REPORT_2_11C_PHASE_A.md via CC_TASK_2_11C_PHASE_B_RULINGS_v1
-- (architect, 2026-08-06). Two things are in this file:
--
--   1. C2 — the attribution columns. Phase A measured that NOTHING stamps an
--      author on `scenarios.accusation_text` or `scenarios.theme_statement`; the
--      instruction's claim that task B1 already did was wrong. The mockup's two
--      "Written in plain words by {who} · {when}" lines therefore had no data
--      behind them. They do now.
--
--   2. The forty-two wording rows and one parameter the rebuilt page needs.
--
-- ## Why the four columns are NULLABLE and NOT backfilled
--
-- Every sentence written before this migration has no recorded author, and the
-- honest-gap law forbids inventing one — stamping `'system (seed)'` or the last
-- editor of the scenario ROW would put a name on a sentence that name may not
-- have written. NULL means "not recorded", the page says so in the stored
-- `rehearsal_attribution_unknown_notice`, and the first edit from this commit
-- forward records the truth.
--
-- ## Why the accusation's pair CLEARS on withdrawal
--
-- `accusation_text` is nullable and "Withdraw it" sets it to NULL. An author
-- line surviving a withdrawn sentence would attribute a sentence that is not
-- there. The write path clears the pair with the text; see
-- `set_scenario_accusation`.
--
-- ## Why `scenarios.updated_at` could not have served
--
-- It is scenario-WIDE: renaming the scenario moves it. Dating the accusation
-- sentence from it would print a date the sentence was not written on, which is
-- worse than printing nothing.
--
-- ON CONFLICT DO NOTHING keeps the seed re-runnable and never stamps over a
-- value a human has edited. The consequence, learned in 2.12: editing an APPLIED
-- migration's VALUES list changes nothing on an environment that already ran it
-- — a correction has to be its own later UPDATE, guarded on the old text.
--
-- FORWARD-ONLY: no down migration.

-- ── C2: who wrote the two sentences, and when ────────────────────────────────

ALTER TABLE scenarios
    ADD COLUMN accusation_text_authored_by TEXT,
    ADD COLUMN accusation_text_authored_at TIMESTAMPTZ,
    ADD COLUMN theme_authored_by          TEXT,
    ADD COLUMN theme_authored_at          TIMESTAMPTZ;

COMMENT ON COLUMN scenarios.accusation_text_authored_by IS
    'Who last wrote the plain-words accusation. NULL means the authorship was '
    'not recorded — sentences written before task 2.11 C. Never backfilled: an '
    'invented author is worse than a named absence. Cleared with the sentence '
    'when it is withdrawn.';

COMMENT ON COLUMN scenarios.accusation_text_authored_at IS
    'When the plain-words accusation was last written. Distinct from '
    'scenarios.updated_at, which moves for any edit to the scenario row.';

COMMENT ON COLUMN scenarios.theme_authored_by IS
    'Who last wrote theme_statement — the rehearsal page''s "What this is". '
    'Same NULL discipline as accusation_text_authored_by.';

COMMENT ON COLUMN scenarios.theme_authored_at IS
    'When theme_statement was last written.';

-- ── The rehearsal page's new words ───────────────────────────────────────────

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    -- ── The instance row's two tags ──────────────────────────────────────────
    -- Ruling C5: the row tag, the expanded banner and the prep-list sentence are
    -- three articulations at three zoom levels, not one sentence three times.
    -- The beta.381 defect was the full SENTENCE twice; these carry no who/when/
    -- where and cannot become it.
    ('rehearsal_answered_tag', 'ANSWERED', 'text', 'ANSWERED',
     NULL, NULL,
     'The small green tag on an instance row a human has paired an answer to. '
     'Read at a glance while scanning the list — keep it short.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_no_answer_tag', 'NO ANSWER', 'text', 'NO ANSWER',
     NULL, NULL,
     'The small red tag on an instance row nobody has answered yet. The quiet '
     'form of the gap: the sentence naming who said it, when and where lives '
     'once, in the prep list.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_no_answer_banner', 'NO ANSWER PREPARED', 'text',
     'NO ANSWER PREPARED',
     NULL, NULL,
     'The red banner inside an OPENED instance row that has no answer. Says the '
     'same thing as the tag, louder, where the row is being worked on. It '
     'deliberately carries no who/when/where — that is the prep list''s sentence, '
     'and repeating it here is the defect this page was rebuilt to remove.',
     NULL, now(), 'system (seed)'),

    -- ── The timeline's two sides ─────────────────────────────────────────────
    ('rehearsal_timeline_side_theirs_label', 'THEY SAY', 'text', 'THEY SAY',
     NULL, NULL,
     'Marks a timeline row as one of THEIR statements. Paired with a neutral '
     'chip; weight and colour together, never colour alone, so the interleaving '
     'survives a monochrome print of the rehearsal packet.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_timeline_side_ours_label', 'OUR ANSWER', 'text', 'OUR ANSWER',
     NULL, NULL,
     'Marks a timeline row as something WE said. Same wording as the label above '
     'a paired answer, one zoom level out.',
     NULL, now(), 'system (seed)'),

    -- ── Who wrote the two sentences (C2) ─────────────────────────────────────
    ('rehearsal_what_attribution_template', 'Written by {who} · {when}', 'text',
     'Written by {who} · {when}',
     NULL, NULL,
     'The authorship line under "What this is". Both placeholders must stay: a '
     'line naming neither the author nor the date attributes nothing. Provenance '
     'of HUMAN authorship is not excluded content — it is the opposite, and it is '
     'what tells a reader this sentence was written rather than derived.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_accusation_attribution_template',
     'Written in plain words by {who} · {when}', 'text',
     'Written in plain words by {who} · {when}',
     NULL, NULL,
     'The authorship line under the accusation. Says "in plain words" because '
     'that is precisely what distinguishes this sentence from the verbatim quotes '
     'beneath it — a reader must never mistake our summary for their words.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_attribution_unknown_notice',
     'Author not recorded — written before authorship was kept.', 'text',
     'Author not recorded — written before authorship was kept.',
     NULL, NULL,
     'Shown in place of the authorship line when the sentence predates the '
     'columns that record it. A named absence, never an invented name and never '
     'a blank: a blank would read as a rendering fault.',
     NULL, now(), 'system (seed)'),

    -- ── The way back, and the way around ─────────────────────────────────────
    ('rehearsal_scenario_page_label', 'Scenario page', 'text', 'Scenario page',
     NULL, NULL,
     'The header control that leaves rehearsal for the scenario''s working page. '
     'Explicit and named, because the browser''s Back button must never be the '
     'only way out.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_crumb_trial_prep_label', 'Trial Prep', 'text', 'Trial Prep',
     NULL, NULL,
     'The breadcrumb link back to Trial Prep. Was a literal in the page file '
     'until task 2.11 C.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_go_to_row_label', 'go to row', 'text', 'go to row',
     NULL, NULL,
     'The link on a prep-list entry that opens the instance row it names. The '
     'prep list is what a human acts on today, so every entry carries the way to '
     'the row rather than leaving them to find it.',
     NULL, now(), 'system (seed)'),

    -- ── The accusation block's list furniture ────────────────────────────────
    ('rehearsal_prep_list_heading', 'What still needs preparing', 'text',
     'What still needs preparing',
     NULL, NULL,
     'Heads the gap list under the instances. Named as WORK rather than as a '
     'problem: every line under it is something a human can do today.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_row_open_hint', 'Click a row to open it.', 'text',
     'Click a row to open it.',
     NULL, NULL,
     'Follows the count line. The rows collapse once a scenario has more '
     'instances than the expand cap, and a feature discoverable only by trying it '
     'is a feature nobody finds — features disclose themselves.',
     NULL, now(), 'system (seed)'),

    -- ── The two authoring sections, in rehearsal''s voice ────────────────────
    ('rehearsal_add_point_label', '+ Add talking point', 'text',
     '+ Add talking point',
     NULL, NULL,
     'Adds one of Marie''s talking points from the rehearsal page. The scenario '
     'page has its own row for the same control (talking_points_add_label): the '
     'two surfaces speak in different voices and change for different reasons.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_add_watch_label', '+ Add watch item', 'text', '+ Add watch item',
     NULL, NULL,
     'Adds something to watch for, from the rehearsal page. "Item" rather than '
     'the scenario page''s "note": this surface never says "watch-list", which is '
     'curation vocabulary.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_point_no_exhibit_notice', 'No exhibit paired yet', 'text',
     'No exhibit paired yet',
     NULL, NULL,
     'Shown under a talking point with no exhibit behind it. Every point says '
     'this today: the pairing editor is tracker task 3.9, and an absence named is '
     'better than a control that cannot do anything.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_points_authoring_note',
     'Authored by you and Marie — the system never rewrites these. Spellcheck underlines while you type; what you save is what is stored.',
     'text',
     'Authored by you and Marie — the system never rewrites these. Spellcheck underlines while you type; what you save is what is stored.',
     NULL, NULL,
     'Sits beside the add control on the rehearsal page. Says two things a human '
     'needs to trust the box: nothing here is generated, and the browser''s '
     'spellcheck advises without changing what is stored.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_what_placeholder',
     'The fight over whether Marie blocked an amicable division of her father''s property.',
     'text',
     'The fight over whether Marie blocked an amicable division of her father''s property.',
     NULL, NULL,
     'The empty-box hint when writing "What this is". One sentence naming the '
     'FIGHT, not our answer to it — an example, never stored as a value.',
     NULL, now(), 'system (seed)'),

    -- ── The fifth section state (the mockup folds "What this is") ────────────
    ('rehearsal_what_default_state', 'open', 'text', 'open',
     NULL, NULL,
     'Whether the "What this is" section starts open. Either "open" or '
     '"collapsed". It folds like the others as of the signed mockup of '
     '2026-08-06; task 2.11 B2 had it fixed open.',
     NULL, now(), 'system (seed)'),

    -- ── The one new number ───────────────────────────────────────────────────
    ('rehearsal_instance_rows_expand_max', '3', 'count', '3',
     1, NULL,
     'How many instances a scenario may have before its rows arrive COMPACT '
     'instead of expanded. At or under this number every row is open on arrival; '
     'above it every row is a one-line summary that opens on click. The list is '
     'never paginated at any size — a witness scrolls, and a page boundary in the '
     'middle of "he said it five times" breaks the one thing the block is for. '
     'Minimum 1: zero would mean a single instance arrives folded, which hides '
     'the only thing on the page.',
     NULL, now(), 'system (seed)'),

    -- ── The scenario page''s talking-points section (ruling C4b) ─────────────
    -- Ten literals lived in TalkingPointsSection.tsx. The component is now shared
    -- with the rehearsal page, whose law is that every word is a stored row, so
    -- they move here rather than travelling into a surface that forbids them.
    ('talking_points_section_heading', 'Marie''s talking points', 'text',
     'Marie''s talking points',
     NULL, NULL,
     'Heads the talking-points section on the scenario working page.',
     NULL, now(), 'system (seed)'),

    ('talking_points_section_meta_template', 'her own words · up to {cap}',
     'text', 'her own words · up to {cap}',
     NULL, NULL,
     'The line beside the talking-points heading. {cap} must stay: it is the '
     'stored talking_points_cap, and a hardcoded number would show the wrong '
     'limit the day the cap changes.',
     NULL, now(), 'system (seed)'),

    ('talking_points_empty_notice',
     'No talking points yet — these are the sentences Marie says when she is pressed on this scenario.',
     'text',
     'No talking points yet — these are the sentences Marie says when she is pressed on this scenario.',
     NULL, NULL,
     'Shown when nobody has written a talking point. Names the absence AND what '
     'the block is for, because the person reading it is the person who fills it.',
     NULL, now(), 'system (seed)'),

    ('talking_points_no_exhibit_notice', 'No exhibit paired yet', 'text',
     'No exhibit paired yet',
     NULL, NULL,
     'Shown under a talking point with no exhibit behind it, on the scenario '
     'page. Task 3.9 brings the pairing editor.',
     NULL, now(), 'system (seed)'),

    ('talking_points_add_label', '+ Add talking point', 'text',
     '+ Add talking point',
     NULL, NULL,
     'Adds a talking point on the scenario working page.',
     NULL, now(), 'system (seed)'),

    ('talking_points_edit_label', 'Edit', 'text', 'Edit',
     NULL, NULL,
     'Opens ONE talking point for editing. Per-row since task 2.11 C: the whole '
     'list used to go into edit mode together, which rewrote every row to change '
     'one and destroyed each row''s authorship in the process.',
     NULL, now(), 'system (seed)'),

    ('talking_points_save_label', 'Save', 'text', 'Save',
     NULL, NULL,
     'Stores an edited talking point. Promises nothing about navigation — the '
     '2.12 lesson about a button describing behaviour it does not have.',
     NULL, now(), 'system (seed)'),

    ('talking_points_saving_label', 'Saving…', 'text', 'Saving…',
     NULL, NULL,
     'Replaces the save control while a write is in flight, so a human can tell '
     'a slow save from an ignored click.',
     NULL, now(), 'system (seed)'),

    ('talking_points_cancel_label', 'Cancel', 'text', 'Cancel',
     NULL, NULL,
     'Abandons an edit to a talking point without storing it.',
     NULL, now(), 'system (seed)'),

    ('talking_points_cap_reached_notice',
     'That is already {cap} points — the most a witness can hold.', 'text',
     'That is already {cap} points — the most a witness can hold.',
     NULL, NULL,
     'Explains the disabled add control. {cap} must stay: it is the stored cap. '
     'A control that refuses without saying why is a control that reads as broken.',
     NULL, now(), 'system (seed)'),

    ('talking_points_field_label_template', 'Talking point {n}', 'text',
     'Talking point {n}',
     NULL, NULL,
     'Names each editing box for a screen reader. {n} must stay: without it every '
     'box in the list announces itself identically.',
     NULL, now(), 'system (seed)'),

    ('talking_points_authoring_note',
     'Authored by you and Marie — the system never rewrites these.', 'text',
     'Authored by you and Marie — the system never rewrites these.',
     NULL, NULL,
     'Sits beside the add control on the scenario page. The rehearsal page has a '
     'longer row of its own that also explains spellcheck.',
     NULL, now(), 'system (seed)'),

    ('talking_points_save_failed_notice',
     'That talking point did not save. Your words are still on screen — try again.',
     'text',
     'That talking point did not save. Your words are still on screen — try again.',
     NULL, NULL,
     'The fallback when a save fails and the failure carries no message of its '
     'own. Says the draft survived, because a human who has just typed a sentence '
     'needs to know it was not thrown away.',
     NULL, now(), 'system (seed)'),

    -- ── The watch-list section''s words ──────────────────────────────────────
    -- Moved for the same reason as the block above: the component is shared with
    -- a surface that forbids literals.
    ('watch_list_section_heading', 'Watch-list', 'text', 'Watch-list',
     NULL, NULL,
     'Heads the watch-list section on the scenario working page. The rehearsal '
     'page uses rehearsal_block_watch_heading instead — it never says "list".',
     NULL, now(), 'system (seed)'),

    ('watch_list_section_meta', 'what the other side will wave around', 'text',
     'what the other side will wave around',
     NULL, NULL,
     'The line beside the watch-list heading.',
     NULL, now(), 'system (seed)'),

    ('watch_list_field_label', 'Flag something to watch for', 'text',
     'Flag something to watch for',
     NULL, NULL,
     'Labels the box for a new watch item.',
     NULL, now(), 'system (seed)'),

    ('watch_list_add_label', '+ Add watch-list note', 'text',
     '+ Add watch-list note',
     NULL, NULL,
     'Opens the box for a new watch item on the scenario working page.',
     NULL, now(), 'system (seed)'),

    ('watch_list_save_label', 'Save', 'text', 'Save',
     NULL, NULL,
     'Stores a new or edited watch item.',
     NULL, now(), 'system (seed)'),

    ('watch_list_edit_label', 'Edit', 'text', 'Edit',
     NULL, NULL,
     'Opens ONE watch item for editing. New in task 2.11 C — until then a wrong '
     'word could only be fixed by removing the item and writing it again, which '
     'threw away who wrote it and when.',
     NULL, now(), 'system (seed)'),

    ('watch_list_cancel_label', 'Cancel', 'text', 'Cancel',
     NULL, NULL,
     'Abandons a new or edited watch item without storing it.',
     NULL, now(), 'system (seed)'),

    ('watch_list_remove_label', 'Remove', 'text', 'Remove',
     NULL, NULL,
     'Deletes a watch item. Distinct from editing one: removing says the thing is '
     'not worth watching for, editing says it was written badly.',
     NULL, now(), 'system (seed)'),

    ('watch_list_edited_suffix', 'edited since written', 'text',
     'edited since written',
     NULL, NULL,
     'Follows the authorship tag on a watch item whose text has changed since it '
     'was first written. The provenance stays honest through an edit.',
     NULL, now(), 'system (seed)'),

    ('watch_list_save_failed_notice',
     'That watch item did not save. Your words are still on screen — try again.',
     'text',
     'That watch item did not save. Your words are still on screen — try again.',
     NULL, NULL,
     'The fallback when a watch-item write fails and carries no message of its own.',
     NULL, now(), 'system (seed)')

ON CONFLICT (key) DO NOTHING;
