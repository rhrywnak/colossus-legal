-- chronology_tables: the case chronology's five tables
--
-- Created: 2026-08-25
-- Target: pipeline database (colossus_legal_v2)
--
-- CASE_CHRONOLOGY_DESIGN_v2 §4, as amended by Roman's rulings of 2026-08-25 on
-- CC_REPORT_TIMELINE_PHASE_A_v1 (R-B: phase is a real column; R-C: the case
-- column is `case_slug`).
--
-- ## Why all five tables are ONE migration
--
-- They are one unit. `chronology_events.phase` references `chronology_phases`,
-- and the three dependent tables reference `chronology_events`. Split into
-- separate files, every file but the first would fail when run alone — which is
-- exactly what the BEGIN/ROLLBACK dry-run exists to catch. They ship together so
-- that "run it alone" and "run it in order" are the same proof, honestly.
--
-- ## THE CHANGE RULE (design R4) IS WHY THIS SHAPE
--
-- "Additive only, absence tolerated. No chronology field is ever REQUIRED after
-- day one." Hence a small fixed core of columns that will never change, plus a
-- JSONB `attributes` bag for everything the case turns out to need. A new fact
-- about an event is a new key in the bag — no migration, no backfill, and old
-- rows keep deserialising because the Rust side reads every attribute as
-- optional.
--
-- `phase` is the first attribute PROMOTED out of that bag, on day one, because
-- it already passes the rule's own promotion test: its vocabulary is closed and
-- already guarded (the Rust enum, the `documents_phase_valid` CHECK, and the
-- phases table below), and every read of the chronology groups by it. Promoted,
-- it is a real foreign key — an unknown phase becomes impossible rather than
-- merely visible.
--
-- ⚑ AND IT LIVES IN EXACTLY ONE PLACE. There is no `attributes.phase` mirror.
-- Two homes for one fact is the kind-vs-side defect this project has already
-- paid for once; a bag key that shadowed the column would drift the first time
-- one of them was written without the other.
--
-- ## Nothing hard-deletes
--
-- `deleted_at` on events and notes (design R10): delete is soft, an Undo line
-- follows it on screen, and history makes every delete attributable. A row is
-- never removed, so a link's target and a note's author stay readable forever.

-- ─── 1 · chronology_phases — the four phases of the case ─────────────────────
--
-- Design R15: "Postgres confirmed, and phases become a small table too." Until
-- now the four phases lived ONLY in `frontend/public/data/timeline.json`, which
-- is baked into the frontend image — so renaming a phase meant an image rebuild,
-- and five surfaces besides the timeline read that file for their labels
-- (measured in CC_REPORT_TIMELINE_READ_AND_REPORT_v1). This table is the one
-- source that survives the move; `timeline.json` retires after the seed one-shot.
--
-- ## The slugs are borrowed, never invented
--
-- `id` carries the SAME four slugs as `domain::case_phase::CasePhase` and as the
-- `documents_phase_valid` CHECK added by 20260817150412. See case_phase.rs's
-- module header for why the backend owns slugs and never labels; this table adds
-- the place the LABELS live once the JSON file is gone.

CREATE TABLE IF NOT EXISTS chronology_phases (
    -- The stored slug. Matches CasePhase::slug() and documents.phase exactly.
    id          TEXT PRIMARY KEY,
    -- What a human reads: PRE-PROBATE / PROBATE / COA / COMPLAINT.
    label       TEXT NOT NULL,
    -- Free text, NOT parsed dates: "2008–2009", "2014–Present". The dash is an
    -- EN-DASH (U+2013) in every seeded row and must stay byte-exact — the page
    -- prints this string raw.
    date_range  TEXT NOT NULL,
    -- #rrggbb, used raw by the frontend as a border and (with an alpha suffix)
    -- as a tint. Data, not a code constant, so a recolour is an UPDATE.
    color       TEXT NOT NULL,
    -- The muted subtitle line under each phase header (design R14). Stored since
    -- 2026 but rendered by nothing until now.
    description TEXT,
    -- Chronological order — the order the page renders phases and the order a
    -- dropdown offers them. Explicit rather than relying on insertion order,
    -- because nothing in SQL promises insertion order on a read.
    sort_order  INTEGER NOT NULL
);

-- The PK stops duplicates; it does not stop a fifth phase appearing by typo. The
-- CHECK repeats the `documents_phase_valid` list verbatim so a row that could
-- never be tagged onto a document also cannot exist here. Adding a real fifth
-- phase is then deliberately three edits (the enum, both CHECKs) rather than one
-- accidental INSERT.
ALTER TABLE chronology_phases DROP CONSTRAINT IF EXISTS chronology_phases_id_valid;
ALTER TABLE chronology_phases ADD CONSTRAINT chronology_phases_id_valid
    CHECK (id IN ('estate', 'probate', 'appeals', 'civil_lawsuit'));

