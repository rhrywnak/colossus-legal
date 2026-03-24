import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

// ── Types ──────────────────────────────────────────────

export interface DocumentEvidence {
  id: string;
  title: string | null;
  verbatim_quote: string | null;
  page_number: number | null;
  kind: string | null;
  weight: string | null;
  speaker: string | null;
  verification: {
    status: string;
    notes: string | null;
    verified_by: string;
    verified_at: string;
  } | null;
  flags: Array<{
    severity: string;
    description: string | null;
    flagged_by: string;
    flagged_at: string;
  }>;
}

export interface DocumentEvidenceResponse {
  document_id: string;
  document_title: string;
  evidence_count: number;
  verified_count: number;
  flagged_count: number;
  evidence: DocumentEvidence[];
}

// ── API calls ──────────────────────────────────────────

export async function fetchDocumentEvidence(
  docId: string
): Promise<DocumentEvidenceResponse> {
  const response = await authFetch(
    `${API_BASE_URL}/api/admin/documents/${encodeURIComponent(docId)}/evidence`
  );
  if (!response.ok) throw new Error(`Failed to fetch evidence: ${response.status}`);
  return response.json();
}

export async function verifyEvidence(
  docId: string,
  evidenceId: string,
  status: string,
  notes: string
): Promise<void> {
  const response = await authFetch(
    `${API_BASE_URL}/api/admin/documents/${encodeURIComponent(docId)}/evidence/${encodeURIComponent(evidenceId)}/verify`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ status, notes }),
    }
  );
  if (!response.ok) throw new Error(`Failed to verify: ${response.status}`);
}

export async function flagEvidence(
  docId: string,
  evidenceId: string,
  severity: string,
  description: string
): Promise<void> {
  const response = await authFetch(
    `${API_BASE_URL}/api/admin/documents/${encodeURIComponent(docId)}/evidence/${encodeURIComponent(evidenceId)}/flag`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ severity, description }),
    }
  );
  if (!response.ok) throw new Error(`Failed to flag: ${response.status}`);
}
