import React, { useEffect, useRef, useState } from "react";
import { Link, useLocation } from "react-router-dom";

// Navigation group structure for dropdown menus
type NavItem = {
  label: string;
  path: string;
};

type NavGroup = {
  label: string;
  items: NavItem[];
};

// Grouped navigation items
const NAV_GROUPS: NavGroup[] = [
  {
    label: "Case",
    items: [
      { label: "Home", path: "/" },
      { label: "Allegations", path: "/allegations" },
      { label: "Claims", path: "/claims" },
      { label: "Damages", path: "/damages" },
    ],
  },
  {
    label: "Evidence",
    items: [
      { label: "Evidence", path: "/evidence" },
      { label: "Explorer", path: "/explorer" },
      { label: "Graph", path: "/graph" },
    ],
  },
  {
    label: "Analysis",
    items: [
      { label: "People", path: "/people" },
      { label: "Contradictions", path: "/contradictions" },
    ],
  },
];

// Standalone links (not in dropdowns)
const STANDALONE_LINKS: NavItem[] = [{ label: "Documents", path: "/documents" }];

// Styles
const headerStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  padding: "0.75rem 1.5rem",
  backgroundColor: "#ffffff",
  borderBottom: "1px solid #e5e7eb",
  fontFamily: "Inter, system-ui, sans-serif",
};

const logoStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "0.5rem",
  textDecoration: "none",
  color: "#1f2937",
  fontWeight: 700,
  fontSize: "1.25rem",
};

const navStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "0.25rem",
};

const dropdownContainerStyle: React.CSSProperties = {
  position: "relative",
};

const dropdownButtonStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "0.25rem",
  padding: "0.5rem 0.75rem",
  backgroundColor: "transparent",
  border: "none",
  borderRadius: "6px",
  cursor: "pointer",
  fontSize: "0.9rem",
  fontWeight: 500,
  color: "#374151",
  fontFamily: "inherit",
};

const dropdownButtonActiveStyle: React.CSSProperties = {
  ...dropdownButtonStyle,
  backgroundColor: "#f3f4f6",
};

const dropdownMenuStyle: React.CSSProperties = {
  position: "absolute",
  top: "100%",
  left: 0,
  marginTop: "0.25rem",
  minWidth: "160px",
  backgroundColor: "#ffffff",
  border: "1px solid #e5e7eb",
  borderRadius: "8px",
  boxShadow: "0 4px 6px -1px rgba(0, 0, 0, 0.1)",
  zIndex: 50,
  padding: "0.25rem 0",
};

const dropdownItemStyle: React.CSSProperties = {
  display: "block",
  padding: "0.5rem 1rem",
  textDecoration: "none",
  color: "#374151",
  fontSize: "0.875rem",
};

const dropdownItemActiveStyle: React.CSSProperties = {
  ...dropdownItemStyle,
  backgroundColor: "#eff6ff",
  color: "#2563eb",
  fontWeight: 500,
};

const standaloneLinkStyle: React.CSSProperties = {
  padding: "0.5rem 0.75rem",
  textDecoration: "none",
  color: "#374151",
  fontSize: "0.9rem",
  fontWeight: 500,
  borderRadius: "6px",
};

const standaloneLinkActiveStyle: React.CSSProperties = {
  ...standaloneLinkStyle,
  backgroundColor: "#eff6ff",
  color: "#2563eb",
};

const loginPlaceholderStyle: React.CSSProperties = {
  padding: "0.5rem 1rem",
  backgroundColor: "#f3f4f6",
  borderRadius: "6px",
  fontSize: "0.875rem",
  color: "#6b7280",
};

