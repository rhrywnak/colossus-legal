// modelJobsView.ts — the jobs panel's pure helpers (CC_TASK_MODEL_JOBS_PANEL_v1).
//
// Only presentation lives here: how one dropdown entry reads, and which admin
// address a link goes to. Every WORD arrives from the server; the marks below
// ("✓", "·") are punctuation, drawn as board 2 draws them.

import type { AiJob, AiJobChange, AiJobOption } from "../../services/aiJobs";
import { adminDataPath, adminPromptsPath } from "../../utils/routePaths";
import type { AdminPanel } from "../../pages/adminGroups";

/**
 * One dropdown entry as board 2 writes it:
 *   "Claude Opus 5.5 ✓"                        (current, no marker)
 *   "question_chat_prompt_v1.md ✓ in use"      (current, with its marker)
 *   "question_chat_prompt_v2.md · new, never used"
 */
export function optionText(o: AiJobOption): string {
  if (o.current) return o.marker ? `${o.label} ✓ ${o.marker}` : `${o.label} ✓`;
  return o.marker ? `${o.label} · ${o.marker}` : o.label;
}

/** Where a panel link goes: an address, and the tab to open there. */
export type LinkTarget = { path: string; panel: AdminPanel | null };

/**
 * The server names a link by a stable token; the page owns its addresses.
 * `null` for a token this build does not know — the link is then drawn as
 * plain text rather than as a link to nowhere.
 */
export function linkTarget(target: string): LinkTarget | null {
  switch (target) {
    case "models":
      return { path: adminPromptsPath(), panel: "models" };
    case "files":
      return { path: adminPromptsPath(), panel: "prompts" };
    case "data":
      return { path: adminDataPath(), panel: "lastRun" };
    default:
      return null;
  }
}

/** The two control columns of a job row, by the names the DTO gives them. */
export type JobColumn = "first" | "instructions";

/**
 * What Save would send: every dropdown whose pick differs from the value in
 * force, keyed by the setting it writes. Empty means nothing to save, and Save
 * stays at rest (board 1's `.btn.off`).
 */
export function pendingChanges(job: AiJob, picked: Partial<Record<JobColumn, string>>): AiJobChange[] {
  const out: AiJobChange[] = [];
  for (const column of ["first", "instructions"] as const) {
    const control = job[column];
    const value = picked[column];
    if (control.kind === "choose" && value !== undefined && value !== control.current) {
      out.push({ key: control.key, value });
    }
  }
  return out;
}
