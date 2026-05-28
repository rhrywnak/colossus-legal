import React from "react";

export const cardStyle: React.CSSProperties = {
  backgroundColor: "var(--bg-surface)", border: "1px solid var(--border-default)", borderRadius: "10px",
  padding: "1.25rem 1.5rem",
};

export const btnPrimary: React.CSSProperties = {
  backgroundColor: "var(--accent-primary)", color: "var(--bg-surface)", border: "none", borderRadius: "6px",
  padding: "0.45rem 1rem", fontSize: "0.82rem", fontWeight: 600, cursor: "pointer",
  fontFamily: "inherit",
};

export const btnSecondary: React.CSSProperties = {
  backgroundColor: "var(--bg-page)", color: "var(--text-secondary)", border: "1px solid var(--border-default)",
  borderRadius: "6px", padding: "0.45rem 1rem", fontSize: "0.82rem", fontWeight: 500,
  cursor: "pointer", fontFamily: "inherit",
};

export const inputStyle: React.CSSProperties = {
  width: "100%", padding: "0.45rem 0.65rem", border: "1px solid var(--border-default)",
  borderRadius: "6px", fontSize: "0.84rem", fontFamily: "inherit",
  boxSizing: "border-box",
};

export const labelStyle: React.CSSProperties = {
  display: "block", fontSize: "0.76rem", fontWeight: 600, color: "var(--text-secondary)",
  marginBottom: "0.25rem",
};

export const msgSuccess: React.CSSProperties = {
  padding: "0.65rem 1rem", backgroundColor: "var(--state-success-bg-soft)", border: "1px solid var(--state-success-bg-soft)",
  borderRadius: "6px", fontSize: "0.84rem", color: "var(--status-active-text)", marginBottom: "1rem",
};

export const msgError: React.CSSProperties = {
  padding: "0.65rem 1rem", backgroundColor: "var(--state-danger-bg-soft)", border: "1px solid var(--state-danger-border)",
  borderRadius: "6px", fontSize: "0.84rem", color: "var(--state-danger-strong)", marginBottom: "1rem",
};
