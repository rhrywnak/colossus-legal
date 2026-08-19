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
//
// ## Keyboard and dismissal (nav cleanup, Part 2)
//
// A menu that only a mouse can open is a menu half the app cannot reach. The
// trigger is a real `<button>` and each leaf a real `<Link>`, so Tab reaches
// them and Enter/Space activates them without a single key handler — that part
// was already right. What was missing:
//
//   · **Esc** did nothing. A menu you can open and not close without moving the
//     mouse somewhere else is a trap for anybody navigating by keyboard.
//   · **Focus went nowhere on close.** Closing with Esc now returns focus to the
//     trigger, which is where the user was; without it focus falls to the top of
//     the document and the next Tab starts over from the logo.
//   · **Nothing announced the menu.** `aria-haspopup` / `aria-expanded` on the
//     trigger and `role="menu"` on the panel are what let a screen reader say
//     "Trial Prep, menu, collapsed" instead of just "Trial Prep, button".
//
// One level only, deliberately: a submenu inside a submenu is the shape this
// cleanup exists to remove.
// =============================================================================

import React, { useEffect, useRef, useState } from "react";
import { Link } from "react-router-dom";

import { isActivePath as isActive, type NavLeafItem } from "./navItems";

/**
 * One leaf link inside a nav group.
 *
 * Re-exported from the nav table rather than declared here: the table is now the
 * one place the menu's shape is defined, and a second structurally-identical
 * type beside it is how the two drift.
 */
export type NavLeaf = NavLeafItem;

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
  minWidth: "230px",
  backgroundColor: "var(--bg-surface)",
  borderRadius: "8px",
  border: "1px solid var(--border-default)",
  boxShadow: "0 2px 8px rgba(0,0,0,0.15)",
  zIndex: 200,
  overflow: "hidden",
};

/** The leaf's small second line — the mockup's one-line description. */
const detailStyle: React.CSSProperties = {
  display: "block",
  fontSize: "0.72rem",
  fontWeight: 400,
  color: "var(--text-disabled)",
  marginTop: "0.1rem",
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
  const triggerRef = useRef<HTMLButtonElement>(null);
  // A leaf path may carry a query string (`?tab=review`); the active test
  // compares PATHS, so it is stripped before the comparison. Without this the
  // Proof Matrix leaf could never highlight, because no `location.pathname`
  // contains a `?`.
  const groupActive = items.some((c) => isActive(c.path.split("?")[0], currentPath));

  // Close on click outside (same pattern as Header's user-account dropdown).
  useEffect(() => {
    if (!open) return;
    const handleClick = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [open]);

  // Close on Esc, and give focus back to the trigger.
  //
  // Bound to the DOCUMENT rather than to the panel: focus may be on the trigger,
  // on a leaf, or (after a mouse click that opened the menu) on neither, and a
  // handler attached to one element only closes the menu from some of those.
  useEffect(() => {
    if (!open) return;
    const handleKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      setOpen(false);
      triggerRef.current?.focus();
    };
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [open]);

  return (
    <div ref={ref} style={{ position: "relative" }}>
      <button
        ref={triggerRef}
        type="button"
        style={groupActive ? triggerActive : triggerBase}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((prev) => !prev)}
      >
        {label}
        <span style={{ fontSize: "0.6rem" }}>▾</span>
      </button>
      {open && (
        <div style={panelStyle} role="menu" aria-label={label}>
          {items.map((child) => {
            const childActive = isActive(child.path.split("?")[0], currentPath);
            return (
              <Link
                key={child.path}
                to={child.path}
                role="menuitem"
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
                {/* The mockup's small second line. Withdrawn entirely when a
                    leaf has none, rather than rendered empty — an empty element
                    still takes its line height, and the menu would show a gap
                    that reads as a label that failed to load. */}
                {child.detail !== undefined && (
                  <span style={detailStyle}>{child.detail}</span>
                )}
              </Link>
            );
          })}
        </div>
      )}
    </div>
  );
};

export default NavDropdown;
