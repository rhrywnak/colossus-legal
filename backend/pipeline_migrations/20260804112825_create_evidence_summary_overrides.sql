-- create_evidence_summary_overrides: a human's correction of a machine-written
-- interrogatory question
--
-- Created: 2026-08-04 11:28:25
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- Task 1.7F Part B (TASK_1_7F_DESIGN_v1, signed 2026-08-04).
--
-- ## What this stores, and what it deliberately does not
--
-- Discovery Evidence carries the interrogatory question it answers in the graph,
-- as `Evidence.question`. Some of those are simply wrong — the measured case on
-- DEV is C-113, where a Court-of-Appeals question about who refused to cooperate
-- is welded to the interrogatory boilerplate "See the responses to the previous
-- interrogatories." A human cannot currently correct it.
--
-- A row here is that correction. The machine's own words stay exactly where they
-- are, in Neo4j, and are NEVER written by this feature — the override lives
-- BESIDE the original rather than over it, which is why "revert to the system
-- text" is a DELETE of this row and never a copy-back. That is not merely a
-- convention: the two live in different databases, so no write path here can
-- reach the original even by mistake.
--
-- ## Domain note: the scope is CASE-WIDE, and that is the whole point (ruling R1)
--
-- One summary per piece of evidence. Correct it while working scenario S-2 and it
-- reads corrected in S-4, because the same quote is the same quote. The rejected
-- alternative — one override per (scenario, evidence) pair — would let a single
-- C-code mean two different things depending on which page you were standing on,
-- which is the defect Roman caught in the codes themselves and had fixed. So the
-- key is the evidence node ALONE, with no scenario column: a scenario column is
-- precisely what would make this per-scenario again.
--
-- ## Why a plain TEXT primary key and not a composite (ruling R8)
--
-- One overridable text per evidence item, keyed by the node. The obvious "what if
-- we later want to override a second field?" is answered and closed: the other
-- machine-written line on a card is `stance.summary`, which is COMPOSED from a
-- verb and an object rather than authored, and the cure for a wrong stance is the
-- human link editor in task 2.10 — not retyping derived text. So a second
-- overridable field is not expected, and this key reads as a decision rather than
-- an oversight.
--
-- ## Why there is no foreign key into the graph
--
-- `graph_node_id` is a CONTENT HASH minted by extraction, not a stable identity —
-- the same reason `scenario_fact_refs` and `scenario_ruling_anchors` carry it
-- unconstrained. Postgres cannot reference a Neo4j node in any case; recording
-- that here so nobody adds a constraint to a column that has nothing to point at.
-- A re-extraction can therefore orphan a row, exactly as it orphaned 26 rulings on
-- 2026-07-24; task 2.5 re-attaches by anchor, and an orphaned override is inert
-- until it does — it is never applied to the wrong evidence, because the key it
-- fails to match is the key it is looked up by.
--
-- ## v2 §8: this is HUMAN-AUTHORED CONTENT
--
-- Re-gathering never edits human content. A scan overwriting a human's correction
-- is exactly the failure §8 exists to prevent, so this table joins
-- HUMAN_AUTHORED_TABLES and both scan-path invariant tests cover it
-- (`scan_and_merge_paths_write_only_their_own_tables`,
-- `no_scan_path_mentions_a_human_authored_table`).

CREATE TABLE evidence_summary_overrides (
    -- The evidence node whose question this corrects. THE WHOLE KEY — no
    -- scenario column, by ruling R1 (see the header). TEXT because the graph id
    -- is a content-hash string, matching `scenario_fact_refs.graph_node_id`.
    graph_node_id TEXT        PRIMARY KEY,

    -- The human's text, replacing the machine's on every surface that shows it.
    -- NOT NULL: an override with no text is not an override, it is a revert, and
    -- a revert deletes the row (ruling R2). Storing an empty string would leave a
    -- card silently showing nothing where a question used to be.
    summary_text  TEXT        NOT NULL,

    -- The authenticated username who wrote it. NOT NULL for the same reason as
    -- `scenario_ruling_anchors.ruled_by`: an anonymous correction is not a record,
    -- and the badge on the card names this person out loud.
    authored_by   TEXT        NOT NULL,

    -- Bound from Rust `Utc::now()` rather than a DB default, matching the house
    -- pattern (`scenario_human_facts`, `scan_runs.started_at`): the application
    -- owns the timestamp.
    --
    -- `created_at` survives an upsert and `updated_at` moves, so "corrected once,
    -- long ago" and "corrected again this morning" stay distinguishable. The badge
    -- shows `updated_at`.
    created_at    TIMESTAMPTZ NOT NULL,
    updated_at    TIMESTAMPTZ NOT NULL,

    -- A blank correction is a revert that failed to happen. The write path trims
    -- and refuses empty input with a message; this is the backstop, so the table
    -- cannot hold a row that would blank a card's question line.
    CONSTRAINT evidence_summary_overrides_text_not_blank
        CHECK (length(btrim(summary_text)) > 0)
);

COMMENT ON TABLE evidence_summary_overrides IS
    'A human''s correction of a machine-written interrogatory question, keyed by '
    'the evidence node alone — CASE-WIDE by ruling R1, because the same quote is '
    'the same quote in every scenario. The machine original stays in Neo4j and is '
    'never written here: revert deletes the row rather than copying text back. '
    'Human-authored content under v2 §8 — no scan path may write it.';

COMMENT ON COLUMN evidence_summary_overrides.graph_node_id IS
    'The evidence node id (content hash) this corrects. The entire primary key: a '
    'scenario column would make the override per-scenario, which is the '
    'two-meanings defect ruling R1 exists to prevent. No foreign key — Postgres '
    'cannot reference a Neo4j node, and the id is ephemeral by law.';

COMMENT ON COLUMN evidence_summary_overrides.summary_text IS
    'The human''s replacement text. Shown instead of Evidence.question wherever '
    'that question is shown; the machine''s words are untouched underneath.';

COMMENT ON COLUMN evidence_summary_overrides.updated_at IS
    'When the correction was last written. Composed into the card''s badge, so a '
    'reader can see how old the human text is.';
