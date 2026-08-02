-- add_c1_identity_and_human_authored_tables: Add C1 identity and human-authored tables
--
-- Created: 2026-08-01 16:23:44
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- Tracker task 1.4. Gives the three human-authored components of a scenario
-- (v2 §2) the storage they need:
--
--   C1 Identity     — two columns on `scenarios`
--   C4 Human facts  — a new `scenario_human_facts` table
--   C5 Talking points — two columns on the EXISTING `scenario_responses` trio
--
-- ## The §8 invariant this schema exists to make structural
--
-- "Editing human content never triggers re-gathering; re-gathering never edits
-- human content." The second half is the one a schema can enforce, and it does
-- so by SEPARATION: the scan and merge paths write exactly three table families
-- — `scenario_fact_refs`, the `scan_run*` trio, and
-- `scenario_candidate_ordinals`. Nothing added or altered here is in that set,
-- so no scan can reach human content without a new writer that a source-scan
-- test would catch (`scenario_human_facts_tests`, mirroring the A0
-- visibility-invariants pattern).
--
-- No ruling or anchor table is touched by this migration.

-- ─── C1: the two identity fields that had no home ───────────────────────────────
--
-- Both NULLABLE, deliberately: a scenario is created before it is framed, and a
-- NOT NULL would force a placeholder — invented prose sitting in the record,
-- indistinguishable later from something a human actually wrote.
--
-- ## Why these are separate columns and not more keys in the `definition` jsonb
--
-- `definition` holds the ATTACK as the other side frames it (`attack_text`) plus
-- a gloss of it (`attack_meaning`). These two are OUR framing, and the three are
-- different sentences with different points of view:
--
--   attack_text     — what they say ("Marie is obstructive")
--   theme_statement — how we answer it in one line (the tagline)
--   motivation      — what they want the jury to believe by saying it
--
-- 1.5's rehearsal mode reads theme_statement beside direction, so collapsing any
-- two of the three would destroy the distinction that mode depends on. They are
-- also spine columns now (filtered and read directly), which is the same reason
-- name/direction/status are columns rather than jsonb keys.
ALTER TABLE scenarios ADD COLUMN theme_statement TEXT;
ALTER TABLE scenarios ADD COLUMN motivation TEXT;

COMMENT ON COLUMN scenarios.theme_statement IS
    'C1: our one-plain-sentence answer to this attack — the tagline version. NOT '
    'the attack itself (that is definition->>attack_text, the other side''s '
    'framing). Read by task 1.5''s rehearsal mode alongside direction.';

COMMENT ON COLUMN scenarios.motivation IS
    'C1: what the other side wants the jury to believe by making this attack. '
    'Distinct from both the attack text and our theme statement.';

-- ─── C4: human facts — knowledge that is in no document ─────────────────────────
--
-- The defining property: these rows have NO citation, and that is correct rather
-- than incomplete. "Switching counsel meant emailing attachments" is true, it is
-- load-bearing, and no document says it. Every other fact in this system carries
-- an anchor precisely so it can be defended; these carry an AUTHOR instead, and
-- the surface labels them so a reader is never misled about which they are
-- looking at (v2 §8: "visibly tagged human-authored, no citation").
CREATE TABLE scenario_human_facts (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Owned by its scenario, like the sibling authored tables.
    scenario_id  UUID NOT NULL REFERENCES scenarios(scenario_id) ON DELETE CASCADE,

    text         TEXT NOT NULL,

    -- Optional date so the fact can land on the C7 timeline beside the dated
    -- record items. A human fact with no date is normal, not deficient.
    occurred_on  DATE,

    -- How precise that date is — the Casefleet date-TYPE pattern the UI study
    -- binds us to (§1.6): exact / around / range / ordered. "Around 4/21/2009"
    -- renders differently from "4/21/2009", and flattening the two would state
    -- more precision than the human claimed.
    --
    -- No CHECK: the vocabulary is validated in code by the DateType enum, the
    -- same evolvable-vocabulary discipline as scenario_fact_refs.status.
    date_type    TEXT,

    -- People this fact concerns, as PLAIN STRINGS. Deliberately not entity ids:
    -- canonical person identity is task B0, and inventing a link here would mean
    -- guessing which "Phillips" was meant. The surface labels these as typed
    -- text, not linked entities, so nobody reads them as resolved references.
    person_refs  TEXT[],

    -- Who wrote it. NOT NULL: a human fact with no author is unattributable, and
    -- attribution is the only provenance this row has in place of a citation.
    authored_by  TEXT        NOT NULL,

    -- Bound from Rust Utc::now(), matching the house pattern (the application
    -- owns the timestamp).
    created_at   TIMESTAMPTZ NOT NULL,
    updated_at   TIMESTAMPTZ NOT NULL,

    -- An empty fact is not a fact. The UI refuses it first; this refuses it if
    -- anything ever reaches the table without passing through the UI.
    CONSTRAINT scenario_human_facts_text_not_blank CHECK (btrim(text) <> ''),

    -- A date type without a date claims precision about nothing.
    CONSTRAINT scenario_human_facts_date_type_needs_a_date
        CHECK (date_type IS NULL OR occurred_on IS NOT NULL)
);

CREATE INDEX idx_scenario_human_facts_scenario ON scenario_human_facts (scenario_id);

COMMENT ON TABLE scenario_human_facts IS
    'C4: knowledge in no document, authored by a human and carrying no citation. '
    'Written ONLY by the augmentation service; no scan, gather or merge path may '
    'touch it (v2 §8). Distinct from neo4j/human_facts.rs, which is a dead '
    'graph-level writer for a different concept — see task 3.8.';

COMMENT ON COLUMN scenario_human_facts.person_refs IS
    'Plain text names, NOT entity ids. Canonical person identity is task B0; '
    'until then the surface labels these as typed rather than linked.';

-- ─── C5: attribution on the existing talking-points tables ──────────────────────
--
-- The `scenario_responses` / `response_items` trio already exists (migration
-- 20260626135022) and models C5 as built: one response per scenario, its ordered
-- items are the ≤cap talking points (ratified 2026-08-01). What it lacks is the
-- author, which §8 requires so the surface can tag the content.
--
-- Nullable, unlike `scenario_human_facts.authored_by`: rows written before this
-- migration have no author to backfill, and inventing one would be a false
-- attribution. There are none today (the trio has never been written to), but a
-- NOT NULL that depends on that being true is a constraint resting on an
-- accident.
ALTER TABLE scenario_responses ADD COLUMN authored_by TEXT;
ALTER TABLE response_items ADD COLUMN authored_by TEXT;

COMMENT ON COLUMN scenario_responses.authored_by IS
    'Who authored this response. NULL only for rows predating task 1.4 (there '
    'are none). Human content is displayed with its author per v2 §8.';

COMMENT ON COLUMN response_items.authored_by IS
    'Who authored this talking point. See scenario_responses.authored_by.';
