-- proof_matrix_v2_rulings_and_reader_wording: a human's keep/remove verdict on a
-- ranked item, its append-only ledger, and every word the reader-facing Proof
-- Matrix speaks
--
-- Created: 2026-09-06 14:17:43
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- PROOF_MATRIX_v2 §2 and §3.
--
-- ## Why these tables exist
--
-- The linking pass of 2026-09-06 wrote a rank, a role and a confidence onto
-- 1,194 Evidence→Allegation edges. Every one of those is a MACHINE claim, and the
-- Matrix is about to be read by Chuck as a reference. A machine ranking nobody
-- has been through is a draft; the same ranking with a human's keep/remove on top
-- of it is a reference. A row here is that human act.
--
-- ## Domain note: the scope is CASE-WIDE, and the missing column says so
--
-- There is deliberately NO `scenario_id` and no `count_number`. An item either
-- belongs under ¶41 or it does not, exactly as the machine's own edge claims —
-- and the same allegation is reached from several Counts through several
-- Elements. A per-Count key would let one decision mean two different things
-- depending on which Element the reader happened to open. Same ruling, same
-- reason, as `evidence_allegation_links` (task 2.10) and `evidence_summary_
-- overrides` (task 1.7F).
--
-- ## Why there is no foreign key into the graph
--
-- `evidence_id` and `allegation_id` are Neo4j node ids: Postgres cannot reference
-- them, and the evidence id is a content hash that a re-extraction can re-mint.
-- An orphaned ruling is INERT rather than wrong — it is looked up by the very key
-- it fails to match, so it can never be applied to the wrong statement. This is
-- the stale-pointer behaviour of 2026-07-24 accepted deliberately, not overlooked.
--
-- ## No NUMERIC columns anywhere in this migration
--
-- beta.364 died at boot decoding a NUMERIC into an `Option<f64>` with no
-- `rust_decimal` in the tree. Every column below is TEXT, UUID or TIMESTAMPTZ.
--
-- ## v2 §8: BOTH tables are HUMAN-AUTHORED CONTENT
--
-- Re-gathering never edits human content. A scan that overwrote a ruling would
-- restore the machine's own order over a decision Roman had already made, and it
-- would do it invisibly. So both tables join `HUMAN_AUTHORED_TABLES` and both
-- scan-path invariant tests cover them.

-- ─── 1. The rulings ────────────────────────────────────────────────────────────

CREATE TABLE evidence_allegation_rulings (
    -- The statement. TEXT because the graph id is a content-hash string, matching
    -- `evidence_allegation_links.graph_node_id`. No FK — see the header.
    --
    -- Named `evidence_id` rather than `graph_node_id` because the §2 API says so
    -- and because this table is only ever keyed by an Evidence node, where the
    -- links table is keyed by whatever node a candidate card projects.
    evidence_id   TEXT        NOT NULL,

    -- The accusation the item was ranked against. Also a graph id, also
    -- unconstrained.
    allegation_id TEXT        NOT NULL,

    -- 'keep' | 'remove'. Deliberately NOT a CHECK constraint, matching
    -- `evidence_allegation_links.cut`: the vocabulary lives in code
    -- (`domain::matrix_ruling::MatrixRuling`) so it can widen without a
    -- migration, and an unknown token is refused LOUDLY at the read boundary —
    -- stronger than a CHECK, because a CHECK cannot tell you the PARSER is
    -- missing.
    ruling        TEXT        NOT NULL,

    -- The authenticated username who ruled. NOT NULL: the row's whole value is
    -- that a HUMAN said this, and the export prints a confirmed mark on its
    -- strength.
    ruled_by      TEXT        NOT NULL,

    -- Bound from Rust `Utc::now()` rather than a DB default, matching the house
    -- pattern: the application owns the timestamp, so the row and the log line
    -- agree. `ruled_at` is the FIRST ruling and survives a re-ruling;
    -- `updated_at` moves, so "ruled in June" and "reversed this morning" stay
    -- distinguishable.
    ruled_at      TIMESTAMPTZ NOT NULL,
    updated_at    TIMESTAMPTZ NOT NULL,

    -- An optional sentence from the human. NULLABLE and, unlike every other
    -- column here, allowed to stay empty forever: §2 specifies `note NULL`, and a
    -- required note would turn a one-click decision into a form.
    note          TEXT,

    -- ONE ROW PER (statement, accusation). Ruling the same pair again is an
    -- UPDATE, never a second contradictory row.
    --
    -- The Matrix reads every ruling for one allegation's whole list in a single
    -- `= ANY($1)` over the evidence ids, so `evidence_id` LEADS the key and that
    -- read is an index scan. The order was chosen, not stumbled into.
    CONSTRAINT evidence_allegation_rulings_pkey
        PRIMARY KEY (evidence_id, allegation_id)
);

