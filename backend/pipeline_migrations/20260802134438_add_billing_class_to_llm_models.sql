-- add_billing_class_to_llm_models: Add billing class to llm models
--
-- Created: 2026-08-02 13:44:38
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- Task 1.7B. The scan control must list LOCAL models first, default to one, and
-- label the others "(API — billed)" so nobody starts a 148-candidate scan on a
-- metered endpoint by accident. Today NOTHING in the system states which a model
-- is. This migration makes it a stored fact.
--
-- ## Why a column and not an inference
--
-- Three proxies exist and all three were rejected:
--
--   provider = 'vllm'      — that is which CLIENT speaks to the endpoint, not who
--                            pays. A self-hosted OpenAI-compatible gateway and a
--                            rented vLLM cloud both break it, and encoding
--                            "anthropic means billed" in Rust would put a
--                            deployment fact in the binary (Rule 13).
--
--   api_endpoint IS NOT NULL — same objection: routing, not billing.
--
--   cost_per_input_token > 0 — the closest, and correct on all seven rows today,
--                            but its failure mode is the expensive one: a NULL
--                            cost means UNKNOWN, never FREE. An API model whose
--                            costs were never filled in would read as local, and
--                            the page would tell a human a scan is free minutes
--                            before it bills them. A UI that renders unknown as
--                            free is worse than one that refuses to guess.
--
-- So the fact is stored, plainly, and the label and ordering are composed from
-- it. `llm_models` already carries deployment truth per row (endpoint, costs,
-- capability tokens); this is one more of the same kind.
--
-- ## Why TEXT and not BOOLEAN
--
-- `is_local = false` answers "not local" — which is not the same claim as
-- "billed", and the difference will matter the first time a third class appears
-- (a free hosted tier, a metered self-host). Plain words also read in psql
-- without a lookup, the same argument `value_kind` carries in `app_settings`.
-- The vocabulary lives in code (`domain::billing_class`), not in a CHECK, for
-- the reason the settings store gives: a CHECK cannot tell you the PARSER is
-- missing, and an unknown token must be refused loudly by the reader.

-- ─── 1. The column ─────────────────────────────────────────────────────────────
--
-- Added with a DEFAULT so the NOT NULL can go on in one statement: every
-- existing row gets a value from the backfill below before anything reads it,
-- and a row inserted by an older binary that does not know the column still
-- lands somewhere honest.
--
-- The default is 'billed', deliberately the CAUTIOUS side: a model nobody has
-- classified is one we cannot promise is free, and the page says so. The reverse
-- default would make an unclassified model look free — the exact failure mode
-- that ruled out the cost-column inference above.
ALTER TABLE llm_models
    ADD COLUMN billing_class TEXT NOT NULL DEFAULT 'billed';

COMMENT ON COLUMN llm_models.billing_class IS
    'Who pays for a call to this model: ''local'' (self-hosted, no metered cost) '
    'or ''billed'' (third-party API, metered). The scan picker orders local '
    'first, defaults to one, and labels the rest "(API — billed)". NOT derivable '
    'from provider or from a NULL cost — a NULL cost means unknown, never free.';

-- ─── 2. The backfill ───────────────────────────────────────────────────────────
--
-- Seeded from `provider` because that is the best evidence available TODAY for
-- rows that already exist: every vllm row on DEV is a self-hosted Qwen on the
-- LAN, every anthropic row is metered. This is a one-time classification of
-- seven known rows, not a rule — after this migration the column is authored,
-- and adding a self-hosted model with a different provider is a value a human
-- sets, not a name the code matches on.
UPDATE llm_models
   SET billing_class = 'local'
 WHERE provider = 'vllm';

-- ─── 3. The visible record ─────────────────────────────────────────────────────
--
-- A migration that classifies rows should say what it classified, so the next
-- reader can check it against the models that exist rather than trust a comment.
-- Expected on DEV at the time of writing: 2 local (Qwen 14B / 32B AWQ), 5 billed
-- (claude-opus-4-6/4-7/4-8, claude-sonnet-4-6/5).
DO $$
DECLARE
    local_count  INTEGER;
    billed_count INTEGER;
BEGIN
    SELECT count(*) FILTER (WHERE billing_class = 'local'),
           count(*) FILTER (WHERE billing_class = 'billed')
      INTO local_count, billed_count
      FROM llm_models;

    RAISE NOTICE 'billing_class backfill: % local, % billed', local_count, billed_count;
END $$;
