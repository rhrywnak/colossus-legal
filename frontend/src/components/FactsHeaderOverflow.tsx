// =============================================================================
// FactsHeaderOverflow — the ⋯ the section's rare, destructive action moved into
// =============================================================================
//
// Mockup: a circular `⋯` pill between History and the fold chevron.
//
// ## Why Reset order is behind it now
//
// It sat on the header row, in the same visual weight as `Scan again` and
// `history`, one control away from the fold. It forgets where a human has placed
// every fact in the scenario — weeks of curation, and "the sequence IS the
// argument" — and `FactsResetOrder` already asks before it runs. A confirmation
// is protection against a click somebody meant; the overflow is protection
// against one they did not.
//
// ## Why it ships holding one item
//
// The mockup draws it, and the reason it exists is that the action inside is
// dangerous — which is true with one item in it. A menu that appears only once
// a second item exists would leave Reset order on the row for exactly as long as
// nothing else needed hiding.

import React, { useEffect, useRef, useState } from "react";

/** The circular pill. Matches the others' hairline and fill. */
const buttonStyle: React.CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  justifyContent: "center",
  width: "30px",
  height: "30px",
  borderRadius: "999px",
  border: "1px solid var(--border-default)",
  background: "var(--bg-surface)",
  color: "var(--text-secondary)",
  fontFamily: "inherit",
  fontSize: "13px",
  cursor: "pointer",
  flexShrink: 0,
  padding: 0,
};

const menuStyle: React.CSSProperties = {
  position: "absolute",
  top: "calc(100% + 6px)",
  right: 0,
  minWidth: "160px",
  padding: "4px",
  borderRadius: "10px",
  border: "1px solid var(--border-default)",
  background: "var(--bg-surface)",
  boxShadow: "var(--shadow-raised)",
  zIndex: 20,
};

const itemStyle: React.CSSProperties = {
  display: "block",
  width: "100%",
  textAlign: "left",
  fontFamily: "inherit",
  fontSize: "13px",
  padding: "8px 10px",
  borderRadius: "7px",
  border: "none",
  background: "none",
  color: "var(--text-primary)",
  cursor: "pointer",
};

interface Props {
  /** The ⋯ button's accessible name — a stored row, because a glyph has none. */
  label: string;
  /**
   * Reset order's stored label, or `null` until the grammar loads.
   *
   * When it is `null` the menu holds nothing and the ⋯ is not rendered at all.
   * That is ruling R4 unchanged: a destructive control that cannot state what it
   * does must not be offered — and an overflow that opens onto an empty box is
   * worse than no overflow, because it reads as a fault.
   */
  resetOrderLabel: string | null;
  onResetOrder: () => void;
}

/**
 * The overflow menu.
 *
 * ## Why it closes on an outside click, and how
 *
 * A menu that stays open once the human has moved on sits over the cards
 * beneath it. The listener is attached only while the menu is OPEN and removed
 * by the effect's cleanup, so a page with a closed menu carries no document
 * handler — and `mousedown` rather than `click`, so the menu is gone before the
 * thing underneath receives the press.
 */
const FactsHeaderOverflow: React.FC<Props> = ({ label, resetOrderLabel, onResetOrder }) => {
  const [open, setOpen] = useState(false);
  const wrapper = useRef<HTMLSpanElement>(null);

  useEffect(() => {
    if (!open) return;
    const onOutside = (e: MouseEvent) => {
      if (!wrapper.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onOutside);
    return () => document.removeEventListener("mousedown", onOutside);
  }, [open]);

  if (resetOrderLabel === null) return null;

  return (
    <span
      ref={wrapper}
      style={{ position: "relative", display: "inline-flex" }}
      onClick={(e) => e.stopPropagation()}
    >
      <button
        type="button"
        style={buttonStyle}
        aria-label={label}
        title={label}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={(e) => {
          e.stopPropagation();
          setOpen(!open);
        }}
      >
        ⋯
      </button>

      {open && (
        <span style={menuStyle} role="menu">
          <button
            type="button"
            role="menuitem"
            style={itemStyle}
            onClick={(e) => {
              e.stopPropagation();
              setOpen(false);
              onResetOrder();
            }}
          >
            {resetOrderLabel}
          </button>
        </span>
      )}
    </span>
  );
};

export default FactsHeaderOverflow;
