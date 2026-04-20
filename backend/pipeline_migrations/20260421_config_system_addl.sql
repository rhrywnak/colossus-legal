-- Additional per-document override columns on pipeline_config.
--
-- Fixes a design-oversight omission from 20260420_config_system.sql:
-- extraction_model and max_tokens are first-class profile parameters
-- and therefore need per-document override columns too. Without these,
-- the ConfigurationPanel could display them but the overrides could not
-- be persisted.
--
-- Additive — no existing data is modified. Both columns are nullable.

ALTER TABLE pipeline_config
    ADD COLUMN IF NOT EXISTS extraction_model TEXT,
    ADD COLUMN IF NOT EXISTS max_tokens INTEGER;
