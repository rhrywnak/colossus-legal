CREATE TABLE IF NOT EXISTS document_audit_log (
    id SERIAL PRIMARY KEY,
    document_id TEXT NOT NULL,
    document_title TEXT NOT NULL,
    action TEXT NOT NULL,
    reason TEXT,
    performed_by TEXT NOT NULL,
    performed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    previous_status TEXT NOT NULL,
    snapshot JSONB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_audit_log_document ON document_audit_log(document_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_action ON document_audit_log(action);
