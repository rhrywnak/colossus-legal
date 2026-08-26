-- chronology_tags: the tag vocabulary, out of the retiring JSON and into a table
--
-- Created: 2026-08-25
-- Target: pipeline database (colossus_legal_v2)
--
-- Ruling R-F, 2026-08-25. Phase A moved the PHASES into `chronology_phases`; the
-- `categories` block of `frontend/public/data/timeline.json` — the five tags'
-- display labels and colours — had no home, and until it did the file could not
-- retire. This is that home, and Phase B deletes the file.
--
-- ## No foreign key from an event's tags, deliberately
--
-- An event's tags live in `chronology_events.attributes.tags`, and the change
-- rule (design R4) is why they stay there: tags are a growing, open vocabulary
-- and a FK would make every new one a migration. What guards them instead is a
-- test — `chronology::guard_tests` asserts every tag on every seeded event has a
-- row here — which catches the same drift without freezing the bag.
--
-- ## The `icon` field is NOT carried, and that is a deliberate omission
--
-- The JSON gives every category an icon (`$`, `⚖`, `📄`, `🔍`, `●`). Measured in
-- CC_REPORT_TIMELINE_READ_AND_REPORT_v1 §F1.6: NOTHING renders it — the page
-- draws a coloured dot and the label, never the glyph. Carrying a column no
-- surface reads would be inventing a field to preserve a field. If a design ever
-- wants icons, adding the column then is one small migration with a real reader
-- attached.

CREATE TABLE IF NOT EXISTS chronology_tags (
    -- The stored token, exactly as `attributes.tags` holds it. Lower snake_case,
    -- because that is what the JSON's `categories` keys are and what every
    -- seeded event already carries.
    id         TEXT PRIMARY KEY,
    -- What a human reads on the chip: "Court Action", not "court_action".
    label      TEXT NOT NULL,
    -- #rrggbb. Used raw by the frontend for the dot, the chip text, and (with an
    -- alpha suffix) the chip's fill. Data, so a recolour is an UPDATE.
    color      TEXT NOT NULL,
    -- The order the filter chips are offered in — the JSON's own order, which is
    -- the order Roman wrote them.
    sort_order INTEGER NOT NULL
);

-- The five rows, VERBATIM from `frontend/public/data/timeline.json` as it stood
-- at v2.0.0-beta.410 (md5 eec44c0018bec97d9b33c5f819d9cef0). Labels and colours
-- byte-exact; a test compares them to the file until the file retires, and to
-- the seeded events' tags afterwards.
--
-- ON CONFLICT DO NOTHING: from Phase C these are editable, and a re-seed must
-- never undo a human's rename.
INSERT INTO chronology_tags (id, label, color, sort_order) VALUES
    ('financial',    'Financial',    '#059669', 1),
    ('court_action', 'Court Action', '#2563eb', 2),
    ('filing',       'Filing',       '#7c3aed', 3),
    ('discovery',    'Discovery',    '#d97706', 4),
    ('personal',     'Personal',     '#64748b', 5)
ON CONFLICT (id) DO NOTHING;

-- ⚑ ROW-COUNT ASSERTION, ruled 2026-08-25.
--
-- A seed or correction that matches zero rows is SILENT in Postgres: no error,
-- no log line, and the old value keeps being served. This block is what turns
-- that into a failed migration. It asserts the end state rather than the number
-- of rows this statement touched, so it is equally true on a first run and on a
-- re-run where ON CONFLICT did nothing — which is the state that actually
-- matters to a reader of the table.
DO $$
DECLARE
    n INTEGER;
BEGIN
    SELECT COUNT(*) INTO n FROM chronology_tags;
    IF n <> 5 THEN
        RAISE EXCEPTION
            'chronology_tags must hold exactly the five seeded tags, found %', n;
    END IF;

    SELECT COUNT(*) INTO n FROM chronology_tags
     WHERE id IN ('financial', 'court_action', 'filing', 'discovery', 'personal');
    IF n <> 5 THEN
        RAISE EXCEPTION
            'chronology_tags is missing one of the five seeded ids (matched %)', n;
    END IF;
END $$;
