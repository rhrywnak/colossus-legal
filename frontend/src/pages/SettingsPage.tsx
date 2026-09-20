// =============================================================================
// SettingsPage — the admin configuration surface (§2b, rebuilt to mockup v3)
// =============================================================================
//
// The law names this page and what it must show: "an admin Settings page listing
// every parameter with its current value, its default, and a one-line
// plain-language meaning; edits take effect on next read."
//
// ## Why it was rebuilt
//
// It listed all 863 parameters in one flat column. Everything the law asked for
// was on the screen and none of it could be found. ADMIN_SETTINGS_MOCKUP_v3
// (2026-09-19) is the ruled answer: four states, never more than a screenful at
// a time.
//
//   1. LANDING      a search box, and the rows that differ from their defaults
//   2. AREA         one area's blocks, collapsed, with their counts
//   3. GROUP OPEN   one block's rows; past forty, a filter pinned to the top
//   4. SEARCH       across every area at once, grouped, capped, count told
//
// ## What this page does not know
//
// It does not know it is in Colossus-Legal. It holds no case name, no scenario
// type, no wording constant, and no list of what the settings are — the areas,
// the blocks, the labels and the counts all arrive from the server, which reads
// them off the `*_KEYS` constants the wording blocks already declare. The only
// import that crosses out of this folder is the settings service itself, and
// `settingsImportRule.test.ts` fails the build if another one appears.
//
// That is not tidiness. A page that renders a contract rather than a product can
// be lifted into another Colossus application whole, and the reusability
// checkpoint is one of the three standing rules.
//
// ## What the browser does NOT do
//
// It does not compose the meaning, the hint, the bounds line, the dormancy
// label, the changed-from-default phrase or any count. It does not validate a
// value: the rules live beside the stored bounds on the backend, and a
// browser-side copy would be a second implementation of the configuration law in
// the one place that cannot see the store. A refusal comes back as a sentence
// naming the parameter and the limit, and is rendered verbatim beside the field
// that caused it.

import React, { useCallback, useEffect, useMemo, useState } from "react";

import {
  fetchSettings,
  type AreaDto,
  type CoupledGroupDto,
  type SettingDto,
} from "../services/settings";
import SettingRow from "../components/settings/SettingRow";
import SettingsAreaRail, {
  OVERVIEW_ID,
} from "../components/settings/SettingsAreaRail";
import SettingsGroup from "../components/settings/SettingsGroup";
import {
  groupsInBlock,
  uncoupledIn,
} from "../components/settings/coupledList";
import {
  changedFromDefault,
  searchSettings,
  splitBySpend,
  SEARCH_RESULT_CAP,
} from "../components/settings/settingsSearch";
import {
  confirmationStyle,
  ellipsisStyle,
  errorStyle,
  groupHeadingStyle,
  mainStyle,
  metaStyle,
  noteStyle,
  pageStyle,
  searchStyle,
} from "../components/settings/settingsPageStyles";

/**
 * The group that edits this row, for a row being LISTED rather than grouped.
 *
 * Search cuts across every area, so it finds rows whose editor lives somewhere
 * else entirely. Those are shown with their value and a pointer, never with a
 * field — the backend refuses a single-row save of them, and offering the field
 * anyway would be inviting the one action that cannot work.
 */
const coupledInto = (
  setting: SettingDto,
  groups: readonly CoupledGroupDto[],
): { id: string; label: string } | undefined => {
  const group = groups.find((candidate) => candidate.id === setting.group_id);
  return group ? { id: group.id, label: group.label } : undefined;
};

/** Plural without the "(s)". */
const count = (n: number, one: string, many = `${one}s`) =>
  `${n} ${n === 1 ? one : many}`;

