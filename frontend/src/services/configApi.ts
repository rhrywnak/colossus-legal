/**
 * Admin config API — typed client for the /api/admin/pipeline/config
 * surface (models, profiles, templates, schemas, system prompts, preview).
 *
 * Patterns mirror pipelineApi.ts:
 *  - authFetch wraps fetch with credentials + timeout
 *  - Each call throws Error(await res.text()) on non-2xx so callers can
 *    surface the server's AppError JSON body in the UI
 *  - Path params are percent-encoded to tolerate dots, dashes, etc.
 */

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

const CONFIG_BASE = `${API_BASE_URL}/api/admin/pipeline`;

const JSON_HEADERS = { "Content-Type": "application/json" };

// ── Types: Models ──────────────────────────────────────────────

export interface LlmModel {
  id: string;
  display_name: string;
  provider: string;
  api_endpoint: string | null;
  max_context_tokens: number | null;
  max_output_tokens: number | null;
  cost_per_input_token: number | null;
  cost_per_output_token: number | null;
  is_active: boolean;
  created_at: string;
  notes: string | null;
}

export interface ModelsResponse {
  models: LlmModel[];
}

export interface CreateModelInput {
  id: string;
  display_name: string;
  provider: string;
  api_endpoint?: string;
  max_context_tokens?: number;
  max_output_tokens?: number;
  cost_per_input_token?: number;
  cost_per_output_token?: number;
  notes?: string;
}

export interface UpdateModelInput {
  display_name?: string;
  provider?: string;
  api_endpoint?: string | null;
  max_context_tokens?: number | null;
  max_output_tokens?: number | null;
  cost_per_input_token?: number | null;
  cost_per_output_token?: number | null;
  is_active?: boolean;
  notes?: string | null;
}

// ── Types: Profiles ────────────────────────────────────────────

export interface ProcessingProfile {
  name: string;
  display_name: string;
  description: string;
  schema_file: string;
  template_file: string;
  system_prompt_file: string | null;
  /**
   * Global-rules fragment filename (e.g. `global_rules_v4.md`). Loaded
   * from `template_dir` and substituted into Pass-1 / Pass-2 prompts at
   * the `{{global_rules}}` placeholder. Null means the profile didn't
   * configure rules. Audit-displayed read-only — no per-document
   * override path on the backend by design.
   */
  global_rules_file: string | null;
  /**
   * Pass-2 relationship-extraction prompt template filename. Surfaced
   * read-only in the Configuration Panel so operators know which Pass-2
   * template will run; per the audit (Gap 2), the override path is
   * deliberately profile-only — operators change Pass-2 templates by
   * editing the profile YAML, not per-document.
   */
  pass2_template_file: string | null;
  extraction_model: string;
  /** Pass-2 relationship-extraction model; null means reuse `extraction_model`. */
  pass2_extraction_model: string | null;
  chunking_mode: string;
  chunk_size: number | null;
  chunk_overlap: number | null;
  /**
   * Flexible chunking parameters (`mode`, `strategy`, `units_per_chunk`,
   * `unit_overlap`, `request_timeout_secs`, ...). The full key set is
   * profile-author-defined; the backend resolver merges per-document
   * overrides at the *key* level. Empty object on profiles that don't
   * configure structured/strategy-aware chunking.
   */
  chunking_config: Record<string, unknown>;
  /**
   * Flexible cross-document context parameters (`traversal_depth`,
   * `always_include_foundation`, ...). Same shape and merge semantics
   * as `chunking_config`.
   */
  context_config: Record<string, unknown>;
  max_tokens: number;
  temperature: number;
  auto_approve_grounded: boolean;
  run_pass2: boolean;
  is_default: boolean;
  /**
   * SHA-256 hex of the profile's YAML body at load time. Always
   * present (the backend's `from_yaml_str` populates it). Audit-trail
   * fingerprint — two runs against the same `name` but different YAML
   * content are distinguishable in `extraction_runs.processing_config`
   * via this hash. The Configuration Panel displays the leading
   * eight characters with the full value on a tooltip.
   */
  profile_hash: string;
}

export interface ProfilesResponse {
  profiles: ProcessingProfile[];
}

// ── Types: File-based resources ────────────────────────────────

export interface TemplateInfo {
  filename: string;
  preview: string;
  size_bytes: number;
}

export interface SchemaInfo {
  filename: string;
  document_type: string;
  version: string;
  description: string;
  entity_type_count: number;
  entity_types: string[];
}

