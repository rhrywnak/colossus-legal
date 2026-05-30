// =============================================================================
// CountDetailPage.tsx — routed per-Count detail page (Stage 1-1)
// -----------------------------------------------------------------------------
// Route: /cases/:slug/counts/:countId  (countId = the Count's count_number)
//
// The full-page replacement for the floating Element detail popup, and the
// layout template for the later Proof Matrix page. Layout: the Count's Elements
// listed at the top; clicking an Element shows its detail below (mapped
// allegations grouped Common / Dedicated / Other, paragraph-sorted, with the
// auto-saved review-notes editor — see ElementDetailContent).
//
// Data path (both halves are existing, BEARS_ON-based endpoints):
//   - The Count + its Elements (with per-Element allegation_count) come from
//     GET /api/cases/:slug/causes-of-action (we select the matching count_number).
//   - Each Element's mapped allegations come lazily from
//     GET /api/cases/:slug/elements/:id/detail (inside ElementDetailContent).
//
// Per-Element allegation counts are real and BEARS_ON-derived. There is NO
// Count-level total here by design — that number is the Proof Matrix's job once
// it exists (decided at Stage 1-1 §4); we do not sum per-Element counts in the
// frontend (would double-count and is business logic).
// =============================================================================

import React, { useEffect, useMemo, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import Breadcrumb from "../components/Breadcrumb";
import BurdenBadge from "../components/BurdenBadge";
import AuthorityPopover from "../components/AuthorityPopover";
import ElementDetailContent from "../components/ElementDetailContent";
import {
  toRomanNumeral,
  formatElementNumber,
  sortElements,
} from "../components/CountCard";
import { CountDetail, getCausesOfAction } from "../services/causesOfAction";
import { DEFAULT_CASE_SLUG } from "../services/caseHeader";

const CountDetailPage: React.FC = () => {
  const { slug: slugParam, countId } = useParams<{
    slug: string;
    countId: string;
  }>();
  const slug = slugParam ?? DEFAULT_CASE_SLUG;
  const navigate = useNavigate();

  // Smart back: prefer history; fall back to Home for a direct/bookmarked load
  // (same idiom as AllegationDetailPage).
  const goBack = () => {
    const idx = (window.history.state as { idx?: number } | null)?.idx ?? 0;
    if (idx > 0) navigate(-1);
    else navigate("/");
  };

  const [count, setCount] = useState<CountDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notFound, setNotFound] = useState(false);
  const [selectedElementId, setSelectedElementId] = useState<string | null>(null);

  // ── Fetch the Count (select it from the causes-of-action payload) ────────
  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    setNotFound(false);

    getCausesOfAction(slug)
      .then((data) => {
        if (cancelled) return;
        const wanted = Number(countId);
        const match = data.counts.find((c) => c.count_number === wanted) ?? null;
        if (!match) {
          setNotFound(true);
        } else {
          setCount(match);
        }
        setLoading(false);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setError(
          err instanceof Error
            ? err.message
            : "Failed to load the Count. Try reloading the page.",
        );
        setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [slug, countId]);

  // Elements sorted for display (order_in_count asc, name tiebreak).
  const elements = useMemo(
    () => (count ? sortElements(count.elements) : []),
    [count],
  );

  // Default the selected Element to the first one once the Count loads, and
  // re-default if the selected id is no longer present (e.g. Count changed).
  useEffect(() => {
    if (elements.length === 0) {
      setSelectedElementId(null);
      return;
    }
    setSelectedElementId((prev) =>
      prev && elements.some((e) => e.element_id === prev)
        ? prev
        : elements[0].element_id,
    );
  }, [elements]);

  // ── Loading / not-found / error ──────────────────────────────────────────
  if (loading) {
    return (
      <div style={{ padding: "2rem", textAlign: "center", color: "var(--text-muted)" }}>
        Loading Count detail...
      </div>
    );
  }

  if (notFound) {
    return (
      <div style={{ padding: "1rem", maxWidth: "1000px" }}>
        <p style={{ color: "var(--text-muted)" }}>
          Count {countId} not found for this case.
        </p>
        <button type="button" onClick={goBack} style={BACK_BTN_STYLE}>
          &larr; Back
        </button>
      </div>
    );
  }

  if (error) {
    return (
      <div
        style={{
          padding: "1rem",
          maxWidth: "1000px",
          backgroundColor: "var(--state-danger-bg-soft)",
          border: "1px solid var(--state-danger-border)",
          borderRadius: "6px",
          color: "var(--state-danger-strong)",
        }}
      >
        {error}
        <div style={{ marginTop: "0.5rem" }}>
          <button type="button" onClick={goBack} style={BACK_BTN_STYLE}>
            &larr; Back
          </button>
        </div>
      </div>
    );
  }

  if (!count) {
    return <div style={{ padding: "1rem" }}>No Count data available.</div>;
  }

  const roman = toRomanNumeral(count.count_number);
  const title = count.count_name
    ? `COUNT ${roman} — ${count.count_name}`
    : `COUNT ${roman}`;
  const burden = count.burden_of_proof?.trim() ? count.burden_of_proof : null;
  const primaryAuthority = count.controlling_authority_primary?.trim()
    ? count.controlling_authority_primary
    : null;

  const selected = elements.find((e) => e.element_id === selectedElementId) ?? null;

  // ── Render ────────────────────────────────────────────────────────────────
  return (
    <div style={{ maxWidth: "1000px", paddingBottom: "4rem" }}>
      <Breadcrumb
        items={[
          { label: "Dashboard", to: "/" },
          { label: `Count ${roman}` },
        ]}
      />

      {/* Count header */}
      <div style={{ marginBottom: "1.25rem" }}>
        <h1 className="count-header" style={{ margin: 0 }}>{title}</h1>
        <div
          className="burden-strip"
          style={{
            marginTop: "6px",
            display: "flex",
            alignItems: "center",
            gap: "6px",
            flexWrap: "wrap",
          }}
        >
          <span>Burden:</span>
          {burden ? <BurdenBadge burden={burden} /> : <span>—</span>}
          {primaryAuthority && (
            <>
              <span>· {primaryAuthority}</span>
              <AuthorityPopover authorities={count.controlling_authorities} />
            </>
          )}
        </div>
      </div>

      {/* Elements list (fixed at the top) */}
      <div style={CARD_STYLE}>
        <div style={LIST_HEADER_STYLE}>Elements</div>
        {elements.length === 0 ? (
          <div style={{ color: "var(--text-muted)", fontSize: "14px" }}>
            No Elements loaded for this Count. Run the canonical Element loader.
          </div>
        ) : (
          <div role="tablist" aria-label="Elements of this Count">
            {elements.map((el, i) => {
              const isSelected = el.element_id === selectedElementId;
              return (
                <div
                  key={el.element_id}
                  role="tab"
                  tabIndex={0}
                  aria-selected={isSelected}
                  onClick={() => setSelectedElementId(el.element_id)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      setSelectedElementId(el.element_id);
                    }
                  }}
                  style={{
                    ...ELEMENT_ROW_STYLE,
                    backgroundColor: isSelected
                      ? "var(--accent-bg-soft)"
                      : "transparent",
                    borderLeft: isSelected
                      ? "3px solid var(--accent-primary)"
                      : "3px solid transparent",
                  }}
                >
                  <span style={ELEMENT_NUMBER_STYLE}>
                    {formatElementNumber(
                      count.count_number,
                      el.order_in_count ?? i + 1,
                    )}
                  </span>
                  <span style={ELEMENT_NAME_STYLE}>{el.element_name}</span>
                  <span style={el.allegation_count > 0 ? BADGE_STYLE : ZERO_BADGE_STYLE}>
                    {el.allegation_count}
                  </span>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* Selected Element's detail (below) */}
      {selected && (
        <div style={{ ...CARD_STYLE, marginTop: "20px" }}>
          <div style={LIST_HEADER_STYLE}>
            {formatElementNumber(
              count.count_number,
              selected.order_in_count ??
                elements.findIndex((e) => e.element_id === selected.element_id) + 1,
            )}{" "}
            — {selected.element_name}
          </div>
          <ElementDetailContent
            caseSlug={slug}
            elementId={selected.element_id}
            elementName={selected.element_name}
          />
        </div>
      )}
    </div>
  );
};

// ─── Styles ───────────────────────────────────────────────────────────────────

const BACK_BTN_STYLE: React.CSSProperties = {
  color: "var(--accent-primary)",
  textDecoration: "none",
  fontSize: "0.9rem",
  background: "none",
  border: "none",
  padding: 0,
  cursor: "pointer",
  fontFamily: "inherit",
};

const CARD_STYLE: React.CSSProperties = {
  border: "1px solid var(--border-default)",
  backgroundColor: "var(--bg-surface)",
  borderRadius: "8px",
  padding: "20px 24px",
};

const LIST_HEADER_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "12px",
  fontWeight: 600,
  letterSpacing: "0.05em",
  textTransform: "uppercase",
  color: "var(--text-secondary)",
  marginBottom: "8px",
};

const ELEMENT_ROW_STYLE: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "12px",
  padding: "10px 12px",
  cursor: "pointer",
  borderRadius: "6px",
};

const ELEMENT_NUMBER_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: "13px",
  fontWeight: 700,
  color: "var(--text-secondary)",
  minWidth: "32px",
};

const ELEMENT_NAME_STYLE: React.CSSProperties = {
  flex: 1,
  fontFamily: "var(--font-sans)",
  fontSize: "14px",
  fontWeight: 500,
  color: "var(--text-primary)",
};

const BADGE_STYLE: React.CSSProperties = {
  display: "inline-block",
  padding: "2px 10px",
  borderRadius: "12px",
  backgroundColor: "var(--accent-bg-soft)",
  color: "var(--accent-primary)",
  fontSize: "13px",
  fontWeight: 600,
};

const ZERO_BADGE_STYLE: React.CSSProperties = {
  color: "var(--text-muted)",
  fontSize: "13px",
};

export default CountDetailPage;
