// Migration: Add status property to all Document nodes
// Run once against both DEV and PROD Neo4j instances
// All existing documents are fully processed, so they get PUBLISHED status

// Set status on all existing documents that don't have one
MATCH (d:Document)
WHERE d.status IS NULL
SET d.status = 'PUBLISHED'
RETURN count(d) AS documents_updated;
