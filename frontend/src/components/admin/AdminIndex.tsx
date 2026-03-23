import React, { useState } from "react";
import { triggerReindex, ReindexResponse } from "../../services/admin";

const cardStyle: React.CSSProperties = {
  backgroundColor: "#ffffff", border: "1px solid #e2e8f0", borderRadius: "10px",
  padding: "1.25rem 1.5rem",
};

const btnPrimary: React.CSSProperties = {
  backgroundColor: "#2563eb", color: "#fff", border: "none", borderRadius: "6px",
  padding: "0.55rem 1.25rem", fontSize: "0.84rem", fontWeight: 600, cursor: "pointer",
  fontFamily: "inherit",
};

const btnDanger: React.CSSProperties = {
  backgroundColor: "#dc2626", color: "#fff", border: "none", borderRadius: "6px",
  padding: "0.55rem 1.25rem", fontSize: "0.84rem", fontWeight: 600, cursor: "pointer",
  fontFamily: "inherit",
};

const AdminIndex: React.FC = () => {
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<ReindexResponse | null>(null);
  const [error, setError] = useState("");
  const [confirmFull, setConfirmFull] = useState(false);

  const handleReindex = async (mode: string) => {
    setLoading(true);
    setError("");
    setResult(null);
    setConfirmFull(false);
    try {
      const res = await triggerReindex(mode);
      setResult(res);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div>
      <div style={cardStyle}>
        <div style={{ fontSize: "0.9rem", fontWeight: 600, color: "#0f172a", marginBottom: "0.5rem" }}>
          Qdrant Vector Index
        </div>
        <p style={{ fontSize: "0.84rem", color: "#475569", margin: "0 0 1rem", lineHeight: 1.5 }}>
          Incremental mode embeds only new nodes not already in Qdrant.
          Full rebuild deletes the collection and re-embeds everything (30-120 seconds).
        </p>

        <div style={{ display: "flex", gap: "0.75rem", alignItems: "center" }}>
          <button style={btnPrimary} onClick={() => handleReindex("incremental")} disabled={loading}>
            {loading ? "Reindexing..." : "Reindex (Incremental)"}
          </button>

          {!confirmFull ? (
            <button style={btnDanger} onClick={() => setConfirmFull(true)} disabled={loading}>
              Full Rebuild
            </button>
          ) : (
            <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
              <span style={{ fontSize: "0.82rem", color: "#dc2626", fontWeight: 500 }}>Are you sure?</span>
              <button style={btnDanger} onClick={() => handleReindex("full")} disabled={loading}>
                Yes, Rebuild
              </button>
              <button
                style={{ ...btnDanger, backgroundColor: "#f1f5f9", color: "#334155" }}
                onClick={() => setConfirmFull(false)}
              >
                Cancel
              </button>
            </div>
          )}
        </div>

        {loading && (
          <div style={{ marginTop: "1rem", padding: "1rem", backgroundColor: "#eff6ff", borderRadius: "6px", fontSize: "0.84rem", color: "#2563eb" }}>
            Running embedding pipeline... This may take up to 2 minutes.
          </div>
        )}

        {error && (
          <div style={{ marginTop: "1rem", padding: "0.65rem 1rem", backgroundColor: "#fef2f2", border: "1px solid #fecaca", borderRadius: "6px", fontSize: "0.84rem", color: "#dc2626" }}>
            {error}
          </div>
        )}

        {result && (
          <div style={{ marginTop: "1rem", padding: "1rem", backgroundColor: "#ecfdf5", border: "1px solid #a7f3d0", borderRadius: "8px" }}>
            <div style={{ fontSize: "0.82rem", fontWeight: 600, color: "#047857", marginBottom: "0.5rem" }}>
              Reindex Complete
            </div>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: "0.75rem" }}>
              {[
                { label: "Mode", value: result.mode },
                { label: "New Points", value: String(result.new_points) },
                { label: "Skipped", value: String(result.skipped) },
                { label: "Duration", value: `${(result.duration_ms / 1000).toFixed(1)}s` },
              ].map((s) => (
                <div key={s.label} style={{ textAlign: "center" }}>
                  <div style={{ fontSize: "1.1rem", fontWeight: 700, color: "#0f172a" }}>{s.value}</div>
                  <div style={{ fontSize: "0.72rem", color: "#64748b" }}>{s.label}</div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default AdminIndex;
