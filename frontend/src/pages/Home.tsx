import React, { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import CaseHeader from "../components/CaseHeader";
import CaseSummaryCard from "../components/CaseSummaryCard";
import TimelineBand from "../components/TimelineBand";
import CountCard from "../components/CountCard";
import { CaseHeaderResponse, DEFAULT_CASE_SLUG, getCaseHeader } from "../services/caseHeader";
import { CountDetail, getCausesOfAction } from "../services/causesOfAction";

// ─── Component ───────────────────────────────────────────────────────────────

/**
 * Home — the case dashboard.
 *
 * Two independent reads drive the page:
 *   - GET /api/cases/:slug                  → the CaseHeader (title, parties, counsel)
 *                                              and the complaint_document_id used by
 *                                              the Case Summary card's "View Complaint".
 *   - GET /api/cases/:slug/causes-of-action → the stacked Count summary cards.
 *
 * Layout, top to bottom: CaseHeader → Case Summary card → Timeline band →
 * Causes of Action. The Case Summary card and Timeline band own their own
 * (static-file) fetches and render their own loading/error, so neither blocks
 * the rest of the page.
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
  const navigate = useNavigate();
  const [header, setHeader] = useState<CaseHeaderResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [counts, setCounts] = useState<CountDetail[] | null>(null);
  const [coaLoading, setCoaLoading] = useState(true);
  const [coaError, setCoaError] = useState<string | null>(null);

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
      {/* 1. Case header — title, court strip, parties, counsel */}
      <CaseHeader data={header} />

      {/* 2. Case Summary card — static prose + null-safe "View Complaint" link.
          The complaint id is resolved dynamically from the case-header payload. */}
      <div style={{ paddingTop: "32px" }}>
        <CaseSummaryCard complaintDocumentId={header.complaint_document_id} />
      </div>

      {/* 3. Timeline band — compact per-phase pills from /data/timeline.json */}
      <TimelineBand />

      {/* 4. Causes of Action — stacked Count summary cards. The header fetch
          above has resolved; this section manages its own loading/error so a
          slow Count query doesn't blank the page. Clicking a card navigates to
          the routed Count-detail page. */}
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
                onOpenCount={() =>
                  navigate(
                    `/cases/${encodeURIComponent(DEFAULT_CASE_SLUG)}/counts/${count.count_number}`,
                  )
                }
              />
            ))}
          </div>
        )}
      </section>
    </div>
  );
};

export default Home;
