// =============================================================================
// ScenarioDeleteConfirm — a minimal, self-contained confirm modal (D1.5).
// =============================================================================
//
// A generic "are you sure?" dialog for a destructive action. Deliberately NOT
// reusing the pipeline confirm modals (DeleteConfirmDialog / ActionConfirmDialog)
// — those are coupled to pipeline types (document titles, `AvailableAction`), so
// borrowing them would drag pipeline concepts into the scenario page. Instead this
// takes plain string props and owns nothing domain-specific, so it can be reused
// by the next feature that needs a confirm.
//
// Failure visibility (the ratified requirement): the caller passes an `error`
// string; when set, it renders inside the dialog and the dialog STAYS OPEN. The
// modal closing is NOT proof the action succeeded — the caller only closes it
// after the async action actually resolves. `busy` disables the buttons while the
// action is in flight so it cannot be double-fired.

import React from "react";

interface Props {
  title: string;
  message: string;
  /** Label for the confirm button (e.g. "Delete"). Defaults to "Confirm". */
  confirmLabel?: string;
  /** True while the confirmed action is in flight — disables both buttons. */
  busy?: boolean;
  /** A failure from the last confirm attempt. Non-null keeps the dialog open and
   *  renders the message; the user can retry or cancel. */
  error?: string | null;
  onConfirm: () => void;
  onCancel: () => void;
}

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
  } as React.CSSProperties,
  dialog: {
    backgroundColor: "var(--bg-surface)",
    borderRadius: "8px",
    padding: "1.5rem",
    maxWidth: "480px",
    width: "90%",
    boxShadow: "0 4px 24px rgba(0,0,0,0.2)",
  } as React.CSSProperties,
  title: {
    fontSize: "1.1rem",
    fontWeight: 700,
    color: "var(--text-primary)",
    marginBottom: "0.75rem",
  } as React.CSSProperties,
  message: {
    fontSize: "0.88rem",
    color: "var(--text-secondary)",
    lineHeight: 1.5,
    marginBottom: "1rem",
  } as React.CSSProperties,
  errorBox: {
    margin: "0 0 1rem",
    padding: "0.6rem 0.8rem",
    backgroundColor: "var(--state-danger-bg-soft)",
    border: "1px solid var(--state-danger-border)",
    borderRadius: "6px",
    color: "var(--state-danger-strong)",
    fontSize: "0.82rem",
  } as React.CSSProperties,
  buttons: {
    display: "flex",
    justifyContent: "flex-end",
    gap: "0.5rem",
  } as React.CSSProperties,
  cancelBtn: {
    padding: "0.4rem 1rem",
    fontSize: "0.84rem",
    fontWeight: 500,
    border: "1px solid var(--border-default)",
    borderRadius: "4px",
    backgroundColor: "var(--bg-surface)",
    color: "var(--text-secondary)",
    cursor: "pointer",
    fontFamily: "inherit",
  } as React.CSSProperties,
  confirmBtn: {
    padding: "0.4rem 1rem",
    fontSize: "0.84rem",
    fontWeight: 600,
    border: "none",
    borderRadius: "4px",
    backgroundColor: "var(--state-danger-strong)",
    color: "var(--bg-surface)",
    cursor: "pointer",
    fontFamily: "inherit",
  } as React.CSSProperties,
  disabledBtn: {
    opacity: 0.5,
    cursor: "not-allowed",
  } as React.CSSProperties,
};

/**
 * A blocking confirm dialog. The overlay click and Cancel both call `onCancel`
 * (disabled while `busy`). Confirm calls `onConfirm`; the parent runs the async
 * action and decides — based on its RESULT, not this click — whether to close.
 */
const ScenarioDeleteConfirm: React.FC<Props> = ({
  title,
  message,
  confirmLabel = "Confirm",
  busy = false,
  error = null,
  onConfirm,
  onCancel,
}) => (
  <div style={S.overlay} onClick={busy ? undefined : onCancel}>
    <div style={S.dialog} onClick={(e) => e.stopPropagation()}>
      <div style={S.title}>{title}</div>
      <div style={S.message}>{message}</div>
      {error && <div style={S.errorBox}>{error}</div>}
      <div style={S.buttons}>
        <button
          type="button"
          style={busy ? { ...S.cancelBtn, ...S.disabledBtn } : S.cancelBtn}
          onClick={onCancel}
          disabled={busy}
        >
          Cancel
        </button>
        <button
          type="button"
          style={busy ? { ...S.confirmBtn, ...S.disabledBtn } : S.confirmBtn}
          onClick={onConfirm}
          disabled={busy}
        >
          {busy ? "Deleting…" : confirmLabel}
        </button>
      </div>
    </div>
  </div>
);

export default ScenarioDeleteConfirm;
