-- chronology_events_links_notes_history: the chronology's four event tables
--
-- Created: 2026-08-25
-- Target: pipeline database (colossus_legal_v2)
--
-- CASE_CHRONOLOGY_DESIGN_v2 §4. Four tables in ONE migration because they are one
-- unit: the three dependent tables carry a foreign key to `chronology_events` and
-- cannot exist without it. Splitting them would produce a migration that fails
-- when run alone, which is exactly what the BEGIN/ROLLBACK dry-run is meant to
-- catch — so they ship together and the dry-run is honest.
--
-- ## THE CHANGE RULE (design R4) IS WHY THIS SHAPE
--
-- "Additive only, absence tolerated. No chronology field is ever REQUIRED after
-- day one." Hence: a small fixed core of columns that will never change, plus a
-- JSONB `attributes` bag for everything the case turns out to need. A new fact
-- about an event is a new key in the bag — no migration, no backfill, and old
-- rows keep deserialising because the Rust side reads every attribute as
-- optional. Attributes that prove stable get PROMOTED to real columns later,
-- one small migration each.
--
-- ## Nothing hard-deletes
--
-- `deleted_at` on events and notes (design R10): delete is soft, an Undo line
-- follows it on screen, and history makes every delete attributable. A row is
-- never removed, so a link's target and a note's author stay readable forever.

-- ─── 1 · chronology_events — the dated facts ──────────────────────────────────

CREATE TABLE IF NOT EXISTS chronology_events (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Which case this event belongs to. Holds the case SLUG — the same value
    -- `scenarios.case_slug` and `authored_entities.case_slug` carry (today:
    -- 'awad_v_catholic_family_service'). Named `case_id` because the design doc
    -- names it `case_id`; see the report's NEEDS A RULING on the name.
    case_id        TEXT NOT NULL,
    -- The date the event happened. NOT NULL: design R11 makes date and title the
    -- only two required fields, forever.
    event_date     DATE NOT NULL,
    -- How much of that date is actually known. 'day' means all three parts are
    -- real; 'month' and 'year' mean the finer parts are padding and a screen
    -- should print "March 2010", not "1 March 2010".
    date_precision TEXT NOT NULL DEFAULT 'day',
    -- Separate from precision on purpose. Precision says WHICH PARTS are known;
    -- `approximate` says the whole thing is a best estimate. The current JSON
    -- carries three such events, each with a full day-precision date that is
    -- nonetheless a guess — collapsing the two ideas would lose that.
    approximate    BOOLEAN NOT NULL DEFAULT FALSE,
    -- Short. What the card shows in bold.
    title          TEXT NOT NULL,
    -- One sentence in plain words, checkable against the source. Optional by
    -- R11: "encouraged but optional".
    fact           TEXT,
    -- tags · people · phase · spine · anything discovered later. See the change
    -- rule above. DEFAULT '{}' so an insert that names no attributes still
    -- produces an object, never NULL — the reader then has exactly one empty
    -- shape to handle instead of two.
    attributes     JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- Who wrote it and who touched it last, from the Authentik login via
    -- `services::practice_notes::attribution`. Nullable because a row written by
    -- the seed one-shot before any human touched it has no updater.
    created_by     TEXT,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by     TEXT,
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Soft delete (R10). NULL = live.
    deleted_at     TIMESTAMPTZ
);

ALTER TABLE chronology_events DROP CONSTRAINT IF EXISTS chronology_events_precision_valid;
ALTER TABLE chronology_events ADD CONSTRAINT chronology_events_precision_valid
    CHECK (date_precision IN ('day', 'month', 'year'));

-- The read order the page uses: oldest first, id breaking ties so two events on
-- the same date never swap places between two reads of the same data.
CREATE INDEX IF NOT EXISTS idx_chronology_events_date
    ON chronology_events (event_date, id);

-- Tag and people filters (design R7) are containment queries against the bag —
-- `attributes @> '{"tags":["Filing"]}'`. GIN is the index type that answers them.
CREATE INDEX IF NOT EXISTS idx_chronology_events_attributes
    ON chronology_events USING GIN (attributes);

