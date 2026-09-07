-- fact_card_v2_tables_and_wording: the scenario fact card Marie can use, and
-- every word it puts on screen
--
-- Created: 2026-09-06 16:12:01
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- FACT_CARD_v2 §1.
--
-- ## Why this table exists
--
-- A scenario's facts are read live from the graph: the quote, the speaker, the
-- page. What the graph does NOT hold is the five things a witness actually needs
-- — what this card SAYS in three seconds, which of her points it backs, which
-- accusation it goes to, how the other side will use it, and her answer. Job B
-- drafted all five for 59 cards; nothing in this build could store them.
--
-- ## Domain note: scenario-scoped, and the key says so
--
-- Keyed `(scenario_id, graph_node_id)`, NOT by node alone. The same statement
-- carries a different sentence in two scenarios — under "Marie is obstructive"
-- the certified letter is proof she offered to cooperate; under "the $50,000" the
-- same words are background. A card keyed on the node would make one scenario's
-- answer appear on another's rehearsal page.
--
-- This is the opposite of the ruling `evidence_allegation_rulings` takes, and
-- deliberately: a ruling is about whether an item belongs under an accusation,
-- which is true everywhere or nowhere; a CARD is about how a fact is used in one
-- attack.
--
-- ## Why there is no foreign key into the graph
--
-- `graph_node_id` is a content hash minted by extraction — the same reason
-- `scenario_fact_refs`, `evidence_summary_overrides` and
-- `evidence_allegation_links` all carry it unconstrained. Postgres cannot
-- reference a Neo4j node. A re-extraction orphans the row; the row is INERT
-- rather than wrong, because it is looked up by the very key it fails to match.
--
-- `scenario_id` IS constrained: scenarios are ours.
--
-- ## No NUMERIC columns anywhere in this migration
--
-- beta.364 died at boot decoding a NUMERIC into an `Option<f64>` with no
-- `rust_decimal` in the tree. Every column below is TEXT, INTEGER, UUID, JSONB or
-- TIMESTAMPTZ.
--
-- ## v2 §8: BOTH tables are HUMAN-AUTHORED CONTENT
--
-- Even the machine-drafted rows. `authored_by = 'machine:job_b_v1'` is a DRAFT
-- awaiting a human, and the card renders it with a grey "draft" mark for exactly
-- that reason — but the row is in the human-authored surface because the moment
-- Chuck edits one field it becomes his words, and a scan that could overwrite it
-- would restore a draft over an edit. Both tables join `HUMAN_AUTHORED_TABLES`
-- and both scan-path invariant tests cover them.

-- ─── 1. The cards ──────────────────────────────────────────────────────────────

