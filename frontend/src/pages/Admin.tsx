import React, { useEffect, useState } from "react";
import { useAuth } from "../context/AuthContext";
import AdminIndex from "../components/admin/AdminIndex";
import AdminChats from "../components/admin/AdminChats";
import AdminAudit from "../components/admin/AdminAudit";
import AdminMetrics from "../components/admin/AdminMetrics";
import AdminModels from "../components/admin/AdminModels";
import AdminProfiles from "../components/admin/AdminProfiles";
import AdminPrompts from "../components/admin/AdminPrompts";
import AdminSchemas from "../components/admin/AdminSchemas";
import AdminSystemPrompts from "../components/admin/AdminSystemPrompts";
import { AdminStatusResponse, getAdminStatus } from "../services/admin";

// ── Styles ────────────────────────────────────────────────────────────────────

const tabBarStyle: React.CSSProperties = {
  display: "flex",
  gap: "0.2rem",
  marginBottom: "1.5rem",
  borderBottom: "2px solid #e2e8f0",
  paddingBottom: "0",
};

const tabBase: React.CSSProperties = {
  padding: "0.6rem 1.25rem",
  fontSize: "0.84rem",
  fontWeight: 500,
  color: "#64748b",
  background: "none",
  border: "none",
  borderBottom: "2px solid transparent",
  marginBottom: "-2px",
  cursor: "pointer",
  fontFamily: "inherit",
  transition: "all 0.15s ease",
};

const tabActive: React.CSSProperties = {
  ...tabBase,
  color: "#2563eb",
  fontWeight: 600,
  borderBottomColor: "#2563eb",
};

const deniedStyle: React.CSSProperties = {
  padding: "3rem",
  textAlign: "center",
  color: "#64748b",
  fontSize: "0.9rem",
};

type Tab =
  | "metrics"
  | "indexing"
  | "chats"
  | "audit"
  | "models"
  | "profiles"
  | "prompts"
  | "schemas"
  | "systemPrompts";

const TABS: { id: Tab; label: string }[] = [
  { id: "metrics", label: "Metrics" },
  { id: "indexing", label: "Indexing" },
  { id: "chats", label: "Chats" },
  { id: "audit", label: "Audit" },
  { id: "models", label: "Models" },
  { id: "profiles", label: "Profiles" },
  { id: "prompts", label: "Prompts" },
  { id: "schemas", label: "Schemas" },
  { id: "systemPrompts", label: "System Prompts" },
];

// ── Component ─────────────────────────────────────────────────────────────────

// Environment badge colors
const envBadgeStyle = (env: string): React.CSSProperties => {
  const colors: Record<string, { bg: string; text: string; border: string }> = {
    dev: { bg: "#fef3c7", text: "#92400e", border: "#fcd34d" },
    prod: { bg: "#fee2e2", text: "#991b1b", border: "#fca5a5" },
  };
  const c = colors[env] || { bg: "#f1f5f9", text: "#475569", border: "#e2e8f0" };
  return {
    display: "inline-block",
    padding: "0.15rem 0.5rem",
    fontSize: "0.72rem",
    fontWeight: 700,
    textTransform: "uppercase",
    letterSpacing: "0.05em",
    borderRadius: "4px",
    backgroundColor: c.bg,
    color: c.text,
    border: `1px solid ${c.border}`,
  };
};

const statusDotStyle = (ok: boolean): React.CSSProperties => ({
  display: "inline-block",
  width: "8px",
  height: "8px",
  borderRadius: "50%",
  backgroundColor: ok ? "#10b981" : "#ef4444",
  marginRight: "0.3rem",
});

const Admin: React.FC = () => {
  const { user, loading } = useAuth();
  const [activeTab, setActiveTab] = useState<Tab>("metrics");
  const [status, setStatus] = useState<AdminStatusResponse | null>(null);

  // Fetch backend status on mount (only if admin)
  useEffect(() => {
    if (!loading && user?.permissions.is_admin) {
      getAdminStatus().then(setStatus).catch(() => {});
    }
  }, [loading, user]);

  // Read environment/version from runtime config (injected by Ansible)
  const config = (window as any).__COLOSSUS_CONFIG__ || {};
  const environment = status?.environment || config.environment || "unknown";
  const version = status?.version || config.version || "unknown";

  if (loading) {
    return (
      <div style={{ padding: "2rem", textAlign: "center", color: "#64748b" }}>
        Loading...
      </div>
    );
  }

  if (!user?.permissions.is_admin) {
    return (
      <div style={deniedStyle}>
        <div style={{ fontSize: "1.1rem", fontWeight: 600, color: "#0f172a", marginBottom: "0.5rem" }}>
          Access Denied
        </div>
        Admin access is required to view this page.
      </div>
    );
  }

  return (
    <div style={{ paddingTop: "1.5rem", paddingBottom: "3rem" }}>
      <div style={{ display: "flex", alignItems: "center", gap: "0.75rem", margin: "0 0 0.5rem" }}>
        <h1 style={{ fontSize: "1.35rem", fontWeight: 700, color: "#0f172a", margin: 0, letterSpacing: "-0.02em" }}>
          Admin
        </h1>
        <span style={envBadgeStyle(environment)}>{environment}</span>
        <span style={{ fontSize: "0.76rem", color: "#64748b", fontWeight: 500 }}>v{version}</span>
      </div>

      {/* Backend connectivity status */}
      {status && (
        <div style={{ display: "flex", gap: "1rem", marginBottom: "1rem", fontSize: "0.76rem", color: "#475569" }}>
          <span><span style={statusDotStyle(status.neo4j_connected)} />Neo4j</span>
          <span><span style={statusDotStyle(status.qdrant_connected)} />Qdrant</span>
          <span><span style={statusDotStyle(status.postgres_connected)} />PostgreSQL</span>
        </div>
      )}

      {/* Tabs */}
      <div style={tabBarStyle}>
        {TABS.map((tab) => (
          <button
            key={tab.id}
            style={activeTab === tab.id ? tabActive : tabBase}
            onClick={() => setActiveTab(tab.id)}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Panels */}
      {activeTab === "metrics" && <AdminMetrics />}
      {activeTab === "indexing" && <AdminIndex />}
      {activeTab === "chats" && <AdminChats />}
      {activeTab === "audit" && <AdminAudit />}
      {activeTab === "models" && <AdminModels />}
      {activeTab === "profiles" && <AdminProfiles />}
      {activeTab === "prompts" && <AdminPrompts />}
      {activeTab === "schemas" && <AdminSchemas />}
      {activeTab === "systemPrompts" && <AdminSystemPrompts />}
    </div>
  );
};

export default Admin;
