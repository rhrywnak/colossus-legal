-- create_scenario_ruling_anchors: Create scenario ruling anchors
--
-- Created: 2026-08-01 10:33:13
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- Implements the §12.1 anchor law (RATIFIED 2026-08-01), tracker task 1.1.
--
-- ## The defect this exists to prevent
--
-- On 2026-07-24/25 a re-extraction destroyed all 26 rulings on this case,
-- including 6 human includes. Nothing was corrupted and nothing errored: rulings
-- key to `scenario_fact_refs.graph_node_id`, that id is a CONTENT HASH, and
-- re-extracting the document minted new hashes. Every ruling silently became a
-- pointer to a node that no longer existed. The curation was not recoverable
-- because nothing recorded WHAT had been ruled on — only which ephemeral id.
--
-- An anchor is the durable answer to "what did the human actually rule on":
-- document, page, the verbatim words, and who said them. Those survive
-- re-extraction because they describe the RECORD, not the graph's current
-- encoding of it. Task 2.5 consumes them to re-attach rulings after a
-- re-extraction; this migration only writes them.
--
-- ## Why a TABLE and not columns on scenario_fact_refs
--
-- `scenario_fact_refs` is UPSERTED — one row per (scenario, node), overwritten by
-- each successive ruling. Anchor columns there would be destroyed by the next
-- ruling on the same candidate: an include, later dropped, would retain only the
-- drop's anchor, and the record of what was included would be gone. That is the
-- same class of silent loss this table exists to prevent, so the anchors are an
-- APPEND-ONLY LEDGER: one row per ruling EVENT, never updated, never deleted.
-- "Every ruling ever made" (v2 §2 C3) is then a plain SELECT.
--
-- ## Why there is deliberately NO foreign key on scenario_id
--
-- Every sibling table here (`scenario_fact_refs`, `scenario_candidate_ordinals`)
-- carries `REFERENCES scenarios(scenario_id) ON DELETE CASCADE`, because those
-- tables hold STATE that is meaningless without its scenario. This table is
-- different in kind: it is the forensic record of decisions a human made. A
-- cascade would mean deleting a scenario also erases the history of every ruling
-- ever made inside it — the 2026-07-24 failure wearing a foreign key, and losing
-- exactly the evidence that would explain what was lost.
--
-- So `scenario_id` is a plain UUID column, indexed but unconstrained. A ledger row
-- may outlive its scenario; that is the point. This is a deliberate departure from
-- the sibling tables' discipline, recorded here so a future reader does not
-- "fix" it by adding the FK back.

CREATE TABLE scenario_ruling_anchors (
    -- Surrogate key: this is an append-only event log, so rows have no natural
    -- unique key — the SAME (scenario, node) is ruled on repeatedly and each
    -- ruling is its own row. Contrast the composite PKs on the sibling state
    -- tables, where one row per pair is exactly the invariant.
    anchor_id        UUID        PRIMARY KEY DEFAULT gen_random_uuid(),

    -- The scenario the ruling was made in. NO foreign key — see the header.
    scenario_id      UUID        NOT NULL,

    -- The graph node as it was identified AT RULING TIME. Recorded for provenance
    -- and for the 2.5 matcher's fast path (an unchanged id needs no re-matching),
    -- never as the durable key — it is precisely the ephemeral value whose churn
    -- caused the defect.
    graph_node_id    TEXT        NOT NULL,

    -- Which ruling this was: include | exclude | defer. Deliberately NOT a CHECK
    -- constraint, matching scenario_fact_refs.status' precedent on this table's
    -- sibling: the vocabulary is validated in code by the RulingKind enum so it can
    -- widen without a migration.
    ruled_status     TEXT        NOT NULL,

    -- ── The anchor proper — the four §12.1 fields ──────────────────────────────
    --
    -- Document is MANDATORY for every ruling: an item with no source document is
    -- not record evidence, and a ruling on it could never be re-anchored or cited.
    -- The write path refuses such a ruling loudly rather than storing a partial
    -- anchor.
    document_id      TEXT        NOT NULL,

    -- Page is nullable, with an explicit state column beside it. A bare NULL cannot
    -- say WHY it is absent, and "the extraction never captured a page" is
    -- operationally different from "this evidence genuinely has no page".
    --
    -- BIGINT, not INTEGER: the source is `BiasInstance.page_number: Option<i64>`,
    -- so BIGINT stores it with no conversion at all. INTEGER would need an
    -- i64→i32 narrowing that can fail, which would mean inventing an error path
    -- for a case that cannot occur — a worse trade than four extra bytes.
    page             BIGINT,
    page_state       TEXT        NOT NULL,

    -- The verbatim words, exactly as the graph held them at ruling time. MANDATORY
    -- for include and exclude (citability law, v2 §9/§17: an item that cannot be
    -- quoted cannot be cited and cannot be durably re-anchored). Nullable at the
    -- column level ONLY because a DEFER is always permitted on an unquotable item
    -- — deferring is how the human parks exactly that problem.
    quote_verbatim   TEXT,

    -- The normalized form the 2.5 matcher will compare on: casefolded and
    -- whitespace-collapsed, per the law's two named operations and no others.
    -- Stored ALONGSIDE the verbatim text, never instead of it — the verbatim quote
    -- is what gets read aloud in a courtroom.
    quote_normalized TEXT,
    quote_state      TEXT        NOT NULL,

    -- Who said it, and the same explicit-state treatment as page. Documentary
    -- evidence (a letter, a record) genuinely has no speaker; that is 'none', not a
    -- gap in the capture.
    speaker          TEXT,
    speaker_state    TEXT        NOT NULL,

    -- ── Provenance of the ruling itself ────────────────────────────────────────
    --
    -- Bound from Rust Utc::now() rather than a DB default, matching the house
    -- pattern (scan_runs.started_at, scenario_candidate_ordinals.assigned_at):
    -- the application owns the timestamp.
    ruled_at         TIMESTAMPTZ NOT NULL,

    -- The authenticated username, or a machine path's identifier. NOT NULL: an
    -- anonymous ruling is not a record, and every current write path has an
    -- authenticated user in hand.
    ruled_by         TEXT        NOT NULL,

    -- Why this candidate was deferred. Required for defer, NULL for every other
    -- ruling — enforced in code, and by the CHECK below.
    defer_reason     TEXT,

    -- A defer with no reason is not a defer, it is an unexplained non-decision;
    -- and a reason on an include/exclude would mean the write path crossed its
    -- vocabularies. Both are refused at the database as a backstop to the code.
    CONSTRAINT scenario_ruling_anchors_defer_reason_iff_defer
        CHECK ((ruled_status = 'defer') = (defer_reason IS NOT NULL))
);

