// Shared strength color configuration used by AnalysisPage and EvidenceExplorerParts

export const STRENGTH_COLORS: Record<string, { bg: string; text: string; bar: string }> = {
  strong: { bg: "var(--state-success-bg-soft)", text: "var(--status-active-text)", bar: "var(--state-success-strong)" },
  moderate: { bg: "var(--accent-bg-soft)", text: "var(--accent-primary-hover)", bar: "var(--accent-primary)" },
  weak: { bg: "var(--burden-warning-bg)", text: "var(--burden-warning-text)", bar: "var(--state-warning-strong)" },
  gap: { bg: "var(--state-danger-bg-soft)", text: "var(--status-dropped-text)", bar: "var(--state-danger-strong)" },
};

export const DEFAULT_STRENGTH_COLOR = { bg: "var(--bg-page)", text: "var(--text-secondary)", bar: "var(--text-disabled)" };

export function getStrengthStyle(category: string) {
  return STRENGTH_COLORS[category] || DEFAULT_STRENGTH_COLOR;
}