// Dropdown component for a navigation group
const NavDropdown: React.FC<{
  group: NavGroup;
  isOpen: boolean;
  onToggle: () => void;
  currentPath: string;
}> = ({ group, isOpen, onToggle, currentPath }) => {
  // Check if any item in this group is active
  const hasActiveItem = group.items.some((item) => item.path === currentPath);

  return (
    <div style={dropdownContainerStyle}>
      <button
        onClick={onToggle}
        style={hasActiveItem ? dropdownButtonActiveStyle : dropdownButtonStyle}
        onMouseEnter={(e) => {
          if (!hasActiveItem) {
            e.currentTarget.style.backgroundColor = "#f3f4f6";
          }
        }}
        onMouseLeave={(e) => {
          if (!hasActiveItem) {
            e.currentTarget.style.backgroundColor = "transparent";
          }
        }}
      >
        {group.label}
        <span style={{ fontSize: "0.7rem", marginLeft: "0.125rem" }}>
          {isOpen ? "\u25B2" : "\u25BC"}
        </span>
      </button>

      {isOpen && (
        <div style={dropdownMenuStyle}>
          {group.items.map((item) => (
            <Link
              key={item.path}
              to={item.path}
              style={
                currentPath === item.path
                  ? dropdownItemActiveStyle
                  : dropdownItemStyle
              }
              onMouseEnter={(e) => {
                if (currentPath !== item.path) {
                  e.currentTarget.style.backgroundColor = "#f9fafb";
                }
              }}
              onMouseLeave={(e) => {
                if (currentPath !== item.path) {
                  e.currentTarget.style.backgroundColor = "transparent";
                }
              }}
            >
              {item.label}
            </Link>
          ))}
        </div>
      )}
    </div>
  );
};

// Main Header component
const Header: React.FC = () => {
  const location = useLocation();
  const [openDropdown, setOpenDropdown] = useState<string | null>(null);
  const headerRef = useRef<HTMLElement>(null);

  // Close dropdown when clicking outside
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (
        headerRef.current &&
        !headerRef.current.contains(event.target as Node)
      ) {
        setOpenDropdown(null);
      }
    }

    document.addEventListener("click", handleClickOutside);

    // Cleanup listener on unmount
    return () => {
      document.removeEventListener("click", handleClickOutside);
    };
  }, []);

  // Close dropdown when route changes
  useEffect(() => {
    setOpenDropdown(null);
  }, [location.pathname]);

  const handleDropdownToggle = (label: string) => {
    setOpenDropdown((current) => (current === label ? null : label));
  };

  return (
    <header ref={headerRef} style={headerStyle}>
      {/* Logo */}
      <Link to="/" style={logoStyle}>
        <span role="img" aria-label="scales">
          {"\u2696\uFE0F"}
        </span>
        COLOSSUS
      </Link>

      {/* Navigation */}
      <nav style={navStyle}>
        {/* Dropdown groups */}
        {NAV_GROUPS.map((group) => (
          <NavDropdown
            key={group.label}
            group={group}
            isOpen={openDropdown === group.label}
            onToggle={() => handleDropdownToggle(group.label)}
            currentPath={location.pathname}
          />
        ))}

        {/* Standalone links */}
        {STANDALONE_LINKS.map((link) => (
          <Link
            key={link.path}
            to={link.path}
            style={
              location.pathname === link.path ||
              location.pathname.startsWith(link.path + "/")
                ? standaloneLinkActiveStyle
                : standaloneLinkStyle
            }
            onMouseEnter={(e) => {
              if (
                location.pathname !== link.path &&
                !location.pathname.startsWith(link.path + "/")
              ) {
                e.currentTarget.style.backgroundColor = "#f3f4f6";
              }
            }}
            onMouseLeave={(e) => {
              if (
                location.pathname !== link.path &&
                !location.pathname.startsWith(link.path + "/")
              ) {
                e.currentTarget.style.backgroundColor = "transparent";
              }
            }}
          >
            {link.label}
          </Link>
        ))}
      </nav>

      {/* Login placeholder */}
      <div style={loginPlaceholderStyle}>Login</div>
    </header>
  );
};

export default Header;
