// =============================================================================
// PageTabs.tsx — tabs that are an ADDRESS, not a piece of component state
// =============================================================================
//
// ## Why a shared component, and why the tab lives in the URL
//
// Proof Review stops being its own route and becomes a tab on the Proof Matrix
// (design §2). The naive version holds the active tab in `useState`, and three
// things break the moment it does: the review half cannot be bookmarked, Back
// does not return to it, and the redirect from the old `/…/proof-review`
// address has nowhere exact to land.
//
// So the tab is a query parameter. `?tab=review` IS the review page's new
// address, `useSearchParams` reads and writes it, and the browser's history
// stack gets an entry per switch — which is what makes Back behave the way a
// person expects after clicking a tab.
//
// ## Rust Learning: this is `#[serde(tag = "…")]`, in a URL
//
// The pattern is the same one the backend uses for a tagged enum: one field
// carries which variant this is, and the reader matches on it. Here the tag is
// a query parameter and the "variant" is which panel renders. The important
// property is shared — an unknown tag falls back to a known default rather than
// rendering nothing, because a URL is user input and users type.

import React from "react";
import { useSearchParams } from "react-router-dom";

/** One tab: the value that appears in `?tab=`, and its label. */
export interface PageTab {
  id: string;
  label: string;
}

/**
 * The query parameter the tab is carried in.
 *
 * A constant rather than the literal `"tab"` in four places: this string is a
 * contract between the tab bar, the redirect in `App.tsx` and the guard test,
 * and three copies of it is how one of them ends up spelled differently.
 */
export const TAB_PARAM = "tab";

/**
 * Which tab is active, from the URL.
 *
 * An unrecognised `?tab=` value yields the FIRST tab rather than an error or a
 * blank page: the URL is something a person can type or a stale bookmark can
 * carry, and "the tab you asked for does not exist" is not worth a screen when
 * the sensible thing is to show the page's default half.
 */
export function activeTab(tabs: PageTab[], search: URLSearchParams): string {
  const asked = search.get(TAB_PARAM);
  return tabs.some((t) => t.id === asked) && asked !== null ? asked : tabs[0].id;
}

const barStyle: React.CSSProperties = {
  display: "flex",
  gap: "0.2rem",
  marginBottom: "1.5rem",
  borderBottom: "2px solid var(--border-default)",
};

const tabBase: React.CSSProperties = {
  padding: "0.5rem 0.9rem",
  fontSize: "0.86rem",
  fontWeight: 500,
  color: "var(--text-muted)",
  background: "none",
  border: "none",
  borderBottom: "2px solid transparent",
  marginBottom: "-2px",
  cursor: "pointer",
  fontFamily: "inherit",
};

const tabActive: React.CSSProperties = {
  ...tabBase,
  color: "var(--accent-primary)",
  borderBottomColor: "var(--accent-primary)",
  fontWeight: 600,
};

/**
 * The tab bar. Renders nothing when there is one tab — a lone tab is a control
 * that cannot do anything.
 */
const PageTabs: React.FC<{ tabs: PageTab[] }> = ({ tabs }) => {
  const [search, setSearch] = useSearchParams();
  const active = activeTab(tabs, search);

  if (tabs.length < 2) return null;

  return (
    <div style={barStyle} role="tablist">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          type="button"
          role="tab"
          aria-selected={active === tab.id}
          style={active === tab.id ? tabActive : tabBase}
          onClick={() => {
            // The FIRST tab clears the parameter instead of writing
            // `?tab=matrix`. The canonical address of the matrix is
            // `/…/proof-matrix`, and a page that rewrites its own URL the moment
            // you look at it makes every bookmark taken from the address bar
            // carry a parameter that means "the default".
            const next = new URLSearchParams(search);
            if (tab.id === tabs[0].id) next.delete(TAB_PARAM);
            else next.set(TAB_PARAM, tab.id);
            setSearch(next);
          }}
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
};

export default PageTabs;
