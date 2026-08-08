-- scenario_authoring_wording: the thirteen strings that define a scenario
--
-- Created: 2026-08-07 15:54:19
-- Target: pipeline database (colossus_legal_v2)
--
-- Task CC_FIX_SCENARIO_DEFINITION_AUTHORING_v1, from the 2026-08-07 diagnostic.
--
-- ## What these rows are for
--
-- On 2026-08-07 a scenario created from the UI arrived looking full: 148
-- candidate cards, identical to the scenario curated beside it. The measured
-- cause was not a copy — it was that the create form cannot author a scenario's
-- `definition`, so every scenario it makes is born with `{}`, and a definition
-- with no `target` silently fell back to the case-default subject. The user had
-- no way to tell "the pool I chose" from "the pool a default chose for me".
--
-- These rows are the words for the fix: the two fields the create form gains
-- (target and the plain-language accusation), the same two on the identity modal
-- so an existing scenario can be completed, and — the load-bearing one — the
-- notice a target-less scenario now shows IN PLACE OF a borrowed candidate pool.
--
-- ## Why `ON CONFLICT (key) DO NOTHING` and no down migration
--
-- Same rule every sibling wording migration follows: seeding is idempotent, and
-- a re-run must never overwrite a value Roman has since edited on the Settings
-- page. Forward-only — the backend applies these at boot via the runtime
-- Migrator, and a wording row removed underneath a running build would make the
-- boot loader refuse to start (which is the intended failure, not a rollback
-- path).
--
-- ## Deploy ordering (the one hazard)
--
-- `SCENARIO_AUTHORING_WORDING_KEYS` declares all thirteen to the boot loader, and
-- a declared key with no row is a REFUSAL to start (v2 §2b — there are no
-- compiled-in defaults to serve with). This migration must therefore be applied
-- before or with the backend image that reads it. It is applied at backend boot
-- by the Migrator, so the ordering holds automatically on a normal deploy.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    -- ── The create form, on the Trial Prep dashboard ─────────────────────────
    --
    -- The label asks in the second person and names the scenario, not the data
    -- model: the human filling this in is choosing a person, not populating
    -- `definition.target`.
    ('scenario_create_target_label', 'Who this scenario is about', 'text',
     'Who this scenario is about',
     NULL, NULL,
     'Labels the target selector on the create-scenario form. The target is the '
     'party every candidate fact must be ABOUT, and it is what the evidence pool '
     'is gathered over.',
     NULL, now(), 'system (seed)'),

    -- Says what the field DOES. A helper reading "select a target" would be a
    -- restatement of the label; this one answers the question the empty
    -- scenario of 2026-08-07 raised.
    ('scenario_create_target_helper',
     'Evidence is gathered about this person and nobody else. A scenario with no target gathers nothing.',
     'text',
     'Evidence is gathered about this person and nobody else. A scenario with no target gathers nothing.',
     NULL, NULL,
     'Sits under the target selector on the create form. States the consequence '
     'of the choice — including the consequence of not making one — because the '
     'person who skips this field is the person who later asks why the scenario '
     'is empty (or, before this fix, why it was full of somebody else''s pool).',
     NULL, now(), 'system (seed)'),

    -- Never a person's name: a pre-selected human being reads, ten minutes
    -- later, as a choice somebody made deliberately.
    ('scenario_create_target_unset_option', 'Choose a person…', 'text',
     'Choose a person…',
     NULL, NULL,
     'The unselected option in the create form''s target list. Deliberately not '
     'a person''s name — a pre-filled target is indistinguishable from a chosen '
     'one, which is the exact confusion this task exists to end.',
     NULL, now(), 'system (seed)'),

    ('scenario_create_accusation_label', 'The accusation, in plain language',
     'text', 'The accusation, in plain language',
     NULL, NULL,
     'Labels the accusation textarea on the create-scenario form. Stored as '
     'definition.attack_meaning — the sentence the Theme Scan judges candidates '
     'against.',
     NULL, now(), 'system (seed)'),

    -- Names the consumer. A writer who knows an LLM reads this sentence writes
    -- a different sentence than one who thinks it is a caption.
    ('scenario_create_accusation_helper',
     'What the other side is actually saying about this person. The scan judges every candidate fact against these words, so write them the way you would say them out loud.',
     'text',
     'What the other side is actually saying about this person. The scan judges every candidate fact against these words, so write them the way you would say them out loud.',
     NULL, NULL,
     'Sits under the accusation textarea on the create form. Names the consumer '
     '(the Theme Scan) so the author knows who reads the sentence and writes for '
     'that reader.',
     NULL, now(), 'system (seed)'),

    -- ── The two create refusals ──────────────────────────────────────────────
    --
    -- Each names the consequence rather than the rule. "Target is required"
    -- states a constraint; these state what goes wrong without it.
    ('scenario_create_target_required_refusal',
     'Choose who this scenario is about — a scenario with no target cannot gather any evidence.',
     'text',
     'Choose who this scenario is about — a scenario with no target cannot gather any evidence.',
     NULL, NULL,
     'The refusal when a scenario is created with no target. Names the '
     'consequence, not the constraint: "required" tells a human what the form '
     'wants, this tells them what breaks.',
     NULL, now(), 'system (seed)'),

    ('scenario_create_accusation_required_refusal',
     'Write the accusation in plain language — without it the scan has nothing to judge candidate facts against.',
     'text',
     'Write the accusation in plain language — without it the scan has nothing to judge candidate facts against.',
     NULL, NULL,
     'The refusal when a scenario is created with a blank accusation sentence. '
     'Names what the sentence is FOR, since a scenario missing it looks complete '
     'until a scan is started against it.',
     NULL, now(), 'system (seed)'),

    -- ── The no-target notice: the sentence this whole task exists for ────────
    --
    -- Rendered IN PLACE OF the candidate queue. Before this row, a target-less
    -- scenario showed 148 cards gathered over a subject nobody chose.
    ('scenario_no_target_notice',
     'No target defined — this scenario cannot gather evidence. Use Edit identity to name who it is about.',
     'text',
     'No target defined — this scenario cannot gather evidence. Use Edit identity to name who it is about.',
     NULL, NULL,
     'Shown where the candidate queue would be, on a scenario whose definition '
     'names no target. THE sentence of the 2026-08-07 fix: the state it '
     'describes previously rendered as a full pool borrowed from the '
     'case-default subject, with only a debug log to distinguish it. Names the '
     'control that fixes it, so the notice is also the instruction.',
     NULL, now(), 'system (seed)'),

    -- ── The identity modal, on the scenario working page ─────────────────────
    ('scenario_identity_target_label', 'Who this scenario is about', 'text',
     'Who this scenario is about',
     NULL, NULL,
     'Labels the target selector in the identity modal. Its own row rather than '
     'the create form''s, because the two moments differ: creating asks you to '
     'choose, editing warns you what changes.',
     NULL, now(), 'system (seed)'),

    -- The reassurance is load-bearing. A curator with two weeks of rulings will
    -- not touch a control that might discard them.
    ('scenario_identity_target_helper',
     'Changing this changes which evidence the scenario gathers. Rulings you have already made are kept.',
     'text',
     'Changing this changes which evidence the scenario gathers. Rulings you have already made are kept.',
     NULL, NULL,
     'Sits under the identity modal''s target selector. Warns that the effect '
     'reaches past the field, and says rulings survive — without that second '
     'sentence a curator with weeks of decisions will not touch the control at '
     'all.',
     NULL, now(), 'system (seed)'),

    ('scenario_identity_target_unset_option', 'Not set — gathers nothing',
     'text', 'Not set — gathers nothing',
     NULL, NULL,
     'The "no target" option in the identity modal. Names the consequence so an '
     'unset target never reads as a neutral blank.',
     NULL, now(), 'system (seed)'),

    ('scenario_target_options_failed_notice',
     'Could not load the people in this case. Close and reopen — do not save, or this scenario''s target would be cleared.',
     'text',
     'Could not load the people in this case. Close and reopen — do not save, or this scenario''s target would be cleared.',
     NULL, NULL,
     'Shown when the party vocabulary cannot be read. Says explicitly not to '
     'save: a modal whose target list failed to load would otherwise write back '
     'an emptied target, turning a transient graph fault into a silently '
     'un-gathering scenario.',
     NULL, now(), 'system (seed)'),

    -- The one combination the modal cannot store. `attack_text` is required by
    -- the definition's parse contract, so "a target but no attack text" has no
    -- valid stored form — and the modal's older behaviour for that case was to
    -- omit the definition entirely, which would drop the target without a word.
    ('scenario_identity_target_needs_attack_text',
     'Write what they say before choosing who this is about — a scenario stores the two together, and saving now would lose the person you picked.',
     'text',
     'Write what they say before choosing who this is about — a scenario stores the two together, and saving now would lose the person you picked.',
     NULL, NULL,
     'Refuses a save that names a target while "what they say" is still blank. '
     'Reachable only on scenarios created before the create form asked for '
     'either field. Named rather than silently dropped: the previous behaviour '
     'omitted the whole definition, and the human watched their chosen target '
     'disappear on reopen with nothing said.',
     NULL, now(), 'system (seed)')

ON CONFLICT (key) DO NOTHING;
