import React, { useEffect, useState } from "react";
import CaseHeader from "../components/CaseHeader";
import { CaseHeaderResponse, getCaseHeader } from "../services/caseHeader";

// ─── Component ───────────────────────────────────────────────────────────────

/**
 * Home — the case dashboard.
 *
 * Phase 2C: the old title/stat blocks (which read the `/api/case-summary`
 * context) are gone. Per HOME_PAGE_REDESIGN_v2.md §5, the parties fold into the
 * CaseHeader and the standalone stat columns are removed — so the page is now
 * driven solely by GET /api/cases/:slug. The Causes of Action placeholder
 * (Phase 2B) stays until Phase 2D-E rebuild the Count tables.
 *
 * ## React Learning: fetch-on-mount with a cancel flag
 * We fetch in an effect and guard state updates with `cancelled`. If the
 * component unmounts before the request resolves (user navigates away), the
 * cleanup sets `cancelled = true` so we don't call setState on an unmounted
 * component. Same pattern as CaseContext — kept local here because this is the
 * only consumer of the case-header endpoint.
 */
const Home: React.FC = () => {
  const [header, setHeader] = useState<CaseHeaderResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

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
            err instanceof Error ? err.message : "Failed to load case header";
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

      {/* 2C: Causes of Action — temporary placeholder.
          The old 2x2 CountCard grid and the "Explore the Case" nav cards were
          removed in Phase 2B. The full-width Count tables arrive in Phase 2C-E.
          This block also doubles as a smoke test for Phase 2A's tokens: if the
          heading/body render in the wrong color (e.g. plain black), the
          tokens.css import is broken — var(--text-secondary)/var(--text-muted)
          should resolve to the palette defined in styles/tokens.css. */}
      <div style={{ padding: '32px 0' }}>
        <h2 style={{
          fontSize: '14px',
          fontWeight: 600,
          textTransform: 'uppercase' as const,
          letterSpacing: '0.05em',
          color: 'var(--text-secondary)',
          marginBottom: '16px'
        }}>
          Causes of Action
        </h2>
        <p style={{
          fontSize: '14px',
          color: 'var(--text-muted)'
        }}>
          Count cards are being rebuilt — coming in the next update.
        </p>
      </div>

    </div>
  );
};

export default Home;
