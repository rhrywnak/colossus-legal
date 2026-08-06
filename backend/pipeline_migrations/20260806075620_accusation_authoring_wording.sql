-- accusation_authoring_wording: every word the accusation section puts on screen
--
-- Created: 2026-08-06 07:56:20
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- Task 2.11 B1, the third and last migration of the storage arc. The first two
-- gave the two human judgments a place to live; this one gives the surface that
-- records them its words.
--
-- ## Why twenty-five rows and not one literal in a component
--
-- Roman's ruling of 2026-08-04, and REHEARSAL_VIEW_DESIGN_v2 restates it as law
-- for this page: "every heading, label, gap message and the Always card text on
-- this page is served from the settings store with a plain description. No
-- literal user-facing string in code." A label compiled into a `.tsx` is the same
-- defect as a compiled-in threshold — it cannot be changed without a rebuild, and
-- nobody but a programmer can change it at all.
--
-- ## The gap messages are wording rows for a reason beyond the law
--
-- The honest-gap law says every absence renders as a NAMED gap. What a gap is
-- named is the whole substance of that law: "NO ANSWER PREPARED — C-14" is a prep
-- list, and "missing" is a shrug. Roman is the one who knows which words make him
-- act, so the words are his to edit without asking for a build.
--
-- ## The three templates that must keep their placeholders
--
-- `accusation_count_template`, the three gap messages, the no-instances notice
-- and the failure template all carry `{…}` tokens, and `wording::
-- REQUIRED_PLACEHOLDERS` refuses an edit that drops one. The reason is the same
-- one 2.10 wrote down: "Said  times, in  documents." is a grammatical sentence
-- with the fact removed, and nothing downstream could tell.
--
-- ON CONFLICT DO NOTHING keeps this re-runnable and never stamps over a value a
-- human has edited. The consequence, learned in 2.12: editing the VALUES list of
-- an APPLIED migration changes nothing on an environment that already ran it — a
-- correction has to be its own later UPDATE, guarded on the old text.
--
-- FORWARD-ONLY: no down migration.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('accusation_section_heading',
     'The accusation, and every time they made it', 'text',
     'The accusation, and every time they made it',
     NULL, NULL,
     'The heading over the accusation section of the working view. It names both '
     'halves of the section on purpose: the standing accusation in plain words, '
     'and the marked instances underneath it.',
     NULL, now(), 'system (seed)'),

    ('accusation_section_hint',
     'Write the accusation in plain words. Then mark each included fact that IS them making it, and pair the fact that answers it. Whatever is left unpaired is your prep list.',
     'text',
     'Write the accusation in plain words. Then mark each included fact that IS them making it, and pair the fact that answers it. Whatever is left unpaired is your prep list.',
     NULL, NULL,
     'The one line under the heading that tells a reader the marking and the '
     'pairing exist and what order to do them in. The surface disclosing itself — '
     'a feature nobody can see is a feature nobody uses.',
     NULL, now(), 'system (seed)'),

    ('accusation_text_label', 'In plain words', 'text', 'In plain words',
     NULL, NULL,
     'Labels the authored accusation sentence — what the marked instances are '
     'instances OF. Deliberately not "what they say": that phrasing is what let a '
     'verbatim quote from the record stand in for the standing accusation.',
     NULL, now(), 'system (seed)'),

    ('accusation_text_placeholder',
     'They say Marie was unreasonable and refused to divide the property.',
     'text',
     'They say Marie was unreasonable and refused to divide the property.',
     NULL, NULL,
     'The grey prompt inside the empty accusation box. A worked example of the '
     'register wanted: one sentence, plain words, stated as what THEY say.',
     NULL, now(), 'system (seed)'),

    ('accusation_text_missing_notice',
     'Nobody has written this accusation in plain words yet. Until somebody does, the rehearsal page names the gap rather than standing a quote in for it.',
     'text',
     'Nobody has written this accusation in plain words yet. Until somebody does, the rehearsal page names the gap rather than standing a quote in for it.',
     NULL, NULL,
     'Shown in place of the accusation when nobody has written one. The honest-gap '
     'law on the working view: the absence is named here as well, so the person '
     'who can fix it sees it where they can fix it.',
     NULL, now(), 'system (seed)'),

    ('accusation_text_edit_label', 'Edit', 'text', 'Edit',
     NULL, NULL,
     'Opens the accusation sentence for editing.',
     NULL, now(), 'system (seed)'),

    ('accusation_text_save_label', 'Save', 'text', 'Save',
     NULL, NULL,
     'Stores the accusation sentence. Deliberately promises nothing about '
     'navigation — the 2.12 lesson about a button describing behaviour it does '
     'not have.',
     NULL, now(), 'system (seed)'),

    ('accusation_text_clear_label', 'Withdraw it', 'text', 'Withdraw it',
     NULL, NULL,
     'Clears the accusation sentence, returning the rehearsal block to its named '
     'gap. Withdrawing a sentence you no longer stand behind is a real act, so it '
     'has a real control rather than being done by saving an empty box.',
     NULL, now(), 'system (seed)'),

    ('accusation_text_cancel_label', 'Cancel', 'text', 'Cancel',
     NULL, NULL,
     'Abandons an edit to the accusation sentence without storing it.',
     NULL, now(), 'system (seed)'),

    ('accusation_count_template',
     'Said {times} times, in {documents} documents.', 'text',
     'Said {times} times, in {documents} documents.',
     NULL, NULL,
     'The count line, computed from the instances actually marked and the '
     'documents they actually sit in. Both placeholders must stay in the text: a '
     'count line missing its numbers reads as a claim with nothing behind it. The '
     'numbers never inflate — an instance whose statement has left the scenario is '
     'a gap below, not a tally mark here.',
     NULL, now(), 'system (seed)'),

    ('accusation_no_instances_notice',
     'No instances marked yet. {included} included facts are waiting here.',
     'text',
     'No instances marked yet. {included} included facts are waiting here.',
     NULL, NULL,
     'Shown when nobody has marked a single instance. {included} must stay in the '
     'text — "none marked" and "none marked, out of forty-six waiting" are '
     'different states of the same scenario, and the second is the one that says '
     'what to do next.',
     NULL, now(), 'system (seed)'),

    ('accusation_mark_label', 'Mark an instance', 'text', 'Mark an instance',
     NULL, NULL,
     'Opens the picker that marks one included fact as an instance of the '
     'accusation — a human saying "this statement IS them making it". The machine '
     'never marks one.',
     NULL, now(), 'system (seed)'),

    ('accusation_unmark_label', 'Not an instance', 'text', 'Not an instance',
     NULL, NULL,
     'Withdraws a marking. The fact stays in the scenario; what is withdrawn is '
     'the judgment that it is the accusation being made.',
     NULL, now(), 'system (seed)'),

    ('accusation_pair_label', 'Pair our answer', 'text', 'Pair our answer',
     NULL, NULL,
     'Opens the picker that pairs one included fact as what we said back to this '
     'instance. Human-only: asserting that one record item rebuts another is the '
     'contradiction judgment the requirement defers, and a page that guessed it '
     'would put words in a witness''s mouth.',
     NULL, now(), 'system (seed)'),

    ('accusation_repair_label', 'Change our answer', 'text', 'Change our answer',
     NULL, NULL,
     'Re-pairs an instance that already has an answer. The new answer replaces the '
     'old one — two stored answers to one accusation would leave the rehearsal '
     'page choosing between them with no basis for choosing.',
     NULL, now(), 'system (seed)'),

    ('accusation_unpair_label', 'Unpair', 'text', 'Unpair',
     NULL, NULL,
     'Removes the answer paired to an instance, returning it to the prep list. '
     'Unpairing is a real act and is recorded as one, not a side effect of '
     'something else.',
     NULL, now(), 'system (seed)'),

    ('accusation_answer_label', 'Our answer:', 'text', 'Our answer:',
     NULL, NULL,
     'Marks the paired answer beneath the instance it answers. "Our" is doing '
     'work: the block above is theirs, this one is ours, and the beta.371 defect '
     'was a block labelled this way that held neither.',
     NULL, now(), 'system (seed)'),

    ('accusation_picker_prompt',
     'Choose one of this scenario''s included facts.', 'text',
     'Choose one of this scenario''s included facts.',
     NULL, NULL,
     'The prompt over the picker, and a statement of its limit: only facts already '
     'ruled into this scenario can be marked or paired. Nothing on this surface '
     'reaches outside what a human has already put in.',
     NULL, now(), 'system (seed)'),

    ('accusation_picker_cancel_label', 'Never mind', 'text', 'Never mind',
     NULL, NULL,
     'Closes the picker without marking or pairing anything.',
     NULL, now(), 'system (seed)'),

    ('accusation_picker_no_match_notice',
     'No included fact matches what you typed.', 'text',
     'No included fact matches what you typed.',
     NULL, NULL,
     'Shown when the picker''s filter leaves nothing. Kept apart from the notice '
     'below because the remedy differs — this one means type something else, that '
     'one means there is nothing left to choose.',
     NULL, now(), 'system (seed)'),

    ('accusation_picker_empty_notice',
     'There is nothing left to choose — every included fact is already used here.',
     'text',
     'There is nothing left to choose — every included fact is already used here.',
     NULL, NULL,
     'Shown when the picker has no candidates at all, before any filtering. An '
     'empty list that says nothing reads as a broken control; this says which of '
     'the two empty states it is.',
     NULL, now(), 'system (seed)'),

    ('accusation_gaps_heading', 'What still needs preparing', 'text',
     'What still needs preparing',
     NULL, NULL,
     'The heading over the gap list. The design calls this list the single most '
     'useful thing on the page, so it is named as work rather than as an error '
     'condition.',
     NULL, now(), 'system (seed)'),

    ('accusation_no_gaps_notice',
     'Every instance marked here has an answer paired to it.', 'text',
     'Every instance marked here has an answer paired to it.',
     NULL, NULL,
     'Shown when the gap list is empty. Says what IS true rather than showing '
     'nothing — an empty list and a list that has not loaded look identical, and '
     'they are very different states.',
     NULL, now(), 'system (seed)'),

    ('accusation_gap_no_answer', 'NO ANSWER PREPARED — {code}', 'text',
     'NO ANSWER PREPARED — {code}',
     NULL, NULL,
     'The prep list''s own line: an instance nobody has answered. {code} must stay '
     'in the text, because a list of forty-six facts that look alike is useless '
     'without the handle that names which one. Loud on purpose — the design says '
     'this gap is the most useful thing on the page.',
     NULL, now(), 'system (seed)'),

    ('accusation_gap_accusation_removed',
     '{code} has an answer paired to it but is no longer in this scenario. The pairing is kept — nothing a human decided disappears quietly.',
     'text',
     '{code} has an answer paired to it but is no longer in this scenario. The pairing is kept — nothing a human decided disappears quietly.',
     NULL, NULL,
     'The Remove law, said out loud: when the accusation side of a pairing leaves '
     'the scenario the row is KEPT and shown here. A pairing that vanished '
     'silently is worse than a visible broken one. {code} names which.',
     NULL, now(), 'system (seed)'),

    ('accusation_gap_answer_removed',
     'The answer paired to {code} is no longer in this scenario. The pairing is kept, and it needs a new answer.',
     'text',
     'The answer paired to {code} is no longer in this scenario. The pairing is kept, and it needs a new answer.',
     NULL, NULL,
     'The other half of the Remove law: the ANSWER left rather than the '
     'accusation. Kept apart from the line above because the remedy differs — this '
     'one needs a new answer, that one needs the statement back or the pairing '
     'withdrawn. {code} names the instance.',
     NULL, now(), 'system (seed)'),

    ('accusation_save_failed_template',
     'That did not save: {detail} Reload the page to see what is actually stored.',
     'text',
     'That did not save: {detail} Reload the page to see what is actually stored.',
     NULL, NULL,
     'Shown when a marking, a pairing or the accusation sentence could not be '
     'written. {detail} must stay in the text — it is the failure''s own words, '
     'the one thing that exists only in the browser, and without it the human is '
     'told something went wrong and nothing about what.',
     NULL, now(), 'system (seed)')

ON CONFLICT (key) DO NOTHING;
