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
  /** The area of the page this row belongs to — decided by the backend. */
  area_id: string;
  /** The block inside that area. The browser holds no copy of the grouping. */
  block_id: string;
  /**
   * "Changed — default: 2048" when the stored value has moved off its default;
   * `null` when it has not.
   *
   * Both halves matter: `!== null` is the fact the landing list is built from,
   * and the string is the sentence rendered under the row. The page never
   * composes that sentence itself, and never re-derives the fact by comparing
   * `value` to `default_value` — that comparison is the server's definition of
   * "changed" and there is only meant to be one of it.
   */
  changed_from_default: string | null;
  /**
   * The coupled group that edits this row, when one does; `null` otherwise.
   *
   * `null` is the ordinary case. A non-null value means this row is NOT
   * editable on its own — the backend refuses a single-row save of it — and the
   * page renders the group's editor in its place.
   */
  group_id: string | null;
  /**
   * Set when Admin → Overview's jobs panel owns this row
   * (CC_TASK_MODEL_JOBS_PANEL_v1): the row is read-only here and this is the
   * sentence saying where to change it. The server refuses a save as well.
   */
  owned_elsewhere: string | null;
};

/** One column of a coupled group — a single stored row holding one list. */
export type CoupledColumnDto = {
  key: string;
  label: string;
  placeholder: string;
};

/**
 * A set of rows the page edits as ONE control.
 *
 * The entries arrive TRANSPOSED — one inner list per entry, one cell per column
 * — which is what makes "one list has two and the other has one" impossible to
 * represent, let alone submit. The store's own encoding (comma-separated lists,
 * a `none` token, de-duplication) never reaches the browser.
 */
export type CoupledGroupDto = {
  id: string;
  label: string;
  note: string;
  /** What one entry is called, for the Add control ("Add a reviewer"). */
  entry_noun: string;
  columns: CoupledColumnDto[];
  entries: string[][];
};

/** One openable group in the rail. */
export type BlockDto = {
  id: string;
  label: string;
  /** Stored rows that landed here. Counted by the backend, never by this page. */
  count: number;
};

/** One entry in the page's left-hand rail. */
export type AreaDto = {
  id: string;
  label: string;
  count: number;
  /** Said under the heading when the area needs explaining; `null` otherwise. */
  note: string | null;
  blocks: BlockDto[];
};

export type SettingsPageDto = {
  /** Live parameters first, then dormant — ordered by the backend. */
  settings: SettingDto[];
  /**
   * The rail, in the order the page shows it, with every count already taken.
   *
   * ## Why the counts are never computed here
   *
   * The mockup this page was built from carried ten area counts summing to 863.
   * Three of them were out of date by the time it was built — the store had
   * moved under it. A count the page worked out from the rows it happens to
   * hold would be right only for as long as it held all of them, which is
   * exactly the assumption a cap or a filter breaks.
   */
  areas: AreaDto[];
  /**
   * The coupled groups, each with its current entries already decoded.
   *
   * Empty is a real answer: a build with no coupled rows serves `[]`.
   */
  groups: CoupledGroupDto[];
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
  // The rail is load-bearing in its own right: without it the page has no
  // grouping and no counts, and would render 860 rows in one flat list — which
  // is precisely the page this one replaced. An empty array is a REAL answer
  // (an empty store), so only a missing or wrong-typed field is refused.
  if (!Array.isArray(parsed.groups)) {
    throw new Error(
      `The settings response carried no coupled groups — backend/frontend ` +
        `contract mismatch. Rows that must be edited together would render as ` +
        `single fields that cannot be saved. If this persists, report it to ` +
        `the site administrator.`,
    );
  }
  if (!Array.isArray(parsed.areas)) {
    throw new Error(
      `The settings response carried no areas — backend/frontend contract ` +
        `mismatch. The page cannot group ${parsed.settings.length} parameters ` +
        `without them. If this persists, report it to the site administrator.`,
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

/**
 * Write a whole coupled group in one go.
 *
 * ## Why there is a second write function at all
 *
 * `setSetting` changes one row. Rows that are read index-aligned cannot be
 * changed one at a time — whichever goes first leaves the lists a different
 * length, and the backend refuses it in either order. That is the defect this
 * endpoint exists to end: v2.1.14 shipped unable to add a second reviewer.
 *
 * `entries` is one inner list per entry, one cell per column, in the group's
 * own column order. Every rule about what is acceptable lives on the backend
 * beside the stored rows; this function carries the text and surfaces the
 * refusal intact, exactly as `setSetting` does.
 */
export async function setSettingGroup(
  groupId: string,
  entries: string[][],
): Promise<SettingChanged> {
  const response = await authFetch(
    `${API_BASE_URL}/api/settings/group/${encodeURIComponent(groupId)}`,
    {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ entries }),
    },
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(`Could not save ${groupId} (HTTP ${response.status}${detail}).`);
  }

  const data: unknown = await response.json();
  const parsed = data as Partial<SettingChanged>;
  if (typeof parsed.message !== "string") {
    throw new Error(
      `The response for ${groupId} carried no confirmation — the change may or ` +
        `may not have been saved. Reload the page to see the stored entries.`,
    );
  }
  return parsed as SettingChanged;
}
