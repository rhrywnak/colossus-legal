-- timeline_subsets: named, ordered subsets of the one case chronology, and the
-- link that attaches them to scenarios
--
-- Created: 2026-08-30
-- Target: pipeline database (colossus_legal_v2)
--
-- TIMELINE_SUBSET_DESIGN_v1 §4 and §10, as amended by Roman's rulings of
-- 2026-08-30 (date order by default with manual reorder; many-to-many kept).
-- CASE_CHRONOLOGY_DESIGN_v2's own rules hold over all of it: R1 (ONE event
-- table, filtered views over it), R4 (additive only), R10 (delete is soft).
--
-- ## ⚑ A SUBSET HOLDS REFERENCES, NEVER COPIES
--
-- The design's first ruling, and the shape below is what makes it structural
-- rather than a habit: `chronology_subset_events` carries a subset id, an event
-- id, a position and a note — and NO title, NO date, NO fact, ever. There is
-- nowhere in this schema for a copy of an event to live, so an edit to an event
-- shows up in every subset that references it, and a soft-deleted event becomes
-- a marked GAP in the subset rather than a stale line nobody notices is wrong.
--
-- That is the whole reason the field's tools model a chronology this way
-- (Everchron's issue chronologies are the master timeline filtered, not copied),
-- and it is the reason this migration adds no column that could ever drift from
-- `chronology_events`.
--
-- ## Why `case_slug` and not `case_id`
--
-- The design brief names the column `case_id`. There is no such column anywhere
-- in this database: `chronology_events`, `scenarios` and `authored_entities` all
-- carry `case_slug TEXT`, ruled 2026-08-25 (Phase A report, R-C — "the case
-- column is `case_slug`"). One word for one fact across the whole database
-- matters more than matching a brief's shorthand, so the name here is the name
-- everything else already uses, with the same type and the same no-FK
-- discipline (cases are not a table in this database).
--
-- ## Nothing here hard-deletes except the scenario link
--
-- A subset soft-deletes (`deleted_at`) with an Undo, like an event. The
-- `scenario_subsets` row is the exception and is deleted outright, because a
-- link is not content: detaching a subset from a scenario removes a pointer, and
-- the subset, its events and its history are all still there to re-attach.

-- ─── 1 · chronology_subsets — the named story ────────────────────────────────

CREATE TABLE IF NOT EXISTS chronology_subsets (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Which case this subset belongs to. Same name, same type and same absence
    -- of a foreign key as `chronology_events.case_slug` — see the header.
    case_slug   TEXT NOT NULL,
    -- Short, and spoken out loud: "The $50,000".
    name        TEXT NOT NULL,
    -- One or two sentences: what this story proves. NOT NULL DEFAULT '' rather
    -- than nullable, because "no description" and "an empty description" are the
    -- same state for a field a human types into — unlike a link's pinpoint,
    -- whose absence is deliberately marked on screen.
    description TEXT NOT NULL DEFAULT '',
    -- Who wrote it and who touched it last, from the Authentik login via
    -- `services::practice_notes::attribution`. NOT NULL, unlike the equivalent
    -- columns on `chronology_events`: that table had to tolerate rows the seed
    -- one-shot wrote before any human existed, and a subset has no such history
    -- — it can only be born through the guarded write path, which always has a
    -- signed-in user.
    created_by  TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by  TEXT NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Soft delete (chronology R10). NULL = live.
    deleted_at  TIMESTAMPTZ
);

-- Two LIVE subsets in one case cannot share a name, case-insensitively.
--
-- A partial unique INDEX and not a table constraint, for two reasons a
-- constraint cannot give: it is expressed over `lower(name)` (so "The $50,000"
-- and "the $50,000" collide, which is what a human means by "same name"), and it
-- is scoped `WHERE deleted_at IS NULL` (so a deleted subset does not squat on
-- its name forever). Postgres allows neither an expression nor a predicate in a
-- UNIQUE table constraint.
CREATE UNIQUE INDEX IF NOT EXISTS chronology_subsets_case_name_live_unique
    ON chronology_subsets (case_slug, lower(name))
    WHERE deleted_at IS NULL;

-- The home section's read: every live subset for one case.
CREATE INDEX IF NOT EXISTS idx_chronology_subsets_case
    ON chronology_subsets (case_slug, deleted_at);

-- ─── 2 · chronology_subset_events — the references, in story order ───────────

CREATE TABLE IF NOT EXISTS chronology_subset_events (
    subset_id UUID NOT NULL REFERENCES chronology_subsets(id) ON DELETE CASCADE,
    -- ⚑ THE REFERENCE. There is no title, no date and no fact column here, and
    -- there never will be (design §4). The read joins `chronology_events`.
    event_id  UUID NOT NULL REFERENCES chronology_events(id) ON DELETE CASCADE,
    -- The STORY order. Defaults to date order when the picker fills it, and the
    -- author may move a line — ruling 2026-08-30 (1). Integer rather than a
    -- fraction: the replace endpoint rewrites the whole ordered set in one
    -- transaction, so there is never a need to insert between two positions.
    position  INTEGER NOT NULL,
    -- One line: why this event is in this story. Same NOT NULL DEFAULT ''
    -- reasoning as `description` above.
    note      TEXT NOT NULL DEFAULT '',
    added_by  TEXT NOT NULL,
    added_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- The natural key: one event appears in one subset at most once. Nothing had
    -- to be invented to address a row, and it leads on `subset_id`, so the
    -- per-subset read needs no second index.
    PRIMARY KEY (subset_id, event_id)
);

-- Two events in one subset cannot share a position.
--
-- ## ⚑ DEFERRABLE INITIALLY DEFERRED, and why that is not laziness
--
-- The picker's Save REPLACES the ordered set in one transaction: rows leave,
-- rows arrive, and rows that stay may swap positions with each other. Any
-- reordering of two rows passes through a moment where both hold the same
-- number, and an IMMEDIATE constraint would abort the transaction at that
-- moment — forcing the write path to invent a shuffle through temporary
-- positions, which is code whose only purpose is to lie to a constraint.
-- Deferred, the check happens once at COMMIT, against the state that will
-- actually be stored. The invariant is exactly as strong; only the instant it is
-- tested moves.
--
-- The PRIMARY KEY above stays IMMEDIATE, which matters: it is the arbiter for
-- the upsert's ON CONFLICT, and Postgres cannot use a deferred constraint there.
ALTER TABLE chronology_subset_events
    DROP CONSTRAINT IF EXISTS chronology_subset_events_position_unique;
ALTER TABLE chronology_subset_events
    ADD CONSTRAINT chronology_subset_events_position_unique
    UNIQUE (subset_id, position) DEFERRABLE INITIALLY DEFERRED;

-- The REVERSE read: "which subsets carry this event". The primary key leads on
-- subset_id and so cannot answer it; the timeline needs to, to mark an event
-- that a story depends on before somebody deletes it.
CREATE INDEX IF NOT EXISTS idx_chronology_subset_events_event
    ON chronology_subset_events (event_id);

-- ─── 3 · scenario_subsets — which scenario carries which story ───────────────
--
-- R2's "attachment is a first-class scenario field", done as a link table so it
-- survives many-to-many without a later migration: S-11 and S-12 both want "The
-- $50,000", and S-14 will want two subsets at once (ruling 2026-08-30 (2)).

CREATE TABLE IF NOT EXISTS scenario_subsets (
    -- Matches `scenarios.scenario_id` exactly — the column is named
    -- `scenario_id`, not `id`, and it is a UUID.
    scenario_id UUID NOT NULL REFERENCES scenarios(scenario_id) ON DELETE CASCADE,
    subset_id   UUID NOT NULL REFERENCES chronology_subsets(id) ON DELETE CASCADE,
    -- The order the scenario's View Timeline selector offers them in. DEFAULT 0
    -- because one attached subset is the common case and its position is
    -- uninteresting; the write path appends at the next position.
    position    INTEGER NOT NULL DEFAULT 0,
    attached_by TEXT NOT NULL,
    attached_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (scenario_id, subset_id)
);

-- The other direction: `carried_by` on the subset list — "carried by S-11,
-- S-12". The primary key leads on scenario_id, so this read needs its own index.
CREATE INDEX IF NOT EXISTS idx_scenario_subsets_subset
    ON scenario_subsets (subset_id);

-- ─── 4 · chronology_subset_history — append-only, snapshot per write ─────────
--
-- The same mechanism as `chronology_event_history` and for the same reason
-- (ruled 2026-08-25): a full JSONB SNAPSHOT after each write rather than one
-- typed column per field, because the field set grows forever and a typed
-- history would need a migration every time — and would silently stop recording
-- what it did not know about. The subset snapshot includes the ORDERED EVENT
-- LIST, so "which twelve, in what order, on that day" is answerable.
--
-- Nothing ever updates or deletes a row here.

CREATE TABLE IF NOT EXISTS chronology_subset_history (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subset_id  UUID NOT NULL REFERENCES chronology_subsets(id) ON DELETE CASCADE,
    action     TEXT NOT NULL,
    snapshot   JSONB NOT NULL,
    changed_by TEXT NOT NULL,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- A closed vocabulary, like the event history's. `events_replaced` is the
-- picker's Save: the whole ordered set is rewritten in one transaction and lands
-- ONE history row, because a person did one thing.
--
-- ⚑ Attaching a subset to a scenario is NOT in this list, deliberately. That
-- writes a `scenario_subsets` row, which is the SCENARIO's fact about the
-- subset, not a change to the subset's own content — and this table's snapshot
-- is the subset's content. See the T1 report's stated choice.
ALTER TABLE chronology_subset_history
    DROP CONSTRAINT IF EXISTS chronology_subset_history_action_valid;
ALTER TABLE chronology_subset_history
    ADD CONSTRAINT chronology_subset_history_action_valid
    CHECK (action IN ('created', 'updated', 'events_replaced', 'deleted', 'restored'));

CREATE INDEX IF NOT EXISTS idx_chronology_subset_history_subset
    ON chronology_subset_history (subset_id, changed_at);

-- ═══ 5 · The words these surfaces will speak (T1.2) ══════════════════════════
--
-- Sixteen strings, seeded HERE, one commit before the screens that render them.
--
-- ## ⚑ BOTH HALVES, OR NEITHER — and here, a third half on credit
--
-- A wording key is real only when all three parties agree: this migration holds
-- the row, `domain::wording_chronology` DECLARES the key, and the frontend asks
-- for it. Boot refuses to start if a declared key has no row here;
-- `dto::chronology_wording_reach_tests` refuses if the frontend asks for a name
-- no field carries — and refuses the OTHER way too, if a declared field has no
-- asker.
--
-- Tasks 2 and 3 are the askers, and they are separate branches. So each of these
-- sixteen is named in that test's `DECLARED_AHEAD_OF_THEIR_SCREEN` list, which
-- exists for exactly this: "declaring a key one commit before its screen costs
-- one line there, and that line is a promise with a name on it rather than a
-- silence". The list empties again as tasks 2 and 3 land.
--
-- ## Why they are chronology rows and not a block of their own
--
-- Same argument the Phase C write controls made: these are the SAME surface
-- family's words. The Subsets section sits under the phase sections on the
-- timeline page, and the picker IS the timeline list. A second block would mean
-- a second payload field, a second reach scan and a second place for a key to
-- hide.
--
-- The three `scenario_*` keys are the exception that proves it: they are spoken
-- on the scenario pages, not the timeline, but what they say is "there is a
-- timeline behind this button". They ride the same block because they are the
-- same feature's vocabulary. See the T1 report's FINDINGS for the delivery
-- question task 3 has to answer (the scenario pages do not read the timeline
-- payload today).

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_section_title',
    'Subsets',
    'text',
    'Subsets',
    NULL, NULL,
    'The heading over the Subsets section on the timeline home page (mockup Screen 1). Domain note: the section sits BELOW the phase sections and changes nothing about them — a subset is a read over the one chronology, never a second list of events.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_section_subtitle',
    'stories told in dates — references to the events above, never copies',
    'text',
    'stories told in dates — references to the events above, never copies',
    NULL, NULL,
    'The muted line under the Subsets heading. Domain note: it states the design''s first ruling on the screen where somebody could get it wrong. A reader who thinks a subset holds copies will expect an edit here not to show up there, and will eventually ask why the two disagree — they cannot.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_add_button',
    '+ Add subset',
    'text',
    '+ Add subset',
    NULL, NULL,
    'The control that opens the new-subset form. Domain note: it wears the same "+ " as chronology_add_event_label because it does the same kind of thing one level up — the glyph is in the stored string, so dropping it is an edit rather than a rebuild.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_carried_by_prefix',
    'Carried by',
    'text',
    'Carried by',
    NULL, NULL,
    'Introduces the scenario codes carrying a subset — "Carried by S-11, S-12". Domain note: stored WITHOUT a trailing space; the renderer supplies the joining one, because a stored value cannot carry a leading or trailing space through the settings store (it trims).',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_gap_count_template',
    '{count} gaps',
    'text',
    '{count} gaps',
    NULL, NULL,
    'How many of a subset''s events have been removed from the chronology. {count} is the number. Domain note: a gap is half the value of a subset — it is the story saying "this happened and it is not on our timeline yet", which is the to-do list the design asks the reader to see rather than a defect to hide.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_removed_event_line',
    'removed from the chronology — Undo lives on the timeline',
    'text',
    'removed from the chronology — Undo lives on the timeline',
    NULL, NULL,
    'The line a subset shows in place of an event that has been soft-deleted on the chronology (design R1). Domain note: the row is MARKED, never dropped. Dropping it would silently shorten a story somebody counted, and it names where the Undo is because the Undo is not here — it is on the timeline, on the event itself.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_size_line_template',
    'A story a person can hold is 12–20 events — this one is {count}.',
    'text',
    'A story a person can hold is 12–20 events — this one is {count}.',
    NULL, NULL,
    'The size line on the picker and on an over-long subset. {count} is this subset''s event count. Domain note: design §5D — twelve to twenty events is the size of a story a person can hold, and the page says so rather than enforcing it. The dash between 12 and 20 is an EN-DASH (U+2013) and must stay byte-exact.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_picker_hint',
    'Tick an event to add it. Order defaults to date; drag the number to change the story order. The note is optional — one line on why this event is in the story.',
    'text',
    'Tick an event to add it. Order defaults to date; drag the number to change the story order. The note is optional — one line on why this event is in the story.',
    NULL, NULL,
    'The instruction line at the top of the event picker. Domain note: it states ruling 2026-08-30 (1) — date order by default, manual reorder allowed — at the moment the author is deciding, which is the only moment the ruling is useful.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_picker_gap_hint',
    'Gaps are not on the chronology — add them with "+ Add event" on the timeline first; the picker only lists what exists.',
    'text',
    'Gaps are not on the chronology — add them with "+ Add event" on the timeline first; the picker only lists what exists.',
    NULL, NULL,
    'Shown in the picker when the author is looking for an event that is not there. Domain note: R1 again, from the other side — the picker cannot offer an event that does not exist, and saying so is what stops somebody concluding the search is broken. The quoted control name is chronology_add_event_label''s value; if that row is ever edited, edit this sentence with it.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_scenario_view_timeline_button',
    'View Timeline',
    'text',
    'View Timeline',
    NULL, NULL,
    'The control that opens a scenario''s attached subset in the floating window. Domain note: R2 as made explicit on 2026-08-30 — it appears on EVERY scenario view (home, detail, cards, questions, practice, rehearsal) whenever at least one subset is attached, and nowhere when none is.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_scenario_timeline_row_label',
    'Timeline:',
    'text',
    'Timeline:',
    NULL, NULL,
    'Labels the attached-subsets row in the scenario header (mockup Screen 2). Domain note: the colon is part of the stored words, so the component that renders the row contains no user-visible character of its own.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_scenario_attach_link',
    'Attach…',
    'text',
    'Attach…',
    NULL, NULL,
    'Opens the list of subsets this scenario could carry. Domain note: the ellipsis is a single character (U+2026) and means "this opens a chooser rather than acting", the same promise chronology_link_document_label makes.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_window_open_timeline',
    'Open on the timeline',
    'text',
    'Open on the timeline',
    NULL, NULL,
    'The floating window''s footer link to the full timeline page filtered to this subset (design §5C). Domain note: it opens the page, it does not navigate the page UNDER the window — the reader is answering a question and must not lose it.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_window_edit',
    'Edit subset',
    'text',
    'Edit subset',
    NULL, NULL,
    'Opens the subset''s name, description and picker from inside the floating window.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_window_footer_template',
    '{on_chronology} on the chronology · {gaps} gaps',
    'text',
    '{on_chronology} on the chronology · {gaps} gaps',
    NULL, NULL,
    'The floating window''s footer count. {on_chronology} is how many of the subset''s events are still live on the chronology; {gaps} is how many have been removed. Domain note: TWO numbers and not one total, because "fifteen events" over a list showing twelve live lines and three struck ones is the sentence that makes a reader distrust the count.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_empty_state',
    'No subsets yet. A subset is a story told in dates — pick events from the phases above.',
    'text',
    'No subsets yet. A subset is a story told in dates — pick events from the phases above.',
    NULL, NULL,
    'Shown in the Subsets section when the case holds none. Domain note: it teaches rather than reporting. This is the only screen where a reader meets the idea for the first time, and "No subsets yet." alone would leave them with nothing to do about it.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

-- ═══ 6 · ⚑ ROW-COUNT AND SHAPE ASSERTIONS (CLAUDE.md 25a) ════════════════════
--
-- A statement matching zero rows is SILENT in Postgres, and a wording key with
-- no row makes the BACKEND REFUSE TO START — a deploy taking DEV down,
-- discovered by the outage rather than by the migration. A CREATE TABLE IF NOT
-- EXISTS that found an existing table of a DIFFERENT shape is equally silent,
-- and would leave the endpoints failing on a column that is not there.
--
-- Both blocks below assert the END STATE, so they are equally true on a first
-- run and on a re-run where every IF NOT EXISTS and ON CONFLICT did nothing.

DO $$
DECLARE
    n INTEGER;
BEGIN
    -- ── the four tables exist ────────────────────────────────────────────────
    SELECT COUNT(*) INTO n FROM information_schema.tables
     WHERE table_schema = 'public'
       AND table_name IN ('chronology_subsets', 'chronology_subset_events',
                          'scenario_subsets', 'chronology_subset_history');
    IF n <> 4 THEN
        RAISE EXCEPTION 'timeline subsets needs all four tables, found %', n;
    END IF;

    -- ── and they are EMPTY, which is what "additive only" means here ─────────
    --
    -- Not decoration: if this migration ever runs against a database where one
    -- of these names was already taken by something else, the count is the only
    -- thing that would say so before the endpoints started answering with
    -- somebody else's rows.
    SELECT (SELECT COUNT(*) FROM chronology_subsets)
         + (SELECT COUNT(*) FROM chronology_subset_events)
         + (SELECT COUNT(*) FROM scenario_subsets)
         + (SELECT COUNT(*) FROM chronology_subset_history)
      INTO n;
    IF n <> 0 THEN
        RAISE EXCEPTION
            'the four timeline-subset tables must be empty after this migration, found % rows', n;
    END IF;

    -- ── the constraints, BY NAME ─────────────────────────────────────────────
    --
    -- Named rather than counted loosely, because each one is load-bearing and
    -- each fails differently: without the live-name index two subsets share a
    -- name and the picker shows the same story twice; without the deferred
    -- position constraint a reorder can store two events at the same number;
    -- without the action CHECK a typo becomes a history row nothing can read.
    SELECT COUNT(*) INTO n FROM pg_constraint
     WHERE conname IN ('chronology_subset_events_position_unique',
                       'chronology_subset_history_action_valid',
                       'chronology_subset_events_pkey',
                       'scenario_subsets_pkey',
                       'chronology_subsets_pkey');
    IF n <> 5 THEN
        RAISE EXCEPTION
            'the five named timeline-subset constraints must all exist, found %', n;
    END IF;

    -- The position constraint must be DEFERRABLE, or the replace endpoint
    -- cannot reorder without a shuffle. Asserted separately because a
    -- constraint of the right NAME and the wrong deferrability would pass the
    -- count above and fail at the first reorder a human tried.
    SELECT COUNT(*) INTO n FROM pg_constraint
     WHERE conname = 'chronology_subset_events_position_unique'
       AND condeferrable AND condeferred;
    IF n <> 1 THEN
        RAISE EXCEPTION
            'chronology_subset_events_position_unique must be DEFERRABLE INITIALLY DEFERRED';
    END IF;

    -- ── the partial unique index on the live name ────────────────────────────
    SELECT COUNT(*) INTO n FROM pg_indexes
     WHERE schemaname = 'public'
       AND indexname = 'chronology_subsets_case_name_live_unique';
    IF n <> 1 THEN
        RAISE EXCEPTION 'chronology_subsets_case_name_live_unique is missing';
    END IF;

    -- ── the foreign keys, by the table they point AT ─────────────────────────
    --
    -- Five: subset_events → subsets, subset_events → events,
    -- scenario_subsets → scenarios, scenario_subsets → subsets,
    -- subset_history → subsets. A missing one is a dangling reference the
    -- design's "references, never copies" rule cannot survive.
    SELECT COUNT(*) INTO n FROM pg_constraint c
     WHERE c.contype = 'f'
       AND c.conrelid::regclass::text IN ('chronology_subset_events',
                                          'scenario_subsets',
                                          'chronology_subset_history');
    IF n <> 5 THEN
        RAISE EXCEPTION
            'the timeline-subset tables need all five foreign keys, found %', n;
    END IF;

    -- ── ⚑ NO COPIED EVENT COLUMN, EVER (design §4) ──────────────────────────
    --
    -- The one rule this whole feature rests on, asserted in the database rather
    -- than trusted to review. A future migration that adds `title` or
    -- `event_date` to the reference table would make a subset able to disagree
    -- with the chronology, which is the exact failure the design exists to
    -- prevent — and it would be invisible until two screens showed two dates.
    SELECT COUNT(*) INTO n FROM information_schema.columns
     WHERE table_schema = 'public'
       AND table_name = 'chronology_subset_events'
       AND column_name IN ('title', 'event_date', 'fact', 'date_precision', 'phase');
    IF n <> 0 THEN
        RAISE EXCEPTION
            'chronology_subset_events carries % copied event column(s). A subset holds REFERENCES, never copies (TIMELINE_SUBSET_DESIGN_v1 §4).', n;
    END IF;
END $$;

DO $$
DECLARE
    n INTEGER;
BEGIN
    -- ── the sixteen wording rows ─────────────────────────────────────────────
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key IN (
            'chronology_subsets_section_title',
            'chronology_subsets_section_subtitle',
            'chronology_subsets_add_button',
            'chronology_subsets_carried_by_prefix',
            'chronology_subsets_gap_count_template',
            'chronology_subsets_removed_event_line',
            'chronology_subsets_size_line_template',
            'chronology_subsets_picker_hint',
            'chronology_subsets_picker_gap_hint',
            'chronology_scenario_view_timeline_button',
            'chronology_scenario_timeline_row_label',
            'chronology_scenario_attach_link',
            'chronology_subsets_window_open_timeline',
            'chronology_subsets_window_edit',
            'chronology_subsets_window_footer_template',
            'chronology_subsets_empty_state'
     );
    IF n <> 16 THEN
        RAISE EXCEPTION
            'the timeline-subset wording block must hold all 16 rows, found %', n;
    END IF;

    -- The blank check is over the WHOLE chronology prefix rather than this
    -- migration's own rows, for the reason the Phase C migration gave: a blank
    -- value anywhere in the block stops the boot loader, and a migration that
    -- proved only its own half would let an earlier row go blank between
    -- deploys without anything noticing.
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key LIKE 'chronology\_%' AND (value IS NULL OR btrim(value) = '');
    IF n <> 0 THEN
        RAISE EXCEPTION
            'a chronology row is blank; the boot loader would refuse to start (% rows)', n;
    END IF;
END $$;
