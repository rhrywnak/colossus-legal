import React from "react";
import { Link, useLocation } from "react-router-dom";
import { useAuth } from "../context/AuthContext";
import { logout } from "../services/auth";

// ─── Navigation items ────────────────────────────────────────────────────────
const NAV_ITEMS = [
  { label: "Home", path: "/" },
  { label: "Evidence", path: "/explorer" },
  { label: "People", path: "/people" },
  { label: "Documents", path: "/documents" },
  { label: "Analysis", path: "/analysis" },
];

// ─── Styles ──────────────────────────────────────────────────────────────────
const headerStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  padding: "0 2rem",
  height: "56px",
  backgroundColor: "#ffffff",
  borderBottom: "1px solid #e2e8f0",
  position: "sticky",
  top: 0,
  zIndex: 100,
};

const logoStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "0.55rem",
  textDecoration: "none",
};

const logoIconStyle: React.CSSProperties = {
  width: "30px",
  height: "30px",
  backgroundColor: "#2563eb",
  borderRadius: "7px",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  fontWeight: 700,
  fontSize: "0.9rem",
  color: "#ffffff",
};

const navContainerStyle: React.CSSProperties = {
  position: "absolute",
  left: "50%",
  transform: "translateX(-50%)",
  display: "flex",
  gap: "0.2rem",
};

const navLinkBase: React.CSSProperties = {
  textDecoration: "none",
  fontSize: "0.84rem",
  fontWeight: 500,
  padding: "0.4rem 0.75rem",
  borderRadius: "6px",
  transition: "all 0.15s ease",
  color: "#64748b",
};

const navLinkActive: React.CSSProperties = {
  ...navLinkBase,
  color: "#2563eb",
  backgroundColor: "#eff6ff",
  fontWeight: 600,
};

const rightSectionStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "1rem",
};

const userBadgeStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "0.5rem",
  fontSize: "0.84rem",
  color: "#334155",
  fontWeight: 500,
};

const avatarStyle: React.CSSProperties = {
  width: "32px",
  height: "32px",
  borderRadius: "50%",
  backgroundColor: "#2563eb",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  fontWeight: 600,
  fontSize: "0.72rem",
  color: "#ffffff",
};

const signOutStyle: React.CSSProperties = {
  color: "#94a3b8",
  background: "none",
  border: "1px solid #e2e8f0",
  padding: "0.32rem 0.75rem",
  borderRadius: "6px",
  fontSize: "0.76rem",
  fontWeight: 500,
  cursor: "pointer",
  fontFamily: "inherit",
  transition: "all 0.15s ease",
};

// ─── Helper: is this nav item active? ────────────────────────────────────────
function isActive(itemPath: string, currentPath: string): boolean {
  if (itemPath === "/") return currentPath === "/";
  return currentPath === itemPath || currentPath.startsWith(itemPath + "/");
}

// ─── Component ───────────────────────────────────────────────────────────────
const Header: React.FC = () => {
  const location = useLocation();
  const { user, loading, isAuthenticated } = useAuth();

  const userName = user?.display_name || user?.username || "Anonymous";
  const userInitials = user?.display_name?.[0]?.toUpperCase() ?? "?";

  return (
    <header style={headerStyle}>
      {/* Left — Logo */}
      <Link to="/" style={logoStyle}>
        <div style={logoIconStyle}>C</div>
        <div>
          <span style={{ fontWeight: 700, fontSize: "1.05rem", color: "#0f172a", letterSpacing: "-0.01em" }}>
            Colossus
          </span>
          <span style={{ fontWeight: 400, color: "#94a3b8", marginLeft: "0.2rem" }}>
            Legal
          </span>
        </div>
      </Link>

      {/* Center — Nav links */}
      <nav style={navContainerStyle}>
        {NAV_ITEMS.map((item) => (
          <Link
            key={item.path}
            to={item.path}
            style={isActive(item.path, location.pathname) ? navLinkActive : navLinkBase}
            onMouseEnter={(e) => {
              if (!isActive(item.path, location.pathname)) {
                e.currentTarget.style.color = "#1e293b";
                e.currentTarget.style.backgroundColor = "#f1f5f9";
              }
            }}
            onMouseLeave={(e) => {
              if (!isActive(item.path, location.pathname)) {
                e.currentTarget.style.color = "#64748b";
                e.currentTarget.style.backgroundColor = "transparent";
              }
            }}
          >
            {item.label}
          </Link>
        ))}
      </nav>

      {/* Right — User badge + Sign Out */}
      <div style={rightSectionStyle}>
        {loading ? (
          <span style={{ fontSize: "0.84rem", color: "#94a3b8" }}>...</span>
        ) : (
          <>
            <div style={userBadgeStyle}>
              <div style={avatarStyle}>{userInitials}</div>
              {userName}
            </div>
            {isAuthenticated && (
              <button
                style={signOutStyle}
                onClick={() => {
                  logout();
                }}
              >
                Sign Out
              </button>
            )}
          </>
        )}
      </div>
    </header>
  );
};

export default Header;
