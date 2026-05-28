import React, { useEffect, useState } from "react";
import CaseHeader from "../components/CaseHeader";
import CountCard from "../components/CountCard";
import ElementDetailPanel from "../components/ElementDetailPanel";
import { CaseHeaderResponse, DEFAULT_CASE_SLUG, getCaseHeader } from "../services/caseHeader";
import { CountDetail, getCausesOfAction } from "../services/causesOfAction";

/**
 * Identity of the Element row the user most recently clicked. The Home page
 * owns this state so the floating panel can render at the page root (outside
 * the table flow) and so clicking a *different* row updates the same panel
 * instance in place (no close/reopen flicker).
 *
 * The three pieces — id, name, count — are exactly what the panel needs to
 * render its header immediately, before its own /detail fetch resolves.
 */
type SelectedElement = {
  elementId: string;
  elementName: string;
  allegationCount: number;
};

// ─── Component ───────────────────────────────────────────────────────────────

/**
 * Home — the case dashboard.
 *
 * Two independent reads drive the page:
 *   - GET /api/cases/:slug                  → the CaseHeader (title, parties, counsel)
 *   - GET /api/cases/:slug/causes-of-action → the stacked CountCard tables
 *
 * The header fetch gates the page (loading/error/empty early-returns); the
 * causes-of-action fetch renders its own loading/error state inside the section,
 * so a slow Count query never blanks the whole page.
 *
 * ## React Learning: fetch-on-mount with a cancel flag
 * Each effect guards its setState calls with `cancelled`. If the component
 * unmounts before a request resolves (user navigates away), the cleanup sets
 * `cancelled = true` so we never setState on an unmounted component.
 */
const Home: React.FC = () => {
  const [header, setHeader] = useState<CaseHeaderResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [counts, setCounts] = useState<CountDetail[] | null>(null);
  const [coaLoading, setCoaLoading] = useState(true);
  const [coaError, setCoaError] = useState<string | null>(null);

  // The Element row currently driving the floating detail panel. `null` =
  // the panel is closed. Clicking a different row mutates this in place; the
  // panel re-fetches against the new id (see ElementDetailPanel useEffect).
  const [selectedElement, setSelectedElement] = useState<SelectedElement | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      try {
        const data = await getCaseHeader();
        if (!cancelled) {
          setHeader(data);
          setLoading(false);
        }
      } catch (err) {
        // No silent failure (Rule 1): surface the message to the user below.
        if (!cancelled) {
          const message =
            err instanceof Error
              ? err.message
              : "Failed to load case header. Try reloading the page.";
          setError(message);
          setLoading(false);
        }
      }
    }

    load();

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      try {
        const data = await getCausesOfAction();
        if (!cancelled) {
          setCounts(data.counts);
          setCoaLoading(false);
        }
      } catch (err) {
        if (!cancelled) {
          const message =
            err instanceof Error
              ? err.message
              : "Failed to load causes of action. Try reloading the page.";
          setCoaError(message);
          setCoaLoading(false);
        }
      }
    }

    load();

    return () => {
      cancelled = true;
    };
  }, []);

  if (loading) {
    return (
      <div style={{ padding: "32px", textAlign: "center", color: "var(--text-secondary)" }}>
        Loading case data...
      </div>
    );
  }

  if (error) {
    return (
      <div
        style={{
          margin: "32px",
          padding: "16px",
          backgroundColor: "var(--bg-surface)",
          border: "1px solid var(--border-default)",
          borderRadius: "6px",
          color: "var(--status-dropped-text)",
        }}
      >
        {error}
      </div>
    );
  }

  // Defensive: after a successful load `header` is set; this guards the
  // narrow window where it is somehow still null without an error.
  if (!header) {
    return (
      <div style={{ padding: "32px", textAlign: "center", color: "var(--text-muted)" }}>
        No case data available.
      </div>
    );
  }

  return (
    <div style={{ paddingTop: "32px", paddingBottom: "4rem" }}>
      {/* 2A (rebuilt): the full case header — title, court strip, parties, counsel */}
      <CaseHeader data={header} />

      {/* 2C (rebuilt): Causes of Action — full-width Count tables (Phase 2D),
          replacing the Phase 2B placeholder. The header fetch above has
          already resolved; this section manages its own loading/error so a
          slow Count query doesn't blank the header. */}
      <section style={{ paddingTop: "32px" }}>
        <h2 className="h2-section-header" style={{ marginBottom: "16px" }}>
          Causes of Action
        </h2>
        {coaLoading ? (
          <div style={{ color: "var(--text-secondary)", fontSize: "14px" }}>
            Loading causes of action...
          </div>
        ) : coaError ? (
          <div style={{ color: "var(--status-dropped-text)", fontSize: "14px" }}>
            {coaError}
          </div>
        ) : !counts || counts.length === 0 ? (
          <div style={{ color: "var(--text-muted)", fontSize: "14px" }}>
            No Counts loaded for this case.
          </div>
        ) : (
          // 32px gap between stacked cards (§7).
          <div style={{ display: "flex", flexDirection: "column", gap: "32px" }}>
            {counts.map((count) => (
              <CountCard
                key={count.count_number}
                count={count}
                onElementClick={(elementId, elementName, allegationCount) =>
                  setSelectedElement({ elementId, elementName, allegationCount })
                }
              />
            ))}
          </div>
        )}
      </section>

      {/* Floating Element detail panel (E2). Conditionally rendered at the
          page root so it sits above the table flow. A click on a different
          Element row updates `selectedElement` and the panel re-fetches in
          place — no close/reopen needed. */}
      {selectedElement && (
        <ElementDetailPanel
          caseSlug={DEFAULT_CASE_SLUG}
          elementId={selectedElement.elementId}
          elementName={selectedElement.elementName}
          allegationCount={selectedElement.allegationCount}
          onClose={() => setSelectedElement(null)}
        />
      )}
    </div>
  );
};

export default Home;
