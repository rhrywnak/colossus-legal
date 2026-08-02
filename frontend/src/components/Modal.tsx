// =============================================================================
// Modal — the overlay shell (task 1.7B)
// =============================================================================
//
// One modal, everything on one screen — the Casefleet Edit-Fact pattern (study
// §1.6). Nothing on the scenario page opens a second page.
//
// ## Where this came from
//
// Lifted from `ScenarioDeleteConfirm`'s overlay chrome, which had carried it
// alone since D1.5: fixed backdrop, click-outside to dismiss, Escape, a busy
// state that disables dismissal while an action is in flight. That is machinery
// this codebase already trusts; the identity modal needed the same, and two
// copies of a dialog is how one of them ends up without the Escape handler.
//
// ## Failure visibility (the ratified requirement, inherited)
//
// A modal closing is NOT proof the action succeeded. The caller keeps it open
// until its async action resolves, and passes `busy` while in flight so the
// dismiss paths are disabled and the action cannot be double-fired.

import React, { useEffect } from "react";

const S = {
  overlay: {
    position: "fixed",
    top: 0,
    left: 0,
    right: 0,
    bottom: 0,
    backgroundColor: "rgba(0, 0, 0, 0.5)",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    zIndex: 9999,
    padding: "2rem 1rem",
  } as React.CSSProperties,
  dialog: {
    backgroundColor: "var(--bg-surface)",
    border: "1px solid var(--border-default)",
    borderRadius: "8px",
    padding: "1.5rem",
    width: "100%",
    boxShadow: "0 4px 24px rgba(0,0,0,0.2)",
    // The modal scrolls INSIDE itself rather than growing past the viewport:
    // the identity form is long, and a dialog whose Save button is below the
    // fold is a dialog people abandon.
    maxHeight: "100%",
    overflowY: "auto",
    fontWeight: 400,
  } as React.CSSProperties,
  title: {
    fontSize: "1.05rem",
    fontWeight: 600,
    color: "var(--text-primary)",
    marginBottom: "1rem",
  } as React.CSSProperties,
};

interface Props {
  title: string;
  /** Caps the dialog width. Confirms are narrow; an editing form is wider. */
  maxWidth?: string;
  /** True while the caller's action is in flight — disables every dismiss path. */
  busy?: boolean;
  onClose: () => void;
  children: React.ReactNode;
}

/**
 * A blocking dialog. Escape and a backdrop click both call `onClose`, and both
 * are inert while `busy` — a half-saved form must not vanish under the fingers
 * of someone reaching for Escape.
 */
const Modal: React.FC<Props> = ({
  title,
  maxWidth = "560px",
  busy = false,
  onClose,
  children,
}) => {
  // ## React Learning: why the key handler lives in an effect
  //
  // Escape has to work wherever focus is — a field, a button, the backdrop — so
  // it is bound to the document rather than to a node. The cleanup return is not
  // optional bookkeeping: without it every open/close cycle would leave another
  // listener behind, and a later Escape would call `onClose` on modals that no
  // longer exist.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !busy) onClose();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [busy, onClose]);

  return (
    <div style={S.overlay} onClick={busy ? undefined : onClose}>
      <div
        role="dialog"
        aria-modal="true"
        aria-label={title}
        style={{ ...S.dialog, maxWidth }}
        onClick={(e) => e.stopPropagation()}
      >
        <div style={S.title}>{title}</div>
        {children}
      </div>
    </div>
  );
};

export default Modal;
