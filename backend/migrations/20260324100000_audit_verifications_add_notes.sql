-- Add notes column to audit_verifications for reviewer comments.
-- Supports the Document Workspace verify/reject workflow.
ALTER TABLE audit_verifications ADD COLUMN IF NOT EXISTS notes TEXT;
