-- add_ready_transitions_and_watch_list_kind: Add ready transitions and watch-list kind
--
-- Created: 2026-08-01 21:50:24
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- Tracker task 1.5 (rehearsal mode). Two additions:
--
--   1. `scenario_status_transitions` — who declared a scenario ready, and who
--      took it back out of rehearsal.
--   2. `scenario_human_facts.kind` — the watch-list note, stored as a second
--      KIND of human fact rather than in a table of its own.

-- ─── 1. The ready gate's record ────────────────────────────────────────────────
--
-- v2 §5 makes the gate mandatory: no scenario enters rehearsal mode without a
-- human declaring it ready. §6 makes both directions human acts. So the act needs
-- a record, and the question that record has to answer is not "is it ready now"
-- (the `scenarios.status` column already says that) but "WHO took S-2 out of
-- rehearsal, and when" — asked at 11pm the night before trial, when a scenario
-- Marie expected to rehearse is missing.
--
-- ## Why a table and not two columns on `scenarios`
--
-- A `ready_declared_by` / `ready_declared_at` pair holds only the latest act. One
-- promote → demote → promote cycle and the demotion is gone, along with the
-- answer to the only question anyone asks. Appending keeps the history for the
-- cost of one small table.
--
-- ## Why no foreign key, and why this is NOT the ruling ledger
--
-- No FK, for the same reason `scenario_ruling_anchors` has none: this is the
-- forensic record of human acts, and a cascade would erase the history of who did
-- what the moment a scenario was deleted. `scenario_id` is indexed, unconstrained.
--
-- It is deliberately a SEPARATE table from the ruling ledger. That ledger records
-- rulings on EVIDENCE (include / exclude / defer, each with its anchor). A
-- readiness declaration is a statement about the whole scenario and carries no
-- anchor; putting it in the same table would mean either a nullable anchor on a
-- table whose entire purpose is that anchors are present, or a ruling kind that
-- is not a ruling.
CREATE TABLE scenario_status_transitions (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),

    -- No FK — see the header. A transition row may outlive its scenario.
    scenario_id  UUID        NOT NULL,

    -- Where it went from and to. Free TEXT, not a CHECK: the status vocabulary
    -- lives in code (`ALLOWED_STATUSES`), and 2026-08-01 ruled that `draft` and
    -- `needs_evidence` both read as v2 "drafted" WITHOUT a CHECK migration. A
    -- constraint here would have to be migrated in lockstep with a vocabulary
    -- that is deliberately evolvable.
    from_status  TEXT        NOT NULL,
    to_status    TEXT        NOT NULL,

    -- Who did it. NOT NULL: an unattributed declaration is not a human act, and
    -- attribution is the whole point of recording the transition (v2 §5 — Roman,
    -- Marie or Chuck may declare ready; §5a — nothing waits on Chuck).
    actor        TEXT        NOT NULL,

    -- Bound from Rust Utc::now(), matching the house pattern.
    at           TIMESTAMPTZ NOT NULL,

    -- A transition that goes nowhere is not a transition. This catches a caller
    -- that "declares ready" a scenario that already is — which should be a no-op
    -- upstream, not a row that makes the history read as if something happened.
    CONSTRAINT scenario_status_transitions_actually_transitions
        CHECK (from_status <> to_status)
);

CREATE INDEX idx_scenario_status_transitions_scenario
    ON scenario_status_transitions (scenario_id, at DESC);

COMMENT ON TABLE scenario_status_transitions IS
    'Append-only record of every scenario ready/drafted transition: who declared '
    'it, and when. NOT the ruling ledger (that is scenario_ruling_anchors, for '
    'evidence rulings with anchors). No FK on scenario_id and no cascade — the '
    'record of a human act must outlive the row it was about.';

-- ─── 2. The watch-list note ────────────────────────────────────────────────────
--
-- v2 §10's fourth rehearsal block is "what the other side will wave around". Phase
-- 2 computes part of it from the evidence cut; the human-flagged notes are
-- authored, and they are structurally identical to a human fact: text, an author,
-- no citation.
--
-- ## Why a discriminator and not a sibling table
--
-- §8's invariants must apply IDENTICALLY to watch-list notes — no scan may write
-- them, editing them must not trigger a gather. One table means one write path,
-- one entry in the scan allowlist, and the existing source-scan guards cover the
-- new kind for free. A sibling table would mean duplicating every guard, and task
-- 1.4's own gate review found exactly that kind of asymmetry (C5 had no
-- caller-family test while C4 did) — a lesson worth not repeating in the same
-- phase.
--
-- Phase 2 also merges these with the computed watch-list on one panel. With a
-- discriminator that is a filter; with two tables it is a union.
--
-- DEFAULT 'fact' backfills existing rows TRUTHFULLY: everything written before
-- this migration was authored through the human-facts form and is a fact.
ALTER TABLE scenario_human_facts
    ADD COLUMN kind TEXT NOT NULL DEFAULT 'fact';

COMMENT ON COLUMN scenario_human_facts.kind IS
    'fact | watch_list. Both are human-authored, uncited, and equally protected '
    'by the v2 §8 invariants — which is why they share one table and one write '
    'path. Validated in code by the HumanFactKind enum, not a DB CHECK, matching '
    'the evolvable-vocabulary discipline used elsewhere in this schema.';
