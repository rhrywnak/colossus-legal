/**
 * useSchema — schema-driven entity type colors and display names.
 *
 * Provides getColor() and getDisplayName() for any entity type label.
 * Colors come from a default map for known types, with a deterministic
 * hash-based fallback for unknown types. Display names auto-convert
 * PascalCase to spaced words (e.g., "ComplaintAllegation" → "Complaint Allegation").
 *
 * Handles both PascalCase ("ComplaintAllegation") and snake_case
 * ("complaint_allegation") inputs by normalizing to PascalCase internally.
 */

// ── Default color map ──────────────────────────────────────────

/**
 * Solid colors for entity type badges (white text on colored background).
 * Used by ContentPanel, ReviewPanel, and anywhere a solid badge is needed.
 */
const DEFAULT_COLORS: Record<string, string> = {
  ComplaintAllegation: "#3B82F6",  // blue
  Evidence:           "#10B981",   // green
  Person:             "#8B5CF6",   // purple
  Organization:       "#F59E0B",   // amber
  Harm:               "#EF4444",   // red
  LegalCount:         "#6366F1",   // indigo
  MotionClaim:        "#EC4899",   // pink
  Document:           "#6B7280",   // gray
};

/**
 * Background/text color pairs for lighter badge styling.
 * Used by RetrievalDetailsPanel, SearchPage, and anywhere a subtle badge is needed.
 */
const DEFAULT_BADGE_COLORS: Record<string, { bg: string; text: string }> = {
  Evidence:            { bg: "#dbeafe", text: "#1e40af" },
  ComplaintAllegation: { bg: "#ede9fe", text: "#5b21b6" },
  MotionClaim:         { bg: "#fef3c7", text: "#92400e" },
  Harm:                { bg: "#fee2e2", text: "#991b1b" },
  LegalCount:          { bg: "#d1fae5", text: "#065f46" },
  Document:            { bg: "#e0e7ff", text: "#3730a3" },
  Person:              { bg: "#fce7f3", text: "#9d174d" },
  Organization:        { bg: "#f3e8ff", text: "#6b21a8" },
};

const DEFAULT_BADGE_FALLBACK = { bg: "#f3f4f6", text: "#374151" };

/**
 * Plural display labels for filter chips and UI sections.
 */
const DEFAULT_PLURAL_LABELS: Record<string, string> = {
  Evidence: "Evidence",
  ComplaintAllegation: "Allegations",
  MotionClaim: "Claims",
  Document: "Documents",
  Person: "People",
  Organization: "Organizations",
  Harm: "Harms",
  LegalCount: "Legal Counts",
};

// ── Normalization helpers ──────────────────────────────────────

/**
 * Normalize snake_case to PascalCase.
 * "legal_count" → "LegalCount", "complaint_allegation" → "ComplaintAllegation"
 * Already-PascalCase inputs pass through unchanged.
 */
function toPascalCase(s: string): string {
  if (!s.includes("_") && s[0] === s[0].toUpperCase()) return s;
  return s
    .split("_")
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1).toLowerCase())
    .join("");
}

/**
 * Convert PascalCase to spaced words.
 * "ComplaintAllegation" → "Complaint Allegation"
 * "LegalCount" → "Legal Count"
 */
function pascalToSpaced(s: string): string {
  return s.replace(/([a-z])([A-Z])/g, "$1 $2");
}

/**
 * Generate a deterministic color from a string label.
 * Produces consistent hex colors for unknown entity types.
 */
function hashColor(label: string): string {
  let hash = 0;
  for (let i = 0; i < label.length; i++) {
    hash = label.charCodeAt(i) + ((hash << 5) - hash);
  }
  // Generate a muted color (hue from hash, fixed saturation/lightness)
  const hue = Math.abs(hash) % 360;
  return `hsl(${hue}, 55%, 50%)`;
}

function hashBadgeColor(label: string): { bg: string; text: string } {
  let hash = 0;
  for (let i = 0; i < label.length; i++) {
    hash = label.charCodeAt(i) + ((hash << 5) - hash);
  }
  const hue = Math.abs(hash) % 360;
  return {
    bg: `hsl(${hue}, 40%, 92%)`,
    text: `hsl(${hue}, 60%, 30%)`,
  };
}

// ── Public API ─────────────────────────────────────────────────

/**
 * Get the solid background color for an entity type.
 * Accepts PascalCase or snake_case input.
 */
export function getColor(label: string): string {
  const key = toPascalCase(label);
  return DEFAULT_COLORS[key] ?? hashColor(key);
}

/**
 * Get the badge color pair (light background + dark text) for an entity type.
 * Accepts PascalCase or snake_case input.
 */
export function getBadgeColor(label: string): { bg: string; text: string } {
  const key = toPascalCase(label);
  return DEFAULT_BADGE_COLORS[key] ?? hashBadgeColor(key);
}

/**
 * Get the human-readable display name for an entity type.
 * "ComplaintAllegation" → "Complaint Allegation"
 * Accepts PascalCase or snake_case input.
 */
export function getDisplayName(label: string): string {
  const key = toPascalCase(label);
  return pascalToSpaced(key);
}

/**
 * Get the plural display label for filter chips and sections.
 * "ComplaintAllegation" → "Allegations"
 * Falls back to display name + "s" for unknown types.
 */
export function getPluralLabel(label: string): string {
  const key = toPascalCase(label);
  return DEFAULT_PLURAL_LABELS[key] ?? pascalToSpaced(key) + "s";
}
