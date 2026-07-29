// =============================================================================
// NotFoundPage.tsx — the catch-all route for a URL that matches nothing.
// -----------------------------------------------------------------------------
// Before this page existed, an unmatched URL rendered the Header over an empty
// <main>: no message, no path, no way back. That is indistinguishable from a
// page that failed to load, which is the wrong diagnosis and sends the reader
// looking for a backend problem that isn't there. Retiring /analysis and
// /decomposition is what exposed it — every stale bookmark landed on the blank.
//
// The page states the outcome, echoes the path that missed (see notFoundHelpers
// for why, and for the shaping it needs), and offers the one action that always
// works. No fetch, no state — a 404 must not be able to fail.
// =============================================================================

import React from "react";
import { Link, useLocation } from "react-router-dom";

import { formatUnknownPath } from "./notFoundHelpers";

const NotFoundPage: React.FC = () => {
  // `useLocation` rather than `window.location`: this is a client-side route
  // miss, so the router's location is the authoritative one — and reading it
  // through the router keeps the component re-rendering correctly if a later
  // navigation lands on another unmatched path without unmounting the page.
  const { pathname } = useLocation();

  return (
    <div style={S.wrap}>
      <div style={S.code}>404</div>
      <h2 style={S.title}>Page not found</h2>
      <p style={S.body}>
        Nothing in this app answers to that address. It is usually a typo, a
        bookmark from a page that has since been retired, or a link that was
        never finished.
      </p>
      <p style={S.pathLabel}>You asked for:</p>
      <code style={S.path}>{formatUnknownPath(pathname)}</code>
      <div style={S.actions}>
        <Link to="/" style={S.homeLink}>
          Go to the dashboard
        </Link>
      </div>
    </div>
  );
};

// ─── styling (tokens.css only) ────────────────────────────────────────────────

const S: Record<string, React.CSSProperties> = {
  wrap: {
    maxWidth: "640px",
    margin: "0 auto",
    padding: "72px 0",
    textAlign: "center",
    fontFamily: "var(--font-sans)",
  },
  code: {
    fontSize: "3.5rem",
    fontWeight: 700,
    lineHeight: 1,
    color: "var(--text-muted)",
    letterSpacing: "0.02em",
  },
  title: {
    margin: "12px 0 0",
    fontSize: "1.5rem",
    fontWeight: 600,
    color: "var(--text-primary)",
  },
  body: {
    margin: "12px auto 24px",
    maxWidth: "48ch",
    fontSize: "0.95rem",
    lineHeight: 1.6,
    color: "var(--text-secondary)",
  },
  pathLabel: {
    margin: "0 0 6px",
    fontSize: "0.72rem",
    fontWeight: 600,
    textTransform: "uppercase",
    letterSpacing: "0.04em",
    color: "var(--text-muted)",
  },
  path: {
    display: "inline-block",
    maxWidth: "100%",
    padding: "8px 12px",
    background: "var(--bg-surface)",
    border: "1px solid var(--border-default)",
    borderRadius: "8px",
    fontFamily: "var(--font-mono)",
    fontSize: "0.85rem",
    color: "var(--text-primary)",
    // A long path wraps inside the box instead of pushing the layout wider; the
    // helper already caps the length, so this only handles a narrow viewport.
    overflowWrap: "anywhere",
  },
  actions: { marginTop: "28px" },
  homeLink: {
    display: "inline-block",
    padding: "10px 18px",
    background: "var(--accent-primary)",
    borderRadius: "8px",
    color: "var(--bg-surface)",
    fontSize: "0.9rem",
    fontWeight: 600,
    textDecoration: "none",
  },
};

export default NotFoundPage;
