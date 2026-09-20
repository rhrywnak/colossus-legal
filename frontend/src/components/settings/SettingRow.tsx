// =============================================================================
// SettingRow — one parameter: what it does, what it is, and how to change it
// =============================================================================
//
// Everything the row says about the parameter arrived composed: the meaning, the
// input hint, the bounds sentence, the dormancy note, the last-changed line and
// the changed-from-default phrase. This component lays them out and sends the
// value back. It does not validate, and it does not word anything.
//
// ## Rust Learning is not the only kind — the React one here is controlled state
//
// `draft` is this row's local copy of the field while it is being typed, and the
// `useEffect` below re-syncs it whenever the STORED value changes underneath.
// Without that sync, saving one row (which reloads every row) would leave every
// other row's draft holding text from before the reload — and the next save
// would submit it.

import React, { useEffect, useState } from "react";

import type { SettingDto } from "../../services/settings";
import { setSetting } from "../../services/settings";
import { highlight } from "./settingsSearch";
import {
  changedStyle,
  editRowStyle,
  errorStyle,
  footStyle,
  markStyle,
  rowKeyStyle,
  rowLabelStyle,
  rowStyle,
  saveButtonStyle,
  valueFieldStyle,
} from "./settingsPageStyles";

/** Text with the searched-for run marked — as elements, never as HTML. */
const Marked: React.FC<{ text: string; query: string }> = ({ text, query }) => (
  <>
    {highlight(text, query).map((segment, at) =>
      segment.hit ? (
        <mark key={at} style={markStyle}>
          {segment.text}
        </mark>
      ) : (
        <React.Fragment key={at}>{segment.text}</React.Fragment>
      ),
    )}
  </>
);

export const SettingRow: React.FC<{
  setting: SettingDto;
  /** The live query, so the row can mark what matched. "" when not searching. */
  query?: string;
  /**
   * The group that edits this row, when one does.
   *
   * Set only where a coupled row is still LISTED — search results reach across
   * every area, so a search for "reviewer" finds rows whose editor is somewhere
   * else. The row then shows its value and says where to change it, rather than
   * offering a field whose save the backend refuses.
   */
  coupledInto?: { id: string; label: string };
  onSaved: (message: string) => void;
}> = ({ setting, query = "", coupledInto, onSaved }) => {
  const [draft, setDraft] = useState(setting.value);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setDraft(setting.value);
  }, [setting.value]);

  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      const changed = await setSetting(setting.key, draft);
      onSaved(changed.message);
    } catch (e: unknown) {
      // Explicit error UI, never a swallowed rejection. The backend's sentence
      // names the parameter and the bound, so it is shown as it arrived.
      setError(e instanceof Error ? e.message : "That change did not save.");
    } finally {
      setSaving(false);
    }
  };

  const dirty = draft.trim() !== setting.value;

  return (
    <div style={rowStyle} data-setting-key={setting.key} data-block={setting.block_id}>
      <div style={rowLabelStyle}>
        <Marked text={setting.meaning} query={query} />
      </div>
      <div style={rowKeyStyle}>
        <Marked text={setting.key} query={query} />
      </div>

      {coupledInto ? (
        <div style={editRowStyle}>
          <input
            value={setting.value}
            readOnly
            aria-label={`${setting.key} (edited in ${coupledInto.label})`}
            style={{ ...valueFieldStyle(false), opacity: 0.75 }}
            data-coupled-readonly
          />
          <span style={footStyle}>
            Edited together in “{coupledInto.label}” — open it under this
            setting&rsquo;s own group to change it.
          </span>
        </div>
      ) : (
      <div style={editRowStyle}>
        <input
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          aria-label={setting.key}
          style={valueFieldStyle(setting.bounds_label !== null)}
          disabled={saving}
        />
        {dirty && (
          <button
            type="button"
            onClick={() => void save()}
            disabled={saving}
            style={saveButtonStyle}
          >
            {saving ? "Saving…" : "Save"}
          </button>
        )}
      </div>
      )}

      <div style={footStyle}>
        {setting.changed_from_default ? (
          <span style={changedStyle} data-changed="yes">
            {setting.changed_from_default}
          </span>
        ) : (
          <span data-changed="no">Default: {setting.default_value}</span>
        )}
        <span>{setting.input_hint}</span>
        {setting.bounds_label && <span>{setting.bounds_label}</span>}
        <span>{setting.last_changed}</span>
      </div>

      {setting.dormant_note && (
        <div style={{ ...footStyle, fontStyle: "italic" }}>{setting.dormant_note}</div>
      )}

      {error && (
        <div role="alert" style={errorStyle}>
          {error}
        </div>
      )}
    </div>
  );
};

export default SettingRow;
