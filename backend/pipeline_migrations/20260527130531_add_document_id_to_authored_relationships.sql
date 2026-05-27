-- add_document_id_to_authored_relationships: Add document_id to authored_relationships
--
-- Created: 2026-05-27 13:05:31
-- Target: pipeline database
--
-- Records which document's Pass-2 extraction asserted an authored
-- relationship. Canonical-loader rows are case-global and leave this NULL;
-- only extracted cross-tier edges (provenance = 'extracted', e.g.
-- PROVES_ELEMENT from an Allegation to a canonical Element) set it. This lets
-- a document re-process delete and re-insert just its own extracted edges
-- (DELETE ... WHERE document_id = $1 AND provenance = 'extracted') without
-- touching canonical rows. Nullable + no index: the table is small and the
-- column is only filtered alongside provenance on the existing case index.

ALTER TABLE authored_relationships ADD COLUMN document_id TEXT;

COMMENT ON COLUMN authored_relationships.document_id IS
    'Owning document for an extracted (Pass-2) cross-tier edge. NULL for '
    'canonical loader rows (case-global). Scopes per-document reconciliation '
    'of extracted edges (provenance = ''extracted'').';
