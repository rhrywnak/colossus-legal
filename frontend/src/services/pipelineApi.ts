import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

const PIPELINE_BASE = `${API_BASE_URL}/api/admin/pipeline`;

// ── Types ──────────────────────────────────────────

export interface PipelineDocument {
  id: string;
  title: string;
  file_path: string;
  file_hash: string;
  status: string;
  document_type: string;
  created_at: string;
  updated_at: string;
}

export interface PipelineStep {
  id: number;
  document_id: string;
  step_name: string;
  status: string;
  started_at: string;
  completed_at: string | null;
  duration_secs: number | null;
  triggered_by: string | null;
  input_params: Record<string, unknown>;
  result_summary: Record<string, unknown>;
  error_message: string | null;
}

export interface HistoryResponse {
  document_id: string;
  steps: PipelineStep[];
}

export interface ExtractionItem {
  id: number;
  entity_type: string;
  label: string;
  verbatim_quote: string | null;
  grounding_status: string | null;
  grounded_page: number | null;
  review_status: string | null;
  reviewed_by: string | null;
  reviewed_at: string | null;
  review_notes: string | null;
  properties: Record<string, unknown>;
}

export interface ItemsResponse {
  document_id: string;
  items: ExtractionItem[];
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
}

export interface MetricsResponse {
  total_documents: number;
  documents_by_status: Record<string, number>;
  total_cost_usd: number;
  avg_cost_per_document: number;
  avg_grounding_rate: number;
  total_steps_executed: number;
  failed_steps: number;
  step_performance: Record<string, StepMetrics>;
}

export interface StepMetrics {
  count: number;
  avg_duration_secs: number;
  min_duration_secs: number;
  max_duration_secs: number;
  failure_count: number;
}

export interface SchemaInfo {
  name: string;
  label: string;
  description: string;
}

// ── API Functions ──────────────────────────────────

export async function fetchPipelineDocuments(): Promise<PipelineDocument[]> {
  const res = await authFetch(`${PIPELINE_BASE}/documents`);
  if (!res.ok) throw new Error(`Failed to fetch documents: ${res.status}`);
  return res.json();
}

export async function fetchDocumentHistory(docId: string): Promise<HistoryResponse> {
  const res = await authFetch(`${PIPELINE_BASE}/documents/${docId}/history`);
  if (!res.ok) throw new Error(`Failed to fetch history: ${res.status}`);
  return res.json();
}

export async function fetchDocumentItems(
  docId: string,
  params?: { page?: number; per_page?: number; review_status?: string; grounding_status?: string; entity_type?: string }
): Promise<ItemsResponse> {
  const query = new URLSearchParams();
  if (params?.page) query.set("page", String(params.page));
  if (params?.per_page) query.set("per_page", String(params.per_page));
  if (params?.review_status) query.set("review_status", params.review_status);
  if (params?.grounding_status) query.set("grounding_status", params.grounding_status);
  if (params?.entity_type) query.set("entity_type", params.entity_type);
  const qs = query.toString();
  const url = `${PIPELINE_BASE}/documents/${docId}/items${qs ? "?" + qs : ""}`;
  const res = await authFetch(url);
  if (!res.ok) throw new Error(`Failed to fetch items: ${res.status}`);
  return res.json();
}

export async function fetchMetrics(): Promise<MetricsResponse> {
  const res = await authFetch(`${PIPELINE_BASE}/metrics`);
  if (!res.ok) throw new Error(`Failed to fetch metrics: ${res.status}`);
  return res.json();
}

export async function fetchSchemas(): Promise<SchemaInfo[]> {
  const res = await authFetch(`${PIPELINE_BASE}/schemas`);
  if (!res.ok) throw new Error(`Failed to fetch schemas: ${res.status}`);
  const data = await res.json();
  return data.schemas;
}

// ── Pipeline step triggers ─────────────────────────

export async function triggerExtractText(docId: string): Promise<unknown> {
  const res = await authFetch(`${PIPELINE_BASE}/documents/${docId}/extract-text`, { method: "POST" });
  if (!res.ok) throw new Error(`Extract text failed: ${res.status}`);
  return res.json();
}

export async function triggerExtract(docId: string): Promise<unknown> {
  const res = await authFetch(`${PIPELINE_BASE}/documents/${docId}/extract`, { method: "POST" });
  if (!res.ok) throw new Error(`Extract failed: ${res.status}`);
  return res.json();
}

export async function triggerVerify(docId: string): Promise<unknown> {
  const res = await authFetch(`${PIPELINE_BASE}/documents/${docId}/verify`, { method: "POST" });
  if (!res.ok) throw new Error(`Verify failed: ${res.status}`);
  return res.json();
}

export async function triggerIngest(docId: string): Promise<unknown> {
  const res = await authFetch(`${PIPELINE_BASE}/documents/${docId}/ingest`, { method: "POST" });
  if (!res.ok) throw new Error(`Ingest failed: ${res.status}`);
  return res.json();
}

export async function triggerIndex(docId: string): Promise<unknown> {
  const res = await authFetch(`${PIPELINE_BASE}/documents/${docId}/index`, { method: "POST" });
  if (!res.ok) throw new Error(`Index failed: ${res.status}`);
  return res.json();
}

export async function fetchCompleteness(docId: string): Promise<unknown> {
  const res = await authFetch(`${PIPELINE_BASE}/documents/${docId}/completeness`);
  if (!res.ok) throw new Error(`Completeness check failed: ${res.status}`);
  return res.json();
}

// ── Review actions ─────────────────────────────────

export async function approveItem(itemId: number, notes?: string): Promise<unknown> {
  const res = await authFetch(`${PIPELINE_BASE}/items/${itemId}/approve`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ notes: notes || "" }),
  });
  if (!res.ok) throw new Error(`Approve failed: ${res.status}`);
  return res.json();
}

export async function rejectItem(itemId: number, reason: string): Promise<unknown> {
  const res = await authFetch(`${PIPELINE_BASE}/items/${itemId}/reject`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ reason }),
  });
  if (!res.ok) throw new Error(`Reject failed: ${res.status}`);
  return res.json();
}

export async function editItem(
  itemId: number,
  updates: { grounded_page?: number; verbatim_quote?: string; notes?: string }
): Promise<unknown> {
  const res = await authFetch(`${PIPELINE_BASE}/items/${itemId}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(updates),
  });
  if (!res.ok) throw new Error(`Edit failed: ${res.status}`);
  return res.json();
}

export async function bulkApprove(docId: string, filter: "grounded" | "all"): Promise<unknown> {
  const res = await authFetch(`${PIPELINE_BASE}/documents/${docId}/approve-all`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ filter }),
  });
  if (!res.ok) throw new Error(`Bulk approve failed: ${res.status}`);
  return res.json();
}

// ── Upload ─────────────────────────────────────────

export async function uploadDocument(
  file: File,
  params: { id: string; title: string; documentType: string; schemaFile: string }
): Promise<PipelineDocument> {
  const formData = new FormData();
  formData.append("file", file);
  formData.append("id", params.id);
  formData.append("title", params.title);
  formData.append("document_type", params.documentType);
  formData.append("schema_file", params.schemaFile);
  const res = await authFetch(`${PIPELINE_BASE}/documents`, {
    method: "POST",
    body: formData,
  });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`Upload failed (${res.status}): ${body}`);
  }
  const data = await res.json();
  return data.document;
}
