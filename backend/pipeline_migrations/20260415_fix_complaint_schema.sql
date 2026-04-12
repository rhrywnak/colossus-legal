-- Fix existing complaint documents to use v2 schema with grounding modes.
-- complaint_v2.yaml has: Party=name_match, LegalCount=heading_match,
-- Harm=derived, ComplaintAllegation=verbatim.
UPDATE pipeline_config
SET schema_file = 'complaint_v2.yaml'
WHERE document_id IN (
    SELECT id FROM documents WHERE document_type = 'complaint'
)
AND schema_file = 'complaint.yaml';
