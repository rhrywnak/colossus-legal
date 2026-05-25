// =============================================================================
// AuthorityPopover.tsx — the ⓘ popover of controlling authorities (§7/§9)
// -----------------------------------------------------------------------------
// Renders a small "ⓘ" button in the CountCard burden strip; clicking it opens a
// card listing every controlling authority for that Count (citation, type,
// role). Toggle by clicking ⓘ again; dismiss by clicking outside or pressing
// Escape.
//
// No popover library is used (the project has none) — this is a manual
// implementation: React state for open/closed, a useRef on the wrapper for
// click-outside detection, and a document keydown handler for Escape (rule 5).
//
// If there are no authorities, nothing renders (not even the trigger) — there
// would be nothing to show (Rule 1 / §4: don't offer an empty affordance).
// =============================================================================

import React, { useEffect, useRef, useState } from "react";
import { Authority } from "../services/causesOfAction";

/** Trigger glyph styling shared between idle/hover states. */
const TRIGGER_BASE: React.CSSProperties = {
  background: "none",
  border: "none",
  padding: 0,
  font: "inherit",
  lineHeight: 1,
  cursor: "pointer",
};

/**
 * AuthorityPopover — click ⓘ to view a Count's controlling authorities.
 *
 * @param authorities the Count's `controlling_authorities` (from the DTO)
 */
const AuthorityPopover: React.FC<{ authorities: Authority[] }> = ({ authorities }) => {
  const [open, setOpen] = useState(false);
  const [hovered, setHovered] = useState(false);
  const wrapperRef = useRef<HTMLSpanElement>(null);

  // While open, dismiss on an outside click or the Escape key. Listeners are
  // attached only when open and removed on close/unmount (no leaks).
  useEffect(() => {
    if (!open) return;

    function onMouseDown(e: MouseEvent) {
      if (wrapperRef.current && !wrapperRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") setOpen(false);
    }

    document.addEventListener("mousedown", onMouseDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("mousedown", onMouseDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open]);

  // Nothing to show → render no trigger at all (§4: no empty affordance).
  if (authorities.length === 0) return null;

  return (
    <span ref={wrapperRef} style={{ position: "relative", display: "inline-block" }}>
      <button
        type="button"
        aria-label="View controlling authorities"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
        style={{
          ...TRIGGER_BASE,
          color: open || hovered ? "var(--accent-primary)" : "var(--text-secondary)",
        }}
      >
        {"ⓘ"}
      </button>

      {open && (
        <div
          role="dialog"
          aria-label="Controlling authorities"
          style={{
            position: "absolute",
            top: "100%",
            left: 0,
            marginTop: "6px",
            zIndex: 1000,
            minWidth: "260px",
            maxWidth: "420px",
            backgroundColor: "var(--bg-surface)",
            border: "1px solid var(--border-default)",
            borderRadius: "8px",
            boxShadow: "0 4px 12px rgba(0,0,0,0.1)",
            padding: "16px",
          }}
        >
          {authorities.map((authority, i) => (
            <div key={`${authority.citation}-${i}`} style={{ marginTop: i > 0 ? "12px" : 0 }}>
              <div style={{ fontSize: "13px", fontWeight: 500, color: "var(--text-primary)" }}>
                {authority.citation}
              </div>
              <div style={{ fontSize: "12px", fontWeight: 400, color: "var(--text-secondary)", marginTop: "2px" }}>
                <span style={{ textTransform: "capitalize", color: "var(--text-muted)" }}>
                  {authority.authority_type.replace(/_/g, " ")}
                </span>
                {authority.role ? ` · ${authority.role}` : ""}
              </div>
            </div>
          ))}
        </div>
      )}
    </span>
  );
};

export default AuthorityPopover;
