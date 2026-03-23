import React, { useState } from "react";
import { useAuth } from "../context/AuthContext";
import AdminDocuments from "../components/admin/AdminDocuments";
import AdminIndex from "../components/admin/AdminIndex";
import AdminChats from "../components/admin/AdminChats";

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

type Tab = "documents" | "indexing" | "chats";

const TABS: { id: Tab; label: string }[] = [
  { id: "documents", label: "Documents" },
  { id: "indexing", label: "Indexing" },
  { id: "chats", label: "Chats" },
];

// ── Component ─────────────────────────────────────────────────────────────────

const Admin: React.FC = () => {
  const { user, loading } = useAuth();
  const [activeTab, setActiveTab] = useState<Tab>("documents");

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
      <h1 style={{ fontSize: "1.35rem", fontWeight: 700, color: "#0f172a", margin: "0 0 1.25rem", letterSpacing: "-0.02em" }}>
        Admin
      </h1>

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
      {activeTab === "documents" && <AdminDocuments />}
      {activeTab === "indexing" && <AdminIndex />}
      {activeTab === "chats" && <AdminChats />}
    </div>
  );
};

export default Admin;
