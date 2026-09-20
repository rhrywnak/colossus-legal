// =============================================================================
// settingsSearch — the four states' arithmetic, with no React in it
// =============================================================================
//
// The Settings page has four states: a landing list, an area's collapsed groups,
// one open group, and a search that cuts across everything. Which rows each of
// them shows is decided here, in pure functions over the rows the server sent.
//
// ## Why this is not "business logic in the frontend"
//
// Nothing here decides what a parameter MEANS, what it may be set to, or which
// block it belongs to — those arrive composed and placed (see
// `services/settings_map.rs`). These functions narrow a list the browser already
// holds. The distinction that matters: if the server changed its mind about any
// of it, not one line below would have to change.
//
// ## Why searching happens in the browser at all
//
// Measured on the live store: 863 rows serialise to 443 KiB, 74 KiB gzipped —
// about 524 bytes a row. The page already fetches every row in one read (it has
// since the first version), an admin opens it deliberately, and filtering 863
// short strings is sub-millisecond. A search endpoint would add a round trip per
// keystroke to save nothing. If the store ever grows to where that stops being
// true, the fix is an endpoint, and the shape below is what it would serve.
//
// ## Nothing in this file knows what application it is in
//
// No case name, no scenario type, no wording constant. The only import is the
// row type from the settings service. That is the whole of what makes this page
// portable: it renders a contract, not a product.

import type { AreaDto, SettingDto } from "../../services/settings";

/**
 * How many rows a search will render before it stops and says how many it found.
 *
 * The true total is always reported: a capped list that does not say it is
 * capped is a list that quietly answers a different question than the one asked.
 */
// STRUCTURAL: quoted from ADMIN_SETTINGS_MOCKUP_v3's own state caption —
// "results grouped, capped at 50 with the count told". It encodes a ruling
// about how the page behaves, not a value that varies by deployment, and the
// page states the true total beside it so the cap is never load-bearing for
// correctness. ⚑ Flagged by architecture-reviewer 2026-09-19 as wanting a
// config source; recorded for Roman's sign-off. Making it genuinely tunable
// means a settings ROW, which needs a migration this task was told not to write
// and would push `domain/settings.rs` past 300 (it stands at 290).
export const SEARCH_RESULT_CAP = 50;

/**
 * Above this many rows, an open group grows its own pinned filter box.
 *
 * Forty is roughly a screen and a half. Below it the filter is clutter over a
 * list you can already see; above it, scrolling has lost you the heading.
 */
// STRUCTURAL: quoted from ADMIN_SETTINGS_MOCKUP_v3's own state caption —
// "a pinned filter appears past 40". Same standing as SEARCH_RESULT_CAP above,
// and flagged by the same review; see that comment for what tunable would cost.
export const GROUP_FILTER_THRESHOLD = 40;

/** One area's slice of a search result. */
export type SearchGroup = {
  areaId: string;
  areaLabel: string;
  /** Every row in this area that matched — not only the ones being shown. */
  matched: number;
  /** The rows to render, after the page-wide cap has been spent. */
  settings: SettingDto[];
};

/** What a search answers. */
export type SearchResults = {
  /** Matches across every area. The number the page states out loud. */
  total: number;
  /** How many of them are actually rendered. Equal to `total` under the cap. */
  shown: number;
  groups: SearchGroup[];
};

/** Case-insensitive, whitespace-trimmed. Empty means "no query". */
export function normaliseQuery(query: string): string {
  return query.trim().toLowerCase();
}

/**
 * Does this row match?
 *
 * Three fields, because a human looking for a setting knows one of three things
 * about it: what it says on screen (`value`), what it does (`meaning`), or what
 * it is called in a log line (`key`). The mockup's own example — typing
 * "reviewer" and expecting both `practice_reviewer_usernames` and the sentence
 * about who may press Done reviewing — needs two of the three.
 */
export function matches(setting: SettingDto, normalised: string): boolean {
  if (normalised === "") return true;
  return (
    setting.key.toLowerCase().includes(normalised) ||
    setting.meaning.toLowerCase().includes(normalised) ||
    setting.value.toLowerCase().includes(normalised)
  );
}

/**
 * Search every area, grouped, in the rail's order, capped at
 * [`SEARCH_RESULT_CAP`] rows.
 *
 * An area contributes a group only if something in it matched; the cap is spent
 * across groups in rail order, so the first areas fill first and a truncated
 * group still reports its own true count.
 */
