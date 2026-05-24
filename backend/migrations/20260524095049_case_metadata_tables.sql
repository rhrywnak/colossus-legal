-- case_metadata_tables: Case-level metadata for the Home page (cases, parties, counsel)
--
-- Created: 2026-05-24 09:50:49
-- Target: main database
--
-- Creates the three tables that back the Home page case header:
--   cases   — one row per matter: caption, court, status, complaint reference
--   parties — plaintiffs / defendants (including dropped) with display ordering
--   counsel — attorney / firm contact info per represented side
--
-- Schema per HOME_PAGE_REDESIGN_v2.md §10. This migration is FORWARD-ONLY
-- (the project does not use down-migrations). It creates schema only — seed
-- data is loaded separately by Roman via AWAD_CASE_DATA_SQL.md §A after this
-- migration applies.
--
-- Table order matters: `parties` and `counsel` each carry a FOREIGN KEY to
-- `cases`, so `cases` must be created first.

-- ── cases ─────────────────────────────────────────────────────
-- One row per litigation matter. Drives the Home page case header
-- (caption, court strip, status pill). Read by the case endpoint via
-- `case_slug`; never exposes `case_id` directly to the frontend.
CREATE TABLE IF NOT EXISTS cases (
    case_id               TEXT PRIMARY KEY,
    case_slug             TEXT NOT NULL UNIQUE,
    display_title         TEXT NOT NULL,
    display_title_full    TEXT,
    court_name            TEXT,
    jurisdiction          TEXT,
    case_number           TEXT,
    filed_date            DATE,
    transferred_from      TEXT,
    transfer_date         DATE,
    status                TEXT NOT NULL DEFAULT 'active',
    complaint_document_id TEXT,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Constrain status to the known lifecycle states. A bad value is a hard
-- error at write time rather than a silent typo that breaks the status pill.
ALTER TABLE cases ADD CONSTRAINT cases_status_check
    CHECK (status IN ('active', 'stayed', 'dismissed', 'settled', 'judgment'));

-- The case endpoint looks up a case by its slug, not its primary key.
CREATE INDEX IF NOT EXISTS idx_cases_case_slug ON cases(case_slug);

COMMENT ON TABLE cases IS
    'Case-level metadata for the Home page case header. One row per matter. '
    'Data source: manually seeded via AWAD_CASE_DATA_SQL.md and edited via SQL. '
    'See HOME_PAGE_REDESIGN_v2.md §10 for design rationale.';
COMMENT ON COLUMN cases.case_id IS
    'Opaque primary key (stable internal identifier). Not exposed to the '
    'frontend — API endpoints address a case by case_slug instead.';
COMMENT ON COLUMN cases.case_slug IS
    'URL-friendly identifier (e.g. awad_v_catholic_family_service). '
    'Used by API endpoints to look up a case without exposing the case_id format.';
COMMENT ON COLUMN cases.display_title IS
    'Short caption for the header (e.g. "Awad v. CFS / Phillips").';
COMMENT ON COLUMN cases.display_title_full IS
    'Full caption for tooltip/detail view when the short title is abbreviated. '
    'NULL when display_title is already the full caption.';
COMMENT ON COLUMN cases.court_name IS
    'Current court of record (e.g. "Bay County Circuit Court").';
COMMENT ON COLUMN cases.jurisdiction IS
    'Governing jurisdiction for choice-of-law display (e.g. "Michigan").';
COMMENT ON COLUMN cases.case_number IS
    'Court docket / case number as filed (e.g. the circuit or probate court '
    'case no.). Free-form text since formats vary by court. NULL until assigned.';
COMMENT ON COLUMN cases.filed_date IS
    'Date the operative complaint was filed in the current court. Distinct from '
    'transfer_date (when venue moved) and from the complaint document ingest date.';
COMMENT ON COLUMN cases.transferred_from IS
    'If venue was transferred, the originating court name (e.g. "Macomb County '
    'Circuit Court"). NULL if the case was filed in the current court.';
COMMENT ON COLUMN cases.transfer_date IS
    'Date venue was transferred. NULL if never transferred.';
COMMENT ON COLUMN cases.status IS
    'Case lifecycle state (constrained by cases_status_check). Drives the '
    'status pill: active | stayed | dismissed | settled | judgment.';
COMMENT ON COLUMN cases.complaint_document_id IS
    'Document id of the operative complaint, linking case metadata to the '
    'ingested source document. NULL until the complaint is registered.';

-- ── parties ───────────────────────────────────────────────────
-- Plaintiffs and Defendants for a case. Dropped/dismissed parties are
-- retained for historical traceability and excluded from the active list
-- in the application layer, not deleted here.
CREATE TABLE IF NOT EXISTS parties (
    party_id        TEXT PRIMARY KEY,
    case_id         TEXT NOT NULL REFERENCES cases(case_id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    role            TEXT NOT NULL,
    entity_type     TEXT,
    status          TEXT NOT NULL DEFAULT 'active',
    dismissal_date  DATE,
    dismissal_basis TEXT,
    notes           TEXT,
    sort_order      INTEGER NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- A party is on exactly one side of the caption.
ALTER TABLE parties ADD CONSTRAINT parties_role_check
    CHECK (role IN ('Plaintiff', 'Defendant'));

-- Constrain party lifecycle to known states (mirrors case status intent).
ALTER TABLE parties ADD CONSTRAINT parties_status_check
    CHECK (status IN ('active', 'dropped', 'dismissed', 'settled'));

-- Common query: all parties of a case, in display order.
CREATE INDEX IF NOT EXISTS idx_parties_case_id_sort_order
    ON parties(case_id, sort_order);

COMMENT ON TABLE parties IS
    'Plaintiffs and Defendants for a case, including dropped/dismissed parties '
    'kept for historical traceability. The Home page active-Defendants list '
    'filters out non-active parties in the application layer.';
COMMENT ON COLUMN parties.case_id IS
    'Owning case. ON DELETE CASCADE: removing a case removes its parties.';
COMMENT ON COLUMN parties.role IS
    'Side of the caption (constrained by parties_role_check): Plaintiff | Defendant.';
COMMENT ON COLUMN parties.entity_type IS
    'Optional classification (e.g. "individual", "organization", "estate") '
    'for display and downstream attribution logic.';
COMMENT ON COLUMN parties.status IS
    'Party lifecycle state (constrained by parties_status_check). Dropped '
    'parties (e.g. Archdiocese of Detroit in Awad) remain in the table for '
    'historical traceability but are excluded from the active Defendants list.';
COMMENT ON COLUMN parties.dismissal_date IS
    'Date the party was dropped/dismissed. NULL while status = active.';
COMMENT ON COLUMN parties.dismissal_basis IS
    'Why the party was dropped/dismissed (e.g. "voluntary dismissal"). '
    'NULL while status = active.';
COMMENT ON COLUMN parties.notes IS
    'Free-form annotation about this party (e.g. relationship to the matter, or '
    'context beyond dismissal_basis). Reference/display only, not used in logic.';
COMMENT ON COLUMN parties.sort_order IS
    'Display order within a role (Plaintiffs grouped together, Defendants '
    'grouped together). Convention: 10, 20, 30 — gaps allow inserts without '
    'renumbering.';

-- ── counsel ───────────────────────────────────────────────────
-- Attorney / firm contact info, attached to a case and tagged with the
-- side they represent. Multiple rows per case (one per attorney of record).
CREATE TABLE IF NOT EXISTS counsel (
    counsel_id      TEXT PRIMARY KEY,
    case_id         TEXT NOT NULL REFERENCES cases(case_id) ON DELETE CASCADE,
    represents_role TEXT NOT NULL,
    firm_name       TEXT,
    attorney_name   TEXT NOT NULL,
    bar_number      TEXT,
    address         TEXT,
    phone           TEXT,
    email           TEXT,
    sort_order      INTEGER NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Counsel represents exactly one side.
ALTER TABLE counsel ADD CONSTRAINT counsel_represents_role_check
    CHECK (represents_role IN ('Plaintiff', 'Defendant'));

-- Common query: all counsel of a case, in display order.
CREATE INDEX IF NOT EXISTS idx_counsel_case_id_sort_order
    ON counsel(case_id, sort_order);

COMMENT ON TABLE counsel IS
    'Attorney/firm contact info per case, tagged with the represented side. '
    'Multiple rows per case (one per attorney of record).';
COMMENT ON COLUMN counsel.case_id IS
    'Owning case. ON DELETE CASCADE: removing a case removes its counsel rows.';
COMMENT ON COLUMN counsel.represents_role IS
    'Which side this counsel represents (constrained by '
    'counsel_represents_role_check): Plaintiff | Defendant.';
COMMENT ON COLUMN counsel.firm_name IS
    'Law firm name. NULL for a solo practitioner with no firm affiliation.';
COMMENT ON COLUMN counsel.attorney_name IS
    'Attorney of record. Required — a counsel row must name an attorney.';
COMMENT ON COLUMN counsel.bar_number IS
    'Attorney bar registration number (state bar of the case jurisdiction). '
    'Free-form text; NULL if unknown.';
COMMENT ON COLUMN counsel.sort_order IS
    'Display order within a represented side. Convention: 10, 20, 30 — gaps '
    'allow inserts without renumbering.';