-- ─── 2 · chronology_event_links — the evidence, at a pinpoint ─────────────────

CREATE TABLE IF NOT EXISTS chronology_event_links (
    event_id    UUID NOT NULL REFERENCES chronology_events(id) ON DELETE CASCADE,
    -- document · statement · scenario · allegation · paperless_document · email ·
    -- transcript · exhibit · url · file. Deliberately NOT a CHECK: the design
    -- lists ten target kinds and expects more, and a CHECK here would make every
    -- new kind a migration. The vocabulary is enforced where it is USED.
    target_type TEXT NOT NULL,
    -- The id in that target's own store. NO FOREIGN KEY, by design: the store
    -- varies with target_type, so no single FK could express it. Resolvability
    -- is checked by the read endpoint (which returns `resolves`) and by the
    -- permanent validation test — never by a constraint that would refuse a link
    -- to a document Roman has not scanned yet.
    target_id   TEXT NOT NULL,
    -- The link text a human reads, e.g. "Morris Affidavit".
    label       TEXT,
    -- Page, paragraph, Q-number, line. Free text, and its ABSENCE is meaningful:
    -- an empty pinpoint is shown as "no pinpoint" so unlinked and unpinpointed
    -- events double as the to-scan to-do list (design R9).
    pinpoint    TEXT,
    created_by  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- The natural key, so no surrogate id had to be invented: one event cannot
    -- link the same target twice, and Phase C's delete addresses a row by the
    -- three columns a human actually picked.
    PRIMARY KEY (event_id, target_type, target_id)
);

-- Every read of an event fetches its links by event_id. The composite PK already
-- leads on event_id, so this index is not repeated here.

-- ─── 3 · chronology_event_notes — attributed, never a shared blob ─────────────

CREATE TABLE IF NOT EXISTS chronology_event_notes (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id   UUID NOT NULL REFERENCES chronology_events(id) ON DELETE CASCADE,
    note       TEXT NOT NULL,
    created_by TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ
);

-- A list of notes per event, each stamped, so three writers never overwrite one
-- shared field (design R8).
CREATE INDEX IF NOT EXISTS idx_chronology_event_notes_event
    ON chronology_event_notes (event_id, created_at);

-- ─── 4 · chronology_event_history — append-only, snapshot per write ───────────
--
-- The mechanism CC proposes and the architect rules on (task A5). A separate
-- append-only table holding a full JSONB SNAPSHOT of the event after each write,
-- rather than one typed column per field, because the change rule says the field
-- set grows forever: a typed history table would need a migration every time an
-- attribute is promoted, and would silently stop recording the fields it did not
-- know about. A snapshot never goes stale.
--
-- NOTHING WRITES TO THIS TABLE IN PHASE A. The write endpoints are Phase C. It
-- exists now because `GET /api/timeline/events/{id}` returns history, and an
-- endpoint that reads a table which does not exist is a 500 waiting for the
-- first click.

CREATE TABLE IF NOT EXISTS chronology_event_history (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id   UUID NOT NULL REFERENCES chronology_events(id) ON DELETE CASCADE,
    -- What happened. Constrained because, unlike target_type, this vocabulary is
    -- closed: there are only so many things that can happen to a row.
    action     TEXT NOT NULL,
    -- The whole event as it stood AFTER this write. Reading history is then a
    -- diff between adjacent snapshots, computed at read time by code that knows
    -- today's field set — not frozen into columns by the code that wrote it.
    snapshot   JSONB NOT NULL,
    changed_by TEXT,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE chronology_event_history DROP CONSTRAINT IF EXISTS chronology_event_history_action_valid;
ALTER TABLE chronology_event_history ADD CONSTRAINT chronology_event_history_action_valid
    CHECK (action IN ('created', 'updated', 'deleted', 'restored'));

CREATE INDEX IF NOT EXISTS idx_chronology_event_history_event
    ON chronology_event_history (event_id, changed_at);
