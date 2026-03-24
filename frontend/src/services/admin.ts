import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

// ── Types ──────────────────────────────────────────────

export interface AdminDocument {
  id: string;
  title: string;
  doc_type: string | null;
  created_at: string | null;
  file_path: string | null;
  evidence_count: number;
  has_pdf: boolean;
  content_hash: string | null;
}

export interface AdminDocumentList {
  documents: AdminDocument[];
  total: number;
}

export interface RegisterDocumentRequest {
  id?: string;
  title: string;
  doc_type: string;
  created_at?: string;
  description?: string;
  file_path?: string;
}

export interface RegisterDocumentResponse {
  id: string;
  title: string;
  content_hash: string;
  pdf_url: string;
}

export interface ImportEvidenceRequest {
  document_id: string;
  evidence: EvidenceImportItem[];
}

export interface EvidenceImportItem {
  id: string;
  title: string;
  content: string;
  verbatim_quote?: string;
  page_number?: number;
  date?: string;
  topic?: string;
  stated_by: string;
  about?: string[];
  supports_counts?: string[];
  contradicts?: { evidence_id: string; topic?: string; value?: string }[];
  rebuts?: string[];
  proves_allegations?: string[];
}

export interface ImportEvidenceResponse {
  created: number;
  relationships: Record<string, number>;
}

export interface ReindexResponse {
  mode: string;
  new_points: number;
  skipped: number;
  total: number;
  duration_ms: number;
}

export interface AdminQAEntry {
  id: string;
  question_preview: string;
  asked_by: string;
  asked_at: string;
  model: string;
  rating: number | null;
  total_ms: number | null;
}

export interface AdminQAListResponse {
  entries: AdminQAEntry[];
  total: number;
  limit: number;
  offset: number;
}

// ── API calls ──────────────────────────────────────────

export async function getAdminDocuments(): Promise<AdminDocumentList> {
  const res = await authFetch(`${API_BASE_URL}/api/admin/documents`);
  if (!res.ok) throw new Error(`Failed to load documents: ${res.status}`);
  return res.json();
}

export async function registerDocument(
  req: RegisterDocumentRequest
): Promise<RegisterDocumentResponse> {
  const res = await authFetch(`${API_BASE_URL}/api/admin/documents`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });
  if (!res.ok) {
    const err = await res.json();
    throw new Error(err.message || "Failed to register document");
  }
  return res.json();
}

export async function importEvidence(
  req: ImportEvidenceRequest
): Promise<ImportEvidenceResponse> {
  const res = await authFetch(`${API_BASE_URL}/api/admin/evidence`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });
  if (!res.ok) {
    const err = await res.json();
    throw new Error(err.message || "Failed to import evidence");
  }
  return res.json();
}

export async function triggerReindex(
  mode: string = "incremental"
): Promise<ReindexResponse> {
  const res = await authFetch(`${API_BASE_URL}/api/admin/reindex`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ mode }),
    timeoutMs: 120_000, // Full rebuilds take 30-120s
  });
  if (!res.ok) {
    const err = await res.json();
    throw new Error(err.message || "Reindex failed");
  }
  return res.json();
}

export async function getAdminQAEntries(
  limit = 50,
  offset = 0,
  user?: string
): Promise<AdminQAListResponse> {
  let url = `${API_BASE_URL}/api/admin/qa-entries?limit=${limit}&offset=${offset}`;
  if (user) url += `&user=${encodeURIComponent(user)}`;
  const res = await authFetch(url);
  if (!res.ok) throw new Error(`Failed to load QA entries: ${res.status}`);
  return res.json();
}

export async function bulkDeleteQAEntries(
  ids: string[]
): Promise<{ deleted: number }> {
  const res = await authFetch(`${API_BASE_URL}/api/admin/qa-entries`, {
    method: "DELETE",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ ids }),
  });
  if (!res.ok) {
    const err = await res.json();
    throw new Error(err.message || "Delete failed");
  }
  return res.json();
}

// ── Upload ────────────────────────────────────────────

export interface UploadResponse {
  filename: string;
  size_bytes: number;
  path: string;
}

export async function uploadDocument(file: File): Promise<UploadResponse> {
  const formData = new FormData();
  formData.append("file", file);

  // NOTE: Do NOT set Content-Type header — the browser sets it with the
  // multipart boundary automatically. Setting it manually breaks the upload.
  const res = await fetch(`${API_BASE_URL}/api/admin/upload`, {
    method: "POST",
    body: formData,
  });

  if (!res.ok) {
    const err = await res.json();
    throw new Error(err.message || "Upload failed");
  }
  return res.json();
}

// ── Admin Status ──────────────────────────────────────

export interface AdminStatusResponse {
  environment: string;
  version: string;
  neo4j_connected: boolean;
  qdrant_connected: boolean;
  postgres_connected: boolean;
}

export async function getAdminStatus(): Promise<AdminStatusResponse> {
  const res = await authFetch(`${API_BASE_URL}/api/admin/status`);
  if (!res.ok) throw new Error(`Failed to load status: ${res.status}`);
  return res.json();
}

// ── QA Delete All ─────────────────────────────────────

export async function deleteAllQAEntries(): Promise<{ deleted: number }> {
  const res = await authFetch(`${API_BASE_URL}/api/admin/qa-entries`, {
    method: "DELETE",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ all: true }),
  });
  if (!res.ok) {
    const err = await res.json();
    throw new Error(err.message || "Delete all failed");
  }
  return res.json();
}