export interface SystemPromptInfo {
  filename: string;
  preview: string;
  size_bytes: number;
}

export interface FileContent {
  filename: string;
  content: string;
  size_bytes: number;
}

export interface CreateFileInput {
  filename: string;
  content: string;
}

export interface UpdateFileInput {
  content: string;
}

// ── Types: Pipeline config patch ───────────────────────────────

/**
 * Per-document overrides that can be persisted via
 * PATCH /api/admin/pipeline/documents/:id/config.
 *
 * Every field is optional — omitting a field preserves the existing
 * column value on the `pipeline_config` row.
 */
export interface PatchConfigInput {
  profile_name?: string;
  extraction_model?: string;
  /**
   * Pass-2 relationship-extraction model override. `undefined` keeps
   * the existing column value; an explicit model id switches pass 2
   * to that model. When left null across both profile and override,
   * pass 2 falls back to `extraction_model`.
   */
  pass2_extraction_model?: string | null;
  template_file?: string;
  system_prompt_file?: string | null;
  chunking_mode?: string;
  chunk_size?: number | null;
  chunk_overlap?: number | null;
  max_tokens?: number | null;
  temperature?: number | null;
  run_pass2?: boolean;
  /**
   * Per-document `chunking_config` override map. Sent as the full
   * desired value: the backend writes this directly to the
   * `pipeline_config.chunking_config` JSONB column. Sub-key clearing
   * therefore requires sending the full updated map (omitting the
   * cleared keys); sending an explicit `null` resets the override
   * entirely so the document re-inherits from the profile at resolve
   * time.
   *
   * The backend's resolver does the per-key merge against the profile's
   * map — the override sent here is the *override* map, not a merged
   * map. See backend `resolve_config` and `merge_map_override`.
   */
  chunking_config?: Record<string, unknown> | null;
  /**
   * Per-document `context_config` override map. Same shape and
   * semantics as `chunking_config`.
   */
  context_config?: Record<string, unknown> | null;
  /**
   * GET-only: base schema file for this document's pipeline_config row.
   * The PATCH handler ignores this field — posting it has no effect.
   */
  schema_file?: string | null;
}

// ── Types: Prompt Preview ──────────────────────────────────────

export interface PromptPreviewInput {
  document_id: string;
  profile_name?: string;
  template_file?: string;
  schema_file?: string;
}

export interface PromptPreviewResponse {
  profile_name: string;
  template_file: string;
  schema_file: string;
  model: string;
  chunking_mode: string;
  assembled_prompt: string;
  estimated_input_tokens: number;
  estimated_cost_usd: number | null;
  chunk_count: number | null;
  notes: string[];
}

// ── Helpers ────────────────────────────────────────────────────

/**
 * Throw an Error carrying the response body as its message.
 *
 * The backend AppError serializer returns JSON like
 * `{"message": "...", "details": {...}}`. Surfacing the raw body lets
 * the UI show the real reason (e.g., "Model 'foo' not found") instead
 * of a generic "Request failed".
 */
async function throwFromResponse(res: Response, op: string): Promise<never> {
  let body = "";
  try {
    body = await res.text();
  } catch {
    // body read failed — fall through to status-based message
  }
  const message = body && body.length > 0
    ? body
    : `${op} failed: ${res.status}`;
  throw new Error(message);
}

/** Discard a response body; used by void-returning endpoints. */
async function discardBody(res: Response): Promise<void> {
  try {
    await res.text();
  } catch {
    // ignore — we're not using the body
  }
}

// ── Models ─────────────────────────────────────────────────────

export async function listModels(): Promise<ModelsResponse> {
  const res = await authFetch(`${CONFIG_BASE}/models`);
  if (!res.ok) await throwFromResponse(res, "listModels");
  return res.json();
}

export async function createModel(input: CreateModelInput): Promise<LlmModel> {
  const res = await authFetch(`${CONFIG_BASE}/models`, {
    method: "POST",
    headers: JSON_HEADERS,
    body: JSON.stringify(input),
  });
  if (!res.ok) await throwFromResponse(res, "createModel");
  return res.json();
}

export async function updateModel(
  id: string,
  input: UpdateModelInput,
): Promise<LlmModel> {
  const res = await authFetch(`${CONFIG_BASE}/models/${encodeURIComponent(id)}`, {
    method: "PUT",
    headers: JSON_HEADERS,
    body: JSON.stringify(input),
  });
  if (!res.ok) await throwFromResponse(res, "updateModel");
  return res.json();
}

