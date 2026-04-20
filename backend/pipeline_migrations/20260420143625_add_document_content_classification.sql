-- add_document_content_classification: Add document content classification
--
-- Created: 2026-04-20 14:36:25
-- Target: pipeline database
--
-- Add content classification columns to documents table.
-- Populated at upload time by the PDF classifier (colossus-pdf v0.10.4).
-- Enables smart OCR routing in ExtractText and content type display
-- in the frontend.

ALTER TABLE documents
    ADD COLUMN IF NOT EXISTS content_type TEXT DEFAULT 'unknown',
    ADD COLUMN IF NOT EXISTS page_count INTEGER,
    ADD COLUMN IF NOT EXISTS text_pages INTEGER,
    ADD COLUMN IF NOT EXISTS scanned_pages INTEGER,
    ADD COLUMN IF NOT EXISTS pages_needing_ocr INTEGER[],
    ADD COLUMN IF NOT EXISTS total_chars INTEGER;
