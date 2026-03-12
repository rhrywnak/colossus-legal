// Shared strength color configuration used by AnalysisPage and EvidenceExplorerParts

export const STRENGTH_COLORS: Record<string, { bg: string; text: string; bar: string }> = {
  strong: { bg: "#dcfce7", text: "#166534", bar: "#22c55e" },
  moderate: { bg: "#dbeafe", text: "#1e40af", bar: "#3b82f6" },
  weak: { bg: "#fef3c7", text: "#92400e", bar: "#f59e0b" },
  gap: { bg: "#fee2e2", text: "#991b1b", bar: "#ef4444" },
};

export const DEFAULT_STRENGTH_COLOR = { bg: "#f3f4f6", text: "#374151", bar: "#9ca3af" };

export function getStrengthStyle(category: string) {
  return STRENGTH_COLORS[category] || DEFAULT_STRENGTH_COLOR;
}
