-- one_card_grammar_wording_and_settings: the words and the two thresholds one
-- evidence card speaks, wherever it appears
--
-- Created: 2026-08-09 12:15:31
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- ONE_CARD_GRAMMAR_DESIGN_v1 (approved 2026-08-09), built under
-- CC_TASK_ONE_CARD_GRAMMAR_PHASE_B_AUTHORIZATION_v1, ruling R6: the eighth
-- wording block, and every sentence this build introduces is a row.
--
-- ## Why this migration ships in the SAME COMMIT as the code
--
-- `domain::wording_card_grammar` declares these keys to the boot loader, and
-- `build_all_wording` REFUSES TO START when a declared key has no row. So a
-- commit carrying the declaration without this file is a commit that takes the
-- backend down at boot. They travel together; `wording_card_grammar_tests.rs`
-- reads this file off disk and fails if any declared key is missing from it
-- (Rule 21, the disk/code consistency pattern).
--
-- ## Two compiled-in strings die here (Standing Rule 2)
--
-- * 'System' — `SYSTEM_AUTHORSHIP_LABEL` in `services::scenario_human_links`.
--   It is the badge saying who wrote the QUESTION, and it sat directly under a
--   seven-line interrogatory where a reader takes it for the attribution of the
--   text above. Measured on the DEV graph 2026-08-09: nine STATED_BY actor
--   names, none of them 'System' — so the speaker chip never said it and could
--   not. The row below says what the badge actually means.
-- * 'extracted' — the `CardSpeaker.attribution` literal in
--   `services::scenario_card::build_speaker`. Required by the §3 sequencing
--   ruling until B0 reconciles one man from two strings, and a sentence a human
--   reads is configuration.
--
-- ## The two thresholds (§2b)
--
-- Roman changes how much of a question shows and how many element chips stand
-- before the "+N more" without a rebuild. Both are `count`, both bounded, and
-- both seeded at the design's initial values (the mockup shows K = 2).
--
-- ## Forward-only, idempotent, no down migration — the house rule for a seed.

-- ─── 1. The two thresholds ────────────────────────────────────────────────────

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('card_question_truncate_chars', '110',
     'count', '110', 1, 2000,
     'How much of a discovery question a card shows before it is ellipsized. '
     'The full text is one click away. Roman''s ruling: the answer is the '
     'evidence and it leads — a six-part interrogatory printed in full pushes '
     'the thing being ruled on off a 13-inch screen.',
     NULL, now(), 'system (seed)'),

    ('card_element_chips_visible_k', '2',
     'count', '2', 0, 50,
     'How many element chips stand on a card before the rest fold behind '
     '"+N more". Element chips COMPRESS, never vanish — they are the quick '
     '"what harm was done" indicator, so hiding them entirely would cost the '
     'card the one thing it says about damages.',
     NULL, now(), 'system (seed)')

ON CONFLICT (key) DO NOTHING;

-- ─── 2. The queue frame (Piece 1) ─────────────────────────────────────────────

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('card_filter_proposed_label', 'Proposed',
     'text', 'Proposed', NULL, NULL,
     'The filter chip for candidates the latest completed scan put forward and '
     'nobody has ruled yet.',
     NULL, now(), 'system (seed)'),

    ('card_filter_deferred_label', 'Deferred',
     'text', 'Deferred', NULL, NULL,
     'The filter chip for candidates parked with a stated reason. They stay in '
     'the queue.',
     NULL, now(), 'system (seed)'),

    ('card_filter_included_label', 'Included',
     'text', 'Included', NULL, NULL,
     'The filter chip for candidates confirmed as facts of this scenario.',
     NULL, now(), 'system (seed)'),

    ('card_filter_excluded_label', 'Excluded',
     'text', 'Excluded', NULL, NULL,
     'The filter chip for candidates set aside for this scenario. The evidence '
     'itself is untouched everywhere else.',
     NULL, now(), 'system (seed)'),

    ('card_filter_full_pool_label', 'Full pool',
     'text', 'Full pool', NULL, NULL,
     'The filter chip for everything ever gathered. It replaces "All", which '
     'read as a to-do list — what it names is the denominator, not the work.',
     NULL, now(), 'system (seed)'),

    ('card_filter_full_pool_explainer',
     'Everything the system ever gathered for this scenario, across all scans. The other filters are slices of this. Nothing in the full pool is lost when you filter.',
     'text',
     'Everything the system ever gathered for this scenario, across all scans. The other filters are slices of this. Nothing in the full pool is lost when you filter.',
     NULL, NULL,
     'The popup beside the Full pool chip. Roman''s addition: Marie and Chuck '
     'will not know the term, and a filter whose meaning a reader has to infer '
     'from its count is a filter they will not press.',
     NULL, now(), 'system (seed)'),

    ('card_filter_progress_template', '{ruled} of {total} {filter} ruled',
     'text', '{ruled} of {total} {filter} ruled', NULL, NULL,
     'The progress line under the filter chips. Must keep {ruled}, {total} and '
     '{filter}. It tracks the ACTIVE filter and never the pool: the line it '
     'replaces read "23 of 148 ruled" over a guilt-bar of 125 nobody owes, '
     'while rule-the-promising is the ratified triage model.',
     NULL, now(), 'system (seed)')

