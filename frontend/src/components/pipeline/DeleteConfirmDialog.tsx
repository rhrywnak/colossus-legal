/**
 * DeleteConfirmDialog — Confirmation dialog for document deletion.
 *
 * Three confirmation levels (determined by the backend, not the frontend):
 * - "simple": basic confirmation, optional reason
 * - "moderate": shows entity count, optional reason
 * - "strict": requires reason + type title to confirm
 */
import React, { useState } from "react";

interface DeleteConfirmDialogProps {
  documentTitle: string;
  /** Confirmation level from the backend state machine */
  confirmationLevel: string;
  itemCount: number;
  onConfirm: (reason?: string) => void;
  onCancel: () => void;
}

// ── Styles ──────────────────────────────────────────────────────

const S = {
  overlay: {
    position: "fixed", top: 0, left: 0, right: 0, bottom: 0,
    backgroundColor: "rgba(0, 0, 0, 0.5)", display: "flex",
    alignItems: "center", justifyContent: "center", zIndex: 9999,
  } as React.CSSProperties,
  dialog: {
    backgroundColor: "#fff", borderRadius: "8px", padding: "1.5rem",
    maxWidth: "480px", width: "90%", boxShadow: "0 4px 24px rgba(0,0,0,0.2)",
  } as React.CSSProperties,
  title: {
    fontSize: "1.1rem", fontWeight: 700, color: "#0f172a", marginBottom: "0.75rem",
  } as React.CSSProperties,
  message: {
    fontSize: "0.88rem", color: "#334155", lineHeight: 1.5, marginBottom: "1rem",
  } as React.CSSProperties,
  label: {
    display: "block", fontSize: "0.8rem", fontWeight: 600, color: "#475569",
    marginBottom: "0.3rem",
  } as React.CSSProperties,
  input: {
    width: "100%", padding: "0.45rem 0.6rem", fontSize: "0.84rem",
    border: "1px solid #cbd5e1", borderRadius: "4px", fontFamily: "inherit",
    boxSizing: "border-box",
  } as React.CSSProperties,
  fieldGroup: { marginBottom: "1rem" } as React.CSSProperties,
  buttons: {
    display: "flex", justifyContent: "flex-end", gap: "0.5rem", marginTop: "1.25rem",
  } as React.CSSProperties,
  cancelBtn: {
    padding: "0.4rem 1rem", fontSize: "0.84rem", fontWeight: 500,
    border: "1px solid #cbd5e1", borderRadius: "4px", backgroundColor: "#fff",
    color: "#334155", cursor: "pointer", fontFamily: "inherit",
  } as React.CSSProperties,
  deleteBtn: {
    padding: "0.4rem 1rem", fontSize: "0.84rem", fontWeight: 600,
    border: "none", borderRadius: "4px", backgroundColor: "#dc2626",
    color: "#fff", cursor: "pointer", fontFamily: "inherit",
  } as React.CSSProperties,
  deleteBtnDisabled: {
    padding: "0.4rem 1rem", fontSize: "0.84rem", fontWeight: 600,
    border: "none", borderRadius: "4px", backgroundColor: "#94a3b8",
    color: "#fff", cursor: "not-allowed", fontFamily: "inherit",
  } as React.CSSProperties,
};

// ── Component ───────────────────────────────────────────────────

const DeleteConfirmDialog: React.FC<DeleteConfirmDialogProps> = ({
  documentTitle,
  confirmationLevel,
  itemCount,
  onConfirm,
  onCancel,
}) => {
  const [reason, setReason] = useState("");
  const [titleConfirm, setTitleConfirm] = useState("");

  const isStrict = confirmationLevel === "strict";
  const isModerate = confirmationLevel === "moderate";

  // Determine message based on confirmation level
  let message: string;
  if (isStrict) {
    message = `Delete "${documentTitle}"? This will remove ${itemCount} entities from the knowledge graph, search vectors, and all processing data. This affects chat answers.`;
  } else if (isModerate) {
    message = `Delete "${documentTitle}"? This will remove the uploaded file, ${itemCount} extracted entities, and all processing data.`;
  } else {
    message = `Delete "${documentTitle}"? The uploaded file and any extracted text will be removed.`;
  }

  // For strict: reason required AND title must match
  const reasonFilled = reason.trim().length > 0;
  const titleMatches = titleConfirm === documentTitle;
  const canDelete = isStrict ? (reasonFilled && titleMatches) : true;
  const buttonLabel = isStrict ? "Delete Permanently" : "Delete";

  return (
    <div style={S.overlay} onClick={onCancel}>
      <div style={S.dialog} onClick={(e) => e.stopPropagation()}>
        <div style={S.title}>Confirm Deletion</div>
        <div style={S.message}>{message}</div>

        {/* Reason field */}
        <div style={S.fieldGroup}>
          <label style={S.label}>
            Reason{isStrict ? " (required)" : " (optional)"}
          </label>
          <input
            style={S.input}
            type="text"
            placeholder="Why is this document being deleted?"
            value={reason}
            onChange={(e) => setReason(e.target.value)}
          />
        </div>

        {/* Title confirmation for strict deletions */}
        {isStrict && (
          <div style={S.fieldGroup}>
            <label style={S.label}>
              Type the document title to confirm: <strong>{documentTitle}</strong>
            </label>
            <input
              style={S.input}
              type="text"
              placeholder={documentTitle}
              value={titleConfirm}
              onChange={(e) => setTitleConfirm(e.target.value)}
            />
          </div>
        )}

        <div style={S.buttons}>
          <button style={S.cancelBtn} onClick={onCancel}>Cancel</button>
          <button
            style={canDelete ? S.deleteBtn : S.deleteBtnDisabled}
            disabled={!canDelete}
            onClick={() => onConfirm(reason.trim() || undefined)}
          >
            {buttonLabel}
          </button>
        </div>
      </div>
    </div>
  );
};

export default DeleteConfirmDialog;
