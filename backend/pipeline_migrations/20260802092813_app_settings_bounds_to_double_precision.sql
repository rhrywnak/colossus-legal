-- app_settings_bounds_to_double_precision: App settings bounds to double precision
--
-- Created: 2026-08-02 09:28:13
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- HOTFIX for the v2.0.0-beta.364 boot failure. The backend applied all 51
-- migrations, seeded the store, then refused to start on the FIRST live read:
--
--   error occurred while decoding column "min_value": mismatched types;
--   Rust type `core::option::Option<f64>` (as SQL type `FLOAT8`) is not
--   compatible with SQL type `NUMERIC`
--
-- The refusal was correct (v2 §16, and §2b leaves nothing to fall back to).
-- What was wrong is this table.
--
-- ## Why NUMERIC could never have worked here
--
-- sqlx implements `Type<Postgres>` for `f64` against `FLOAT8` only. `NUMERIC`
-- decodes solely into `Decimal` / `BigDecimal`, and NEITHER `rust_decimal` NOR
-- `bigdecimal` is in this workspace's dependency tree. So a bare `NUMERIC`
-- column in colossus-legal has no decodable Rust type AT ALL — this was not a
-- lossy-coercion warning that sqlx happened to be strict about, it was a column
-- nothing in the binary could read.
--
-- ## Why the schema moves and not the code
--
-- Every OTHER `NUMERIC` column in this database is read through a `::float8`
-- cast in its projection (`llm_models.cost_per_*_token` and
-- `default_temperature`, `processing_profiles.temperature`,
-- `scan_runs.computed_cost`, `extraction_runs.cost_usd`). That cast is the
-- house pattern, and it is the RIGHT pattern there: those columns are money or
-- carry a declared 2-decimal contract, so exact decimal storage is deliberate
-- and the cast is a read-side accommodation of it.
--
-- These two columns never earned that. The 1.6 migration's own comment says
-- "NUMERIC rather than TEXT because these are compared, never displayed raw" —
-- the choice under consideration was NUMERIC-vs-TEXT, and nothing ever wanted
-- decimal exactness. Both values become `f64` the moment they leave the row:
-- `Bounds::check` compares them as `f64`, `bounds_label` formats them as `f64`.
-- Storing them as DOUBLE PRECISION makes the column BE what every consumer
-- already treats it as, and closes the seam at the source: a future
-- hand-written projection cannot reopen it, whereas a projection cast must be
-- remembered at every new query site forever.
--
-- ## Why the cast is lossless, measured
--
-- The entire numeric content of this table on DEV is {0, 1} and NULL — the
-- seven seeded rows carry min/max of (0,1), (0,1), (0,NULL), (1,NULL),
-- (1,NULL), (0,1), (NULL,NULL). Verified before this migration was written:
-- `min_value = trunc(min_value)` held for all seven rows in both columns. Every
-- value is exactly representable in f64 with room to spare, and stays so for
-- anything a human can type on the Settings page — a bound is a declaration of
-- range, not a measurement.
--
-- NOTE what this does NOT touch: `value` and `default_value` are TEXT ('0.80',
-- '9/10', '240') and are parsed by `domain::settings`. No stored PARAMETER
-- changes here. Two bound columns holding zeros and ones change storage type.
--
-- ## Why both columns in one statement
--
-- sqlx fails on the first undecodable column of a row, so `max_value` is an
-- IDENTICAL failure queued behind `min_value` — fixing one alone moves the same
-- boot refusal one column to the right. One statement, one table rewrite,
-- neither column forgettable.
--
-- The regression test that should have caught this ships with this migration:
-- `the_migration_declares_only_types_this_build_can_decode` (lib target, no
-- database needed) parses the CREATE TABLE and refuses any column whose
-- declared SQL type the decoding Rust type cannot read.

ALTER TABLE app_settings
    ALTER COLUMN min_value TYPE DOUBLE PRECISION USING min_value::float8,
    ALTER COLUMN max_value TYPE DOUBLE PRECISION USING max_value::float8;

COMMENT ON COLUMN app_settings.min_value IS
    'Inclusive lower bound, NULL meaning unbounded. DOUBLE PRECISION, not '
    'NUMERIC: the value is compared and displayed as an f64 by every consumer, '
    'and sqlx cannot decode NUMERIC in this build at all.';

COMMENT ON COLUMN app_settings.max_value IS
    'Inclusive upper bound, NULL meaning unbounded. DOUBLE PRECISION for the '
    'same reason as min_value.';