ON CONFLICT (key) DO NOTHING;

-- ─── 3. The card body (Piece 2) ───────────────────────────────────────────────

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('card_question_expand_label', 'show full question',
     'text', 'show full question', NULL, NULL,
     'The control that unfolds a collapsed question.',
     NULL, now(), 'system (seed)'),

    ('card_question_collapse_label', 'hide full question',
     'text', 'hide full question', NULL, NULL,
     'The control that folds an unfolded question away again.',
     NULL, now(), 'system (seed)'),

    ('card_question_machine_authorship_label', 'Question as transcribed from the document',
     'text', 'Question as transcribed from the document', NULL, NULL,
     'Who wrote the text of the QUESTION when no human has corrected it. It '
     'replaces the compiled-in "System", which sat under a seven-line '
     'interrogatory and was read as the speaker of the answer beneath it. This '
     'badge is not a speaker and must never be worded as one.',
     NULL, now(), 'system (seed)'),

    ('card_speaker_extracted_label', 'extracted',
     'text', 'extracted', NULL, NULL,
     'The provenance label beside a raw extracted speaker name. Required by the '
     'entity-identity sequencing ruling until the people registry reconciles one '
     'person from two strings; it is provenance, not clutter, and it retires '
     'then and not before.',
     NULL, now(), 'system (seed)'),

    ('card_speaker_absent_label', 'speaker not extracted',
     'text', 'speaker not extracted', NULL, NULL,
     'The speaker chip on an item whose source recorded nobody. Documentary '
     'evidence genuinely has no speaker; this says so rather than the card '
     'inventing a name or guessing at one.',
     NULL, now(), 'system (seed)'),

    ('card_elements_more_template', '+{count} more',
     'text', '+{count} more', NULL, NULL,
     'The control that reveals the element chips beyond the visible few. Must '
     'keep {count} — a fold that does not say how much is folded is a fold '
     'nobody opens.',
     NULL, now(), 'system (seed)'),

    ('card_elements_fewer_label', 'show fewer',
     'text', 'show fewer', NULL, NULL,
     'The control that folds the element chips away again.',
     NULL, now(), 'system (seed)'),

    ('card_context_show_label', 'Show context',
     'text', 'Show context', NULL, NULL,
     'The control that reveals the source text surrounding the quote. Behind a '
     'click because the card must never say the same thing twice.',
     NULL, now(), 'system (seed)'),

    ('card_context_hide_label', 'Hide context',
     'text', 'Hide context', NULL, NULL,
     'The control that hides the surrounding source text again.',
     NULL, now(), 'system (seed)'),

    ('card_scan_reason_label', 'Scan:',
     'text', 'Scan:', NULL, NULL,
     'Introduces the scan''s own justification on a card. The sentence has to '
     'name its speaker: a judge''s reason rendered unattributed reads as the '
     'record''s own position.',
     NULL, now(), 'system (seed)')

ON CONFLICT (key) DO NOTHING;

-- ─── 4. Linking (Pieces 4a, 4b) ───────────────────────────────────────────────

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('card_link_typeahead_placeholder', 'Type A-41, or a word from any allegation…',
     'text', 'Type A-41, or a word from any allegation…', NULL, NULL,
     'The type-ahead''s placeholder. It names the A- prefix on purpose: the '
     'prefix exists because a human has to be able to TYPE it, and nobody can '
     'type the paragraph glyph it replaced.',
     NULL, now(), 'system (seed)'),

    ('card_link_typeahead_intro',
     'This statement is not linked to anything they have accused you of. Link it and the ruling buttons wake up. A link belongs to the statement — it follows this item everywhere it appears.',
     'text',
     'This statement is not linked to anything they have accused you of. Link it and the ruling buttons wake up. A link belongs to the statement — it follows this item everywhere it appears.',
     NULL, NULL,
     'Said above the type-ahead on a locked card. It states the condition in '
     'plain words on the card''s own face and says what to do about it, '
     'including the case-wide scope, before anybody commits.',
     NULL, now(), 'system (seed)'),

    ('card_link_typeahead_no_match', 'No allegation matches what you typed.',
     'text', 'No allegation matches what you typed.', NULL, NULL,
     'Said when the type-ahead matches nothing. An empty list with no sentence '
     'reads as a control that broke.',
     NULL, now(), 'system (seed)'),

    ('card_link_woke_ruling_template',
     'Linked. {code} can be ruled now — the link follows this statement everywhere it appears.',
     'text',
     'Linked. {code} can be ruled now — the link follows this statement everywhere it appears.',
     NULL, NULL,
     'Said when a link wakes a locked card''s ruling buttons. Must keep {code}. '
     'Every action on this surface acknowledges itself — the beta.386 law.',
     NULL, now(), 'system (seed)')

