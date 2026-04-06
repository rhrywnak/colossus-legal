// TODO: B-4 — v1 dead code. Part of the manual audit workflow superseded
// by the v2 pipeline review system. Remove when v1 is fully deprecated.
import React, { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { AuditHealthResponse, AuditCheck, getAuditHealth } from "../../services/admin";
import { cardStyle, btnPrimary, msgError } from "./adminStyles";

const statusColors: Record<string, { border: string; icon: string }> = {
  pass: { border: "#10b981", icon: "pass" },
  warn: { border: "#f59e0b", icon: "warn" },
  fail: { border: "#ef4444", icon: "fail" },
};

const severityBadge = (severity: string): React.CSSProperties => {
  const c: Record<string, { bg: string; text: string }> = {
    critical: { bg: "#fee2e2", text: "#991b1b" },
    high: { bg: "#fef3c7", text: "#92400e" },
    low: { bg: "#f1f5f9", text: "#475569" },
  };
  const s = c[severity] || c.low;
  return {
    display: "inline-block", padding: "0.1rem 0.4rem", fontSize: "0.68rem",
    fontWeight: 600, borderRadius: "3px", backgroundColor: s.bg,
    color: s.text, textTransform: "uppercase", marginRight: "0.4rem",
  };
};

const statusIcon = (status: string) =>
  status === "pass" ? "\u2705" : status === "warn" ? "\u26A0\uFE0F" : "\u274C";

const AdminAudit: React.FC = () => {
  const navigate = useNavigate();
  const [data, setData] = useState<AuditHealthResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});

  const fetchHealth = async () => {
    setLoading(true);
    setError("");
    try {
      const res = await getAuditHealth();
      setData(res);
      // Auto-expand checks with issues
      const exp: Record<string, boolean> = {};
      res.checks.forEach((c) => { if (c.status !== "pass") exp[c.name] = true; });
      setExpanded(exp);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { fetchHealth(); }, []);

  const toggle = (name: string) =>
    setExpanded((prev) => ({ ...prev, [name]: !prev[name] }));

  if (loading && !data) {
    return <div style={{ textAlign: "center", padding: "2rem", color: "#64748b" }}>Running health checks...</div>;
  }

  return (
    <div>
      {error && <div style={msgError}>{error}</div>}

      {/* Summary cards */}
      {data && (
        <>
          <div style={{ display: "flex", gap: "0.75rem", marginBottom: "1rem" }}>
            {[
              { label: "Documents", value: data.summary.total_documents },
              { label: "Evidence", value: data.summary.total_evidence },
              { label: "Qdrant Pts", value: data.summary.total_qdrant_points },
              { label: "Complete", value: `${data.summary.completeness_pct.toFixed(0)}%` },
            ].map((s) => (
              <div key={s.label} style={{ ...cardStyle, flex: 1, textAlign: "center" }}>
                <div style={{ fontSize: "1.4rem", fontWeight: 700, color: "#0f172a" }}>{s.value}</div>
                <div style={{ fontSize: "0.72rem", color: "#64748b" }}>{s.label}</div>
              </div>
            ))}
          </div>

          {/* Check results */}
          <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem", marginBottom: "1rem" }}>
            {data.checks.map((check) => (
              <CheckCard
                key={check.name}
                check={check}
                isExpanded={!!expanded[check.name]}
                onToggle={() => toggle(check.name)}
                onNavigate={(id) => navigate(`/admin/documents/${id}/audit`)}
              />
            ))}
          </div>

          {/* Footer */}
          <div style={{ display: "flex", alignItems: "center", gap: "1rem" }}>
            <button style={btnPrimary} onClick={fetchHealth} disabled={loading}>
              {loading ? "Checking..." : "Run Health Check"}
            </button>
            <span style={{ fontSize: "0.76rem", color: "#64748b" }}>
              Last checked: {new Date(data.checked_at).toLocaleString()}
            </span>
          </div>
        </>
      )}
    </div>
  );
};

// ── Check Card sub-component ──────────────────────────

interface CheckCardProps {
  check: AuditCheck;
  isExpanded: boolean;
  onToggle: () => void;
  onNavigate: (docId: string) => void;
}

const CheckCard: React.FC<CheckCardProps> = ({ check, isExpanded, onToggle, onNavigate }) => {
  const colors = statusColors[check.status] || statusColors.fail;
  return (
    <div style={{
      ...cardStyle, borderLeft: `4px solid ${colors.border}`,
      padding: "0.75rem 1rem",
    }}>
      <div
        style={{ display: "flex", alignItems: "center", cursor: "pointer", gap: "0.5rem" }}
        onClick={onToggle}
      >
        <span style={{ fontSize: "1rem" }}>{statusIcon(check.status)}</span>
        <span style={{ fontWeight: 600, fontSize: "0.84rem", color: "#0f172a", flex: 1 }}>
          {check.name.replace(/_/g, " ")}
        </span>
        <span style={{ fontSize: "0.78rem", color: "#64748b" }}>{check.message}</span>
        <span style={{ fontSize: "0.76rem", color: "#94a3b8" }}>{isExpanded ? "\u25B2" : "\u25BC"}</span>
      </div>

      {isExpanded && check.details.length > 0 && (
        <div style={{ marginTop: "0.5rem", paddingLeft: "1.5rem" }}>
          {check.details.map((issue, i) => (
            <div key={i} style={{
              fontSize: "0.78rem", padding: "0.25rem 0",
              borderBottom: i < check.details.length - 1 ? "1px solid #f1f5f9" : "none",
              display: "flex", alignItems: "center", gap: "0.4rem",
            }}>
              <span style={severityBadge(issue.severity)}>{issue.severity}</span>
              <span style={{ color: "#64748b" }}>{issue.resource_type}</span>
              <span style={{ color: "#0f172a", fontWeight: 500 }}>{issue.resource_id}</span>
              <span style={{ color: "#475569" }}>— {issue.description}</span>
              {issue.resource_type === "document" && (
                <button
                  style={{ marginLeft: "auto", fontSize: "0.72rem", color: "#2563eb", background: "none", border: "none", cursor: "pointer", fontFamily: "inherit" }}
                  onClick={(e) => { e.stopPropagation(); onNavigate(issue.resource_id); }}
                >
                  Audit
                </button>
              )}
            </div>
          ))}
        </div>
      )}

      {isExpanded && check.details.length === 0 && (
        <div style={{ marginTop: "0.4rem", paddingLeft: "1.5rem", fontSize: "0.78rem", color: "#64748b" }}>
          No issues found.
        </div>
      )}
    </div>
  );
};

export default AdminAudit;