-- Rulings are read by scenario (the ledger for one scenario's history) …
CREATE INDEX idx_scenario_ruling_anchors_scenario
    ON scenario_ruling_anchors (scenario_id);

-- … and by document, which is how task 2.5 will work: a document is re-extracted,
-- and every ruling anchored to it must be re-matched against the new nodes.
CREATE INDEX idx_scenario_ruling_anchors_document
    ON scenario_ruling_anchors (document_id);

COMMENT ON TABLE scenario_ruling_anchors IS
    'Append-only ledger of every ruling (include/exclude/defer) with the durable '
    'anchor recorded AT RULING TIME: document, page, verbatim + normalized quote, '
    'speaker. Rulings key to anchors, never to content-hash graph node ids, which '
    'are ephemeral by law. NO foreign key on scenario_id and no cascade — deleting '
    'a scenario must not erase the record of what was ruled inside it. Written by '
    'task 1.1; consumed by the re-anchor matching pass (task 2.5).';

COMMENT ON COLUMN scenario_ruling_anchors.graph_node_id IS
    'The graph node id at ruling time. Provenance and a fast path for the matcher, '
    'NEVER the durable key — this is the ephemeral content hash whose churn '
    'destroyed 26 rulings on 2026-07-24.';

COMMENT ON COLUMN scenario_ruling_anchors.page_state IS
    'present | none. An explicit state so an absent page is a RECORDED fact rather '
    'than an unexplained NULL.';

COMMENT ON COLUMN scenario_ruling_anchors.quote_state IS
    'present | none. Only a defer may carry none: include and exclude require a '
    'quote (citability law), and the write path refuses them without one.';

COMMENT ON COLUMN scenario_ruling_anchors.speaker_state IS
    'present | none. Documentary evidence genuinely has no speaker; that is none, '
    'not a capture gap.';

COMMENT ON COLUMN scenario_ruling_anchors.quote_normalized IS
    'Casefolded and whitespace-collapsed form of quote_verbatim — the two '
    'operations the law names, and no others. Comparison surface for task 2.5.';

-- ─── The defer reason on the state row ──────────────────────────────────────────
--
-- The ledger records that a defer HAPPENED and why. The workbench also needs the
-- CURRENT reason on the live row, so the queue view (task 1.3) can show why an
-- undecided candidate is parked without walking the ledger for every card.
--
-- Nullable, and NULL is the honest value for the overwhelming majority of rows: a
-- candidate nobody has touched is undecided WITHOUT having been deferred. That
-- distinction is the whole point — `defer_reason IS NOT NULL` is what separates an
-- explicit "parked, because …" from "never looked at".
ALTER TABLE scenario_fact_refs
    ADD COLUMN defer_reason TEXT;

COMMENT ON COLUMN scenario_fact_refs.defer_reason IS
    'Why this candidate was deferred, set when a human rules defer (status stays '
    'undecided). NULL means never deferred — which is NOT the same as undecided: '
    'an untouched candidate is undecided with no reason. Cleared when a later '
    'ruling supersedes the defer.';
