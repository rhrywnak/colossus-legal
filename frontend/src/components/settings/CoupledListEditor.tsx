// =============================================================================
// CoupledListEditor — several stored rows, edited as one table
// =============================================================================
//
// The control the reviewer bench needed and did not have. Rows of cells, one
// column per stored row, add and remove together, ONE Save that writes them all
// in a single transaction.
//
// ## Why it names nothing
//
// It does not know what a reviewer is. The heading, the note, the column labels,
// the placeholders and the word for one entry all arrive from the server, which
// declares the coupling beside the key lists it already owns. That is what lets
// the Settings page stay liftable into another Colossus application —
// `settingsImportRule.test.ts` fails the build if anything here starts naming
// this case — and it means the next coupled pair needs no new component.
//
// ## The shape is the fix
//
// Add a row and every column gains a cell; remove one and every column loses
// one. "The logins have two and the names have one" is not a state this control
// can be in, which is why the length check it used to deadlock on is never
// consulted here.

import React, { useEffect, useState } from "react";

import type { CoupledGroupDto } from "../../services/settings";
import { setSettingGroup } from "../../services/settings";
import {
  addEntry,
  editCell,
  isDirty,
  removeEntry,
  whyNotSubmittable,
  type Entry,
} from "./coupledList";
import {
  editRowStyle,
  errorStyle,
  footStyle,
  noteStyle,
  rowKeyStyle,
  rowLabelStyle,
  rowStyle,
  saveButtonStyle,
  valueFieldStyle,
} from "./settingsPageStyles";

const cellsRowStyle: React.CSSProperties = {
  display: "flex",
  gap: "0.5rem",
  alignItems: "center",
  flexWrap: "wrap",
  marginBottom: "0.4rem",
};

const removeStyle: React.CSSProperties = {
  fontSize: "0.72rem",
  background: "transparent",
  border: "1px solid var(--border-default)",
  borderRadius: "0.375rem",
  padding: "0.3rem 0.6rem",
  color: "var(--text-secondary)",
  fontFamily: "inherit",
  cursor: "pointer",
};

const headingCellStyle: React.CSSProperties = {
  fontSize: "0.72rem",
  fontWeight: 700,
  color: "var(--text-muted)",
};

const addStyle: React.CSSProperties = {
  ...removeStyle,
  color: "var(--accent-primary)",
  borderColor: "var(--accent-primary)",
  marginTop: "0.2rem",
};

/**
 * The column headings, once, above the rows.
 *
 * Laid out with the SAME flex rule as a cell — `valueFieldStyle` widths, the
 * same row gap — rather than as free text. Headings that merely sat near their
 * columns drifted out of line the moment a column was wide, which is how a
 * two-column table stops reading as one.
 */
const ColumnHeadings: React.FC<{ group: CoupledGroupDto }> = ({ group }) => (
  <div style={{ ...cellsRowStyle, marginBottom: "0.25rem" }}>
    {group.columns.map((column) => (
      <span
        key={column.key}
        style={{
          ...headingCellStyle,
          // Matches the cell beneath it exactly: same grow, same bounds.
          flex: 1,
          minWidth: "12rem",
          maxWidth: "25rem",
        }}
      >
        {column.label}
      </span>
    ))}
    {/* Holds the width of the Remove button so the last heading does not
        stretch under it. `aria-hidden` because it says nothing. */}
    <span aria-hidden style={{ ...removeStyle, visibility: "hidden" }}>
      Remove
    </span>
  </div>
);

/**
 * One entry: a cell per column, and the control that removes the whole row.
 *
 * Remove takes the ENTRY, never a single cell — which is the shape argument in
 * one component. There is no control here that can shorten one column.
 */
const EntryRow: React.FC<{
  group: CoupledGroupDto;
  entry: string[];
  at: number;
  disabled: boolean;
  onCell: (column: number, value: string) => void;
  onRemove: () => void;
}> = ({ group, entry, at, disabled, onCell, onRemove }) => (
  <div style={cellsRowStyle} data-entry={at}>
    {group.columns.map((column, c) => (
      <input
        key={column.key}
        value={entry[c] ?? ""}
        onChange={(e) => onCell(c, e.target.value)}
        placeholder={column.placeholder}
        aria-label={`${column.label}, ${group.entry_noun} ${at + 1}`}
        style={{ ...valueFieldStyle(false), minWidth: "12rem" }}
        disabled={disabled}
      />
    ))}
    <button
      type="button"
      onClick={onRemove}
      disabled={disabled}
      style={removeStyle}
      aria-label={`Remove ${group.entry_noun} ${at + 1}`}
    >
      Remove
    </button>
  </div>
);