-- The Matrix's other read is "every ruling bearing on THIS allegation", which the
-- primary key cannot serve because `allegation_id` is its trailing column.
CREATE INDEX idx_evidence_allegation_rulings_allegation
    ON evidence_allegation_rulings (allegation_id);

COMMENT ON TABLE evidence_allegation_rulings IS
    'A human''s keep/remove verdict on one machine-ranked Evidence->Allegation '
    'item. CASE-WIDE — no scenario and no count column, because an item belongs '
    'under an accusation everywhere or nowhere. Human-authored content under '
    'v2 §8: no scan path may write it.';

COMMENT ON COLUMN evidence_allegation_rulings.ruling IS
    'keep | remove. Vocabulary owned by domain::matrix_ruling::MatrixRuling, not '
    'by a CHECK — a CHECK cannot tell you the parser is missing.';

COMMENT ON COLUMN evidence_allegation_rulings.note IS
    'An optional sentence from the human. NULL is the ordinary state: requiring '
    'one would turn a one-click decision into a form.';

-- ─── 2. The ledger ─────────────────────────────────────────────────────────────
--
-- ## Why a ledger and not just the table above
--
-- The table above holds only the CURRENT verdict, and a withdrawal leaves NO row
-- there at all. Without this, an item kept and then un-kept would be
-- indistinguishable from one nobody ever looked at — which is precisely the
-- distinction §3's ordering is built on.
--
-- Append-only, no FK to the rulings table: the record of a human decision must
-- outlive the row it was about, exactly as in `evidence_allegation_link_events`
-- and `scenario_status_transitions`.
CREATE TABLE evidence_allegation_ruling_events (
    -- Surrogate key: this is an event log, so rows have no natural unique key —
    -- the SAME pair is ruled, reversed and withdrawn repeatedly, and each act is
    -- its own row.
    event_id      UUID        PRIMARY KEY DEFAULT gen_random_uuid(),

    evidence_id   TEXT        NOT NULL,
    allegation_id TEXT        NOT NULL,

    -- 'rule' | 'rerule' | 'withdraw'. Code-owned vocabulary
    -- (`domain::matrix_ruling::RulingAction`), no CHECK — see the note above.
    action        TEXT        NOT NULL,

    -- The verdict as it stood after the act. NULL on 'withdraw' and only there:
    -- there is no verdict in force when a ruling is being taken back, and storing
    -- the old one would make the ledger read as though the withdrawal had
    -- asserted something.
    ruling        TEXT,

    -- The note as it stood after the act, so the ledger carries the human's own
    -- words and not just the fact that words existed.
    note          TEXT,

    -- Who, and when. NOT NULL: an unattributed change to a proof surface is the
    -- thing this table exists to make impossible.
    actor         TEXT        NOT NULL,
    at            TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_evidence_allegation_ruling_events_pair
    ON evidence_allegation_ruling_events (evidence_id, allegation_id, at DESC);

COMMENT ON TABLE evidence_allegation_ruling_events IS
    'Append-only history of every keep, remove, reversal and withdrawal: what, '
    'which way, who, when. The rulings table holds only the current verdict and a '
    'withdrawal leaves no row there — so without this, an item kept and then '
    'un-kept would be indistinguishable from one nobody ever read. Human-authored '
    'under v2 §8.';

COMMENT ON COLUMN evidence_allegation_ruling_events.ruling IS
    'The verdict in force after the act. NULL on a withdrawal, and only there.';

-- ─── 3. The words (v2 §2b — every visible word is a stored row) ────────────────
--
-- THE CONFIGURATION LAW: a literal user-facing string in this task's code is a
-- defect of the same class as a compiled-in threshold. The Matrix is now a
-- READER-FACING page — Chuck opens it as a reference — so the law binds hardest
-- here: every label, every mark under a quote, every heading in the Word export.
--
-- Twenty-seven rows below. Twenty-six extend the `matrix_*` wording block
-- (`domain::wording_matrix`), which the boot loader enumerates by name; the
-- twenty-seventh, `matrix_visible_items`, is a PARAMETER rather than a sentence and
-- lives in `settings_store::REQUIRED_KEYS` instead. A key declared in either list
-- with no row here makes the backend REFUSE TO START, which is why the disk/code
-- consistency tests in `wording_matrix_tests.rs` and `settings_store_tests.rs`
-- both read this file.
--
-- The templates carrying `{placeholders}` are registered in
-- `domain::wording_templates::REQUIRED_PLACEHOLDERS`, so an edit on the Settings
-- page that dropped one is refused by name rather than silently shipping a
-- sentence with its facts removed.
--
-- ON CONFLICT DO NOTHING keeps the migration idempotent (house precedent).
-- `updated_by` is 'system (seed)' so a never-edited row stays visibly
-- distinguishable from one a human has set.
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    -- ── The short list and what is behind it ─────────────────────────────────
    ('matrix_more_template', '{count} more', 'text', '{count} more',
     NULL, NULL,
     'The control at the foot of a paragraph''s evidence list that opens the '
     'items beyond the first few. {count} is required — "more" without a number '
     'hides how much is behind it, which is the reason it is behind anything.',
     NULL, now(), 'system (seed)'),

    ('matrix_show_hidden_template', 'Show hidden ({count})', 'text',
     'Show hidden ({count})',
     NULL, NULL,
     'The toggle at the foot of a paragraph that reveals items the machine put '
     'aside or a human removed. {count} is required: a toggle that will not say '
     'how much it is hiding reads as decoration.',
     NULL, now(), 'system (seed)'),

    ('matrix_hide_hidden_label', 'Hide those again', 'text', 'Hide those again',
     NULL, NULL,
     'The same toggle once the hidden items are showing.',
     NULL, now(), 'system (seed)'),

    -- ── The two buttons on every row ─────────────────────────────────────────
    ('matrix_keep_label', 'Keep', 'text', 'Keep',
     NULL, NULL,
     'The button confirming that an item belongs under this paragraph. One '
     'click; the row moves to the top of the list.',
     NULL, now(), 'system (seed)'),

    ('matrix_remove_label', 'Remove', 'text', 'Remove',
     NULL, NULL,
     'The button taking an item out of the paragraph''s list. Reversible: the '
     'item moves to the hidden group with an Undo beside it, and is never '
     'deleted.',
     NULL, now(), 'system (seed)'),

    ('matrix_undo_label', 'Undo', 'text', 'Undo',
     NULL, NULL,
     'The control beside a hidden item that takes the removal back.',
     NULL, now(), 'system (seed)'),

    -- ── Who said this belongs here ───────────────────────────────────────────
    ('matrix_machine_label', 'machine', 'text', 'machine',
     NULL, NULL,
     'The grey mark under a quote nobody has confirmed: the ranking is the '
     'machine''s and no human has been through it. Every unruled item carries it, '
     'so an unread list can never be mistaken for a reviewed one.',
     NULL, now(), 'system (seed)'),

    ('matrix_kept_template', 'kept · {actor}', 'text', 'kept · {actor}',
     NULL, NULL,
     'The green mark under a quote a human confirmed. {actor} is required — an '
     'unattributed confirmation is not a confirmation, and this mark is what the '
     'Word export prints its tick from.',
     NULL, now(), 'system (seed)'),

    ('matrix_conflict_label', 'conflicts', 'text', 'conflicts',
     NULL, NULL,
     'The amber mark on an item that both supports and disputes the same '
     'paragraph — 28 such edges exist. Flagged, never hidden: a reader decides.',
     NULL, now(), 'system (seed)'),

    -- ── How sure the machine was ─────────────────────────────────────────────
    ('matrix_confidence_high_label', 'high', 'text', 'high',
     NULL, NULL,
     'The confidence mark on an item the linking pass was most sure of.',
     NULL, now(), 'system (seed)'),

    ('matrix_confidence_medium_label', 'medium', 'text', 'medium',
     NULL, NULL,
     'The confidence mark on an item the linking pass was less sure of.',
     NULL, now(), 'system (seed)'),

    ('matrix_confidence_low_label', 'low', 'text', 'low',
     NULL, NULL,
     'The confidence mark on an item the linking pass was least sure of. No edge '
     'carries this today — the 2026-09-06 pass emitted only high and medium — but '
     'the token is in the declared vocabulary, so the word exists BEFORE a pass '
     'emits one. Without it the first low-confidence item would render with no '
     'mark at all, which is how an unrated item reads, and a reader would be told '
     'nothing rather than told "low".',
     NULL, now(), 'system (seed)'),

    ('matrix_confidence_unrated_label', 'unrated', 'text', 'unrated',
     NULL, NULL,
     'The mark on an item that predates the linking pass and therefore carries no '
     'confidence at all. Distinct from "medium" on purpose: 250 items were never '
     'rated, and printing "medium" over them would invent a judgment.',
     NULL, now(), 'system (seed)'),

    -- ── Requests for admission read as one line ──────────────────────────────
    ('matrix_rfa_template', 'RFA {number} — {request} — {answer}', 'text',
     'RFA {number} — {request} — {answer}',
     NULL, NULL,
     'How a request-for-admission card reads on one line. All three parts are '
     'required: the request without the answer is not evidence of anything, and '
     'the answer without the request is a bare "Admitted."',
     NULL, now(), 'system (seed)'),

    ('matrix_rfa_unnumbered_template', '{request} — {answer}', 'text',
     '{request} — {answer}',
     NULL, NULL,
     'The same line for a card whose request number cannot be derived from its '
     'title. The number is omitted rather than guessed — a wrong RFA number in a '
     'court filing is worse than none.',
     NULL, now(), 'system (seed)'),

    -- ── The page''s own furniture ────────────────────────────────────────────
    ('matrix_legend_line',
     'Red bar = the complaint''s words · green = supports · red on a quote = disputes',
     'text',
     'Red bar = the complaint''s words · green = supports · red on a quote = disputes',
     NULL, NULL,
     'The one line under the Count buttons explaining the page''s colours, so a '
     'reader who has never seen it before does not have to infer them.',
     NULL, now(), 'system (seed)'),

    ('matrix_export_button_label', 'Export this count (Word)', 'text',
     'Export this count (Word)',
     NULL, NULL,
     'The button that downloads the selected Count as a Word document.',
     NULL, now(), 'system (seed)'),

    -- ── The Word document ────────────────────────────────────────────────────
    ('matrix_export_title_template', 'Count {number} — {name}', 'text',
     'Count {number} — {name}',
     NULL, NULL,
     'The exported document''s title. Both parts required: a document headed '
     '"Count 3" with no cause of action named is not usable at a hearing.',
     NULL, now(), 'system (seed)'),

    ('matrix_export_supporting_heading', 'Supporting', 'text', 'Supporting',
     NULL, NULL,
     'The heading over the items that support a paragraph, in the Word export.',
     NULL, now(), 'system (seed)'),

    ('matrix_export_disputing_heading', 'Disputing', 'text', 'Disputing',
     NULL, NULL,
     'The heading over the items that dispute a paragraph, in the Word export.',
     NULL, now(), 'system (seed)'),

    ('matrix_export_confirmed_mark', '✓', 'text', '✓',
     NULL, NULL,
     'The mark printed in front of an item a human confirmed, in the Word export. '
     'Stored because the FOOTER names it — "items marked ✓ confirmed by Roman" — '
     'and a mark compiled into the renderer while its explanation lives in a '
     'stored row is two halves of one sentence that can drift apart. Edit one and '
     'edit the other. NO trailing space: the store trims every text value on read, '
     'so a value could not carry one even if it were written here. The renderer '
     'supplies the space that joins the mark to the quote.',
     NULL, now(), 'system (seed)'),

    ('matrix_export_empty_line', 'No evidence in the corpus.', 'text',
     'No evidence in the corpus.',
     NULL, NULL,
     'Printed under a heading with nothing beneath it. An empty heading reads as '
     'a formatting fault; this sentence says the gap is real and is the finding.',
     NULL, now(), 'system (seed)'),

    -- ── The two empty legs on screen ─────────────────────────────────────────
    --
    -- Worded SEPARATELY, and not folded into the export's one sentence. A
    -- paragraph with nothing on either side would otherwise print the same words
    -- twice with nothing to say which side is which — and the two states mean
    -- genuinely different things: nothing in the record backs this accusation,
    -- versus nothing in the record answers it. The export prints one sentence
    -- because there the heading above it already names the side.
    ('matrix_no_supporting_line', 'Nothing in the record supports this yet.',
     'text', 'Nothing in the record supports this yet.',
     NULL, NULL,
     'Shown under a paragraph with no supporting evidence. The gap is the '
     'finding, so it is a sentence rather than blank space.',
     NULL, now(), 'system (seed)'),

    ('matrix_no_disputing_line', 'Nothing in the record disputes this.', 'text',
     'Nothing in the record disputes this.',
     NULL, NULL,
     'Shown under a paragraph with no disputing evidence. Deliberately different '
     'words from the supporting gap: one says our proof is missing, the other '
     'says theirs is.',
     NULL, now(), 'system (seed)'),

    ('matrix_export_footer_template',
     'Generated {date}. Machine-ranked; items marked ✓ confirmed by Roman.',
     'text',
     'Generated {date}. Machine-ranked; items marked ✓ confirmed by Roman.',
     NULL, NULL,
     'The footer of every exported Count. {date} is required — a printed proof '
     'summary with no generation date cannot be told apart from last month''s.',
     NULL, now(), 'system (seed)'),

    -- ── When a decision does not save ────────────────────────────────────────
    ('matrix_ruling_failed_template',
     'That decision did not save: {detail} Reload the page — what you see now may not be what is stored.',
     'text',
     'That decision did not save: {detail} Reload the page — what you see now may not be what is stored.',
     NULL, NULL,
     'Shown when a Keep, Remove or Undo could not be written. {detail} is '
     'required: the row updates optimistically, so without a reason the reader is '
     'left looking at a screen that disagrees with the database and cannot tell.',
     NULL, now(), 'system (seed)'),

    -- ── The one number ───────────────────────────────────────────────────────
    ('matrix_visible_items', '5', 'count', '5', 1, NULL,
     'How many items each paragraph shows before "N more", and how many the Word '
     'export prints per list. One number for both, so the page and the document a '
     'reader takes away from it cannot disagree about what the top of a list is.',
     NULL, now(), 'system (seed)')
ON CONFLICT (key) DO NOTHING;

-- ─── 4. The row-count assertion (CLAUDE.md rule 25a) ───────────────────────────
--
-- A statement matching zero rows is SILENT in Postgres. `ON CONFLICT DO NOTHING`
-- makes that possible on purpose — but it also means a key mistyped here inserts
-- nothing, this migration reports success, and the backend then refuses to start
-- with a missing-key error that names the key and not the migration. Assert the
-- END state instead: all twenty-seven keys are present, whatever this run did.
DO $$
DECLARE
    present INTEGER;
BEGIN
    SELECT count(*) INTO present
      FROM app_settings
     WHERE key IN (
        'matrix_more_template', 'matrix_show_hidden_template',
        'matrix_hide_hidden_label', 'matrix_keep_label', 'matrix_remove_label',
        'matrix_undo_label', 'matrix_machine_label', 'matrix_kept_template',
        'matrix_conflict_label', 'matrix_confidence_high_label',
        'matrix_confidence_medium_label', 'matrix_confidence_unrated_label',
        'matrix_confidence_low_label', 'matrix_export_confirmed_mark',
        'matrix_rfa_template', 'matrix_rfa_unnumbered_template',
        'matrix_legend_line', 'matrix_export_button_label',
        'matrix_export_title_template', 'matrix_export_supporting_heading',
        'matrix_export_disputing_heading', 'matrix_export_empty_line',
        'matrix_export_footer_template', 'matrix_ruling_failed_template',
        'matrix_no_supporting_line', 'matrix_no_disputing_line',
        'matrix_visible_items');

    IF present <> 27 THEN
        RAISE EXCEPTION
            'proof-matrix v2 wording: expected 27 settings rows after this '
            'migration, found %. The backend enumerates every matrix_* key at '
            'boot and refuses to start when one is missing, so this migration '
            'fails here rather than at the next deploy.', present;
    END IF;
END $$;
