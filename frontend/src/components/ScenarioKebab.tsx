// =============================================================================
// ScenarioKebab — the ⋯ menu, the ONLY home for destructive actions (D7)
// =============================================================================
//
// Defect D7: `Delete` sat as a bare button in the middle of the 1.7B header row,
// one mis-click from `Mark ready to rehearse`. Design §2.1 puts every destructive
// action behind this menu and nowhere else.
//
// ## Delete, and ONLY Delete (ruling R1)
//
// §2.1 as drafted said the kebab holds "Archive and Delete". Phase A measured that
// Archive does not exist — no endpoint, and the `scenarios_status_check` constraint
// allows only `draft | needs_evidence | ready`, so there is no state to archive
// INTO. Roman ruled option (a): no Archive item, and no disabled placeholder
// either. A control that cannot act is a promise the page cannot keep, and the
// absent-not-fake law applies to menu items exactly as it does to panels. Archive
// joins this menu with task 3.6.
//
// ## Why a real popover and not a `<select>`
//
// A menu of destructive actions must not be operable by arrow keys alone — a
// keyboard user tabbing through a header should not be one keystroke from Delete.
// A button that toggles a panel makes the second, deliberate click the commitment,
// which is the same shape the confirm modal downstream relies on.

import React, { useEffect, useRef, useState } from "react";

const HAIRLINE = "1px solid var(--border-default)";

const triggerStyle: React.CSSProperties = {
  border: "none",
  background: "none",
  fontSize: "1.1rem",
  lineHeight: 1,
  color: "var(--text-muted)",
  cursor: "pointer",
  padding: "0.25rem 0.5rem",
  fontFamily: "inherit",
};

const menuStyle: React.CSSProperties = {
  position: "absolute",
  right: 0,
  top: "calc(100% + 4px)",
  minWidth: "11rem",
  background: "var(--bg-surface)",
  border: HAIRLINE,
  borderRadius: "8px",
  boxShadow: "0 4px 12px rgba(16,24,40,.10)",
  padding: "0.25rem",
  zIndex: 20,
};

const itemStyle: React.CSSProperties = {
  display: "block",
  width: "100%",
  textAlign: "left",
  border: "none",
  background: "none",
  fontFamily: "inherit",
  fontSize: "0.85rem",
  padding: "0.45rem 0.6rem",
  borderRadius: "6px",
  cursor: "pointer",
  color: "var(--state-danger-strong)",
};

interface Props {
  /** Opens the delete confirmation. The kebab never deletes anything itself. */
  onDelete: () => void;
}

const ScenarioKebab: React.FC<Props> = ({ onDelete }) => {
  const [open, setOpen] = useState(false);
  const wrapRef = useRef<HTMLDivElement | null>(null);

  // Close on an outside click or Escape — the house pattern (`AuthorityPopover`).
  // One document listener while open, removed when it closes.
  useEffect(() => {
    if (!open) return;
    function onDown(event: MouseEvent) {
      if (!wrapRef.current?.contains(event.target as Node)) setOpen(false);
    }
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape") setOpen(false);
    }
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div ref={wrapRef} style={{ position: "relative", display: "inline-block" }}>
      <button
        type="button"
        style={triggerStyle}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label="More actions"
        title="More actions"
        onClick={() => setOpen((v) => !v)}
      >
        ⋯
      </button>

      {open && (
        <div role="menu" style={menuStyle}>
          <button
            type="button"
            role="menuitem"
            style={itemStyle}
            onClick={() => {
              setOpen(false);
              onDelete();
            }}
          >
            Delete scenario…
          </button>
          {/* Archive's slot is RESERVED and renders nothing until task 3.6 gives
              it an endpoint and a status to move to (ruling R1). Deliberately not
              a disabled item: a greyed "Archive" tells the human the feature is
              here and broken, when the truth is that it has not been built. */}
        </div>
      )}
    </div>
  );
};

export default ScenarioKebab;
