// =============================================================================
// adminGroups.ts — the five admin areas, and the tabs inside each
// =============================================================================
//
// ## What changed, and why it is more than a re-labelling
//
// The admin surface was ONE route (`/admin`) with nine tabs held in `useState`.
// Nine tabs is not a menu, it is a wall — and none of them was addressable:
// the audit log could not be bookmarked, could not be returned to with Back, and
// could not be linked to from anywhere in the app or from a message to Roman.
//
// The five groups below are real ROUTES. The tabs within a group stay component
// state, and that split is the point: "which of the five admin areas am I in"
// is a PLACE, and "which of the five prompt tables am I looking at" is a view of
// one place. Addresses for the first, state for the second.
//
// Nothing inside any panel changed. Every one of the nine existing components is
// rendered by exactly the same JSX it was before, under a different heading.

/** The five addressable admin areas. */
export type AdminGroup = "overview" | "prompts" | "data" | "logs" | "settings";

/** Every panel that exists. One id per existing admin component. */
export type AdminPanel =
  | "metrics"
  | "indexing"
  | "chats"
  | "audit"
  | "models"
  | "profiles"
  | "prompts"
  | "schemas"
  | "systemPrompts";

/** One group: its heading, and the panels it shows as sub-tabs. */
export interface AdminGroupSpec {
  heading: string;
  /**
   * The sub-tabs, in the order the design lists them.
   *
   * A group with ONE panel renders no tab bar at all — a single tab is a
   * control that cannot do anything, and drawing one would be the page
   * pretending to offer a choice.
   */
  panels: Array<{ id: AdminPanel; label: string }>;
  /**
   * Does this group show the store-connectivity strip (Neo4j · Qdrant ·
   * PostgreSQL)?
   *
   * Part 1's measurement: those three are not tabs, they are status dots
   * rendered above the tab bar, and they used to appear on all nine tabs
   * regardless of relevance. They belong on Overview (is anything down?) and on
   * Data (these are the stores this page is about) and nowhere else.
   */
  stores: boolean;
}

export const ADMIN_GROUPS: Record<AdminGroup, AdminGroupSpec> = {
  overview: {
    heading: "Admin",
    panels: [{ id: "metrics", label: "Metrics" }],
    stores: true,
  },
  prompts: {
    heading: "Prompt Management",
    panels: [
      { id: "prompts", label: "Prompts" },
      { id: "systemPrompts", label: "System Prompts" },
      { id: "profiles", label: "Profiles" },
      { id: "schemas", label: "Schemas" },
      { id: "models", label: "Models" },
    ],
    stores: false,
  },
  data: {
    heading: "Data",
    panels: [{ id: "indexing", label: "Indexing" }],
    stores: true,
  },
  logs: {
    heading: "Logs",
    panels: [
      { id: "chats", label: "Chats" },
      { id: "audit", label: "Audit" },
    ],
    stores: false,
  },
  // Settings is a group in the MENU and a page of its own in the router — it
  // renders `SettingsPage` unchanged rather than a panel from the list above.
  // Present here so the five groups are enumerable in one place (the reachability
  // test walks this table), with an empty panel list saying it has no sub-tabs.
  settings: { heading: "Settings", panels: [], stores: false },
};

/**
 * The panel a group opens on.
 *
 * The FIRST in its list, always — the order in the table is the design's order,
 * so "the first one" and "the one the design puts first" cannot disagree.
 * Returns `null` for a group with no panels (Settings), which the page reads as
 * "render the page's own content, not a panel".
 */
export function defaultPanel(group: AdminGroup): AdminPanel | null {
  return ADMIN_GROUPS[group].panels[0]?.id ?? null;
}