CREATE TABLE scenario_fact_cards (
    scenario_id   UUID        NOT NULL
                  REFERENCES scenarios(scenario_id) ON DELETE CASCADE,

    -- The statement this card is about. TEXT, unconstrained — see the header.
    graph_node_id TEXT        NOT NULL,

    -- What Marie says in three seconds. Job B was asked for ≤14 words; the limit
    -- is NOT a CHECK, because a human editing a title to fifteen words is making
    -- an editorial choice on their own surface and a database refusal is the
    -- wrong place to argue with it. The loader enforces it on the MACHINE's
    -- drafts, which is where it belongs.
    title         TEXT,

    -- Which of the scenario's talking points this card backs, by its 1-based
    -- position. NULL is the ordinary state and means "backs no point in
    -- particular" — the card renders an em dash, not a gap.
    --
    -- Deliberately the POSITION and not the `response_items.id`: the position is
    -- what is printed beside the point on both surfaces, and `set_talking_points`
    -- deletes and re-inserts every row, so an id here would be orphaned by an
    -- ordinary edit of the list.
    backs_position INTEGER,

    -- Which accusations this card goes to, and which way:
    -- `[{"allegation_id": "...", "stance": "supports"}]`, at most two.
    --
    -- JSONB rather than a child table because it is a small ordered list read and
    -- written whole, never queried into — the same call `authored_entities`
    -- makes for `item_data`. The cap and the stance vocabulary live in code
    -- (`domain::fact_card`), not in a CHECK, for the reason every sibling states:
    -- a CHECK cannot tell you the parser is missing.
    supports      JSONB,

    -- How the other side uses this fact against her.
    watch_out     TEXT,

    -- Her reply. The one field on this card written FOR a witness to say out
    -- loud, which is why the rehearsal page leads with it.
    answer        TEXT,

    -- ── Per-field authorship ─────────────────────────────────────────────────
    --
    -- §1 names one `authored_by`; §2 says "any field with authored_by machine
    -- shows a draft mark" and "PUT one field; authored_by becomes the user, draft
    -- mark goes". Those two only reconcile per FIELD: a row-level column would
    -- clear the draft mark on all five the moment one was edited, and the mockup
    -- shows a card with an edited Answer and a still-draft Watch out.
    --
    -- So the row carries `authored_by` as §1 asks — who last wrote ANY field, the
    -- provenance of the row as a whole — and each field carries its own, which is
    -- what the mark is read from. Five columns rather than a JSON map because a
    -- typed reader beats a parse, and because each one is asserted by name.
    title_authored_by          TEXT,
    backs_position_authored_by TEXT,
    supports_authored_by       TEXT,
    watch_out_authored_by      TEXT,
    answer_authored_by         TEXT,

    -- Who last wrote any field on this row, and when. `machine:job_b_v1` for a
    -- row the loader created and nobody has touched.
    authored_by   TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL,
    edited_at     TIMESTAMPTZ NOT NULL,

    -- ONE CARD PER (scenario, statement). Editing a field is an UPDATE; a second
    -- row for the same pair would be two answers to one question on a surface
    -- whose whole job is to give a witness one.
    CONSTRAINT scenario_fact_cards_pkey PRIMARY KEY (scenario_id, graph_node_id)
);

COMMENT ON TABLE scenario_fact_cards IS
    'The five sentences a witness needs about one statement in one scenario: the '
    'title, the point it backs, the accusations it goes to, how the other side '
    'uses it, and her answer. SCENARIO-SCOPED — the same statement reads '
    'differently in two attacks. Human-authored content under v2 §8, machine '
    'drafts included: a scan that overwrote one would restore a draft over an '
    'edit.';

COMMENT ON COLUMN scenario_fact_cards.supports IS
    'At most two [{allegation_id, stance}]. Cap and vocabulary owned by '
    'domain::fact_card, not by a CHECK — a CHECK cannot tell you the parser is '
    'missing.';

COMMENT ON COLUMN scenario_fact_cards.backs_position IS
    'The 1-based POSITION of the talking point, not its row id: the position is '
    'what is printed beside the point, and set_talking_points deletes and '
    're-inserts every item row on an ordinary edit.';

-- The rehearsal page reads "every card in this scenario" and the working page
-- reads the same set; the primary key already serves both because `scenario_id`
-- leads it. No second index is added for a read that does not exist yet.

