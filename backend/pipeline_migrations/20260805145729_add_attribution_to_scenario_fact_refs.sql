-- add_attribution_to_scenario_fact_refs: who weighed it, who placed it, and when
--
-- Created: 2026-08-05 14:57:29
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- Task 2.13c, item 15. From the .378 acceptance click-through: ten facts on S-2
-- carried `tier = 'background'` and NOTHING recorded who put them there or when.
-- The most likely author was Roman testing his own deploy, but "most likely" is
-- the whole problem — a curation judgment on a litigation matter had no author.
-- Rulings have carried an actor since 1a.3 (`scenario_ruling_anchors.ruled_by`);
-- weight and order shipped in 2.13 without one. This closes that.
--
-- ## Four columns, not two
--
-- Weighing a fact and placing it are INDEPENDENT acts under the every-card-
-- independent law: one human may decide a fact carries the scenario while another
-- decides where it sits. A single `curated_by` would answer neither question
-- reliably — whoever wrote last would erase the other. So each act records its
-- own actor and its own timestamp.
--
-- ## NULLABLE, and the NULL is meaningful
--
-- NULL means "no human has performed this act on this row". That is the honest
-- state for every pre-existing row, and it stays honest for a fact nobody has
-- weighed or placed. The alternative — backfilling an author — would put a name
-- against a decision that person did not make, which is worse than no name at
-- all on a record that may be read in a legal context (Standing Rule 1: the
-- unknown state must stay distinguishable, never guessed).
--
-- ## Current state, NOT a history — ruled 2026-08-05
--
-- These columns answer "who last touched this", and re-weighting overwrites the
-- previous author with no trace. That is deliberate for this task: weight and
-- order are PRESENTATION judgments, and the architect's ruling is that
-- current-state meets the need. They follow `defer_reason`'s precedent, which is
-- likewise current-state with the history living in the ledger.
--
-- The difference worth stating plainly: tier and order have NO ledger, so there
-- is no history anywhere. If the forensic standard ever requires one, it is its
-- own append-only table and its own task — FILED, deliberately not built here.
--
-- FORWARD-ONLY: no down migration. A correction is a further forward migration.

ALTER TABLE scenario_fact_refs
    ADD COLUMN tier_updated_by TEXT;

ALTER TABLE scenario_fact_refs
    ADD COLUMN tier_updated_at TIMESTAMPTZ;

ALTER TABLE scenario_fact_refs
    ADD COLUMN order_updated_by TEXT;

ALTER TABLE scenario_fact_refs
    ADD COLUMN order_updated_at TIMESTAMPTZ;

COMMENT ON COLUMN scenario_fact_refs.tier_updated_by IS
    'The username that last set this fact''s weight, or NULL when no human has weighed '
    'it. Never backfilled: a name against a decision somebody did not make is worse '
    'than no name. Current state only — there is no weight history.';

COMMENT ON COLUMN scenario_fact_refs.tier_updated_at IS
    'When the weight was last set, or NULL alongside tier_updated_by. The pair moves '
    'together; one without the other is half a fact.';

COMMENT ON COLUMN scenario_fact_refs.order_updated_by IS
    'The username that last placed this fact in the scenario''s order, or NULL when '
    'nobody has dragged it. Separate from tier_updated_by because weighing and placing '
    'are independent acts and either human may be the more recent.';

COMMENT ON COLUMN scenario_fact_refs.order_updated_at IS
    'When the position was last set, or NULL alongside order_updated_by.';
