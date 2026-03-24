-- v2.0 Phase 1: Foundation tables for document processing workflow
--
-- document_extractions: AI-extracted claims awaiting human review
-- admin_audit_log: Every admin action recorded for accountability
-- audit_findings: Issues found during document/evidence audit
-- audit_verifications: Per-evidence verification status

-- ── Document Extractions ──────────────────────────────────────
-- Stores raw AI extraction output before human review.
-- Each row is one extracted claim from one document.
-- Version column supports re-extraction (preserving history).
CREATE TABLE IF NOT EXISTS document_extractions (
    id              BIGSERIAL PRIMARY KEY,
    document_id     VARCHAR(200) NOT NULL,
    version         INTEGER NOT NULL DEFAULT 1,
    extraction_data JSONB NOT NULL,
    -- extraction_data contains: title, verbatim_quote, page_number,
    -- stated_by, about, supports_counts, topic, confidence
    status          VARCHAR(20) NOT NULL DEFAULT 'pending',
    -- pending | approved | excluded | rejected
    exclude_reason  VARCHAR(100),
    extracted_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    extracted_by    VARCHAR(100) NOT NULL,
    -- extracted_by: 'claude-sonnet' or 'claude-opus' (the model)
    reviewed_by     VARCHAR(100),
    reviewed_at     TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_doc_extractions_document_id
    ON document_extractions(document_id);
CREATE INDEX IF NOT EXISTS idx_doc_extractions_status
    ON document_extractions(status);
CREATE INDEX IF NOT EXISTS idx_doc_extractions_doc_version
    ON document_extractions(document_id, version);

-- ── Admin Audit Log ──────────────────────────────────────────
-- Records every admin action for accountability and debugging.
-- This is append-only — rows are never updated or deleted.
CREATE TABLE IF NOT EXISTS admin_audit_log (
    id          BIGSERIAL PRIMARY KEY,
    username    VARCHAR(100) NOT NULL,
    action      VARCHAR(100) NOT NULL,
    -- action examples: document.register, document.upload,
    -- evidence.import, reindex.trigger, qa.delete, qa.bulk_delete
    resource_type VARCHAR(50),
    -- resource_type: document, evidence, qa_entry, index
    resource_id VARCHAR(200),
    details     JSONB,
    -- details: action-specific data (request body summary, counts, etc.)
    ip_address  VARCHAR(45),
    performed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_audit_log_username
    ON admin_audit_log(username);
CREATE INDEX IF NOT EXISTS idx_audit_log_action
    ON admin_audit_log(action);
CREATE INDEX IF NOT EXISTS idx_audit_log_performed_at
    ON admin_audit_log(performed_at);
CREATE INDEX IF NOT EXISTS idx_audit_log_resource
    ON admin_audit_log(resource_type, resource_id);

-- ── Audit Findings ───────────────────────────────────────────
-- Issues discovered during document/evidence audit.
-- Can be document-level or evidence-level.
CREATE TABLE IF NOT EXISTS audit_findings (
    id              BIGSERIAL PRIMARY KEY,
    document_id     VARCHAR(200) NOT NULL,
    evidence_id     VARCHAR(200),
    -- NULL for document-level findings
    finding_type    VARCHAR(50) NOT NULL,
    -- missing_quote, wrong_page, wrong_speaker, missing_relationship,
    -- orphaned_node, pdf_not_found, missing_page_number, etc.
    severity        VARCHAR(20) NOT NULL,
    -- critical, high, low
    description     TEXT,
    status          VARCHAR(20) NOT NULL DEFAULT 'open',
    -- open | resolved | wont_fix
    found_by        VARCHAR(100) NOT NULL,
    found_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_by     VARCHAR(100),
    resolved_at     TIMESTAMPTZ,
    resolution      TEXT
);

CREATE INDEX IF NOT EXISTS idx_audit_findings_document_id
    ON audit_findings(document_id);
CREATE INDEX IF NOT EXISTS idx_audit_findings_status
    ON audit_findings(status);
CREATE INDEX IF NOT EXISTS idx_audit_findings_severity
    ON audit_findings(severity);

-- ── Audit Verifications ──────────────────────────────────────
-- Per-evidence verification records from manual audit.
-- One row per evidence item per audit pass.
CREATE TABLE IF NOT EXISTS audit_verifications (
    id              BIGSERIAL PRIMARY KEY,
    document_id     VARCHAR(200) NOT NULL,
    evidence_id     VARCHAR(200) NOT NULL,
    verified_by     VARCHAR(100) NOT NULL,
    verified_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status          VARCHAR(20) NOT NULL
    -- verified | issue_found
);

CREATE INDEX IF NOT EXISTS idx_audit_verifications_document_id
    ON audit_verifications(document_id);
CREATE INDEX IF NOT EXISTS idx_audit_verifications_evidence_id
    ON audit_verifications(evidence_id);
