/**
 * Shared display names for pipeline processing steps.
 * Single source of truth — used by AdminMetrics, ProcessingPanel, etc.
 */
export const STEP_DISPLAY_NAMES: Record<string, string> = {
  upload: "Upload",
  extract_text: "Read Document",
  extract: "Analyze Content",
  verify: "Verify Accuracy",
  review: "Human Review",
  ingest: "Build Knowledge Graph",
  index: "Enable Search",
  completeness: "Quality Check",
};

/** Ordered step keys matching the pipeline sequence. */
export const STEP_ORDER = Object.keys(STEP_DISPLAY_NAMES);
