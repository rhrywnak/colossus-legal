/**
 * Map internal evidence status values to proper legal terminology.
 *
 * "Proven" implies a court ruling (jury's job). "Supported" means
 * we have evidence backing the allegation — appropriate for pre-trial.
 */
const STATUS_DISPLAY: Record<string, string> = {
  PROVEN: "Supported",
  PARTIAL: "Partially Supported",
  UNPROVEN: "Unsupported",
};

export function displayStatus(status: string | undefined): string {
  if (!status) return "";
  return STATUS_DISPLAY[status.toUpperCase()] || status;
}
