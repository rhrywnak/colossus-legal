-- create_evidence_allegation_links: a human's link from a statement to the
-- accusation it bears on, and which way it cuts
--
-- Created: 2026-08-04 13:27:30
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- Task 2.10 (LINK_TO_ACCUSATION_DESIGN, signed 2026-08-04).
--
-- ## Why this table exists
--
-- Measured on DEV, 2026-08-04: scenario S-2's candidate pool is 148 items. Only
-- 44 carry an Evidence→Allegation edge from the extraction; 104 carry none, and
-- 94 of those have never been ruled on at all. §7.5 makes an unlinked card
-- defer-only by law — there is nothing for it to support or dispute — so two
-- thirds of the pool cannot be ruled. A row here is a human supplying the link
-- the extraction never made, which is what makes those cards rulable.
--
-- ## Domain note: the scope is CASE-WIDE, and the missing column says so
--
-- There is deliberately NO `scenario_id`. A statement that bears on ¶41 bears on
-- ¶41 in every scenario, exactly as the machine's own graph edges do — those are
-- properties of the statement and the complaint, not of the page a human happened
-- to be standing on. This is the same ruling that made `evidence_summary_overrides`
-- case-wide (task 1.7F, ruling R1), for the same reason: a per-scenario key is what
-- would let one C-code mean two different things depending on where it was opened.
--
-- The consequence is visible to the human before they commit — the link panel
-- carries the `link_scope_notice` wording seeded below, so nobody discovers a
-- case-wide effect after the fact.
--
-- ## Why the cut is REQUIRED
--
-- "It supports us" / "They'll use it against us". A link with no cut would feed a
-- readiness verdict that cannot tell ammunition from hazard, so the write path
-- refuses one and the column is NOT NULL. That is the design's validation rule,
-- which the research (coding-panel practice) names as the thing that keeps a fast
-- review honest.
--
-- ## Why there is no foreign key into the graph
--
-- `graph_node_id` is a CONTENT HASH minted by extraction, not a stable identity —
-- the same reason `scenario_fact_refs`, `scenario_ruling_anchors` and
-- `evidence_summary_overrides` all carry it unconstrained. Postgres cannot
-- reference a Neo4j node in any case. A re-extraction can therefore orphan a row,
-- exactly as it orphaned 26 rulings on 2026-07-24; task 2.5 re-attaches by anchor,
-- and an orphaned link is inert until it does — it is never applied to the wrong
-- statement, because the key it fails to match is the key it is looked up by.
--
-- `allegation_id` is unconstrained for the same reason: the Allegation node lives
-- in Neo4j too.
--
-- ## No NUMERIC columns anywhere in this migration
--
-- beta.364 died at boot decoding a NUMERIC into an `Option<f64>` with no
-- `rust_decimal` in the tree. Every column below is TEXT, UUID or TIMESTAMPTZ —
-- all three already decoded elsewhere in this codebase — and the way that defect
-- stays fixed is by not reintroducing the type.
--
-- ## v2 §8: BOTH tables are HUMAN-AUTHORED CONTENT
--
-- Re-gathering never edits human content. A scan overwriting a human's link would
-- restore the very silence the human just repaired, and would do it invisibly. So
-- both tables join `HUMAN_AUTHORED_TABLES` and both scan-path invariant tests
-- cover them (`scan_and_merge_paths_write_only_their_own_tables`,
-- `no_scan_path_mentions_a_human_authored_table`).

-- ─── 1. The links ──────────────────────────────────────────────────────────────

CREATE TABLE evidence_allegation_links (
    -- The statement. TEXT because the graph id is a content-hash string, matching
    -- `scenario_fact_refs.graph_node_id`. No FK — see the header.
    graph_node_id TEXT        NOT NULL,

    -- The accusation it bears on. Also a graph id, also unconstrained.
    allegation_id TEXT        NOT NULL,

    -- 'supports' | 'against'. Deliberately NOT a CHECK constraint, matching
    -- `scenario_fact_refs.status` and `scenario_ruling_anchors.ruled_status`: the
    -- vocabulary lives in code (`domain::link_cut::LinkCut`) so it can widen
    -- without a migration, and an unknown token is refused LOUDLY at the read
    -- boundary — which is a stronger guarantee than a CHECK, because a CHECK
    -- cannot tell you the PARSER is missing.
    cut           TEXT        NOT NULL,

    -- The authenticated username who linked it. NOT NULL for the same reason as
    -- `scenario_ruling_anchors.ruled_by`: an unattributed link is not a record,
    -- and this link is what makes a card rulable.
    authored_by   TEXT        NOT NULL,

    -- Bound from Rust `Utc::now()` rather than a DB default, matching the house
    -- pattern (`scenario_human_facts`, `evidence_summary_overrides`): the
    -- application owns the timestamp, so the row and the log line agree.
    --
    -- `created_at` survives a re-cut and `updated_at` moves, so "linked in June"
    -- and "re-cut this morning" stay distinguishable.
    created_at    TIMESTAMPTZ NOT NULL,
    updated_at    TIMESTAMPTZ NOT NULL,

    -- ONE ROW PER (statement, accusation). A statement bearing on three
    -- accusations is three rows, which is the design's storage note stated as a
    -- constraint: the composite key is what makes "link the same pair again with a
    -- different cut" an UPDATE rather than a second, contradictory row.
    --
    -- The card path reads every link for a whole pool in one `= ANY($1)` over the
    -- statement ids, so `graph_node_id` LEADS the key and that read is an index
    -- scan. The order was chosen, not stumbled into.
    CONSTRAINT evidence_allegation_links_pkey
        PRIMARY KEY (graph_node_id, allegation_id)
);

