// =============================================================================
// RehearsalWatchBlock — what they will wave around, editable here
// =============================================================================
//
// The mockup's `.watchitem` and its Edit control, plus the add row. Same shape,
// same shared editor and same guarded routes as the points block next door — the
// two lists differ in what they are FOR, not in how a human works on them.
//
// ## Why an item is addressed by its id and a point by its number
//
// A talking point has a server-owned `item_index` that is also the number
// printed beside it, so the printed position is a real address. A watch item has
// no such column: its rows are a set, and a position would shift under a
// concurrent add. So the payload carries the row's id — which renders nowhere
// and is addressing, not content (ruling C1).

import React, { useState } from "react";

import AuthoredLineEditor from "./AuthoredLineEditor";
import {
  editButtonStyle,
  editorFieldStyle,
  errorStyle,
} from "./rehearsalStyles";
import {
  addRowStyle,
  watchItemStyle,
} from "./rehearsalRowStyles";
import { absentStyle } from "./scenarioSectionStyles";
import type { RehearsalWatchItem, RehearsalWording } from "../services/rehearsal";

interface Props {
  items: RehearsalWatchItem[];
  /** The stored sentence when nobody has flagged anything. */
  watchGap: string | null;
  wording: RehearsalWording;
  /** Store one edited item, addressed by its row id. */
  onEdit: (id: string, text: string) => Promise<void>;
  /** Store a new item. */
  onAdd: (text: string) => Promise<void>;
}

const textStyle: React.CSSProperties = { fontSize: "15px", maxWidth: "60ch" };

const RehearsalWatchBlock: React.FC<Props> = ({
  items,
  watchGap,
  wording,
  onEdit,
  onAdd,
}) => {
  const [adding, setAdding] = useState(false);
  const [draft, setDraft] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const w = wording.editor;

  const add = () => {
    const trimmed = draft.trim();
    if (trimmed.length === 0) return;

    setSaving(true);
    onAdd(trimmed)
      .then(() => {
        setDraft("");
        setAdding(false);
        setError(null);
      })
      // The draft survives a failure and the failure is named (Standing Rule 1).
      .catch((e: unknown) => {
        setError(e instanceof Error ? e.message : w.save_failed_template);
      })
      .finally(() => setSaving(false));
  };

  return (
    <>
      {items.length === 0 && !adding && watchGap && <p style={absentStyle}>{watchGap}</p>}

      {items.map((item) => (
        <div key={item.id} style={watchItemStyle}>
          <span style={{ color: "var(--text-muted)", marginTop: "2px" }}>●</span>
          <div style={{ flex: 1 }}>
            <AuthoredLineEditor
              text={item.text}
              wording={{
                editLabel: w.edit_label,
                saveLabel: w.save_label,
                cancelLabel: w.cancel_label,
              }}
              onSave={(text) => onEdit(item.id, text)}
              saveFailedNotice={w.save_failed_template}
              fieldLabel={wording.block_watch_heading}
              textStyle={textStyle}
              fieldStyle={editorFieldStyle}
              buttonStyle={editButtonStyle}
              errorStyle={errorStyle}
            />
          </div>
        </div>
      ))}

      {adding ? (
        <div style={{ marginTop: "14px" }}>
          <textarea
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            aria-label={wording.add_watch_label}
            rows={2}
            style={editorFieldStyle}
            spellCheck
          />
          {error && (
            <div role="alert" style={errorStyle}>
              {error}
            </div>
          )}
          <div style={{ display: "flex", gap: "6px", marginTop: "6px" }}>
            <button
              type="button"
              style={editButtonStyle}
              disabled={saving || draft.trim().length === 0}
              onClick={add}
            >
              {w.save_label}
            </button>
            <button
              type="button"
              style={editButtonStyle}
              disabled={saving}
              onClick={() => {
                setAdding(false);
                setDraft("");
                setError(null);
              }}
            >
              {w.cancel_label}
            </button>
          </div>
        </div>
      ) : (
        <div style={addRowStyle}>
          <button type="button" style={editButtonStyle} onClick={() => setAdding(true)}>
            {wording.add_watch_label}
          </button>
        </div>
      )}
    </>
  );
};

export default RehearsalWatchBlock;
