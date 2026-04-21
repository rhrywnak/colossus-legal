/**
 * DeleteConfirmDialog — Confirmation dialog for document deletion.
 *
 * For "strict" confirmation (PUBLISHED/INGESTED/INDEXED documents),
 * shows a reason text input — the backend requires a reason for audit.
 * For other levels, a single click confirms.
 *
 * The dialog stays open during the delete operation and displays any
 * errors inline — no silent failures.
 */
import React, { useState } from "react";

interface DeleteConfirmDialogProps {
  documentTitle: string;
  /** "simple" | "moderate" | "strict" — from backend state machine */
  confirmationLevel?: string;
  itemCount: number;
  onConfirm: (reason?: string) => Promise<void>;
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
    fontSize: "0.88rem", color: "#334155", lineHeight: 1.5, marginBottom: "0.5rem",
  } as React.CSSProperties,
  list: {
    fontSize: "0.84rem", color: "#475569", lineHeight: 1.6,
    marginBottom: "0.75rem", paddingLeft: "1.25rem",
  } as React.CSSProperties,
  warning: {
    fontWeight: 600, color: "#dc2626", fontSize: "0.88rem",
    marginBottom: "1rem",
  } as React.CSSProperties,
  reasonLabel: {
    fontSize: "0.84rem", fontWeight: 600, color: "#334155",
    marginBottom: "0.25rem", display: "block",
  } as React.CSSProperties,
  reasonInput: {
    width: "100%", padding: "0.5rem 0.6rem", fontSize: "0.84rem",
    border: "1px solid #cbd5e1", borderRadius: "4px", fontFamily: "inherit",
    color: "#334155", marginBottom: "0.75rem", boxSizing: "border-box",
  } as React.CSSProperties,
  reasonHint: {
    fontSize: "0.76rem", color: "#94a3b8", marginBottom: "0.75rem",
  } as React.CSSProperties,
  errorBox: {
    padding: "0.5rem 0.75rem", backgroundColor: "#fef2f2",
    border: "1px solid #fecaca", borderRadius: "4px", color: "#991b1b",
    fontSize: "0.8rem", marginBottom: "0.75rem",
  } as React.CSSProperties,
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
  confirmationLevel = "simple",
  itemCount,
  onConfirm,
  onCancel,
}) => {
  const [reason, setReason] = useState("");
  const [deleting, setDeleting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const isStrict = confirmationLevel === "strict";
  const canDelete = isStrict ? reason.trim().length > 0 : true;

  const handleDelete = async () => {
    if (!canDelete || deleting) return;
    setDeleting(true);
    setError(null);
    try {
      await onConfirm(isStrict ? reason.trim() : undefined);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Delete failed");
      setDeleting(false);
    }
  };

  return (
    <div style={S.overlay} onClick={deleting ? undefined : onCancel}>
      <div style={S.dialog} onClick={(e) => e.stopPropagation()}>
        <div style={S.title}>Delete Document</div>
        <div style={S.message}>
          This will permanently delete <strong>{documentTitle}</strong>:
        </div>
        <ul style={S.list}>
          <li>The document and extracted text</li>
          {itemCount > 0 && <li>All extraction results ({itemCount} entities)</li>}
          <li>Graph data in Neo4j</li>
          <li>Search index data</li>
        </ul>

        {isStrict && (
          <>
            <label style={S.reasonLabel}>Reason for deletion</label>
            <input
              style={S.reasonInput}
              type="text"
              placeholder="e.g. Re-uploading with corrected scan"
              value={reason}
              onChange={(e) => setReason(e.target.value)}
              disabled={deleting}
              autoFocus
            />
            <div style={S.reasonHint}>
              Required for documents that have been processed and published.
            </div>
          </>
        )}

        {error && <div style={S.errorBox}>{error}</div>}

        <div style={S.warning}>This cannot be undone.</div>
        <div style={S.buttons}>
          <button
            style={S.cancelBtn}
            onClick={onCancel}
            disabled={deleting}
            autoFocus={!isStrict}
          >
            Cancel
          </button>
          <button
            style={canDelete && !deleting ? S.deleteBtn : S.deleteBtnDisabled}
            onClick={handleDelete}
            disabled={!canDelete || deleting}
          >
            {deleting ? "Deleting..." : "Delete Document"}
          </button>
        </div>
      </div>
    </div>
  );
};

export default DeleteConfirmDialog;
