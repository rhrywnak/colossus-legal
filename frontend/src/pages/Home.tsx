import React, { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import CaseHeader from "../components/CaseHeader";
import CaseSummaryCard from "../components/CaseSummaryCard";
import TimelineBand from "../components/TimelineBand";
import CountCard from "../components/CountCard";
import { CaseHeaderResponse, DEFAULT_CASE_SLUG, getCaseHeader } from "../services/caseHeader";
import { CountDetail, getCausesOfAction } from "../services/causesOfAction";
import { getCaseSummaryDoc } from "../services/caseSummaryDoc";
import {
  getProofMatrixRollup,
  indexAllegationTotals,
} from "../services/proofMatrix";

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

  // Per-Count plain-language descriptions, keyed by count_number (as a string),
  // from the static case-summary doc. Supplementary content for the Count cards:
  // the cards render fine without it (each shows no description line), so a load
  // failure here must NOT block or error the Causes of Action section.
  const [countDescriptions, setCountDescriptions] = useState<
    Record<string, string>
  >({});

  // Per-Count deduped allegation totals, keyed by count_number, from the
  // proof-matrix rollup endpoint. Supplementary content for the Count cards:
  // each card degrades to a muted `—` without it, so a load failure here must
  // NOT block or error the Causes of Action section.
  const [allegationTotals, setAllegationTotals] = useState<
    Record<number, number>
  >({});

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

  // Load the per-Count descriptions from the static case-summary doc.
  //
  // ## React Learning: a deliberately non-blocking fetch
  // Unlike the two fetches above (which gate the page / the CoA section), these
  // descriptions are supplementary — the Count cards degrade gracefully without
  // them. So on failure we do NOT setError/block; we log a contextual message
  // (Rule 1: the failure is observable, not swallowed) and leave the map empty,
  // and the cards simply omit their description line. The same file is also read
  // by CaseSummaryCard, which surfaces a visible card-level error if it is truly
  // broken — so the user still gets one clear error rather than two.
  useEffect(() => {
    let cancelled = false;

    getCaseSummaryDoc()
      .then((doc) => {
        if (!cancelled) setCountDescriptions(doc.count_descriptions);
      })
      .catch((err: unknown) => {
        const message =
          err instanceof Error ? err.message : "unknown error";
        // Observable, not silent — but non-fatal for this section.
        console.error(`Home: could not load Count descriptions — ${message}`);
      });

    return () => {
      cancelled = true;
    };
  }, []);

  // Load the per-Count deduped allegation totals from the proof-matrix rollup.
  //
  // ## React Learning: another deliberately non-blocking fetch
  // Like the descriptions above, these totals are supplementary — the Count
  // cards render a muted `—` without them. So on failure we do NOT setError or
  // block; we log a contextual message (Rule 1: the failure is observable, not
  // swallowed) and leave the map empty, and each card keeps its `—` placeholder.
  useEffect(() => {
    let cancelled = false;

    getProofMatrixRollup()
      .then((rollup) => {
        if (!cancelled) setAllegationTotals(indexAllegationTotals(rollup.counts));
      })
      .catch((err: unknown) => {
        const message = err instanceof Error ? err.message : "unknown error";
        // Observable, not silent — but non-fatal for this section.
        console.error(`Home: could not load allegation totals — ${message}`);
      });

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
          // 2-column grid matching the frozen PROD layout. `1fr 1fr` keeps the
          // two columns equal; 16px gap is the project's card-gap spacing step.
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(2, 1fr)",
              gap: "16px",
            }}
          >
            {counts.map((count) => (
              <CountCard
                key={count.count_number}
                count={count}
                // Look up by count_number (string) — the only stable id the
                // causes-of-action payload exposes (no count slug on the wire).
                description={countDescriptions[String(count.count_number)]}
                // Deduped allegation total from the proof-matrix rollup, keyed
                // by count_number; undefined while pending → card shows `—`.
                allegationTotal={allegationTotals[count.count_number]}
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
