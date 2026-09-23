// =============================================================================
// navItems.ts — the navigation bar, as DATA
// =============================================================================
//
// ## Why this is a table and not JSX
//
// `Header.tsx` held the bar as two JSX literal arrays with the paths written
// inline — `"/explorer"`, `` `/cases/${DEFAULT_CASE_SLUG}/proof-matrix` ``. Two
// consequences, both of which this file ends:
//
// 1. **Nothing could check the bar.** The route-link guard reads builders out of
//    `routePaths.ts`; a template literal in a component is invisible to it. A
//    menu entry pointing at a route nobody declares was a 404 waiting for
//    whoever clicked it, and Part 1 found exactly that class of defect live
//    elsewhere in the app (`/admin/documents/:id/audit`).
// 2. **The shape and the content were tangled.** Rendering logic and "what is in
//    the menu" sat in the same expression, so changing the menu meant editing
//    JSX and re-reading the render to be sure nothing else moved.
//
// Every path below comes from a `routePaths` builder. That is the whole point:
// the existing guard now covers the entire bar for free.
//
// ## The descriptions are the MOCKUP'S, character for character
//
// `NAV_MOCKUP_v1_2026-08-19.html` writes a one-line description under each menu
// leaf, and those exact strings are reproduced below — em dashes, middle dots,
// curly quotes and all. They are not paraphrased and not improved.
//
// ONE of them carries a fact that can go stale: "44 parties, merged". That is a
// count of case data sitting in shared navigation chrome, and the day a merge
// changes it the menu is quietly wrong with nothing to catch it. Reproduced as
// drawn because the mockup is the signed spec; flagged in the report, because a
// number in a menu is a number nobody will remember to update.
//
// ## Why the labels are here rather than in the wording store
//
// The task permits either ("wording store / a declared nav table"). These eight
// strings are STRUCTURAL — they name places in the app, they are the same in
// every case and every deployment, and a settings row that could rename "Admin"
// to something else would be a way to make the app unnavigable from a database
// edit. The strings a WITNESS reads are in the store; the strings that name the
// furniture are here. See the practice deck's wording blocks for the other side
// of that line.

import { DEFAULT_CASE_SLUG } from "../services/caseHeader";
import {
  adminDataPath,
  adminLogsPath,
  adminPath,
  adminPromptsPath,
  adminSettingsPath,
  askPath,
  caseHealthPath,
  documentsPath,
  forYouPath,
  homePath,
  peoplePath,
  proofMatrixPath,
  timelinePath,
  trialPrepPath,
} from "../utils/routePaths";

/**
 * One leaf inside a dropdown.
 *
 * `detail` is the mockup's small second line. Optional because the top-level
 * flat links have no room for one, and `undefined` renders nothing rather than
 * an empty element that would still take vertical space.
 */
export interface NavLeafItem {
  label: string;
  path: string;
  detail?: string;
}

/** A bar entry: either a flat link (`path`) or a group (`children`). */
export interface NavBarItem {
  label: string;
  path?: string;
  children?: NavLeafItem[];
}

/**
 * The bar: Home · Trial Prep ▾ · Documents · Chat.
 *
 * Five items where there were eight. What left the top level: Proof Matrix ▾
 * (its two live leaves moved into Trial Prep ▾, its third was Evidence and is
 * removed), Case Health (into Trial Prep ▾), Settings (into Admin ▾).
 *
 * ## Domain note: Case Health moved, and the old comment argued it should not
 *
 * `Header.tsx` carried a note saying Case Health must stay top-level because
 * "the connection rate is impossible to overlook, and a leaf two clicks deep
 * would reintroduce exactly the invisibility it exists to remove". That argument
 * lost to the bar wrapping at 965 px — which makes EVERY item invisible, Case
 * Health included. It is now one click inside Trial Prep ▾, where the work it
 * reports on lives. Recorded rather than deleted, because it was a real
 * argument and somebody may want to re-make it.
 */
