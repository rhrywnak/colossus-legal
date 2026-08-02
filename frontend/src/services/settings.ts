// =============================================================================
// settings.ts — client for the admin Settings page (task 1.6, v2 §2b)
// -----------------------------------------------------------------------------
// Endpoints:
//   GET /api/settings      → every parameter, composed for display
//   PUT /api/settings/:key → change one
//
// Same idioms as the sibling services: `authFetch` (credentials + a 30s
// AbortController timeout), `encodeURIComponent` on every path param,
// `readErrorMessage` to surface the backend `{message}`, every non-2xx throws.
//
// ## The browser composes nothing, and validates nothing
//
// The meaning, the input hint, the bounds sentence and the dormancy label all
// arrive built. The VALUE is sent as typed and validated on the backend against
// the parameter's stored kind and bounds — a browser-side check would be a
// second implementation of the configuration law, in the one place that cannot
// see the store. A refusal comes back as a sentence naming the parameter, the
// limit and what to type instead; the page renders it verbatim.
//
// The shapes mirror backend/src/dto/settings.rs verbatim.
// =============================================================================

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { readErrorMessage } from "./fetchUtils";

export type SettingDto = {
  /** The stable key — what a log line or a psql query names. */
  key: string;
  /** The current value, as text, exactly as stored and as it is edited. */
  value: string;
  /** What it shipped as, so a human can see they have moved it. */
  default_value: string;
  /** One line of plain language with its § citation. */
  meaning: string;
  /** What to type — "a whole number, e.g. 240". */
  input_hint: string;
  /** "Between 0 and 1" / "At least 1"; `null` when unbounded both ways. */
  bounds_label: string | null;
  /** Present when nothing reads this parameter yet. `null` when it is live. */
  dormant_note: string | null;
  /** "Last changed by Roman on 2026-08-01", or that it never has been. */
  last_changed: string;
};

export type SettingsPageDto = {
  /** Live parameters first, then dormant — ordered by the backend. */
  settings: SettingDto[];
};

export type SettingChanged = {
  key: string;
  value: string;
  /** The plain confirmation, composed server-side. Rendered verbatim. */
  message: string;
};

/** Load every parameter. */
export async function fetchSettings(): Promise<SettingsPageDto> {
  const response = await authFetch(`${API_BASE_URL}/api/settings`);

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to load the settings (HTTP ${response.status}${detail}). Try again.`,
    );
  }

  const data: unknown = await response.json();
  const parsed = data as Partial<SettingsPageDto>;

  // Validate the load-bearing shape so a contract mismatch throws HERE with
  // context rather than as an `undefined.map` inside the page.
  if (!Array.isArray(parsed.settings)) {
    throw new Error(
      `The settings response is missing its parameter list — backend/frontend ` +
        `contract mismatch. If this persists, report it to the site administrator.`,
    );
  }
  return parsed as SettingsPageDto;
}

/**
 * Change one parameter.
 *
 * The value is sent as typed. Every rule about what is acceptable lives in the
 * backend beside the stored bounds; this function's job is to carry the text and
 * to surface the refusal intact.
 */
export async function setSetting(key: string, value: string): Promise<SettingChanged> {
  const response = await authFetch(
    `${API_BASE_URL}/api/settings/${encodeURIComponent(key)}`,
    {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ value }),
    },
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Could not change ${key} (HTTP ${response.status}${detail}).`,
    );
  }

  const data: unknown = await response.json();
  const parsed = data as Partial<SettingChanged>;
  if (typeof parsed.message !== "string" || typeof parsed.value !== "string") {
    throw new Error(
      `The response for ${key} carried no confirmation — the change may or may ` +
        `not have been saved. Reload the page to see its actual value.`,
    );
  }
  return parsed as SettingChanged;
}