const SettingsPage: React.FC = () => {
  const [settings, setSettings] = useState<SettingDto[] | null>(null);
  const [areas, setAreas] = useState<AreaDto[]>([]);
  const [groups, setGroups] = useState<CoupledGroupDto[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [confirmation, setConfirmation] = useState<string | null>(null);

  const [areaId, setAreaId] = useState<string>(OVERVIEW_ID);
  const [openBlockId, setOpenBlockId] = useState<string | null>(null);
  const [query, setQuery] = useState("");

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const page = await fetchSettings();
      setSettings(page.settings);
      setAreas(page.areas);
      setGroups(page.groups);
      setError(null);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "The settings did not load.");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  /** After a save: show the backend's confirmation and re-read the stored values. */
  const onSaved = useCallback(
    (message: string) => {
      setConfirmation(message);
      // Re-read rather than patching local state: the stored value is the truth,
      // the backend trims and normalises what it was sent, and the
      // changed-from-default phrase is the server's to recompute.
      void load();
    },
    [load],
  );

  const selectArea = useCallback((next: string) => {
    setAreaId(next);
    setOpenBlockId(null);
    setQuery("");
  }, []);

  // The total is the sum of the counts the server took, not the length of the
  // array in hand. They are equal today because the page holds every row; using
  // the server's numbers keeps the sentence true if it ever stops holding them.
  const total = useMemo(
    () => areas.reduce((sum, area) => sum + area.count, 0),
    [areas],
  );

  // One pass, kept across renders: the search and the split of what the cap
  // reached are derived from the same three inputs, and calling `splitBySpend`
  // at each of its three use sites would re-filter 866 rows every keystroke.
  const results = useMemo(() => {
    if (query.trim() === "" || settings === null) return null;
    const found = searchSettings(settings, areas, query);
    return { ...found, ...splitBySpend(found) };
  }, [query, settings, areas]);

  const changed = useMemo(
    () => (settings === null ? [] : changedFromDefault(settings)),
    [settings],
  );

  if (loading) return <div style={pageStyle}>Loading the settings…</div>;

  if (error) {
    return (
      <div style={pageStyle}>
        <div>
          <div role="alert" style={errorStyle}>
            {error}
          </div>
          <button type="button" onClick={() => void load()} style={{ marginTop: "1rem" }}>
            Retry
          </button>
        </div>
      </div>
    );
  }

  if (!settings) return <div style={pageStyle}>The settings are unavailable.</div>;

  const area = areas.find((candidate) => candidate.id === areaId) ?? null;

  return (
    <div style={pageStyle}>
      <SettingsAreaRail
        areas={areas}
        selectedAreaId={areaId}
        onSelect={selectArea}
      />

      <main style={mainStyle}>
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={`Search all ${total} settings — plain words work, and so do key names`}
          aria-label="Search every setting"
          style={searchStyle}
          data-settings-search
        />

        {confirmation && (
          <div role="status" style={confirmationStyle}>
            {confirmation}
          </div>
        )}

        {/* ── STATE 4 · SEARCH ─────────────────────────────────────────── */}
        {results ? (
          <>
            <div style={metaStyle} data-state="search">
              {count(results.total, "match", "matches")} for “{query.trim()}”
              across every area
              {results.shown < results.total &&
                ` · showing the first ${SEARCH_RESULT_CAP}`}
            </div>
            {results.total === 0 && (
              <div style={ellipsisStyle}>
                Nothing matches “{query.trim()}”. The search reads each
                parameter&rsquo;s meaning, its key and its current value.
              </div>
            )}
            {results.rendered.map((group) => (
              <section key={group.areaId}>
                <h2 style={groupHeadingStyle}>
                  {group.areaLabel} · {count(group.matched, "match", "matches")}
                </h2>
                {group.settings.map((setting) => (
                  <SettingRow
                    key={setting.key}
                    setting={setting}
                    query={query}
                    coupledInto={coupledInto(setting, groups)}
                    onSaved={onSaved}
                  />
                ))}
                {group.settings.length < group.matched && (
                  <div style={ellipsisStyle}>
                    … {group.matched - group.settings.length} more in{" "}
                    {group.areaLabel}. Narrow the search to see them.
                  </div>
                )}
              </section>
            ))}
            {results.alsoMatched.length > 0 && (
              <div style={ellipsisStyle}>
                Also matched, beyond the first {SEARCH_RESULT_CAP}:{" "}
                {results.alsoMatched
                  .map((group) => `${group.areaLabel} (${group.matched})`)
                  .join(", ")}
                . Narrow the search, or pick the area on the left.
              </div>
            )}
          </>
        ) : area ? (
          /* ── STATES 2 & 3 · AN AREA, ITS BLOCKS, ONE OF THEM OPEN ─────── */
          <>
            <div style={metaStyle} data-state={openBlockId ? "group" : "area"}>
              {area.label} · {count(area.count, "setting")} in{" "}
              {count(area.blocks.length, "group")} · open one
            </div>
            {area.note && <div style={noteStyle}>{area.note}</div>}
            {area.blocks.map((block) => (
              <SettingsGroup
                key={block.id}
                block={block}
                // Coupled rows are rendered by their group's editor, so they are
                // withheld from the row list here — see `uncoupledIn`.
                settings={uncoupledIn(settings, block.id)}
                groups={groupsInBlock(settings, groups, block.id)}
                open={openBlockId === block.id}
                onToggle={() =>
                  setOpenBlockId(openBlockId === block.id ? null : block.id)
                }
                onSaved={onSaved}
              />
            ))}
          </>
        ) : (
          /* ── STATE 1 · LANDING ────────────────────────────────────────── */
          <>
            <div style={metaStyle} data-state="landing">
              {count(total, "setting")} in {count(areas.length, "area")} · pick an
              area on the left, or just type
            </div>
            <h2 style={groupHeadingStyle}>Changed from default</h2>
            {changed.length === 0 ? (
              <div style={ellipsisStyle}>
                Every parameter is sitting on the value it shipped with.
              </div>
            ) : (
              changed.map((setting) => (
                <SettingRow
                  key={setting.key}
                  setting={setting}
                  coupledInto={coupledInto(setting, groups)}
                  onSaved={onSaved}
                />
              ))
            )}
          </>
        )}
      </main>
    </div>
  );
};

export default SettingsPage;