-- ─── 2. The ledger ─────────────────────────────────────────────────────────────
--
-- ## Why a ledger for a table that is only ever updated
--
-- The card table holds the CURRENT sentence. The value of the record is that a
-- machine drafted it and a human replaced it — "Chuck rewrote the answer on the
-- 6th" is the fact a reader needs when the answer on screen is not the one they
-- remember, and an UPDATE destroys it. Append-only, no FK to the card table: the
-- record of a human decision outlives the row it was about, exactly as in
-- `evidence_allegation_link_events` and `scenario_status_transitions`.
CREATE TABLE scenario_fact_card_events (
    -- Surrogate key: this is an event log, so rows have no natural unique key —
    -- the same field is rewritten repeatedly and each act is its own row.
    event_id      UUID        PRIMARY KEY DEFAULT gen_random_uuid(),

    scenario_id   UUID        NOT NULL,
    graph_node_id TEXT        NOT NULL,

    -- Which field was written: `title` | `backs_position` | `supports` |
    -- `watch_out` | `answer`, or `card` when the loader wrote the whole row.
    -- Code-owned vocabulary (`domain::fact_card::CardField`), no CHECK.
    field         TEXT        NOT NULL,

    -- The value as it stood AFTER the act, rendered as text. NULL when the act
    -- cleared the field, and only then — an event with no value and no clear
    -- would be a record of nothing.
    --
    -- Domain note: this is the one place case CONTENT is copied rather than
    -- referenced. It has to be: the point of the ledger is what the sentence USED
    -- to say, and a pointer to the current row cannot answer that.
    value         TEXT,

    actor         TEXT        NOT NULL,
    at            TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_scenario_fact_card_events_card
    ON scenario_fact_card_events (scenario_id, graph_node_id, at DESC);

COMMENT ON TABLE scenario_fact_card_events IS
    'Append-only history of every field written on a fact card: which field, what '
    'it became, who, when. The card table holds only the current sentence, and an '
    'edit destroys the draft it replaced — this is what lets a reader see that a '
    'human rewrote it. Human-authored under v2 §8.';

-- ─── 3. The words (v2 §2b — every visible word is a stored row) ────────────────
--
-- THE CONFIGURATION LAW: a literal user-facing string in this task's code is a
-- defect of the same class as a compiled-in threshold. This card is read by
-- MARIE, on the stand, so the law binds hardest here.
--
-- Twenty-five rows below: twenty-three extend a NEW `fact_card_*` wording block
-- (`domain::wording_fact_card`), and two are parameters rather than sentences —
-- how many cards open before the rest collapse, and whose statements are ours.
--
-- ## Why a new block rather than more keys on `card_grammar`
--
-- The house test: which SURFACE speaks these, and does its vocabulary move
-- independently? `card_grammar` speaks to a CURATOR triaging a queue — filters,
-- fold controls, provenance badges. These speak to a WITNESS preparing to answer
-- a question under oath. They will move when Marie finds a word confusing, which
-- has nothing to do with what a filter chip says.
--
-- The templates carrying `{placeholders}` are registered in
-- `domain::wording_templates::REQUIRED_PLACEHOLDERS`, so a Settings-page edit
-- that dropped one is refused by name rather than silently shipping a sentence
-- with its facts removed.
--
-- ON CONFLICT DO NOTHING keeps the seed idempotent (house precedent).
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    -- ── The five row labels ──────────────────────────────────────────────────
    ('fact_card_proof_label', 'Proof', 'text', 'Proof',
     NULL, NULL,
     'The label on the row carrying the statement''s own words.',
     NULL, now(), 'system (seed)'),

    ('fact_card_backs_label', 'Backs', 'text', 'Backs',
     NULL, NULL,
     'The label on the row naming which of her talking points this card backs.',
     NULL, now(), 'system (seed)'),

    ('fact_card_supports_label', 'Supports', 'text', 'Supports',
     NULL, NULL,
     'The label on the row naming the accusation this card goes to. The same '
     'link the Proof Matrix reads.',
     NULL, now(), 'system (seed)'),

    ('fact_card_watch_out_label', 'Watch out', 'text', 'Watch out',
     NULL, NULL,
     'The label on the row saying how the other side will use this fact.',
     NULL, now(), 'system (seed)'),

    ('fact_card_answer_label', 'Answer', 'text', 'Answer',
     NULL, NULL,
     'The label on the row carrying her reply — the one sentence on the card '
     'written to be said out loud.',
     NULL, now(), 'system (seed)'),

    -- ── What an empty field says ─────────────────────────────────────────────
    ('fact_card_empty_value', '—', 'text', '—',
     NULL, NULL,
     'Printed in a row nobody has filled in. The row is NEVER hidden: a card '
     'missing its Answer is a card somebody still has to write, and hiding it '
     'would hide the work rather than the gap.',
     NULL, now(), 'system (seed)'),

    -- ── The marks ────────────────────────────────────────────────────────────
    ('fact_card_draft_mark', 'draft', 'text', 'draft',
     NULL, NULL,
     'The small grey mark on a field the machine wrote and no human has edited. '
     'It is what keeps a drafted deck from reading as a prepared one.',
     NULL, now(), 'system (seed)'),

    ('fact_card_context_label', 'Show context', 'text', 'Show context',
     NULL, NULL,
     'The control that opens the source text around a quote.',
     NULL, now(), 'system (seed)'),

    ('fact_card_context_hide_label', 'Hide context', 'text', 'Hide context',
     NULL, NULL,
     'The same control once the surrounding text is showing.',
     NULL, now(), 'system (seed)'),

    -- ── The composed lines ───────────────────────────────────────────────────
    ('fact_card_backs_template', 'Point {position} — {text}', 'text',
     'Point {position} — {text}',
     NULL, NULL,
     'How the Backs row reads. Both parts required: a position with no words '
     'sends the reader to another page, and words with no position cannot be '
     'matched to the point they back.',
     NULL, now(), 'system (seed)'),

    ('fact_card_supports_template', '{verb} {code} — {text}', 'text',
     '{verb} {code} — {text}',
     NULL, NULL,
     'How the Supports row reads — "Supports A-22 — CFS could have returned the '
     'money". All three required: the verb says which way it cuts, the code is '
     'what a lawyer cites, and the text is what the accusation actually says.',
     NULL, now(), 'system (seed)'),

    ('fact_card_stance_supports_verb', 'Supports', 'text', 'Supports',
     NULL, NULL,
     'The verb in the Supports row for a card that helps the accusation stand.',
     NULL, now(), 'system (seed)'),

    ('fact_card_stance_rebuts_verb', 'Disputes', 'text', 'Disputes',
     NULL, NULL,
     'The verb for a card whose stance is `rebuts` — it cuts against the '
     'accusation. The WORD and the TOKEN are deliberately different: the graph '
     'and Job B both say "rebuts", and a witness reads "Disputes". Never a '
     'negation of "Supports" — a reader skimming five cards must not have to '
     'parse "not supports".',
     NULL, now(), 'system (seed)'),

    ('fact_card_rfa_template', 'RFA {number} — {request} — {answer}', 'text',
     'RFA {number} — {request} — {answer}',
     NULL, NULL,
     'How an answer-only card reads in the Proof row. All three required: the '
     'request without the answer is not evidence of anything, and the answer '
     'without the request is a bare "Admitted."',
     NULL, now(), 'system (seed)'),

    ('fact_card_rfa_unnumbered_template', '{request} — {answer}', 'text',
     '{request} — {answer}',
     NULL, NULL,
     'The same line for a card whose request number cannot be derived from its '
     'own field. The number is omitted rather than guessed — a wrong RFA number '
     'in a court filing is worse than none.',
     NULL, now(), 'system (seed)'),

    ('fact_card_deck_template', '{shown} of {total} shown · {collapsed} more collapsed',
     'text', '{shown} of {total} shown · {collapsed} more collapsed',
     NULL, NULL,
     'The line above the deck. All three numbers required: a deck that says how '
     'many are shown without saying how many exist is the shape that let 148 '
     'borrowed cards read as a queue.',
     NULL, now(), 'system (seed)'),

    -- ── Editing ──────────────────────────────────────────────────────────────
    ('fact_card_edit_label', 'Edit', 'text', 'Edit',
     NULL, NULL,
     'The control that opens a field for editing.',
     NULL, now(), 'system (seed)'),

    ('fact_card_save_label', 'Save', 'text', 'Save',
     NULL, NULL,
     'The control that stores one edited field.',
     NULL, now(), 'system (seed)'),

    ('fact_card_cancel_label', 'Cancel', 'text', 'Cancel',
     NULL, NULL,
     'The control that closes an editor without storing.',
     NULL, now(), 'system (seed)'),

    ('fact_card_save_failed_template',
     'That edit did not save: {detail} Reopen the field — what you see now may not be what is stored.',
     'text',
     'That edit did not save: {detail} Reopen the field — what you see now may not be what is stored.',
     NULL, NULL,
     'Shown when a field edit could not be written. {detail} is required: the '
     'field closes on save, so without a reason the reader is left looking at a '
     'sentence the database does not hold.',
     NULL, now(), 'system (seed)'),

    -- ── The rehearsal page ───────────────────────────────────────────────────
    ('fact_card_no_answer_notice', 'No answer written yet.', 'text',
     'No answer written yet.',
     NULL, NULL,
     'Shown on the rehearsal page under an accusation card nobody has answered. '
     'The card is still shown: an unanswered accusation is the most important '
     'thing on that page, and hiding it would hide the work.',
     NULL, now(), 'system (seed)'),

    ('fact_card_accusation_heading', 'The Accusation', 'text', 'The Accusation',
     NULL, NULL,
     'The heading over the other side''s own statements, in date order, each '
     'with her answer.',
     NULL, now(), 'system (seed)'),

    ('fact_card_no_cards_notice',
     'No cards have been written for this scenario yet.', 'text',
     'No cards have been written for this scenario yet.',
     NULL, NULL,
     'Shown where a section would otherwise be blank. A blank section reads as a '
     'broken page; this says the work has not been done.',
     NULL, now(), 'system (seed)'),

    -- ── Whose statement it is (FACT_CARD_v2 §3) ─────────────────────────────
    --
    -- §3's Accusation section holds "cards whose speaker/author is the other side
    -- (Phillips, CFS, court)". Those are CASE-SPECIFIC NAMES, and Rule 2 is
    -- explicit that person aliases never compile in — so the stored list is the
    -- inverse: whose statements are OURS. Everything else is the other side or
    -- the record.
    --
    -- ## Why the inverse, and not a list of the opposition
    --
    -- Two reasons. It is far shorter — this case has one witness and a hundred
    -- opposing speakers, unnormalised ("George Phillips" and "George R. Phillips"
    -- are two rows in the graph). And it fails SAFE: a name nobody has listed
    -- reads as the other side, which puts an extra card in front of Marie rather
    -- than hiding one she has to answer.
    -- `value_kind` is `text`, like every other token list in this table
    -- (`matrix_tier_*_pairs`): the store's kinds describe how a value PARSES, and
    -- a comma-separated list parses as text. `token_list_of` splits it at the
    -- read boundary.
    ('rehearsal_our_side_speakers', 'Marie Awad', 'text', 'Marie Awad',
     NULL, NULL,
     'The speakers whose statements are OURS, comma-separated. Everything else — '
     'including a document with no recorded speaker — is treated as the other '
     'side or the court on the rehearsal page, and appears under The Accusation '
     'with her answer beneath it. A name not listed here shows an extra card, '
     'never one fewer.',
     NULL, now(), 'system (seed)'),

    -- ── The one number ───────────────────────────────────────────────────────
    ('fact_card_visible_count', '10', 'count', '10', 1, NULL,
     'How many cards open in full before the rest collapse to their title and '
     'source line. Every card is still present and one click opens it — this is '
     'how much a reader sees without scrolling past twelve full cards.',
     NULL, now(), 'system (seed)')
ON CONFLICT (key) DO NOTHING;

-- ─── 4. The row-count assertion (CLAUDE.md rule 25a) ───────────────────────────
--
-- A statement matching zero rows is SILENT in Postgres, and `ON CONFLICT DO
-- NOTHING` makes that possible on purpose. A key mistyped above inserts nothing,
-- this migration reports success, and the backend then refuses to start with a
-- missing-key error naming the key and not the migration. Assert the END state.
DO $$
DECLARE
    present INTEGER;
BEGIN
    SELECT count(*) INTO present
      FROM app_settings
     WHERE key IN (
        'fact_card_proof_label', 'fact_card_backs_label',
        'fact_card_supports_label', 'fact_card_watch_out_label',
        'fact_card_answer_label', 'fact_card_empty_value',
        'fact_card_draft_mark', 'fact_card_context_label',
        'fact_card_context_hide_label', 'fact_card_backs_template',
        'fact_card_supports_template', 'fact_card_stance_supports_verb',
        'fact_card_stance_rebuts_verb', 'fact_card_rfa_template',
        'fact_card_rfa_unnumbered_template', 'fact_card_deck_template',
        'fact_card_edit_label', 'fact_card_save_label',
        'fact_card_cancel_label', 'fact_card_save_failed_template',
        'fact_card_no_answer_notice', 'fact_card_accusation_heading',
        'fact_card_no_cards_notice', 'fact_card_visible_count',
        'rehearsal_our_side_speakers');

    IF present <> 25 THEN
        RAISE EXCEPTION
            'fact-card v2 wording: expected 25 settings rows after this '
            'migration, found %. The backend enumerates every fact_card_* key at '
            'boot and refuses to start when one is missing, so this migration '
            'fails here rather than at the next deploy.', present;
    END IF;
END $$;
