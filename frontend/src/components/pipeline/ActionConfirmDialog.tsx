/**
 * ActionConfirmDialog — Generic confirmation modal for any pipeline
 * action whose `requires_confirmation` flag is true (reprocess, ingest,
 * etc.). Shown before the action fires; destructive styling (amber).
 */
import React from "react";
import { AvailableAction } from "../../services/pipelineApi";

interface ActionConfirmDialogProps {
  action: AvailableAction;
  onConfirm: () => void;
  onCancel: () => void;
}

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
    fontSize: "1rem", fontWeight: 700, color: "#0f172a", marginBottom: "0.5rem",
  } as React.CSSProperties,
  message: {
    fontSize: "0.84rem", color: "#334155", marginBottom: "1rem", lineHeight: 1.5,
  } as React.CSSProperties,
  buttons: {
    display: "flex", justifyContent: "flex-end", gap: "0.5rem",
  } as React.CSSProperties,
  cancelBtn: {
    padding: "0.4rem 1rem", fontSize: "0.84rem", fontWeight: 500,
    border: "1px solid #cbd5e1", borderRadius: "4px", backgroundColor: "#fff",
    color: "#334155", cursor: "pointer", fontFamily: "inherit",
  } as React.CSSProperties,
  confirmBtn: {
    padding: "0.4rem 1rem", fontSize: "0.84rem", fontWeight: 600,
    border: "none", borderRadius: "4px", backgroundColor: "#d97706",
    color: "#fff", cursor: "pointer", fontFamily: "inherit",
  } as React.CSSProperties,
};

const ActionConfirmDialog: React.FC<ActionConfirmDialogProps> = ({
  action, onConfirm, onCancel,
}) => (
  <div style={S.overlay} onClick={onCancel}>
    <div style={S.dialog} onClick={(e) => e.stopPropagation()}>
      <div style={S.title}>{action.label}</div>
      <div style={S.message}>{action.description}</div>
      <div style={S.buttons}>
        <button style={S.cancelBtn} onClick={onCancel}>Cancel</button>
        <button style={S.confirmBtn} onClick={onConfirm}>{action.label}</button>
      </div>
    </div>
  </div>
);

export default ActionConfirmDialog;
