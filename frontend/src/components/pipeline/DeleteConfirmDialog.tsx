/**
 * DeleteConfirmDialog — Simple confirmation dialog for document deletion.
 *
 * Shows what will be deleted and requires a single click to confirm.
 * Cancel button has autoFocus to prevent accidental deletion via Enter.
 */
import React from "react";

interface DeleteConfirmDialogProps {
  documentTitle: string;
  /** Unused — kept for backward compatibility */
  confirmationLevel?: string;
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
};

// ── Component ───────────────────────────────────────────────────

const DeleteConfirmDialog: React.FC<DeleteConfirmDialogProps> = ({
  documentTitle,
  itemCount,
  onConfirm,
  onCancel,
}) => {
  return (
    <div style={S.overlay} onClick={onCancel}>
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
        <div style={S.warning}>This cannot be undone.</div>
        <div style={S.buttons}>
          {/* Cancel has autoFocus — pressing Enter doesn't accidentally delete */}
          <button style={S.cancelBtn} onClick={onCancel} autoFocus>Cancel</button>
          <button style={S.deleteBtn} onClick={() => onConfirm()}>
            Delete Document
          </button>
        </div>
      </div>
    </div>
  );
};

export default DeleteConfirmDialog;
