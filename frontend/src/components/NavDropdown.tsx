// =============================================================================
// NavDropdown.tsx — a hand-rolled nav-group dropdown (e.g. "Proof Matrix ▾")
// -----------------------------------------------------------------------------
// Extracted from Header.tsx (which would otherwise exceed the 300-line module
// limit). Mirrors the user-account dropdown pattern in Header: a toggle button,
// an absolutely-positioned panel, and click-outside dismissal via a useRef +
// useEffect mousedown listener. No menu dependency.
//
// The trigger highlights when the current route matches ANY child link; each
// child link highlights when it is the active route.
// =============================================================================

import React, { useEffect, useRef, useState } from "react";
import { Link } from "react-router-dom";

/** One leaf link inside a nav group. */
export type NavLeaf = { label: string; path: string };

// Local copy of the active-route test. Header keeps its own identical helper for
// its flat links; duplicating these three lines is cheaper than a shared module
// that would create a circular import between Header and NavDropdown.
function isActive(itemPath: string, currentPath: string): boolean {
  if (itemPath === "/") return currentPath === "/";
  return currentPath === itemPath || currentPath.startsWith(itemPath + "/");
}

// ─── Styles (design tokens only; match Header's nav-link + dropdown styling) ──

const triggerBase: React.CSSProperties = {
  textDecoration: "none",
  fontSize: "0.84rem",
  fontWeight: 500,
  padding: "0.4rem 0.6rem",
  borderRadius: "6px",
  color: "var(--text-muted)",
  whiteSpace: "nowrap",
  border: "none",
  background: "transparent",
  cursor: "pointer",
  fontFamily: "inherit",
  display: "flex",
  alignItems: "center",
  gap: "0.2rem",
};

const triggerActive: React.CSSProperties = {
  ...triggerBase,
  color: "var(--accent-primary)",
  background: "var(--accent-bg-soft)",
  fontWeight: 600,
};

// Left-aligned panel (the user-account dropdown is right-aligned).
const panelStyle: React.CSSProperties = {
  position: "absolute",
  top: "100%",
  left: 0,
  marginTop: "0.35rem",
  minWidth: "180px",
  backgroundColor: "var(--bg-surface)",
  borderRadius: "8px",
  border: "1px solid var(--border-default)",
  boxShadow: "0 2px 8px rgba(0,0,0,0.15)",
  zIndex: 200,
  overflow: "hidden",
};

const itemStyle: React.CSSProperties = {
  display: "block",
  width: "100%",
  padding: "0.5rem 1rem",
  fontSize: "0.82rem",
  textDecoration: "none",
  textAlign: "left",
};

const NavDropdown: React.FC<{
  label: string;
  items: NavLeaf[];
  currentPath: string;
}> = ({ label, items, currentPath }) => {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const groupActive = items.some((c) => isActive(c.path, currentPath));

  // Close on click outside (same pattern as Header's user-account dropdown).
  useEffect(() => {
    if (!open) return;
    const handleClick = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [open]);

  return (
    <div ref={ref} style={{ position: "relative" }}>
      <button
        type="button"
        style={groupActive ? triggerActive : triggerBase}
        onClick={() => setOpen((prev) => !prev)}
      >
        {label}
        <span style={{ fontSize: "0.6rem" }}>▾</span>
      </button>
      {open && (
        <div style={panelStyle}>
          {items.map((child) => {
            const childActive = isActive(child.path, currentPath);
            return (
              <Link
                key={child.path}
                to={child.path}
                style={{
                  ...itemStyle,
                  color: childActive ? "var(--accent-primary)" : "var(--text-secondary)",
                  fontWeight: childActive ? 600 : 500,
                }}
                onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = "var(--bg-page)"; }}
                onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = "transparent"; }}
                onClick={() => setOpen(false)}
              >
                {child.label}
              </Link>
            );
          })}
        </div>
      )}
    </div>
  );
};

export default NavDropdown;
