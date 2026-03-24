/**
 * DocumentWorkspace — Side-by-side PDF viewer + evidence panel.
 *
 * This is a placeholder. The full implementation comes in Phase 2B.
 * For now, it shows the document ID and mode from the URL.
 */
import React from "react";
import { useParams, useLocation } from "react-router-dom";

const DocumentWorkspace: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const location = useLocation();
  const mode = location.pathname.endsWith("/audit") ? "audit" : "view";

  return (
    <div style={{ padding: "3rem", textAlign: "center", color: "#64748b" }}>
      <h2 style={{ color: "#0f172a", fontSize: "1.2rem", marginBottom: "0.5rem" }}>
        Document Workspace
      </h2>
      <p>
        <strong>{mode}</strong> mode for <code>{id}</code>
      </p>
      <p style={{ fontSize: "0.84rem" }}>Coming in Phase 2B.</p>
    </div>
  );
};

export default DocumentWorkspace;
