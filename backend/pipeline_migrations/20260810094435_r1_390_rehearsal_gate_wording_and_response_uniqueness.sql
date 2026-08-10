-- r1_390_rehearsal_gate_wording_and_response_uniqueness
--
-- Created: 2026-08-10 09:44:35
-- Target: pipeline database (colossus_legal_v2)
--
-- Task CC_TASK_R1_SURFACE_REPAIR_BATCH_v1, .390 correctness batch, under the
-- rulings in CC_TASK_R1_RULINGS_v1.
--
-- Two unrelated things ride together because they are one deploy: three wording
-- rows the .390 build reads at boot, and the constraint that makes a scenario's
-- talking points a single row by construction rather than by convention.
--
-- ## Part 1 — the three wording rows
--
-- `scenario_rehearsal_link_blocked_reason` (Piece 1b). Until .390 the scenario
-- page's "Rehearsal view →" control rendered identically on a Draft scenario and
-- a Ready one, and clicking it on a Draft one delivered a DIFFERENT scenario's
-- rehearsal (the S-5 → S-2 substitution, audit defects 1-4). The control is
-- status-gated now, and a gated control has to say why it is gated — in Roman's
-- words, not in a hover tooltip nobody finds.
--
-- `scenario_identity_meaning_needs_attack_text` (Piece 5a). Its sibling
-- `scenario_identity_target_needs_attack_text` already refuses a save that would
-- drop a chosen TARGET. The same omission silently dropped a typed
-- "what that is meant to imply" (audit defect 16), and the two need different
-- sentences: they name different fields and different remedies, and one sentence
-- covering both would tell half the readers the wrong thing.
--
-- `rehearsal_picker_heading` (Piece 1d, Roman's ruling). A bare
-- `/cases/:slug/rehearsal` used to open on the first Ready scenario by default.
-- Roman ruled that nothing is ever shown that Marie did not pick, so the address
-- renders a short list of the Ready scenarios instead. This is that list's title.
-- Its EMPTY state is not a new row: `rehearsal_nothing_ready_notice` already
-- says exactly the right thing and is reused rather than duplicated.
--
-- ## Why `ON CONFLICT (key) DO NOTHING` and no down migration
--
-- The rule every sibling wording migration follows: seeding is idempotent, and a
-- re-run must never overwrite a value Roman has since edited on the Settings
-- page. Forward-only — the backend applies these at boot via the runtime
-- Migrator, and a wording row removed underneath a running build would make the
-- boot loader refuse to start (the intended failure, not a rollback path).
--
-- ## Deploy ordering (the standing hazard)
--
-- `SCENARIO_AUTHORING_WORDING_KEYS` and `REHEARSAL_WORDING_KEYS` declare these
-- three to the boot loader, and a declared key with no row is a REFUSAL to start
-- (v2 §2b — there are no compiled-in defaults to serve with). This migration
-- must therefore be applied before or with the backend image that reads it. It
-- is applied at backend boot by the Migrator, so the ordering holds on a normal
-- deploy. The reverse direction is safe: rolling back to .389 leaves three rows
-- nothing reads.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    -- ── Piece 1b: the gated rehearsal control ────────────────────────────────
    --
    -- `{status}` is REQUIRED (see `wording_templates::REQUIRED_PLACEHOLDERS`) and
    -- is filled with the status the scenario actually carries, not the word
    -- "Draft". The column permits `needs_evidence` and ruling 6 of
    -- CC_TASK_R1_RULINGS_v1 deliberately keeps that value READABLE, so a sentence
    -- hardcoding "Draft" would state a falsehood on the one row it was written to
    -- explain. The placeholder is what keeps the sentence true for every value
    -- the CHECK constraint allows.
    ('scenario_rehearsal_link_blocked_reason',
     'Not in rehearsal — this scenario is {status}. Switch it to Ready on this page first.',
     'text',
     'Not in rehearsal — this scenario is {status}. Switch it to Ready on this page first.',
     NULL, NULL,
     'Why the "Rehearsal view" control on a scenario page is inert. Rehearsal '
     'mode serves Ready scenarios only (v2 §5/§10), and before .390 the control '
     'looked alive on every scenario and silently delivered a different one. '
     '{status} must stay in the text — it names the state the scenario is '
     'actually in, and the column permits more than one non-Ready value.',
     'ScenarioHeaderTiers (scenario page header)', NOW(), 'migration'),

    -- ── Piece 5a: the second half of the definition-loss refusal ─────────────
    --
    -- Deliberately NOT a reworded version of the target sentence. A human who
    -- typed a gloss and a human who picked a person have made different edits and
    -- need different instructions; one sentence covering both would name the
    -- wrong field for whichever of them is reading it.
    ('scenario_identity_meaning_needs_attack_text',
     'Write what they say before writing what it is meant to imply — a scenario stores the two together, and saving now would lose the words you just typed.',
     'text',
     'Write what they say before writing what it is meant to imply — a scenario stores the two together, and saving now would lose the words you just typed.',
     NULL, NULL,
     'Refuses a save that carries a "what that is meant to imply" gloss while '
     '"what they say" is still blank. The stored definition requires an '
     'attack_text, so the whole definition is omitted without one — which used to '
     'discard the typed gloss with nothing said (the .389 defect this row '
     'closes). Sibling of scenario_identity_target_needs_attack_text; the two '
     'name different fields on purpose.',
     'ScenarioIdentityModal (identity editor)', NOW(), 'migration'),

    -- ── Piece 1d: the rehearsal front door ──────────────────────────────────
    --
    -- The list's title. Its empty state reuses `rehearsal_nothing_ready_notice`
    -- rather than adding a second sentence that would have to be kept in step
    -- with the first.
    ('rehearsal_picker_heading',
     'Choose a scenario to rehearse',
     'text',
     'Choose a scenario to rehearse',
     NULL, NULL,
     'The heading over the list of Ready scenarios shown at the bare rehearsal '
     'address. Roman ruled 2026-08-10 that nothing is ever shown that Marie did '
     'not pick, which retired the old behaviour of opening on the first Ready '
     'scenario by default.',
     'RehearsalPage (rehearsal front door)', NOW(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ── Part 2 — one talking-points row per scenario, by construction ───────────
--
-- `scenario_responses` has carried one row per scenario since it was created,
-- but only by convention: three read/write paths take `.first()` off the list
-- (`services::rehearsal_assembly`, and two in `services::scenario_augmentation`),
-- and nothing in the schema stopped a second row from appearing. A second row
-- would have made the REHEARSAL page — the surface a witness works from — render
-- the older row's points silently.
--
-- Measured on DEV before shipping this: 1 row total across the whole database,
-- owned by S-2, and zero scenarios with more than one. The constraint is
-- satisfied by the current data with room to spare.
--
-- If it ever fails on another environment, that is the constraint doing its job:
-- the duplicate rows have to be reconciled by hand, because only a human can say
-- which set of talking points is the real one.
ALTER TABLE scenario_responses
    ADD CONSTRAINT scenario_responses_scenario_id_key UNIQUE (scenario_id);

-- The plain index is superseded: a UNIQUE constraint is backed by its own unique
-- index on the same column, so keeping this one means two indexes doing one job —
-- twice the write cost and two names for the same lookup.
DROP INDEX IF EXISTS idx_scenario_responses_scenario;
