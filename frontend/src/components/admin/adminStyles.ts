import React from "react";

export const cardStyle: React.CSSProperties = {
  backgroundColor: "#ffffff", border: "1px solid #e2e8f0", borderRadius: "10px",
  padding: "1.25rem 1.5rem",
};

export const btnPrimary: React.CSSProperties = {
  backgroundColor: "#2563eb", color: "#fff", border: "none", borderRadius: "6px",
  padding: "0.45rem 1rem", fontSize: "0.82rem", fontWeight: 600, cursor: "pointer",
  fontFamily: "inherit",
};

export const btnSecondary: React.CSSProperties = {
  backgroundColor: "#f1f5f9", color: "#334155", border: "1px solid #e2e8f0",
  borderRadius: "6px", padding: "0.45rem 1rem", fontSize: "0.82rem", fontWeight: 500,
  cursor: "pointer", fontFamily: "inherit",
};

export const inputStyle: React.CSSProperties = {
  width: "100%", padding: "0.45rem 0.65rem", border: "1px solid #e2e8f0",
  borderRadius: "6px", fontSize: "0.84rem", fontFamily: "inherit",
  boxSizing: "border-box",
};

export const labelStyle: React.CSSProperties = {
  display: "block", fontSize: "0.76rem", fontWeight: 600, color: "#475569",
  marginBottom: "0.25rem",
};

export const msgSuccess: React.CSSProperties = {
  padding: "0.65rem 1rem", backgroundColor: "#ecfdf5", border: "1px solid #a7f3d0",
  borderRadius: "6px", fontSize: "0.84rem", color: "#047857", marginBottom: "1rem",
};

export const msgError: React.CSSProperties = {
  padding: "0.65rem 1rem", backgroundColor: "#fef2f2", border: "1px solid #fecaca",
  borderRadius: "6px", fontSize: "0.84rem", color: "#dc2626", marginBottom: "1rem",
};