/**
 * Save, and — when it is unavailable — why.
 *
 * Saying why costs one line and replaces a dead control. The backend refuses
 * the same things with better sentences; this only keeps the operator from
 * pressing a button that cannot work.
 */
const SaveBar: React.FC<{
  dirty: boolean;
  saving: boolean;
  blocked: string | null;
  onSave: () => void;
}> = ({ dirty, saving, blocked, onSave }) => {
  if (!dirty) return null;
  return (
    <div style={editRowStyle}>
      <button
        type="button"
        onClick={onSave}
        disabled={saving || blocked !== null}
        style={saveButtonStyle}
        data-save-group
      >
        {saving ? "Saving…" : "Save"}
      </button>
      {blocked && <span style={footStyle}>{blocked}</span>}
    </div>
  );
};

/**
 * The editor's state: the table being edited, and the one write it can make.
 *
 * ## Rust Learning has a React counterpart — a custom hook
 *
 * A function whose name begins with `use` may call other hooks, and React
 * treats it as part of the component that calls it. Pulling the state out this
 * way is the same move as extracting a helper in Rust: the component below
 * becomes only the drawing, and everything about WHEN the table changes and
 * what happens on save is testable reasoning in one place.
 */
function useCoupledEntries(group: CoupledGroupDto, onSaved: (message: string) => void) {
  const [entries, setEntries] = useState<Entry[]>(group.entries);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Follow the stored entries when the page re-reads after any save, for the
  // reason `SettingRow` re-syncs its draft: otherwise a second edit would
  // submit a table built from values that are no longer there.
  useEffect(() => {
    setEntries(group.entries);
  }, [group.entries]);

  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      const changed = await setSettingGroup(
        group.id,
        entries.map((entry) => entry.map((cell) => cell.trim())),
      );
      onSaved(changed.message);
    } catch (e: unknown) {
      // The backend's sentence names the group, the column and the row. Shown
      // as it arrived — it is more specific than anything this side could say.
      setError(e instanceof Error ? e.message : "Those changes did not save.");
    } finally {
      setSaving(false);
    }
  };

  return { entries, setEntries, saving, error, save };
}

/**
 * The stored keys behind the control, and any refusal it came back with.
 *
 * The keys are shown for the reason every row on this page shows its key: it is
 * what a log line or a psql query names, and an operator reading a refusal about
 * `practice_reviewer_usernames` needs to see that name on the control that
 * writes it.
 */
const EditorFooter: React.FC<{ group: CoupledGroupDto; error: string | null }> = ({
  group,
  error,
}) => (
  <>
    <div style={rowKeyStyle}>{group.columns.map((column) => column.key).join(" · ")}</div>
    {error && (
      <div role="alert" style={errorStyle}>
        {error}
      </div>
    )}
  </>
);

export const CoupledListEditor: React.FC<{
  group: CoupledGroupDto;
  onSaved: (message: string) => void;
}> = ({ group, onSaved }) => {
  const { entries, setEntries, saving, error, save } = useCoupledEntries(group, onSaved);

  return (
    <div style={rowStyle} data-coupled-group={group.id}>
      <div style={rowLabelStyle}>{group.label}</div>
      <div style={{ ...noteStyle, margin: "0.5rem 0 0.75rem" }}>{group.note}</div>

      <ColumnHeadings group={group} />

      {entries.map((entry, at) => (
        // The index IS the identity here: entries have no id, and two reviewers
        // may share neither login nor name, so reordering is not offered.
        <EntryRow
          key={at}
          group={group}
          entry={entry}
          at={at}
          disabled={saving}
          onCell={(column, value) => setEntries(editCell(entries, at, column, value))}
          onRemove={() => setEntries(removeEntry(entries, at))}
        />
      ))}

      <button
        type="button"
        onClick={() => setEntries(addEntry(group, entries))}
        disabled={saving}
        style={addStyle}
        data-add-entry
      >
        + Add a {group.entry_noun}
      </button>

      <SaveBar
        dirty={isDirty(group.entries, entries)}
        saving={saving}
        blocked={whyNotSubmittable(group, entries)}
        onSave={() => void save()}
      />

      <EditorFooter group={group} error={error} />
    </div>
  );
};

export default CoupledListEditor;