export async function deleteModel(id: string): Promise<void> {
  const res = await authFetch(`${CONFIG_BASE}/models/${encodeURIComponent(id)}`, {
    method: "DELETE",
  });
  if (!res.ok) await throwFromResponse(res, "deleteModel");
  await discardBody(res);
}

export async function toggleModel(id: string): Promise<LlmModel> {
  const res = await authFetch(
    `${CONFIG_BASE}/models/${encodeURIComponent(id)}/toggle`,
    { method: "PUT" },
  );
  if (!res.ok) await throwFromResponse(res, "toggleModel");
  return res.json();
}

// ── Profiles ───────────────────────────────────────────────────

export async function listProfiles(): Promise<ProfilesResponse> {
  const res = await authFetch(`${CONFIG_BASE}/profiles`);
  if (!res.ok) await throwFromResponse(res, "listProfiles");
  return res.json();
}

export async function getProfile(name: string): Promise<ProcessingProfile> {
  const res = await authFetch(
    `${CONFIG_BASE}/profiles/${encodeURIComponent(name)}`,
  );
  if (!res.ok) await throwFromResponse(res, "getProfile");
  return res.json();
}

export async function createProfile(
  profile: ProcessingProfile,
): Promise<ProcessingProfile> {
  const res = await authFetch(`${CONFIG_BASE}/profiles`, {
    method: "POST",
    headers: JSON_HEADERS,
    body: JSON.stringify(profile),
  });
  if (!res.ok) await throwFromResponse(res, "createProfile");
  return res.json();
}

export async function updateProfile(
  name: string,
  profile: ProcessingProfile,
): Promise<ProcessingProfile> {
  const res = await authFetch(
    `${CONFIG_BASE}/profiles/${encodeURIComponent(name)}`,
    {
      method: "PUT",
      headers: JSON_HEADERS,
      body: JSON.stringify(profile),
    },
  );
  if (!res.ok) await throwFromResponse(res, "updateProfile");
  return res.json();
}

export async function deactivateProfile(name: string): Promise<void> {
  const res = await authFetch(
    `${CONFIG_BASE}/profiles/${encodeURIComponent(name)}`,
    { method: "DELETE" },
  );
  if (!res.ok) await throwFromResponse(res, "deactivateProfile");
  await discardBody(res);
}

// ── Templates ──────────────────────────────────────────────────

export async function listTemplates(): Promise<{ templates: TemplateInfo[] }> {
  const res = await authFetch(`${CONFIG_BASE}/templates`);
  if (!res.ok) await throwFromResponse(res, "listTemplates");
  return res.json();
}

export async function getTemplate(filename: string): Promise<FileContent> {
  const res = await authFetch(
    `${CONFIG_BASE}/templates/${encodeURIComponent(filename)}`,
  );
  if (!res.ok) await throwFromResponse(res, "getTemplate");
  return res.json();
}

export async function createTemplate(
  input: CreateFileInput,
): Promise<FileContent> {
  const res = await authFetch(`${CONFIG_BASE}/templates`, {
    method: "POST",
    headers: JSON_HEADERS,
    body: JSON.stringify(input),
  });
  if (!res.ok) await throwFromResponse(res, "createTemplate");
  return res.json();
}

export async function updateTemplate(
  filename: string,
  input: UpdateFileInput,
): Promise<FileContent> {
  const res = await authFetch(
    `${CONFIG_BASE}/templates/${encodeURIComponent(filename)}`,
    {
      method: "PUT",
      headers: JSON_HEADERS,
      body: JSON.stringify(input),
    },
  );
  if (!res.ok) await throwFromResponse(res, "updateTemplate");
  return res.json();
}

export async function deleteTemplate(filename: string): Promise<void> {
  const res = await authFetch(
    `${CONFIG_BASE}/templates/${encodeURIComponent(filename)}`,
    { method: "DELETE" },
  );
  if (!res.ok) await throwFromResponse(res, "deleteTemplate");
  await discardBody(res);
}

// ── Schemas ────────────────────────────────────────────────────

export async function listSchemas(): Promise<{ schemas: SchemaInfo[] }> {
  const res = await authFetch(`${CONFIG_BASE}/schemas`);
  if (!res.ok) await throwFromResponse(res, "listSchemas");
  return res.json();
}

export async function getSchema(filename: string): Promise<FileContent> {
  const res = await authFetch(
    `${CONFIG_BASE}/schemas/${encodeURIComponent(filename)}`,
  );
  if (!res.ok) await throwFromResponse(res, "getSchema");
  return res.json();
}

