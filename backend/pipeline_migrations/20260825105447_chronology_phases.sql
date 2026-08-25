-- chronology_phases: the four phases of the case, as a table
--
-- Created: 2026-08-25
-- Target: pipeline database (colossus_legal_v2)
--
-- CASE_CHRONOLOGY_DESIGN_v2 R15: "Postgres confirmed, and phases become a small
-- table too, served by the backend." Until now the four phases lived ONLY in
-- `frontend/public/data/timeline.json`, which is baked into the frontend image —
-- so renaming a phase meant an image rebuild, and five surfaces besides the
-- timeline read that file for their labels (measured in
-- CC_REPORT_TIMELINE_READ_AND_REPORT_v1). This table is the one source that
-- survives the move; `timeline.json` retires after the seed one-shot.
--
-- ## The slugs are borrowed, never invented
--
-- `id` carries the SAME four slugs as `domain::case_phase::CasePhase` and as the
-- `documents_phase_valid` CHECK added by 20260817150412. Three declarations of
-- one vocabulary is the shape the project already chose (see case_phase.rs's
-- module header for why the backend owns slugs and never labels); this migration
-- adds the fourth place the LABELS live — and then the JSON file, which was the
-- third, goes away entirely.
--
-- ## Why a CHECK as well as a primary key
--
-- The PK stops duplicates; it does not stop a fifth phase appearing by typo. The
-- CHECK repeats the `documents_phase_valid` list verbatim so a row that could
-- never be tagged onto a document also cannot exist here. Adding a real fifth
-- phase is then deliberately three edits (enum, both CHECKs) rather than one
-- accidental INSERT.

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
