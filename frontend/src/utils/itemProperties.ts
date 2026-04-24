// Format entity-type-specific properties for display in the Review tab.
//
// The backend returns the LLM's property blob verbatim as
// `ExtractionItem.properties` (JSONB). Each entity type has different
// keys — this module centralises which keys to surface and how to
// format them, so the Review row can show e.g. "role: plaintiff ·
// party_type: person" without every caller knowing the schema.
//
// Keep this list tight. A cluttered row defeats the purpose — scan
// value, not exhaustive dump.

import type { ExtractionItem } from "../services/pipelineApi";

// A single formatted property to render in the Review row.
export interface FormattedProperty {
  label: string;
  value: string;
}

// Max entries in list-valued properties (e.g. applies_to) before
// collapsing with "+N more".
const LIST_PREVIEW_LIMIT = 3;

// Entity-type → ordered list of property keys to surface.
// Fallback keys (e.g. `entity_kind` for `party_type`) are tried in order.
const KEY_MAP: Record<string, Array<{ label: string; keys: string[] }>> = {
  Party: [
    { label: "role", keys: ["role"] },
    { label: "party_type", keys: ["party_type", "entity_kind"] },
  ],
  Person: [
    { label: "role", keys: ["role"] },
    { label: "party_type", keys: ["party_type", "entity_kind"] },
  ],
  Organization: [
    { label: "role", keys: ["role"] },
    { label: "party_type", keys: ["party_type", "entity_kind"] },
  ],
  ComplaintAllegation: [
    { label: "paragraph", keys: ["paragraph_number", "paragraph_ref"] },
    { label: "category", keys: ["category"] },
    { label: "severity", keys: ["severity"] },
    { label: "applies_to", keys: ["applies_to"] },
  ],
};

// Coerce a JSON value to a short display string. Returns null if the
// value is empty / missing / unsupported (drops the entry from the row).
function formatValue(raw: unknown): string | null {
  if (raw == null) return null;
  if (typeof raw === "string") {
    const trimmed = raw.trim();
    return trimmed.length === 0 ? null : trimmed;
  }
  if (typeof raw === "number" || typeof raw === "boolean") {
    return String(raw);
  }
  if (Array.isArray(raw)) {
    const parts = raw
      .map((v) => (typeof v === "string" ? v.trim() : typeof v === "number" ? String(v) : null))
      .filter((v): v is string => v !== null && v.length > 0);
    if (parts.length === 0) return null;
    if (parts.length <= LIST_PREVIEW_LIMIT) return parts.join(", ");
    const head = parts.slice(0, LIST_PREVIEW_LIMIT).join(", ");
    return `${head}, +${parts.length - LIST_PREVIEW_LIMIT} more`;
  }
  return null;
}

// Pull the first non-empty value found under any of `keys` in `props`.
function firstValue(
  props: Record<string, unknown>,
  keys: string[],
): string | null {
  for (const key of keys) {
    const formatted = formatValue(props[key]);
    if (formatted !== null) return formatted;
  }
  return null;
}

// Build the display list for a Review row. Returns [] when no relevant
// properties are present — caller should skip rendering the row.
//
// Uses `resolved_entity_type` (post-ingest label like "Person") first
// because it's the user-visible type; falls back to the immutable LLM
// label (`entity_type`) for pre-ingest items.
export function formatItemProperties(item: ExtractionItem): FormattedProperty[] {
  const props = item.properties;
  if (!props || typeof props !== "object") return [];

  const effectiveType = item.resolved_entity_type ?? item.entity_type;
  const spec = KEY_MAP[effectiveType] ?? KEY_MAP[item.entity_type];
  if (!spec) return [];

  const out: FormattedProperty[] = [];
  for (const { label, keys } of spec) {
    const value = firstValue(props as Record<string, unknown>, keys);
    if (value !== null) out.push({ label, value });
  }
  return out;
}