export async function createSchema(
  input: CreateFileInput,
): Promise<FileContent> {
  const res = await authFetch(`${CONFIG_BASE}/schemas`, {
    method: "POST",
    headers: JSON_HEADERS,
    body: JSON.stringify(input),
  });
  if (!res.ok) await throwFromResponse(res, "createSchema");
  return res.json();
}

export async function updateSchema(
  filename: string,
  input: UpdateFileInput,
): Promise<FileContent> {
  const res = await authFetch(
    `${CONFIG_BASE}/schemas/${encodeURIComponent(filename)}`,
    {
      method: "PUT",
      headers: JSON_HEADERS,
      body: JSON.stringify(input),
    },
  );
  if (!res.ok) await throwFromResponse(res, "updateSchema");
  return res.json();
}

export async function deleteSchema(filename: string): Promise<void> {
  const res = await authFetch(
    `${CONFIG_BASE}/schemas/${encodeURIComponent(filename)}`,
    { method: "DELETE" },
  );
  if (!res.ok) await throwFromResponse(res, "deleteSchema");
  await discardBody(res);
}

// ── System Prompts ─────────────────────────────────────────────

export async function listSystemPrompts(): Promise<{
  system_prompts: SystemPromptInfo[];
}> {
  const res = await authFetch(`${CONFIG_BASE}/system-prompts`);
  if (!res.ok) await throwFromResponse(res, "listSystemPrompts");
  return res.json();
}

export async function getSystemPrompt(filename: string): Promise<FileContent> {
  const res = await authFetch(
    `${CONFIG_BASE}/system-prompts/${encodeURIComponent(filename)}`,
  );
  if (!res.ok) await throwFromResponse(res, "getSystemPrompt");
  return res.json();
}

export async function createSystemPrompt(
  input: CreateFileInput,
): Promise<FileContent> {
  const res = await authFetch(`${CONFIG_BASE}/system-prompts`, {
    method: "POST",
    headers: JSON_HEADERS,
    body: JSON.stringify(input),
  });
  if (!res.ok) await throwFromResponse(res, "createSystemPrompt");
  return res.json();
}

export async function updateSystemPrompt(
  filename: string,
  input: UpdateFileInput,
): Promise<FileContent> {
  const res = await authFetch(
    `${CONFIG_BASE}/system-prompts/${encodeURIComponent(filename)}`,
    {
      method: "PUT",
      headers: JSON_HEADERS,
      body: JSON.stringify(input),
    },
  );
  if (!res.ok) await throwFromResponse(res, "updateSystemPrompt");
  return res.json();
}

export async function deleteSystemPrompt(filename: string): Promise<void> {
  const res = await authFetch(
    `${CONFIG_BASE}/system-prompts/${encodeURIComponent(filename)}`,
    { method: "DELETE" },
  );
  if (!res.ok) await throwFromResponse(res, "deleteSystemPrompt");
  await discardBody(res);
}

// ── Pipeline config patch ──────────────────────────────────────

export async function patchDocumentConfig(
  documentId: string,
  input: PatchConfigInput,
): Promise<void> {
  const res = await authFetch(
    `${CONFIG_BASE}/documents/${encodeURIComponent(documentId)}/config`,
    {
      method: "PATCH",
      headers: JSON_HEADERS,
      body: JSON.stringify(input),
    },
  );
  if (!res.ok) await throwFromResponse(res, "patchDocumentConfig");
  await discardBody(res);
}

/**
 * Read the per-document overrides persisted on `pipeline_config`.
 *
 * Returns the same shape the PATCH endpoint accepts — every field is
 * optional. Used by the Configuration Panel to seed its initial state so
 * profile values auto-populated at upload time (e.g., `chunking_mode: "full"`
 * from a profile YAML) show up in the UI immediately.
 */
export async function getDocumentConfig(
  documentId: string,
): Promise<PatchConfigInput> {
  const res = await authFetch(
    `${CONFIG_BASE}/documents/${encodeURIComponent(documentId)}/config`,
  );
  if (!res.ok) await throwFromResponse(res, "getDocumentConfig");
  return res.json();
}

// ── Prompt Preview ─────────────────────────────────────────────

export async function previewPrompt(
  input: PromptPreviewInput,
): Promise<PromptPreviewResponse> {
  const res = await authFetch(`${CONFIG_BASE}/preview-prompt`, {
    method: "POST",
    headers: JSON_HEADERS,
    body: JSON.stringify(input),
  });
  if (!res.ok) await throwFromResponse(res, "previewPrompt");
  return res.json();
}
