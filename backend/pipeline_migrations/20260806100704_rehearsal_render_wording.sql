-- rehearsal_render_wording: every word the rehearsal page speaks (task 2.11 B2)
--
-- Created: 2026-08-06 10:07:04
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- REHEARSAL_VIEW_DESIGN_v2 (SIGNED 08-06) + REHEARSAL_B2_ADDENDUM_v1 (SIGNED
-- 08-06), §4: "every heading, label, gap message, and the Always card from
-- wording rows."
--
-- The shipped rehearsal page carried roughly eighteen literals in code — the
-- purpose line, all four block labels, the position sentence, both empty states,
-- and the Always card itself. Every one of them moves here. Roman edits this
-- page's language without a build; nobody else could edit it at all.
--
-- ## The Always card moves, and its ARGUMENT moves with it
--
-- `domain::human_authored::STANDING_CARD` argued in its own doc comment that the
-- card is witness-prep DOCTRINE rather than configuration — ABA Formal Opinion
-- 508's substance in four sentences — and therefore belonged in code. The signed
-- addendum overrules the conclusion, and the architect's ruling (2026-08-06)
-- says the argument is honored where it now belongs: in this row's DESCRIPTION,
-- so no future editor blanks it casually. The const is deleted; the reasoning is
-- below, in `rehearsal_always_lines`.
--
-- ## Four rows are STATE, not prose, and are parsed into a closed enum
--
-- The `rehearsal_*_default_state` rows hold `open` or `collapsed`. They are
-- `text` because the store has no boolean kind (ruled 2026-08-05), and they are
-- parsed ONCE on the server into an enum — the `fact_tier_background_default_state`
-- pattern — so a typo is a named refusal rather than a section that silently
-- flips shut on a page a witness is reading from.
--
-- ## One row is a NUMBER
--
-- `rehearsal_timeline_min_distinct_dates` is the conditional-timeline threshold
-- ruled 2026-08-06 (a block that renders at two or more distinct dates and shows
-- an honest-gap line otherwise). A `count` with a minimum of 2: a threshold of
-- one would draw a "timeline" from a single point, which is a dot.
--
-- ON CONFLICT DO NOTHING keeps this re-runnable and never stamps over a value a
-- human has edited. The consequence, learned in 2.12: editing an APPLIED
-- migration's VALUES list changes nothing on an environment that already ran it —
-- a correction has to be its own later UPDATE, guarded on the old text.
--
-- FORWARD-ONLY: no down migration.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    -- ── The page's own chrome ────────────────────────────────────────────────
    ('rehearsal_page_heading', 'Rehearsal', 'text', 'Rehearsal',
     NULL, NULL,
     'The heading at the top of the rehearsal page.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_purpose_line',
     'Your testimony-prep view — what they say, every time they said it, and what we say back. Only scenarios marked Ready appear here.',
     'text',
     'Your testimony-prep view — what they say, every time they said it, and what we say back. Only scenarios marked Ready appear here.',
     NULL, NULL,
     'One line saying what this view is FOR and what decides its contents. Roman '
     'opened the first version and could not tell what it was for: a slim surface '
     'with no self-description reads as a broken one, and the second half matters '
     'as much as the first — an empty view is otherwise indistinguishable from a '
     'failure.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_position_template', 'Scenario {n} of {total}', 'text',
     'Scenario {n} of {total}',
     NULL, NULL,
     'The position line. Both numbers must stay in the text: a rehearsal is worked '
     'through in order, and a reader who cannot see how far along they are cannot '
     'pace the session.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_previous_label', 'Back', 'text', 'Back',
     NULL, NULL,
     'Moves to the previous ready scenario.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_next_label', 'Next', 'text', 'Next',
     NULL, NULL,
     'Moves to the next ready scenario.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_nothing_ready_notice',
     'Nothing is ready to rehearse yet. A scenario appears here once someone switches it to Ready on its page.',
     'text',
     'Nothing is ready to rehearse yet. A scenario appears here once someone switches it to Ready on its page.',
     NULL, NULL,
     'Shown when no scenario in the case is marked Ready. A real state, not a '
     'failure — so it says what to do rather than reporting an error.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_not_ready_notice',
     '{code} is not ready to rehearse yet. A scenario appears here once someone switches it to Ready on its page.',
     'text',
     '{code} is not ready to rehearse yet. A scenario appears here once someone switches it to Ready on its page.',
     NULL, NULL,
     'Shown when somebody opens the rehearsal address of a scenario nobody has '
     'declared ready. NOT a "page not found": the address is right and the '
     'scenario simply is not ready, and saying so is the only honest answer. '
     '{code} must stay in the text so the reader knows which scenario is meant — '
     'it is the only thing this page may say about a scenario it is not showing.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_expand_all_label', 'Open everything', 'text', 'Open everything',
     NULL, NULL,
     'Opens every collapsible section at once.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_collapse_all_label', 'Fold everything', 'text', 'Fold everything',
     NULL, NULL,
     'Folds every collapsible section away. The Always card is never folded.',
     NULL, now(), 'system (seed)'),

    -- ── The seven block headings ─────────────────────────────────────────────
    ('rehearsal_block_what_heading', 'What this is', 'text', 'What this is',
     NULL, NULL,
     'The first block: one plain sentence on what this fight is about.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_block_accusation_heading',
     'The accusation, and every time they made it', 'text',
     'The accusation, and every time they made it',
     NULL, NULL,
     'The page''s central block. Names both halves: the standing accusation in '
     'plain words, and every marked instance of them making it.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_block_timeline_heading', 'The timeline', 'text', 'The timeline',
     NULL, NULL,
     'Both sides in date order — the 50,000-foot view of the accusation repeating '
     'and our answers to it.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_block_points_heading', 'Your points, in your words', 'text',
     'Your points, in your words',
     NULL, NULL,
     'Marie''s own talking points. "In your words" is doing work: these are themes '
     'she authored, never a script to recite — over-scripting is an ethics hazard '
     'and produces worse testimony.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_block_watch_heading', 'Watch for', 'text', 'Watch for',
     NULL, NULL,
     'What the other side will wave around, in plain words.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_always_heading', 'Always', 'text', 'Always',
     NULL, NULL,
     'The heading over the standing card — the one block on this page that never '
     'folds away.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_always_lines',
     'Tell the truth. · Answer only what''s asked. · "I don''t recall" is fine if it''s true. · Don''t guess.',
     'text',
     'Tell the truth. · Answer only what''s asked. · "I don''t recall" is fine if it''s true. · Don''t guess.',
     NULL, NULL,
     'The standing card, shown on every rehearsal screen and never collapsible. '
     'Separate the lines with " · ". THINK BEFORE EDITING THIS ROW: these four '
     'sentences are witness-preparation DOCTRINE, not house style — they are ABA '
     'Formal Opinion 508''s substance in four lines, and they are the reason this '
     'whole page is theme-level rather than a script. They lived in code until '
     '2026-08-06 for exactly that reason; they were moved here so one wording '
     'serves the page and the exports, not because they are a preference. An '
     'empty value is refused at startup.',
     NULL, now(), 'system (seed)'),

    -- ── The collapsed-section headers, which must stay honest ────────────────
    ('rehearsal_accusation_header_template', 'said {times} times · {gaps} gaps',
     'text', 'said {times} times · {gaps} gaps',
     NULL, NULL,
     'The accusation section''s header line, VISIBLE WHETHER THE SECTION IS OPEN '
     'OR FOLDED. Both numbers must stay in the text. This is the law that answers '
     'the known hazard of collapsible sections — content behind a fold gets missed '
     '— so a folded section still says how many gaps are waiting. A gap count is '
     'never hidden.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_timeline_header_template', '{entries} dated items', 'text',
     '{entries} dated items',
     NULL, NULL,
     'The timeline section''s header count, visible folded or open. {entries} must '
     'stay in the text.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_points_header_template', '{shown} of {cap}', 'text',
     '{shown} of {cap}',
     NULL, NULL,
     'The talking-points header, visible folded or open. Both numbers must stay: '
     '"2 of 3" says there is room for another, which "2" does not.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_watch_header_template', '{count} to watch for', 'text',
     '{count} to watch for',
     NULL, NULL,
     'The watch-for header count, visible folded or open. {count} must stay.',
     NULL, now(), 'system (seed)'),

    -- ── The named gaps ───────────────────────────────────────────────────────
    ('rehearsal_what_gap',
     'Nobody has written what this scenario is about yet.', 'text',
     'Nobody has written what this scenario is about yet.',
     NULL, NULL,
     'Shown in place of the first block when nobody has framed it. The page '
     'renders only what a human placed, and names every absence.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_accusation_text_gap',
     'Nobody has written the accusation in plain words yet.', 'text',
     'Nobody has written the accusation in plain words yet.',
     NULL, NULL,
     'Shown in place of the accusation when nobody has authored one. It is NEVER '
     'stood in for by a quote from the record: doing that promoted one piece of '
     'evidence into the summary of all of it, which is the defect this whole task '
     'exists to end.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_no_instances_notice',
     'Nobody has marked any instances of this accusation yet.', 'text',
     'Nobody has marked any instances of this accusation yet.',
     NULL, NULL,
     'Shown in place of the count line when nothing has been marked. Kept apart '
     'from the missing-accusation notice above because they are different states '
     'with different remedies: one means nobody has written the sentence, the '
     'other means nobody has yet said which statements ARE it. It counts nothing '
     'on purpose — "46 facts are waiting" is curation vocabulary, and there is '
     'nothing a witness can do with it.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_gap_no_answer',
     'NO ANSWER PREPARED — {who}, {when}, {where}', 'text',
     'NO ANSWER PREPARED — {who}, {when}, {where}',
     NULL, NULL,
     'An instance nobody has paired an answer to. THE PREP LIST — the design calls '
     'this the single most useful thing on the page, so it is loud on purpose. All '
     'three placeholders must stay: this surface never names a fact by an internal '
     'handle, so who said it, when, and where is the only way to find it.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_gap_accusation_removed',
     'An answer is paired to a statement that is no longer part of this scenario.',
     'text',
     'An answer is paired to a statement that is no longer part of this scenario.',
     NULL, NULL,
     'A human paired an answer, and the statement it answered has since left the '
     'scenario. The pairing is KEPT and shown rather than deleted — a decision '
     'that vanished silently is worse than a visible broken one.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_gap_answer_removed',
     'The answer to {who}, {when} is no longer part of this scenario.', 'text',
     'The answer to {who}, {when} is no longer part of this scenario.',
     NULL, NULL,
     'The other half of the same law: the ANSWER left rather than the statement. '
     'Kept apart because the remedy differs — this one needs a new answer. Both '
     'placeholders must stay so the reader knows which instance lost its answer.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_gap_instance_unavailable',
     'One statement marked as an instance could not be loaded from the record.',
     'text',
     'One statement marked as an instance could not be loaded from the record.',
     NULL, NULL,
     'A marked instance whose statement no longer resolves in the record store. '
     'Deliberately NOT worded as somebody having removed it: this is a system '
     'fault, not a human act, and blaming a human for it would also bury the '
     'signal the re-anchoring work needs. It does not count toward "said N times" '
     '— the page cannot claim what it cannot load — but it always appears here and '
     'in the header''s gap count.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_points_gap', 'No talking points yet.', 'text',
     'No talking points yet.',
     NULL, NULL,
     'Shown when the talking-points block is empty. A ready scenario with no points '
     'is a real state — readiness does not require them — so this is a statement, '
     'not a warning.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_watch_gap', 'Nothing flagged yet.', 'text', 'Nothing flagged yet.',
     NULL, NULL,
     'Shown when nothing is on the watch list. Deliberately not phrased as a '
     'warning: nothing flagged is a legitimate and often correct state.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_instance_when_gap', 'No date on this statement', 'text',
     'No date on this statement',
     NULL, NULL,
     'Shown in an instance''s date column when the record carries no date for that '
     'statement. Named rather than left blank, and never filled in from the '
     'document''s title — measured on this case, the document titles carry years '
     'that disagree with the statements inside them.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_instance_who_gap', 'Speaker not recorded', 'text',
     'Speaker not recorded',
     NULL, NULL,
     'Shown when the record does not say who made a statement. A blank would read '
     'as a rendering fault; this says the record is silent, which is a different '
     'and checkable thing.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_timeline_gap_template',
     'Not enough dated items to draw a timeline — {undated} of {total} carry no date.',
     'text',
     'Not enough dated items to draw a timeline — {undated} of {total} carry no date.',
     NULL, NULL,
     'Shown instead of the timeline when what has been placed cannot draw one. '
     'Both numbers must stay: the design promises the block will state how many '
     'items it is NOT showing, and a bare "not enough dates" hides the size of the '
     'gap. The threshold itself is rehearsal_timeline_min_distinct_dates.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_timeline_filter_prompt', 'Show', 'text', 'Show',
     NULL, NULL,
     'Labels the timeline''s person filter. The design keeps the timeline strictly '
     'chronological — its force is the repetition over time — and offers the '
     'filter so all of one person''s entries are one click away.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_timeline_filter_all_label', 'Everyone', 'text', 'Everyone',
     NULL, NULL,
     'The person filter''s unfiltered option.',
     NULL, now(), 'system (seed)'),

    -- ── Sources: the citation the research says she cannot rehearse without ──
    ('rehearsal_source_label_template', '{document}, p. {page}', 'text',
     '{document}, p. {page}',
     NULL, NULL,
     'Names where a statement was made. Both placeholders must stay. An examiner '
     'or a witness who cannot produce the source on the spot loses credibility, '
     'which is why this citation is on a witness surface that excludes every other '
     'piece of impeachment machinery.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_source_open_label', 'Open', 'text', 'Open',
     NULL, NULL,
     'Opens the source document at the page the statement is on, in one click.',
     NULL, now(), 'system (seed)'),

    -- ── Which sections start open ────────────────────────────────────────────
    ('rehearsal_accusation_default_state', 'open', 'text', 'open',
     NULL, NULL,
     'Whether the accusation section starts open. Either "open" or "collapsed" — '
     'any other value is refused at startup rather than guessed at. It starts open '
     'because it is the page.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_timeline_default_state', 'collapsed', 'text', 'collapsed',
     NULL, NULL,
     'Whether the timeline starts open. Either "open" or "collapsed". It starts '
     'folded because witness-prep doctrine is one topic at a time, and the timeline '
     'is the overview rather than the work.',
     NULL, now(), 'system (seed)'),

    ('rehearsal_points_default_state', 'open', 'text', 'open',
     NULL, NULL,
     'Whether the talking-points section starts open. Either "open" or "collapsed".',
     NULL, now(), 'system (seed)'),

    ('rehearsal_watch_default_state', 'open', 'text', 'open',
     NULL, NULL,
     'Whether the watch-for section starts open. Either "open" or "collapsed".',
     NULL, now(), 'system (seed)'),

    -- ── The one number ───────────────────────────────────────────────────────
    ('rehearsal_timeline_min_distinct_dates', '2', 'count', '2',
     2, NULL,
     'How many DISTINCT dates the placed items must carry before the timeline is '
     'drawn at all. Evaluated over what a human has actually placed — the marked '
     'instances and their paired answers — never over the whole included pool, so '
     'the page never promises a timeline the placed set cannot draw. Minimum 2: a '
     'threshold of one would draw a timeline from a single point, which is a dot.',
     NULL, now(), 'system (seed)')

ON CONFLICT (key) DO NOTHING;
