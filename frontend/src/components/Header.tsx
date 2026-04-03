import React, { useEffect, useRef, useState } from "react";
import { Link, useLocation } from "react-router-dom";
import { useAuth } from "../context/AuthContext";
import { logout } from "../services/auth";

const AUTHENTIK_SETTINGS_URL = "https://auth.cogmai.com/if/user/#/settings";

// ─── Navigation items ────────────────────────────────────────────────────────
const NAV_ITEMS = [
  { label: "Home", path: "/" },
  { label: "Evidence", path: "/explorer" },
  { label: "People", path: "/people" },
  { label: "Documents", path: "/documents" },
  { label: "Chat", path: "/ask" },
];

// Admin-only items — shown when user.permissions.is_admin
const ADMIN_ITEMS = [
  { label: "Admin", path: "/admin" },
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
  flexShrink: 0,
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
  display: "flex",
  gap: "0.2rem",
  flexWrap: "wrap",
  justifyContent: "center",
  flex: "1 1 auto",
  minWidth: 0,
};

const navLinkBase: React.CSSProperties = {
  textDecoration: "none",
  fontSize: "0.84rem",
  fontWeight: 500,
  padding: "0.4rem 0.6rem",
  borderRadius: "6px",
  transition: "all 0.15s ease",
  color: "#64748b",
  whiteSpace: "nowrap",
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
  flexShrink: 0,
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

// ─── Dropdown styles ────────────────────────────────────────────────────────
const dropdownStyle: React.CSSProperties = {
  position: "absolute", top: "100%", right: 0, marginTop: "0.35rem",
  minWidth: "220px", backgroundColor: "#ffffff", borderRadius: "8px",
  border: "1px solid #e2e8f0", boxShadow: "0 2px 8px rgba(0,0,0,0.15)",
  zIndex: 200, overflow: "hidden",
};
const dropdownItem: React.CSSProperties = {
  display: "block", width: "100%", padding: "0.5rem 1rem", fontSize: "0.82rem",
  color: "#334155", textDecoration: "none", border: "none", background: "none",
  textAlign: "left", cursor: "pointer", fontFamily: "inherit",
};
const dropdownDivider: React.CSSProperties = {
  height: "1px", backgroundColor: "#e2e8f0", margin: "0.25rem 0",
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
  const [dropdownOpen, setDropdownOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  const userName = user?.display_name || user?.username || "Anonymous";
  const userInitials = user?.display_name?.[0]?.toUpperCase() ?? "?";

  // Close dropdown on click outside
  useEffect(() => {
    if (!dropdownOpen) return;
    const handleClick = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setDropdownOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [dropdownOpen]);

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
          <span style={{ fontSize: "0.72rem", color: "#94a3b8", marginLeft: "0.35rem", fontWeight: 400 }}>
            v{__APP_VERSION__}
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
        {user?.permissions.is_admin && ADMIN_ITEMS.map((item) => (
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

      {/* Right — User dropdown */}
      <div style={rightSectionStyle}>
        {loading ? (
          <span style={{ fontSize: "0.84rem", color: "#94a3b8" }}>...</span>
        ) : (
          <div ref={dropdownRef} style={{ position: "relative" }}>
            <div
              style={{ ...userBadgeStyle, cursor: "pointer" }}
              onClick={() => setDropdownOpen((prev) => !prev)}
            >
              <div style={avatarStyle}>{userInitials}</div>
              {userName}
            </div>

            {dropdownOpen && isAuthenticated && (
              <div style={dropdownStyle}>
                {/* User info */}
                <div style={{ padding: "0.6rem 1rem" }}>
                  <div style={{ fontSize: "0.84rem", fontWeight: 600, color: "#0f172a" }}>
                    {userName}
                  </div>
                  {user?.email && (
                    <div style={{ fontSize: "0.76rem", color: "#64748b", marginTop: "0.1rem" }}>
                      {user.email}
                    </div>
                  )}
                  {user?.groups && user.groups.length > 0 && (
                    <div style={{ fontSize: "0.72rem", color: "#94a3b8", marginTop: "0.15rem" }}>
                      {user.groups.join(", ")}
                    </div>
                  )}
                </div>
                <div style={dropdownDivider} />

                {/* Account links */}
                <a
                  href={AUTHENTIK_SETTINGS_URL}
                  target="_blank"
                  rel="noopener noreferrer"
                  style={dropdownItem}
                  onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = "#f1f5f9"; }}
                  onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = "transparent"; }}
                  onClick={() => setDropdownOpen(false)}
                >
                  Account Settings
                </a>
                <a
                  href={AUTHENTIK_SETTINGS_URL}
                  target="_blank"
                  rel="noopener noreferrer"
                  style={dropdownItem}
                  onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = "#f1f5f9"; }}
                  onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = "transparent"; }}
                  onClick={() => setDropdownOpen(false)}
                >
                  Change Password
                </a>
                <div style={dropdownDivider} />

                {/* Sign out */}
                <button
                  style={{ ...dropdownItem, color: "#dc2626" }}
                  onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = "#fef2f2"; }}
                  onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = "transparent"; }}
                  onClick={() => { setDropdownOpen(false); logout(); }}
                >
                  Sign Out
                </button>
              </div>
            )}
          </div>
        )}
      </div>
    </header>
  );
};

export default Header;
