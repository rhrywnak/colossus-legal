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
import { ADMIN_GROUPS, defaultPanel, type AdminGroup, type AdminPanel } from "./adminGroups";

// ── Styles ────────────────────────────────────────────────────────────────────

const tabBarStyle: React.CSSProperties = {
  display: "flex",
  gap: "0.2rem",
  marginBottom: "1.5rem",
  borderBottom: "2px solid var(--border-default)",
  paddingBottom: "0",
};

const tabBase: React.CSSProperties = {
  padding: "0.6rem 1.25rem",
  fontSize: "0.84rem",
  fontWeight: 500,
  color: "var(--text-muted)",
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
  color: "var(--accent-primary)",
  fontWeight: 600,
  borderBottomColor: "var(--accent-primary)",
};

const deniedStyle: React.CSSProperties = {
  padding: "3rem",
  textAlign: "center",
  color: "var(--text-muted)",
  fontSize: "0.9rem",
};


// ── Component ─────────────────────────────────────────────────────────────────

// Environment badge colors
const envBadgeStyle = (env: string): React.CSSProperties => {
  const colors: Record<string, { bg: string; text: string; border: string }> = {
    dev: { bg: "var(--burden-warning-bg)", text: "var(--burden-warning-text)", border: "var(--burden-warning-bg)" },
    prod: { bg: "var(--state-danger-bg-soft)", text: "var(--status-dropped-text)", border: "var(--state-danger-border)" },
  };
  const c = colors[env] || { bg: "var(--bg-page)", text: "var(--text-secondary)", border: "var(--border-default)" };
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
  backgroundColor: ok ? "var(--state-success-strong)" : "var(--state-danger-strong)",
  marginRight: "0.3rem",
});

/**
 * One admin area. Which one is decided by the ROUTE, not by a click.
 *
 * `group` arrives as a prop from `App.tsx` rather than being read from
 * `useParams`: the five groups are five declared routes with fixed paths, so
 * the value is known statically at the route and a param would let a typo in the
 * URL produce a group nothing in `ADMIN_GROUPS` describes.
 */
const Admin: React.FC<{ group: AdminGroup }> = ({ group }) => {
  const { user, loading } = useAuth();
  const spec = ADMIN_GROUPS[group];
  const [activePanel, setActivePanel] = useState<AdminPanel | null>(defaultPanel(group));
  const [status, setStatus] = useState<AdminStatusResponse | null>(null);
  const [statusError, setStatusError] = useState<string | null>(null);

  // Moving between groups is a route change, and React re-uses this component
  // across it — so without this the sub-tab from the group just left would
  // survive into the new one and render a panel that group does not list.
  useEffect(() => {
    setActivePanel(defaultPanel(group));
  }, [group]);

  // Fetch backend status on mount (only if admin).
  //
  // Standing Rule 1: the `.catch(() => {})` this replaces swallowed the failure
  // whole — the status strip simply did not appear, which is indistinguishable
  // from "this group does not show one". A failed status read now says so.
  useEffect(() => {
    if (loading || !user?.permissions.is_admin) return;
    setStatusError(null);
    getAdminStatus()
      .then(setStatus)
      .catch((error: unknown) => {
        // eslint-disable-next-line no-console
        console.error("admin: the backend status could not be read", error);
        setStatusError("The backend status could not be read. The stores may still be up.");
      });
  }, [loading, user]);

  // Read environment/version from runtime config (injected by Ansible)
  const config = (window as any).__COLOSSUS_CONFIG__ || {};
  const environment = status?.environment || config.environment || "unknown";
  const version = status?.version || config.version || "unknown";

  if (loading) {
    return (
      <div style={{ padding: "2rem", textAlign: "center", color: "var(--text-muted)" }}>
        Loading...
      </div>
    );
  }

  if (!user?.permissions.is_admin) {
    return (
      <div style={deniedStyle}>
        <div style={{ fontSize: "1.1rem", fontWeight: 600, color: "var(--text-primary)", marginBottom: "0.5rem" }}>
          Access Denied
        </div>
        Admin access is required to view this page.
      </div>
    );
  }

  return (
    <div style={{ paddingTop: "1.5rem", paddingBottom: "3rem" }}>
      <div style={{ display: "flex", alignItems: "center", gap: "0.75rem", margin: "0 0 0.5rem" }}>
        <h1 style={{ fontSize: "1.35rem", fontWeight: 700, color: "var(--text-primary)", margin: 0, letterSpacing: "-0.02em" }}>
          {spec.heading}
        </h1>
        <span style={envBadgeStyle(environment)}>{environment}</span>
        <span style={{ fontSize: "0.76rem", color: "var(--text-muted)", fontWeight: 500 }}>v{version}</span>
      </div>

      {/* Backend connectivity status — Overview and Data only. */}
      {spec.stores && statusError !== null && (
        <div style={{ marginBottom: "1rem", fontSize: "0.76rem", color: "var(--state-danger-strong)" }} role="alert">
          {statusError}
        </div>
      )}
      {spec.stores && status && (
        <div style={{ display: "flex", gap: "1rem", marginBottom: "1rem", fontSize: "0.76rem", color: "var(--text-secondary)" }}>
          <span><span style={statusDotStyle(status.neo4j_connected)} />Neo4j</span>
          <span><span style={statusDotStyle(status.qdrant_connected)} />Qdrant</span>
          <span><span style={statusDotStyle(status.postgres_connected)} />PostgreSQL</span>
        </div>
      )}

      {/* Sub-tabs. A group with ONE panel draws no bar: a single tab is a
          control that cannot do anything, and rendering it would be the page
          offering a choice it does not have. */}
      {spec.panels.length > 1 && (
        <div style={tabBarStyle}>
          {spec.panels.map((panel) => (
            <button
              key={panel.id}
              style={activePanel === panel.id ? tabActive : tabBase}
              aria-pressed={activePanel === panel.id}
              onClick={() => setActivePanel(panel.id)}
            >
              {panel.label}
            </button>
          ))}
        </div>
      )}

      {/* Panels — every one of the nine unchanged inside, re-homed only. */}
      {activePanel === "metrics" && <AdminMetrics />}
      {activePanel === "indexing" && <AdminIndex />}
      {activePanel === "chats" && <AdminChats />}
      {activePanel === "audit" && <AdminAudit />}
      {activePanel === "models" && <AdminModels />}
      {activePanel === "profiles" && <AdminProfiles />}
      {activePanel === "prompts" && <AdminPrompts />}
      {activePanel === "schemas" && <AdminSchemas />}
      {activePanel === "systemPrompts" && <AdminSystemPrompts />}
    </div>
  );
};

export default Admin;
