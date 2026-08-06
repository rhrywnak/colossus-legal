-- accusation_instances_and_answer_pairings: the two human judgments 2.11 adds
--
-- Created: 2026-08-05 20:30:20
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- Task 2.11 B1, from REHEARSAL_VIEW_DESIGN_v2 (SIGNED 2026-08-06).
--
-- Roman's model IS the spec: "George portrayed Marie as unreasonable. He
-- mentioned it 5 times in 5 different documents. Marie rebutted 5 times." Two
-- judgments have to be recorded for that page to exist, and BOTH are human:
--
--   accusation_instance — this included statement IS them making the accusation
--   answer_pairing      — this record item is what we said back to that instance
--
-- The machine never asserts either. It cannot: "this rebuts that" is exactly the
-- contradiction judgment the requirement defers to a layer we have not built,
-- and guessing it is what a rehearsal page must never do.
--
-- ## Why these are KIND ROWS here and not a pairs table of their own
--
-- `scenario_human_facts` already carries `kind` (fact, watch_list, and §2d's
-- ratified annotation), already has `authored_by` NOT NULL and timestamps, and
-- already has ONE write path with the §8 guards on it. A dedicated table would
-- mean a second write path and a second set of guards — and task 1.4's gate
-- review found an asymmetry of precisely that sort, where C5 lacked a guard C4
-- had. Duplicated guards is how that happens.
--
-- ## §12 — anchored, never internal ids
--
-- Both columns hold GRAPH NODE IDS. A judgment keyed to an internal row id would
-- not survive re-processing, and this scenario has already been taught that
-- lesson: 26 of its saved references point at nodes that no longer resolve.
--
-- ## The CHECKs name no kind, deliberately
--
-- `text` is NOT NULL with a non-blank CHECK, because a fact and a watch-list note
-- are things a human WROTE. An instance and a pairing are not: they carry no
-- prose, only anchors. The obvious fix — exempting the two new kinds by name —
-- would bake the vocabulary into the database, which is exactly what this table's
-- sibling `scenario_fact_refs` documents itself as refusing to do.
--
-- So the constraint asks about the ANCHOR instead: a row either says something,
-- or it points at something. A sixth kind gets the safe default (text required)
-- without this file knowing it exists.
--
-- ## Re-pairing overwrites; the partial indexes are what make that true
--
-- One answer per accusation, one instance per anchor. Without these, pairing a
-- second answer would silently add a row and the page would have to choose
-- between two answers with no basis for choosing.
--
-- ## Remove does NOT cascade — ruled 2026-08-06, and it is law for this surface
--
-- When either side of a pairing leaves the scenario, the row is KEPT and the page
-- renders it as a named gap. Deleting it would make a human's pairing disappear
-- with nothing on screen to say so — and the honest-gap law is that every absence
-- is named. A visible broken pairing beats a silent vanish. There is therefore NO
-- foreign key on either anchor column: they point into Neo4j, which Postgres
-- cannot reference, and even a same-database reference would have to be
-- ON DELETE RESTRICT rather than CASCADE.
--
-- FORWARD-ONLY: no down migration.

ALTER TABLE scenario_human_facts
    ADD COLUMN anchor_graph_node_id TEXT;

ALTER TABLE scenario_human_facts
    ADD COLUMN answers_graph_node_id TEXT;

-- Text is required of rows that SAY something; anchor rows point instead.
ALTER TABLE scenario_human_facts
    DROP CONSTRAINT scenario_human_facts_text_not_blank;

ALTER TABLE scenario_human_facts
    ADD CONSTRAINT scenario_human_facts_says_or_points
    CHECK (btrim(text) <> '' OR anchor_graph_node_id IS NOT NULL);

-- An answer that names no answering item is not a pairing.
ALTER TABLE scenario_human_facts
    ADD CONSTRAINT scenario_human_facts_answer_needs_anchor
    CHECK (answers_graph_node_id IS NULL OR anchor_graph_node_id IS NOT NULL);

CREATE UNIQUE INDEX scenario_human_facts_one_instance_per_anchor
    ON scenario_human_facts (scenario_id, anchor_graph_node_id)
    WHERE kind = 'accusation_instance';

CREATE UNIQUE INDEX scenario_human_facts_one_pairing_per_anchor
    ON scenario_human_facts (scenario_id, anchor_graph_node_id)
    WHERE kind = 'answer_pairing';

COMMENT ON COLUMN scenario_human_facts.anchor_graph_node_id IS
    'The statement this judgment is ABOUT, as a Neo4j node id (§12 — anchored, never '
    'an internal id, so it survives re-processing). NULL on a fact or watch-list note, '
    'which are about nothing but themselves.';

COMMENT ON COLUMN scenario_human_facts.answers_graph_node_id IS
    'For an answer_pairing: the record item that ANSWERS the anchored accusation. '
    'NULL on every other kind. Never inferred — the machine does not assert rebuttal.';
