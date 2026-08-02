-- allow_anchorless_removals: Allow anchorless removals
--
-- Created: 2026-08-01 13:04:19
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- Follow-up to 20260801103313, which created `scenario_ruling_anchors` with
-- `document_id TEXT NOT NULL`. Removals need that relaxed.
--
-- ## Why a second migration instead of editing the first
--
-- The sibling migration was written minutes ago and has not been deployed, so
-- editing it in place would be tempting and would leave one clean file. But sqlx
-- keys applied migrations by version AND checksum: if that file HAS been applied
-- anywhere — a local database, an experiment — editing it turns the next backend
-- boot into a checksum-mismatch failure. Forward-only is the discipline precisely
-- so that "has it run yet?" never has to be answered correctly. A second file is
-- right whether or not the first has run.
--
-- ## Why removals cannot carry a mandatory document
--
-- Ratified 2026-08-01: §12.1's list of rulings is NON-EXHAUSTIVE — any verb that
-- changes a candidate's ruling state writes a ledger row. That brings
-- `DELETE /facts/:graph_node_id` in, and un-saving is the one ruling that must
-- never be refused: a human removing an item is undoing something, and blocking
-- an undo because the underlying evidence has since vanished would trap them with
-- a reference they can neither use nor discard.
--
-- But a vanished node is exactly the case where the anchor fields cannot be read.
-- So a removal may legitimately carry NO document, and the ledger has to accept
-- that. Every other ruling still REQUIRES a document — enforced in code by
-- `RulingKind::requires_document`, AND at the database by the second CHECK below.
--
-- A column-level NOT NULL cannot express "except for removals", but a table-level
-- CHECK can, because it sees `ruled_status` alongside `document_id`. So the
-- guarantee stays structural at both layers rather than becoming application-only:
-- a migration-driven backfill, a future machine ruling path, or a direct psql
-- session cannot insert an anchorless include.
--
-- The cost is that the CHECK names 'remove' literally, so a future ruling kind
-- that also tolerates a missing document needs a migration to widen it. That is
-- the right trade: such a kind would be a deliberate change to the anchor law, and
-- forcing it through a migration is the point.
--
-- The absence gets the same treatment as page and speaker: an explicit state
-- column, so a NULL document is a RECORDED fact rather than an unexplained gap.

ALTER TABLE scenario_ruling_anchors
    ALTER COLUMN document_id DROP NOT NULL;

-- DEFAULT 'present' so any row already written by the sibling migration's build
-- (all of which carried a NOT NULL document) is backfilled truthfully; then the
-- default is DROPPED so every future write must state the value explicitly. A
-- lingering default would let a caller omit the column and silently claim
-- 'present' for an absent document — the precise failure this column prevents.
ALTER TABLE scenario_ruling_anchors
    ADD COLUMN document_state TEXT NOT NULL DEFAULT 'present';

ALTER TABLE scenario_ruling_anchors
    ALTER COLUMN document_state DROP DEFAULT;

-- The value and its state must agree. Unlike page/speaker/quote — whose states are
-- derived in one Rust function and therefore cannot contradict their values — this
-- one is worth a database CHECK because `document_id` is the field the task-2.5
-- matcher indexes on: a row claiming 'present' with a NULL document would be found
-- by the document index and then fail to match anything.
ALTER TABLE scenario_ruling_anchors
    ADD CONSTRAINT scenario_ruling_anchors_document_state_agrees
        CHECK ((document_state = 'present') = (document_id IS NOT NULL));

-- Only a removal may be anchorless. This is what the dropped NOT NULL used to
-- guarantee for every row, restored for the four ruling kinds that still require a
-- document — the code refuses them first (`RulingKind::requires_document`), and
-- this refuses them if anything ever reaches the table without going through it.
ALTER TABLE scenario_ruling_anchors
    ADD CONSTRAINT scenario_ruling_anchors_document_required_unless_removal
        CHECK (ruled_status = 'remove' OR document_id IS NOT NULL);

COMMENT ON COLUMN scenario_ruling_anchors.document_id IS
    'The source document, or NULL for a removal recorded after the graph node had '
    'already vanished. Every ruling EXCEPT remove requires it (enforced in code by '
    'RulingKind::requires_document) — a removal must be recordable even when '
    'nothing about the item can still be read.';

COMMENT ON COLUMN scenario_ruling_anchors.document_state IS
    'present | none. Only a removal may be none. Paired with document_id by a '
    'CHECK, because this is the field task 2.5 indexes on.';