ON CONFLICT (key) DO NOTHING;

-- ─── 5. The fact wrapper (Piece 5) ────────────────────────────────────────────

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('card_weight_picker_label', 'Weight',
     'text', 'Weight', NULL, NULL,
     'The label on the weight picker. The three tier NAMES are separate rows on '
     'the curation surface and are unchanged; this is the word that says what '
     'the control sets. It replaces a star that cycled through three states '
     'without naming any of them.',
     NULL, now(), 'system (seed)'),

    ('card_weight_changed_template', 'Weight set: {code} now reads {tier}.',
     'text', 'Weight set: {code} now reads {tier}.', NULL, NULL,
     'Said when a weight is stored. Must keep {code} and {tier} — on a list of '
     'forty-six facts that look alike, an acknowledgment that cannot name the '
     'card it is about is the same as no acknowledgment.',
     NULL, now(), 'system (seed)'),

    ('card_weight_undo_label', 'undo',
     'text', 'undo', NULL, NULL,
     'The control that takes a weight change back. Roman was moved to the '
     'background pile by a control he had signed off three days earlier; the '
     'way back has to be on screen at the moment it happens.',
     NULL, now(), 'system (seed)'),

    ('card_reset_order_label', 'Reset order',
     'text', 'Reset order', NULL, NULL,
     'The section-header control that forgets every human placement in this '
     'scenario. One control in the header replaces the per-card "Clear my '
     'order", which sat on forty-six cards to do a thing done once.',
     NULL, now(), 'system (seed)'),

    ('card_reset_order_confirm',
     'Forget where you have placed every fact in this scenario? The sequence is the argument, and this cannot be undone.',
     'text',
     'Forget where you have placed every fact in this scenario? The sequence is the argument, and this cannot be undone.',
     NULL, NULL,
     'The question asked before the order is reset. It is confirmed, unlike the '
     'per-card control it replaces, because it discards work across the whole '
     'list rather than on one card.',
     NULL, now(), 'system (seed)'),

    ('card_reset_order_confirm_yes', 'Reset the order',
     'text', 'Reset the order', NULL, NULL,
     'The button that confirms the reset. It names the act rather than saying '
     '"Yes", so a human reading only the buttons still knows what happens.',
     NULL, now(), 'system (seed)'),

    ('card_reset_order_confirm_cancel', 'Keep my order',
     'text', 'Keep my order', NULL, NULL,
     'The button that abandons the reset, named for what it preserves.',
     NULL, now(), 'system (seed)'),

    ('card_reset_order_done_template', 'Order reset — {count} facts returned to their natural order.',
     'text', 'Order reset — {count} facts returned to their natural order.',
     NULL, NULL,
     'Said when the reset has run. Must keep {count}: a reset that reports '
     'nothing looks identical to one that found nothing to reset.',
     NULL, now(), 'system (seed)'),

    ('card_reset_order_failed_template', 'The order could not be reset: {reason}',
     'text', 'The order could not be reset: {reason}', NULL, NULL,
     'Said when the reset could not be written. Must keep {reason}.',
     NULL, now(), 'system (seed)')

ON CONFLICT (key) DO NOTHING;

-- ─── 6. Chips as currency (Piece 7) ───────────────────────────────────────────

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('card_chip_filter_hint_template', 'Show only: {value}',
     'text', 'Show only: {value}', NULL, NULL,
     'The tooltip on a clickable chip. Must keep {value}. Chips are '
     'cross-reference currency: clicking one narrows the list to it.',
     NULL, now(), 'system (seed)'),

    ('card_chip_filter_clear_template', 'Showing only {value} — show everything',
     'text', 'Showing only {value} — show everything', NULL, NULL,
     'The control that drops a chip filter. Must keep {value}, because a list '
     'that is narrowed has to say what it is narrowed TO before it offers to '
     'widen again.',
     NULL, now(), 'system (seed)')

ON CONFLICT (key) DO NOTHING;
