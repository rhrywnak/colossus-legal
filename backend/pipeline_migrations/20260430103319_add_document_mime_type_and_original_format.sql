-- add_document_mime_type_and_original_format: Add document mime_type and original_format
--
-- Created: 2026-04-30 10:33:19
-- Target: pipeline database
--
-- Multi-format document ingestion: track the detected MIME type and
-- short format key for each uploaded document.
--
-- mime_type: detected from file content (magic bytes) at upload time,
--   e.g. "application/pdf",
--   "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
--   "text/plain"
--
-- original_format: short key used by ExtractText to route to the correct
--   extractor, e.g. "pdf", "docx", "txt"
--
-- Both columns are nullable for backward compatibility — existing documents
-- (all PDFs) will have NULL here. The pipeline treats NULL original_format
-- as "pdf" (the only format previously supported).

ALTER TABLE documents ADD COLUMN IF NOT EXISTS mime_type TEXT;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS original_format TEXT;