-- The four rows, VERBATIM from frontend/public/data/timeline.json as it stood at
-- v2.0.0-beta.409 (md5 eec44c0018bec97d9b33c5f819d9cef0). Every label, range,
-- colour and description is byte-for-byte what the page renders today, including
-- the U+2013 en-dashes in date_range.
--
-- ON CONFLICT DO NOTHING, not DO UPDATE: from Phase C these rows are editable in
-- the app, and a seed must never quietly undo a human's rename. sqlx will not
-- re-run an applied migration anyway — this is the belt to that braces.
INSERT INTO chronology_phases (id, label, date_range, color, description, sort_order) VALUES
    ('estate',        'PRE-PROBATE', '2008–2009',    '#b45309', 'The $50,000 conversion, guardianship petition, and Emil Awad''s passing',            1),
    ('probate',       'PROBATE',     '2009–2011',    '#2563eb', 'CFS appointed as Personal Representative, estate administration, auction, sanctions', 2),
    ('appeals',       'COA',         '2011–2013',    '#7c3aed', 'Two Court of Appeals cases, Judge Tighe''s post-appeal order',                       3),
    ('civil_lawsuit', 'COMPLAINT',   '2014–Present', '#059669', 'Marie files suit in Macomb County Circuit Court, discovery, motions for default',     4)
ON CONFLICT (id) DO NOTHING;

-- ─── 2 · chronology_events — the dated facts ─────────────────────────────────

CREATE TABLE IF NOT EXISTS chronology_events (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Which case this event belongs to. The same value and the same NAME as
    -- `scenarios.case_slug` and `authored_entities.case_slug` — one word for one
    -- fact across the whole database.
    case_slug      TEXT NOT NULL,
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
    -- Which phase of the case this event sits in. A REAL COLUMN with a REAL
    -- foreign key, ruled 2026-08-25 — see the header. NOT NULL because every
    -- event in the case belongs to a phase of it: unlike `documents.phase`,
    -- where NULL means "nobody has filed this yet", an event with no phase would
    -- have nowhere to render.
    phase          TEXT NOT NULL REFERENCES chronology_phases(id),
    -- Short. What the card shows in bold.
    title          TEXT NOT NULL,
    -- One sentence in plain words, checkable against the source. Optional by
    -- R11: "encouraged but optional".
    fact           TEXT,
    -- tags · people · spine · source · anything discovered later. NOT phase —
    -- that is the column above, and it has no mirror here.
    -- DEFAULT '{}' so an insert that names no attributes still produces an
    -- object, never NULL — the reader then has exactly one empty shape to handle
    -- instead of two.
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

-- Every read of the page sections by phase (design R6, R16), and the FK alone
-- does not index the referencing side.
CREATE INDEX IF NOT EXISTS idx_chronology_events_phase
    ON chronology_events (phase, event_date);

-- Tag and people filters (design R7) are containment queries against the bag —
-- `attributes @> '{"tags":["Filing"]}'`. GIN is the index type that answers them.
CREATE INDEX IF NOT EXISTS idx_chronology_events_attributes
    ON chronology_events USING GIN (attributes);

-- ─── 3 · chronology_event_links — the evidence, at a pinpoint ────────────────

CREATE TABLE IF NOT EXISTS chronology_event_links (
    event_id    UUID NOT NULL REFERENCES chronology_events(id) ON DELETE CASCADE,
    -- document · statement · scenario · allegation · paperless_document · email ·
    -- transcript · exhibit · url · file. Deliberately NOT a CHECK: the design
    -- lists ten target kinds and expects more, and a CHECK here would make every
    -- new kind a migration. The vocabulary is enforced where it is USED.
    target_type TEXT NOT NULL,
    -- The id in that target's own store. NO FOREIGN KEY, by design: the store
    -- varies with target_type, so no single FK could express it. Resolvability
    -- is reported by the read endpoint (as `resolution`) and checked by the
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
    -- three columns a human actually picked. It leads on event_id, so the
    -- per-event read needs no second index.
    PRIMARY KEY (event_id, target_type, target_id)
);

-- ─── 4 · chronology_event_notes — attributed, never a shared blob ────────────

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

-- ─── 5 · chronology_event_history — append-only, snapshot per write ──────────
--
-- The mechanism CC proposed and Roman accepted on 2026-08-25 (report v1, R-A).
-- A separate append-only table holding a full JSONB SNAPSHOT of the event after
-- each write, rather than one typed column per field, because the change rule
-- says the field set grows forever: a typed history table would need a migration
-- every time an attribute is promoted, and would silently stop recording the
-- fields it did not know about. A snapshot never goes stale.
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