export const NAV_ITEMS: NavBarItem[] = [
  { label: "Home", path: homePath() },
  {
    label: "Trial Prep",
    children: [
      {
        label: "Scenarios",
        path: trialPrepPath(DEFAULT_CASE_SLUG),
        detail: "the accusations, each with its rulings — today's “War Room”",
      },
      {
        label: "Proof Matrix",
        path: proofMatrixPath(DEFAULT_CASE_SLUG),
        detail: "elements → allegations → verified proof · tabs: Matrix · Proof Review",
      },
      {
        label: "People",
        path: peoplePath(),
        detail: "44 parties, merged",
      },
      {
        label: "Case Health",
        path: caseHealthPath(DEFAULT_CASE_SLUG),
        detail: "is the proof ready? — per document",
      },
      {
        label: "Timeline",
        path: timelinePath(),
        detail: "the chronology, from Roman's spreadsheet",
      },
    ],
  },
  // "For you" is TOP-LEVEL and deliberately so: it is the one entry that
  // carries a count, and a badge two clicks inside a dropdown is a badge nobody
  // sees. The label is here rather than in the wording store under ruling Q5 —
  // it names a place in the app, like every other label in this table, and the
  // bar must not be blank until a fetch lands. Every string ON the page itself
  // is a settings row.
  { label: "For you", path: forYouPath(DEFAULT_CASE_SLUG) },
  { label: "Documents", path: documentsPath() },
  { label: "Chat", path: askPath() },
];

/**
 * Admin ▾ — shown only when `user.permissions.is_admin`.
 *
 * Five real ADDRESSES where there were nine tabs in component state. A tab that
 * is not addressable cannot be bookmarked, cannot be reached by Back, and cannot
 * be linked to from anywhere — which is why "the audit log" was a place you
 * could only get to by clicking through.
 */
export const ADMIN_ITEMS: NavLeafItem[] = [
  { label: "Overview", path: adminPath(), detail: "stores · metrics · step performance" },
  {
    label: "Prompt Management",
    path: adminPromptsPath(),
    detail: "Prompts · System Prompts · Profiles · Schemas · Models",
  },
  { label: "Data", path: adminDataPath(), detail: "Neo4j · Qdrant · PostgreSQL · Indexing" },
  { label: "Logs", path: adminLogsPath(), detail: "Chats · Audit" },
  { label: "Settings", path: adminSettingsPath(), detail: "the settings & wording store" },
];

/**
 * What the bar shows in place of the For you count when the count could not be
 * read, and what a screen reader says for it.
 *
 * ## Why these two strings are HERE and not in the wording store
 *
 * The same argument the header of this file makes for the labels, and one more
 * that is specific to them: the count arrives on its own small request, and
 * when that request fails NOTHING has arrived — including any sentence the
 * store might have held to describe the failure. A stored row for this case
 * would be a row that is unreachable exactly when it is needed. So it is
 * chrome, like "Admin", and lives with the chrome.
 */
// STRUCTURAL: these two must be available at exactly the moment the wording
// store is not — they describe the failure of the request that would have
// carried a stored sentence. A settings row for this case is a row that is
// unreachable when it is needed.
export const FOR_YOU_COUNT_UNAVAILABLE_MARK = "!";
// STRUCTURAL: same reason as the mark above.
export const FOR_YOU_COUNT_UNAVAILABLE_LABEL =
  "For you: the count could not be read — open the page to see the list";

/**
 * Is this bar item the one the current route belongs to?
 *
 * Exported so `Header` and `NavDropdown` share ONE definition. They each carried
 * a private copy of these three lines with a comment explaining that duplicating
 * them was cheaper than a shared module — which was true when the alternative
 * was a circular import between those two files. It is not true now that the
 * table itself is a third module both already depend on.
 *
 * `/` is special-cased: every path starts with it, so the prefix test would
 * make Home permanently active.
 */
export function isActivePath(itemPath: string, currentPath: string): boolean {
  if (itemPath === "/") return currentPath === "/";
  return currentPath === itemPath || currentPath.startsWith(itemPath + "/");
}

/** Is any leaf of this group the active route? Drives the group's highlight. */
export function isGroupActive(children: NavLeafItem[], currentPath: string): boolean {
  return children.some((child) => isActivePath(child.path.split("?")[0], currentPath));
}