COMMENT ON TABLE evidence_allegation_links IS
    'A human''s link from a statement to an accusation it bears on, with which '
    'way it cuts. CASE-WIDE — no scenario column, because a statement bears on an '
    'accusation everywhere or nowhere. This is what makes a card the extraction '
    'never linked rulable at all (§7.5). Human-authored content under v2 §8: no '
    'scan path may write it.';

COMMENT ON COLUMN evidence_allegation_links.cut IS
    'supports | against — which way the statement runs for our side. Required: an '
    'unlabelled link would feed a readiness verdict that cannot tell ammunition '
    'from hazard. Vocabulary owned by domain::link_cut::LinkCut, not by a CHECK.';

COMMENT ON COLUMN evidence_allegation_links.graph_node_id IS
    'The evidence node id (content hash). No foreign key — Postgres cannot '
    'reference a Neo4j node, and the id is ephemeral by law. A re-extraction '
    'orphans the row; task 2.5 re-attaches by anchor.';

-- ─── 2. The ledger ─────────────────────────────────────────────────────────────
--
-- ## Why a ledger and not just the table above
--
-- The link table holds only the CURRENT state. A link changes what a human is
-- permitted to rule, so it is a human decision of the same class as a ruling —
-- and the rulings ledger exists because "who decided this, and when" outlives the
-- decision itself. Unlinking, in particular, leaves NO row behind in the state
-- table: without this ledger, a link that was made and withdrawn would be
-- indistinguishable from one that was never made.
--
-- Append-only, no FK to the link table: the record of a human decision must
-- outlive the row it was about, exactly as in `app_setting_changes` and
-- `scenario_status_transitions`.
CREATE TABLE evidence_allegation_link_events (
    -- Surrogate key: this is an event log, so rows have no natural unique key —
    -- the SAME pair is linked, re-cut and unlinked repeatedly, and each act is its
    -- own row. Contrast the composite PK on the state table above, where one row
    -- per pair is exactly the invariant.
    event_id      UUID        PRIMARY KEY DEFAULT gen_random_uuid(),

    graph_node_id TEXT        NOT NULL,
    allegation_id TEXT        NOT NULL,

    -- 'link' | 'recut' | 'unlink'. Code-owned vocabulary
    -- (`domain::link_cut::LinkAction`), no CHECK — see the state table's note.
    action        TEXT        NOT NULL,

    -- The cut as it stood after the act. NULL on 'unlink' and only there: there is
    -- no cut to record when the link is being withdrawn, and storing the old one
    -- would make the ledger read as though the withdrawal had asserted something.
    cut           TEXT,

    -- Who, and when. NOT NULL: an unattributed change to what is rulable is the
    -- thing this table exists to make impossible.
    actor         TEXT        NOT NULL,
    at            TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_evidence_allegation_link_events_node
    ON evidence_allegation_link_events (graph_node_id, at DESC);

COMMENT ON TABLE evidence_allegation_link_events IS
    'Append-only history of every link, re-cut and unlink: what, which way, who, '
    'when. The link table holds only the current state, and an unlink leaves no '
    'row there at all — so without this, a link made and withdrawn would be '
    'indistinguishable from one never made. Human-authored under v2 §8.';

COMMENT ON COLUMN evidence_allegation_link_events.cut IS
    'The cut in force after the act. NULL on an unlink, and only there.';

-- ─── 3. The wording (v2 §2b, extended from numbers to text — Roman, 2026-08-04) ─
--
-- THE CONFIGURATION LAW, EXTENDED:
--
--   "Every heading, prompt, label, button and message this task introduces is
--    served from the settings store with a default and a plain-language
--    description, and is editable on the admin Settings page. A literal
--    user-facing string in this task's code is a defect of the same class as a
--    compiled-in threshold."
--
-- So this task ships zero user-facing literals. Twenty rows below carry every
-- word the link control puts on screen; one more carries how long the short list
-- is. `app_settings` needs no schema change to hold them — its `value_kind`
-- column is free TEXT precisely so that "adding a kind is a code change plus a
-- seed row, never a schema migration" (that migration's own header). The kind
-- added by this task is 'text', parsed by `domain::settings::parse_text`.
--
-- ## The two templates, and why the placeholders are validated on write
--
-- `link_summary_template` and `link_progress_template` carry `{placeholders}`
-- filled server-side. A well-meaning edit that dropped one would silently produce
-- "You linked this to  · ." — a sentence with the facts removed. So the write
-- path refuses a template missing any placeholder its key requires, naming them
-- (`domain::wording::REQUIRED_PLACEHOLDERS`). Composition stays server-side; the
-- browser renders finished sentences and concatenates nothing.
--
-- ## Why `link_show_all_label` takes a placeholder rather than naming a number
--
-- The design drafted "Show all 38". Measured on DEV the complaint holds 120
-- allegations. A literal count in a stored string would be wrong the moment a
-- document is re-processed, so the count is filled at render time from what the
-- graph actually holds.
--
-- ON CONFLICT DO NOTHING keeps the migration idempotent (the house precedent).
-- `updated_by` is 'system (seed)' so a never-edited row stays visibly
-- distinguishable from one a human has set. `consumed_by` is NULL on every row:
-- all twenty-one are read by live code the moment this ships.
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    -- ── The panel's own words ────────────────────────────────────────────────
    ('link_panel_intro',
     'This statement isn''t linked to anything they''ve accused you of. Link it and you can rule it.',
     'text',
     'This statement isn''t linked to anything they''ve accused you of. Link it and you can rule it.',
     NULL, NULL,
     'The sentence above the link control on a card the extraction never linked to '
     'an accusation. It says why the card cannot be ruled and what to do about it.',
     NULL, now(), 'system (seed)'),

    ('link_scope_notice',
     'An accusation link belongs to the statement, not to this scenario — linking it here links it wherever this item appears.',
     'text',
     'An accusation link belongs to the statement, not to this scenario — linking it here links it wherever this item appears.',
     NULL, NULL,
     'The warning that a link is case-wide, shown before anyone commits to one. A '
     'human should never discover a case-wide effect after the fact.',
     NULL, now(), 'system (seed)'),

    ('link_allegations_heading', 'What it bears on', 'text', 'What it bears on',
     NULL, NULL,
     'The heading over the list of accusations you tick.',
     NULL, now(), 'system (seed)'),

    ('link_show_all_label', 'Show all {count}', 'text', 'Show all {count}',
     NULL, NULL,
     'The button that opens the full complaint list. {count} is filled with how '
     'many accusations the case actually has, so it can never go stale.',
     NULL, now(), 'system (seed)'),

    ('link_filter_placeholder', 'Type to filter', 'text', 'Type to filter',
     NULL, NULL,
     'The grey prompt inside the box that narrows the full accusation list.',
     NULL, now(), 'system (seed)'),

    ('link_no_match_notice', 'No accusation matches that.', 'text',
     'No accusation matches that.',
     NULL, NULL,
     'Shown when the filter box leaves nothing. An empty list with no sentence '
     'reads as a broken screen.',
     NULL, now(), 'system (seed)'),

    ('link_cut_heading', 'How it cuts', 'text', 'How it cuts',
     NULL, NULL,
     'The heading over the two buttons that say which way the statement runs.',
     NULL, now(), 'system (seed)'),

    ('link_cut_supports_label', 'It supports us', 'text', 'It supports us',
     NULL, NULL,
     'The button for a statement that helps our side.',
     NULL, now(), 'system (seed)'),

    ('link_cut_against_label', 'They''ll use it against us', 'text',
     'They''ll use it against us',
     NULL, NULL,
     'The button for a statement the other side will wield.',
     NULL, now(), 'system (seed)'),

    ('link_cut_supports_phrase', 'it supports us', 'text', 'it supports us',
     NULL, NULL,
     'The same choice as it reads INSIDE a sentence — "You linked this to 41 · it '
     'supports us." Stored separately from the button label so nothing in code has '
     'to change the case of a word a human wrote.',
     NULL, now(), 'system (seed)'),

    ('link_cut_against_phrase', 'they''ll use it against us', 'text',
     'they''ll use it against us',
     NULL, NULL,
     'The mid-sentence form of the other choice.',
     NULL, now(), 'system (seed)'),

    ('link_save_label', 'Save and next', 'text', 'Save and next',
     NULL, NULL,
     'The button that saves the link and moves to the next stuck card.',
     NULL, now(), 'system (seed)'),

    ('link_cancel_label', 'Cancel', 'text', 'Cancel',
     NULL, NULL,
     'The button that closes the link control without saving.',
     NULL, now(), 'system (seed)'),

    ('link_unlink_label', 'Unlink', 'text', 'Unlink',
     NULL, NULL,
     'The one-click control that takes a link back.',
     NULL, now(), 'system (seed)'),

    -- ── Refusals ─────────────────────────────────────────────────────────────
    ('link_missing_cut_refusal',
     'Say which way this cuts before saving — a link with no cut can''t tell ammunition from hazard.',
     'text',
     'Say which way this cuts before saving — a link with no cut can''t tell ammunition from hazard.',
     NULL, NULL,
     'Shown when someone saves without choosing how the statement cuts. The '
     'backend refuses the same way, in these same words.',
     NULL, now(), 'system (seed)'),

    ('link_missing_allegation_refusal',
     'Tick at least one accusation before saving.', 'text',
     'Tick at least one accusation before saving.',
     NULL, NULL,
     'Shown when someone saves without ticking anything.',
     NULL, now(), 'system (seed)'),

    -- ── The two composed sentences ───────────────────────────────────────────
    ('link_summary_template', 'You linked this to {allegations} · {cut}.', 'text',
     'You linked this to {allegations} · {cut}.',
     NULL, NULL,
     'The sentence a card shows after a human links it. {allegations} is the list '
     'of accusations, {cut} is the mid-sentence phrase for the choice above. Both '
     'placeholders are required — an edit that drops one is refused.',
     NULL, now(), 'system (seed)'),

    ('link_progress_template', '{linked} of {total} linked.', 'text',
     '{linked} of {total} linked.',
     NULL, NULL,
     'The running count of stuck cards a human has linked. Both numbers are '
     'counted from the saved pool, never from this session''s clicks. Both '
     'placeholders are required.',
     NULL, now(), 'system (seed)'),

    ('link_empty_options_notice',
     'No accusations are stored for this case yet, so there is nothing to link to.',
     'text',
     'No accusations are stored for this case yet, so there is nothing to link to.',
     NULL, NULL,
     'Shown when the complaint has produced no accusations at all. Distinct from '
     'the filter''s empty state: one means "nothing matches what you typed", this '
     'means "there is nothing to match".',
     NULL, now(), 'system (seed)'),

    -- ── What the queue says after a link write ───────────────────────────────
    ('link_unlink_found_nothing',
     'That link was already gone — your view was out of date, and it has been refreshed.',
     'text',
     'That link was already gone — your view was out of date, and it has been refreshed.',
     NULL, NULL,
     'Shown when Unlink removed nothing because the pair was not linked after all. '
     'Distinct from a successful unlink, which says nothing: the two look identical '
     'on screen otherwise, and only one of them means the page was out of date.',
     NULL, now(), 'system (seed)'),

    ('link_save_failed_template',
     'That link did not save: {detail} The queue has been reloaded from the server, so what you see now is what is stored.',
     'text',
     'That link did not save: {detail} The queue has been reloaded from the server, so what you see now is what is stored.',
     NULL, NULL,
     'Shown when a link or unlink could not be written. {detail} is the reason the '
     'server or the network gave, and is required — without it the message says '
     'something went wrong and nothing about what.',
     NULL, now(), 'system (seed)'),

    -- ── The 1.7F fold-in (Roman, 2026-08-04) ─────────────────────────────────
    ('question_revert_label', 'Restore the system''s wording', 'text',
     'Restore the system''s wording',
     NULL, NULL,
     'The control that throws away a human''s corrected question and shows the '
     'machine''s again. Appears both inside the question editor and beside the '
     'author badge on the card, so reverting never requires opening an editor.',
     NULL, now(), 'system (seed)'),

    -- ── The one number ───────────────────────────────────────────────────────
    ('link_short_list_max', '8', 'count', '8', 1, NULL,
     'How many accusations the short list offers before "Show all". The short list '
     'is the scenario''s own anchor first, then the accusations its pool already '
     'touches most. Keeping it small is what makes the choice fast AND accurate; '
     'the full list is always one click away.',
     NULL, now(), 'system (seed)')
ON CONFLICT (key) DO NOTHING;