export function searchSettings(
  settings: readonly SettingDto[],
  areas: readonly AreaDto[],
  query: string,
): SearchResults {
  const normalised = normaliseQuery(query);
  const hits = settings.filter((setting) => matches(setting, normalised));

  const byArea = new Map<string, SettingDto[]>();
  for (const hit of hits) {
    const bucket = byArea.get(hit.area_id);
    if (bucket) bucket.push(hit);
    else byArea.set(hit.area_id, [hit]);
  }

  let budget = SEARCH_RESULT_CAP;
  const groups: SearchGroup[] = [];
  for (const area of areas) {
    const matched = byArea.get(area.id);
    if (!matched || matched.length === 0) continue;
    const take = Math.max(0, Math.min(budget, matched.length));
    budget -= take;
    groups.push({
      areaId: area.id,
      areaLabel: area.label,
      matched: matched.length,
      settings: matched.slice(0, take),
    });
  }

  return {
    total: hits.length,
    shown: groups.reduce((sum, group) => sum + group.settings.length, 0),
    groups,
  };
}

/** A search split into what is rendered and what the cap left out. */
export type SearchSpend = {
  /** Groups that got at least one row. Rendered in full, headings and all. */
  rendered: SearchGroup[];
  /** Groups the cap never reached. Named in one line, with their true counts. */
  alsoMatched: SearchGroup[];
};

/**
 * Split a result into the groups worth rendering and the groups to name.
 *
 * ## Why this exists — found in the browser, not in a fixture
 *
 * Searching "the" on the live store matches 837 rows. Practice alone holds 295
 * of them, so it takes the whole fifty-row cap and the other ten areas each
 * render a heading, no rows, and a line saying how many more there are. Eleven
 * headings with nothing under them reads as a page that has broken, not as a
 * page that has stopped early — and the ten counts, which are true and useful,
 * are the part that gets lost in the noise.
 *
 * So the groups the cap never reached are named in a single line instead. The
 * arithmetic is unchanged and nothing is hidden: every area that matched is
 * still named, with its own true count.
 */
export function splitBySpend(results: SearchResults): SearchSpend {
  return {
    rendered: results.groups.filter((group) => group.settings.length > 0),
    alsoMatched: results.groups.filter((group) => group.settings.length === 0),
  };
}

/**
 * The landing list: every row whose stored value has moved off its default.
 *
 * Ruled 2026-09-19. The alternative — the most recently touched rows — reads
 * well and measures badly: 498 of the store's rows were last written by a
 * migration and 353 by the seed, so "recently changed" ordered by time is a wall
 * of whatever the last deploy stamped. Eight rows differ from their defaults.
 * That is the list a human means.
 *
 * The FACT is the server's: a row is in this list exactly when it arrived
 * carrying `changed_from_default`. Nothing here re-decides it by comparing
 * strings, which would be a second definition of "changed" in the one place that
 * cannot see the store.
 */
export function changedFromDefault(
  settings: readonly SettingDto[],
): SettingDto[] {
  return settings.filter((setting) => setting.changed_from_default !== null);
}

/** Does an open group of this size get its own pinned filter box? */
export function needsGroupFilter(rowCount: number): boolean {
  return rowCount > GROUP_FILTER_THRESHOLD;
}

/** Narrow the rows already open in one group. Uncapped — the group is the cap. */
export function filterWithin(
  settings: readonly SettingDto[],
  query: string,
): SettingDto[] {
  const normalised = normaliseQuery(query);
  return settings.filter((setting) => matches(setting, normalised));
}

/** One run of text, flagged when it is part of what the query matched. */
export type Segment = { text: string; hit: boolean };

/**
 * Split `text` into alternating plain and matched runs, so the page can mark the
 * matched ones without building HTML from a string.
 *
 * ## Why segments rather than marked-up HTML
 *
 * A function returning `"…<mark>reviewer</mark>…"` would have to be rendered
 * with `dangerouslySetInnerHTML`, which turns every stored setting value — text
 * an admin types — into markup the browser will execute. Returning data and
 * letting JSX render it keeps that door shut by construction.
 */
export function highlight(text: string, query: string): Segment[] {
  const normalised = normaliseQuery(query);
  if (normalised === "") return [{ text, hit: false }];

  const segments: Segment[] = [];
  const haystack = text.toLowerCase();
  let from = 0;
  for (;;) {
    const at = haystack.indexOf(normalised, from);
    if (at === -1) break;
    if (at > from) segments.push({ text: text.slice(from, at), hit: false });
    segments.push({ text: text.slice(at, at + normalised.length), hit: true });
    from = at + normalised.length;
  }
  if (from < text.length) segments.push({ text: text.slice(from), hit: false });
  return segments.length > 0 ? segments : [{ text, hit: false }];
}
