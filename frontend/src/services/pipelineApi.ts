import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

const PIPELINE_BASE = `${API_BASE_URL}/api/admin/pipeline`;

// Timeout for long-running pipeline operations (LLM calls, graph writes, embedding).
// Configurable via VITE_PIPELINE_LONG_TIMEOUT_MS; defaults to 10 minutes.
const PIPELINE_LONG_TIMEOUT_MS = Number(
  import.meta.env.VITE_PIPELINE_LONG_TIMEOUT_MS ?? 600000
);
const LONG_TIMEOUT = { timeoutMs: PIPELINE_LONG_TIMEOUT_MS };

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
  assigned_reviewer?: string | null;
  assigned_at?: string | null;
  total_cost_usd: number | null;
  has_failed_steps: boolean;
  /** Tabs the current user can see (computed by backend). */
  visible_tabs?: string[];
  /** Whether the current user can view/interact with this document. */
  can_view?: boolean;
  /** Display grouping: "published" | "processing" | "in_review" | "uploaded". */
  status_group?: string;
}

export interface KnownUser {
  username: string;
  display_name: string;
  email: string;
  last_seen_at: string;
}

export interface AssignReviewerResponse {
  document_id: string;
  assigned_reviewer?: string | null;
  assigned_at?: string | null;
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

// ── Actions (state machine) ──────────────────────────

export interface AvailableAction {
  action: string;
  label: string;
  method: string;
  requires_confirmation: boolean;
  description: string;
  is_navigation: boolean;
  endpoint: string;
}

export interface PipelineStage {
  name: string;
  label: string;
  order: number;
  status: "completed" | "available" | "pending" | "failed";
  duration_secs: number | null;
  summary: string | null;
  action: AvailableAction | null;
}

export interface ExecutionHistoryEntry {
  step_name: string;
  label: string;
  status: string;
  started_at: string;
  duration_secs: number | null;
  triggered_by: string | null;
  summary: Record<string, unknown> | null;
  error_message: string | null;
}

export interface DocumentActions {
  document_id: string;
  current_status: string;
  pipeline_stages: PipelineStage[];
  available_actions: AvailableAction[];
  execution_history: ExecutionHistoryEntry[];
  delete_confirmation_level: string;
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
  /** Whether this item can be approved (computed by backend). */
  can_approve?: boolean;
  /** Whether this item can be rejected (computed by backend). */
  can_reject?: boolean;
  /** Whether this item can be edited (computed by backend). */
  can_edit?: boolean;
}

export interface ReviewSummary {
  pending: number;
  approved: number;
  rejected: number;
  edited: number;
  total: number;
}

export interface ItemsResponse {
  document_id: string;
  items: ExtractionItem[];
  summary?: ReviewSummary;
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
}

export interface EstimatesData {
  avg_cost_per_document: number | null;
  avg_total_duration_per_document_secs: number | null;
  documents_remaining: number;
  estimated_remaining_cost_usd: number | null;
  estimated_remaining_time_secs: number | null;
  confidence: string;
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
  estimates: EstimatesData;
}

export interface StepMetrics {
  label: string;
  order: number;
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

export interface DocumentError {
  document_id: string;
  document_title: string;
  document_status: string;
  failed_step: string;
  error_message: string | null;
  failed_at: string;
  triggered_by: string | null;
  retry_count: number;
}

export interface ErrorsResponse {
  documents_with_errors: DocumentError[];
  total_errors: number;
  documents_with_no_errors: number;
}

export interface ReviewerWorkload {
  username: string;
  display_name: string | null;
  assigned_documents: number;
  reviewed_documents: number;
  pending_documents: number;
  total_items: number;
  approved_items: number;
  pending_items: number;
  rejected_items: number;
}

export interface WorkloadResponse {
  reviewers: ReviewerWorkload[];
  unassigned_documents: number;
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

export async function fetchErrors(): Promise<ErrorsResponse> {
  const res = await authFetch(`${PIPELINE_BASE}/documents/errors`);
  if (!res.ok) throw new Error(`Failed to fetch errors: ${res.status}`);
  return res.json();
}

export async function fetchWorkload(): Promise<WorkloadResponse> {
  const res = await authFetch(`${PIPELINE_BASE}/reviewers/workload`);
  if (!res.ok) throw new Error(`Failed to fetch workload: ${res.status}`);
  return res.json();
}

export async function fetchUsers(): Promise<KnownUser[]> {
  const res = await authFetch(`${API_BASE_URL}/api/users`);
  if (!res.ok) throw new Error(`Failed to fetch users: ${res.status}`);
  return res.json();
}

export async function assignReviewer(
  docId: string,
  reviewer: string | null,
): Promise<AssignReviewerResponse> {
  const res = await authFetch(`${PIPELINE_BASE}/documents/${docId}/assign`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ reviewer }),
  });
  if (!res.ok) throw new Error(`Assign reviewer failed: ${res.status}`);
  return res.json();
}

export async function fetchDocumentActions(docId: string): Promise<DocumentActions> {
  const res = await authFetch(`${PIPELINE_BASE}/documents/${docId}/actions`);
  if (!res.ok) throw new Error(`Failed to fetch actions: ${res.status}`);
  return res.json();
}

/**
 * Generic pipeline action trigger. Calls the endpoint returned by the
 * backend state machine, substituting the document ID.
 */
export async function triggerPipelineAction(
  docId: string,
  endpoint: string,
  method: string = "POST",
): Promise<unknown> {
  const resolvedPath = endpoint.replace("{id}", docId);
  const url = `${PIPELINE_BASE}${resolvedPath}`;
  const res = await authFetch(url, {
    method,
    headers: { "Content-Type": "application/json" },
    ...LONG_TIMEOUT,
  });
  if (!res.ok) {
    const body = await res.text().catch(() => "");
    throw new Error(body || `Pipeline action failed: ${res.status}`);
  }
  return res.json().catch(() => ({}));
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
    ...LONG_TIMEOUT,
  });
  if (!res.ok) throw new Error(`Bulk approve failed: ${res.status}`);
  return res.json();
}

// ── Delete ─────────────────────────────────────────

export async function deleteDocument(docId: string, reason?: string): Promise<void> {
  const options: RequestInit = { method: "DELETE" };
  if (reason) {
    options.headers = { "Content-Type": "application/json" };
    options.body = JSON.stringify({ reason });
  }
  const res = await authFetch(`${PIPELINE_BASE}/documents/${docId}`, options);
  if (!res.ok) throw new Error(`Delete failed: ${res.status}`);
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
