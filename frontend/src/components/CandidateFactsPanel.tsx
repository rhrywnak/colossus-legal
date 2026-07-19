// =============================================================================
// CandidateFactsPanel — the scenario candidate workbench (Phase 1a.6).
// =============================================================================
//
// Cut over from the old bias-query finder to the gather workbench: it loads the
// full candidate pool (`GET …/facts/gather` — every Evidence node ABOUT the
// scenario's subject, each with its three-state status), narrows it with a
// client-side STATUS filter (default `undecided`), and lets a human rule on each
// candidate (include / drop / un-drop via `POST …/facts/:id/action`).
//
// One gather-driven, status-grouped list is the whole surface (the 1a.6 merge):
// the old separate "Curated facts" saved-list is now just the `included` filter.
// The orphan guarantee is preserved separately — see the `listScenarioFacts`
// call and `findOrphans` below.
//
// The subject is resolved SERVER-SIDE by the gather endpoint (definition.target
// → case default), so this panel no longer seeds filters from the definition and
// no case-specific name is hardcoded here (Standing Rule 2).

import React, { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";

import {
  applyFactAction,
  gatherCandidates,
  type CandidateDto,
  type FactAction,
} from "../services/scenarioGather";
import { listScenarioFacts, type ScenarioFactDto } from "../services/scenarioFacts";
import { API_BASE_URL } from "../services/api";
import PdfViewer from "./shared/PdfViewer";
import EvidenceCard, { type ViewPdfTarget } from "../pages/BiasExplorer/EvidenceCard";
import type { ScenarioDefinition } from "../pages/trialPrepData";
import {
  ACTION_LABEL,
  actionsForStatus,
  countByStatus,
  filterByStatus,
  findOrphans,
  orphansVisibleUnder,
  roleConfidenceLabel,
  candidateChip,
  STATUS_FILTERS,
  STATUS_FILTER_LABEL,
  type StatusFilter,
} from "./candidateWorkbench";

interface Props {
  slug: string;
  scenarioId: string;
  /** @deprecated (1a.6) Vestigial. The gather endpoint resolves the subject
   *  server-side (definition.target → case default), so the panel no longer
   *  seeds filters from the definition. Kept on the signature only to preserve
   *  the mount contract; a follow-up tidy chunk removes it (and `candidateSeed`).
   */
  definition?: ScenarioDefinition;
  /** Bumped by the parent when something OUTSIDE this panel changed the candidate
   *  facts — today, a Merge selected in the Theme Scan panel. Any change to this
   *  value re-fetches the pool, so a merged judgment strip appears immediately
   *  instead of waiting for a manual collapse/expand or a page reload. */
  externalRefresh?: number;
}

const toggleStyle: React.CSSProperties = {
  background: "var(--bg-surface)",
  border: "1px solid var(--border-default)",
  borderRadius: "6px",
  padding: "0.5rem 0.9rem",
  fontSize: "0.85rem",
  fontWeight: 600,
  color: "var(--text-primary)",
  cursor: "pointer",
};

const panelStyle: React.CSSProperties = {
  marginTop: "0.75rem",
  border: "1px solid var(--border-default)",
  borderRadius: "8px",
  padding: "0.9rem",
  backgroundColor: "var(--bg-page)",
};

const controlRow: React.CSSProperties = {
  display: "flex",
  flexWrap: "wrap",
  alignItems: "center",
  gap: "0.6rem",
  marginBottom: "0.75rem",
};

const selectStyle: React.CSSProperties = {
  padding: "0.35rem 0.5rem",
  fontSize: "0.82rem",
  border: "1px solid var(--border-default)",
  borderRadius: "5px",
  backgroundColor: "var(--bg-surface)",
  color: "var(--text-primary)",
};

const messageStyle: React.CSSProperties = {
  fontSize: "0.84rem",
  color: "var(--text-muted)",
  padding: "0.5rem 0",
};

// Small, non-blocking "Updating…" marker shown during a post-ruling refresh. It
// sits above the (still-mounted) list rather than replacing it, so the refresh is
// observable without resetting scroll (Bug 1). Muted + italic so it recedes.
const refreshNoteStyle: React.CSSProperties = {
  fontSize: "0.76rem",
  fontStyle: "italic",
  color: "var(--text-muted)",
  padding: "0 0 0.4rem",
};

const errorStyle: React.CSSProperties = {
  margin: "0.5rem 0",
  padding: "0.6rem 0.8rem",
  backgroundColor: "var(--state-danger-bg-soft)",
  border: "1px solid var(--state-danger-border)",
  borderRadius: "6px",
  color: "var(--state-danger-strong)",
  fontSize: "0.82rem",
};

const staleCardStyle: React.CSSProperties = {
  backgroundColor: "var(--bg-surface)",
  border: "1px dashed var(--state-warning-strong)",
  borderRadius: "8px",
  padding: "0.85rem 1rem",
  fontSize: "0.82rem",
  color: "var(--text-secondary)",
};

// One button style per ruling. `include` reads as the affirmative accent; `drop`
// as the danger exclusion; `undrop` as a neutral recovery.
const actionBtnStyle: Record<FactAction, React.CSSProperties> = {
  include: {
    padding: "0.2rem 0.6rem",
    fontSize: "0.74rem",
    fontWeight: 600,
    border: "1px solid var(--accent-primary)",
    borderRadius: "5px",
    backgroundColor: "var(--accent-bg-soft)",
    color: "var(--accent-primary)",
    cursor: "pointer",
  },
  drop: {
    padding: "0.2rem 0.6rem",
    fontSize: "0.74rem",
    fontWeight: 600,
    border: "1px solid var(--state-danger-border)",
    borderRadius: "5px",
    backgroundColor: "var(--state-danger-bg-soft)",
    color: "var(--state-danger-strong)",
    cursor: "pointer",
  },
  undrop: {
    padding: "0.2rem 0.6rem",
    fontSize: "0.74rem",
    fontWeight: 600,
    border: "1px solid var(--border-default)",
    borderRadius: "5px",
    backgroundColor: "var(--bg-surface)",
    color: "var(--text-primary)",
    cursor: "pointer",
  },
};

// Status summary row — replicates the Document → Review screen's count line
// (ReviewPanel.tsx) by construction: same markup shape, same theme tokens, so
// the two read identically without extracting a shared component (rule of three:
// two consumers, one shipped, does not justify extraction). Token mapping:
// undecided → Review's neutral "pending", included → its green "approved",
// dropped → its red "rejected". No hex (Rule 11); tokens only (Rule 2).
const summaryRowStyle: React.CSSProperties = {
  display: "flex",
  gap: "0.75rem",
  alignItems: "center",
  marginBottom: "0.5rem",
  flexWrap: "wrap",
};

const undecidedCountStyle: React.CSSProperties = {
  fontSize: "0.76rem",
  color: "var(--text-secondary)",
  fontWeight: 600,
};

const includedCountStyle: React.CSSProperties = {
  fontSize: "0.76rem",
  color: "var(--status-active-text)",
};

const droppedCountStyle: React.CSSProperties = {
  fontSize: "0.76rem",
  color: "var(--status-dropped-text)",
};

// The "x of y" count beside the filter dropdown — matches ReviewPanel's
// `{filtered.length} / {items.length}` span (tokens + size), worded "of".
const xOfYStyle: React.CSSProperties = {
  fontSize: "0.72rem",
  color: "var(--text-muted)",
  alignSelf: "center",
};

// Bounded scroll window for the candidate list. Keeps the flex-column layout and
// row gap the list had before, plus a capped height so it scrolls internally
// instead of owning the page. `minHeight` keeps a usable window on short viewports;
// the small right padding stops the scrollbar overlapping the card borders.
const scrollRegionStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: "0.6rem",
  maxHeight: "60vh",
  minHeight: "360px",
  overflowY: "auto",
  paddingRight: "0.4rem",
};

// Bug 2 — the list ↔ viewer split. The list column and the PDF side-panel sit in
// this flex row so the reviewer compares a card against its source WITHOUT losing
// their place (no navigation, no modal that hides the list). `alignItems:
// stretch` lets the viewer column match the list's height; `minWidth: 0` on both
// columns is the classic flexbox fix that lets a flex child actually shrink (its
// default `min-width: auto` would otherwise keep the PDF from narrowing).
const bodyRowStyle: React.CSSProperties = {
  display: "flex",
  gap: "0.75rem",
  alignItems: "stretch",
};

const listColStyle: React.CSSProperties = {
  flex: 1,
  minWidth: 0,
};

// The right-hand PDF panel. Fixed to the same 60vh the list scrolls within, so
// the two columns align; `PdfViewer` fills this height (its root is `height:
// 100%`). Bordered + clipped so the viewer's own toolbar/scroll stay inside.
const viewerColStyle: React.CSSProperties = {
  flex: 1,
  minWidth: 0,
  display: "flex",
  flexDirection: "column",
  height: "60vh",
  border: "1px solid var(--border-default)",
  borderRadius: "8px",
  overflow: "hidden",
  backgroundColor: "var(--bg-surface)",
};

const viewerHeaderStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "0.5rem",
  padding: "0.4rem 0.6rem",
  borderBottom: "1px solid var(--border-default)",
};

const viewerTitleStyle: React.CSSProperties = {
  flex: 1,
  minWidth: 0,
  fontSize: "0.8rem",
  fontWeight: 600,
  color: "var(--text-secondary)",
  overflow: "hidden",
  textOverflow: "ellipsis",
  whiteSpace: "nowrap",
};

const viewerCloseStyle: React.CSSProperties = {
  padding: "0.2rem 0.6rem",
  fontSize: "0.74rem",
  fontWeight: 600,
  border: "1px solid var(--border-default)",
  borderRadius: "5px",
  backgroundColor: "var(--bg-page)",
  color: "var(--text-primary)",
  cursor: "pointer",
};

// `minHeight: 0` lets this flex child shrink below its content so the viewer's
// own inner scroll (not the page) owns the overflow — the flexbox counterpart to
// the `minWidth: 0` on the columns.
const viewerFrameStyle: React.CSSProperties = {
  flex: 1,
  minHeight: 0,
};

// The lead strip that now renders INSIDE each candidate card (via EvidenceCard's
// `leadBadge` slot), so the machine's judgment is bound to the fact it judges
// (§2 — fixes the old panel-side orphaned badge). Holds the id chip and, for a
// scored candidate only, the "role · NN%" badge. A row so the two sit side by side.
const leadStripStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "0.4rem",
};

// The "role · NN%" judgment badge — neutral, accent-primary text + border, echoing
// the Theme Scan panel's `roleBadge` (§3). Per the ratified decision we do NOT
// invent a per-role color map; one neutral badge for every role. Rendered ONLY for
// scored candidates — its ABSENCE is the "human-added / unscored" signal (§2), so
// there is no longer any "unscored" text marker.
const roleBadgeStyle: React.CSSProperties = {
  fontSize: "0.72rem",
  fontWeight: 600,
  color: "var(--accent-primary)",
  background: "var(--bg-page)",
  border: "1px solid var(--border-default)",
  borderRadius: "6px",
  padding: "2px 8px",
  whiteSpace: "nowrap",
};

// The stable per-card id chip (§4) — a short monospace reference handle (`#a3f9k2`)
// derived from the fact's durable `evidence_id`. A `<button>` so it is copy-to-
// clipboard on click; the full id rides in its `title` for hover. Muted so it reads
// as metadata, not an action.
const idChipStyle: React.CSSProperties = {
  fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
  fontSize: "0.7rem",
  fontWeight: 600,
  color: "var(--text-muted)",
  background: "var(--bg-page)",
  border: "1px solid var(--border-default)",
  borderRadius: "5px",
  padding: "1px 6px",
  cursor: "pointer",
  whiteSpace: "nowrap",
};

// Inline copy-outcome notices beside the chip — a success tick and an explicit
// failure note (Standing Rule 1: distinct states, distinct observables). Small and
// muted/danger-tinted so they read as transient feedback, not chrome.
const copyOkStyle: React.CSSProperties = {
  fontSize: "0.68rem",
  fontWeight: 600,
  color: "var(--status-active-text)",
  whiteSpace: "nowrap",
};

const copyFailStyle: React.CSSProperties = {
  fontSize: "0.68rem",
  fontWeight: 600,
  color: "var(--state-danger-strong)",
  whiteSpace: "nowrap",
};

const CandidateFactsPanel: React.FC<Props> = ({ slug, scenarioId, externalRefresh }) => {
  const [open, setOpen] = useState(false);

  const [candidates, setCandidates] = useState<CandidateDto[] | null>(null);
  const [orphans, setOrphans] = useState<ScenarioFactDto[]>([]);
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("undecided");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Per-row action error (keyed by node id) — a ruling failure is shown on the
  // row that failed, not as a page-level banner.
  const [actionError, setActionError] = useState<{ id: string; message: string } | null>(
    null,
  );

  // Bumped after a successful ruling to re-fetch the whole pool, so the UI is a
  // pure reflection of persisted state (no optimistic drift).
  const [refreshKey, setRefreshKey] = useState(0);

  // A merge in the Theme Scan panel writes candidate facts THIS panel is showing,
  // so the parent bumps `externalRefresh` and the load effect below re-runs.
  // Without it the human merges picks and sees an unchanged list — the panels are
  // siblings on one page, with no shared store to invalidate.
  const refreshSignal = `${refreshKey}:${externalRefresh ?? 0}`;

  // --- Bug 1: preserve scroll position across a post-ruling refetch. ---------
  //
  // A ruling triggers a full refetch (`refreshKey` bump → load effect). Two
  // things would otherwise throw the reviewer back to the top of a long list:
  //   1. the load effect flips `loading` true, which used to tear down the whole
  //      scroll container and replace it with a "Loading…" message — remounting a
  //      FRESH scroll `<div>` at `scrollTop = 0` when the refetch resolved; and
  //   2. even with the container kept mounted, the ruled row leaves its filtered
  //      view, so the list shrinks by one and the content under the viewport
  //      shifts up.
  //
  // The fix is two-part: (a) keep the list mounted during a refresh so the scroll
  // node persists (see the render gate on `initialLoading` below), and (b) capture
  // the exact `scrollTop` at ruling time and restore it once the refreshed list
  // has committed (the `useLayoutEffect` below), covering the one-row-shrink.
  //
  // ## React Learning: why a ref, not state
  // `scrollTop` is imperative DOM read/write, not rendered output — storing it in
  // `useState` would trigger extra renders and lag a frame behind the DOM. A
  // `useRef` is a mutable box that survives renders WITHOUT causing one, which is
  // exactly right for "remember this value between a click and the next commit".
  const scrollRegionRef = useRef<HTMLDivElement | null>(null);
  // Scroll offset captured when a ruling is issued; `null` means "no pending
  // restore" (initial load, filter change, etc.) so the restore effect no-ops.
  const pendingScrollTop = useRef<number | null>(null);

  // Bug 2: the source PDF a reviewer chose to inspect, rendered in a right-hand
  // side-panel BESIDE the list — so viewing a source never navigates away and the
  // filter + scroll they built up survive while they compare card ↔ source.
  // `null` = no viewer open. It is plain UI state, so a ruling refetch leaves it
  // untouched (the viewer stays put while the list refreshes underneath it).
  const [viewerTarget, setViewerTarget] = useState<ViewPdfTarget | null>(null);

  // Outcome of the most recent id-chip copy, keyed by the chip's evidence_id so the
  // notice shows on the chip the reviewer clicked. `ok` distinguishes a success tick
  // from a copy failure (Standing Rule 1 — the two states look different), so a
  // failed clipboard write is OBSERVABLE, not silent.
  const [copyNotice, setCopyNotice] = useState<{ id: string; ok: boolean } | null>(
    null,
  );

  // Load the whole pool + the saved list (for the orphan check) whenever the
  // panel is open. Both come from ONE fetch each; the status filter then works
  // in memory (see `filterByStatus`). `listScenarioFacts` is fetched alongside
  // gather ONLY to surface confirmed facts whose graph node has vanished (they
  // are absent from the pool-driven gather response — `findOrphans`).
  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    setLoading(true);
    setError(null);
    Promise.all([
      gatherCandidates(slug, scenarioId),
      listScenarioFacts(slug, scenarioId),
    ])
      .then(([gather, saved]) => {
        if (cancelled) return;
        const combined = [...gather.pool, ...gather.dropped];
        const knownIds = new Set(combined.map((c) => c.content.evidence_id));
        setCandidates(combined);
        setOrphans(findOrphans(saved, knownIds));
        setLoading(false);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setError(
          err instanceof Error
            ? err.message
            : "Failed to load candidate facts. Try again.",
        );
        setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [open, slug, scenarioId, refreshSignal]);

  // Filter to the chosen status only — NO client-side sort. The backend already
  // returns the pool in ascending candidate-id order, and that order is the
  // workbench's contract: stable across visits, and never moving a card because it
  // was scanned, merged, scored, Included, or Dropped. Re-sorting here (as the old
  // `sortByConfidence` did) reshuffled the list under the person curating it.
  // `filterByStatus` preserves input order, so C-order survives the filter.
  const visible = useMemo(
    () => filterByStatus(candidates ?? [], statusFilter),
    [candidates, statusFilter],
  );

  // Restore the captured scroll offset once a post-ruling refetch has re-rendered
  // the list. Keyed on `visible` (a fresh array each time the pool or filter
  // changes): after the refetch swaps in new `candidates`, `visible` re-derives,
  // this fires, and we put the viewport back where the reviewer left it.
  //
  // ## React Learning: useLayoutEffect vs useEffect
  // `useLayoutEffect` runs SYNCHRONOUSLY after the DOM mutates but BEFORE the
  // browser paints, so we set `scrollTop` in the same frame the new rows commit —
  // the user never sees a flash at the top. A plain `useEffect` runs after paint,
  // which would show a visible jump-then-correct. Scroll restoration is the
  // textbook case for `useLayoutEffect`.
  useLayoutEffect(() => {
    if (pendingScrollTop.current == null) return;
    const el = scrollRegionRef.current;
    if (el) el.scrollTop = pendingScrollTop.current;
    // Clear so a later filter change (which also re-derives `visible`) does not
    // yank the viewport back to a stale ruling offset.
    pendingScrollTop.current = null;
  }, [visible]);

  const handleAction = (graphNodeId: string, action: FactAction) => {
    setActionError(null);
    // Capture where the reviewer is BEFORE the refetch re-renders the list, so the
    // restore effect above can put them back (Bug 1). A click never scrolls, so
    // reading it here (rather than mid-fetch) is accurate.
    pendingScrollTop.current = scrollRegionRef.current?.scrollTop ?? null;
    applyFactAction(slug, scenarioId, graphNodeId, action)
      .then(() => setRefreshKey((k) => k + 1))
      .catch((err: unknown) => {
        setActionError({
          id: graphNodeId,
          message:
            err instanceof Error ? err.message : "Failed to apply this ruling.",
        });
      });
  };

  // Copy the FULL evidence_id (the chip shows only the short `#a3f9k2` handle) so a
  // reviewer can paste the exact id when talking to Chuck / querying the DB (§4).
  //
  // The outcome is SURFACED on the clicked chip via `copyNotice` — a tick on
  // success, an explicit "copy failed" (pointing at the hover tooltip) on failure —
  // so a clipboard rejection (permission denied / insecure context / API absent) is
  // never silent (Standing Rule 1). The failure is also logged for diagnostics. This
  // is a UI convenience, so the surface is an inline chip notice rather than a
  // page-level banner, but it IS a user-facing observable, not a swallowed error.
  const copyChip = (evidenceId: string) => {
    const clip = navigator.clipboard;
    if (!clip) {
      // No clipboard API (insecure context / unsupported) — say so, don't no-op.
      setCopyNotice({ id: evidenceId, ok: false });
      return;
    }
    clip
      .writeText(evidenceId)
      .then(() => setCopyNotice({ id: evidenceId, ok: true }))
      .catch((e: unknown) => {
        console.warn("Candidate facts: could not copy id to clipboard:", e);
        setCopyNotice({ id: evidenceId, ok: false });
      });
  };

  if (!open) {
    return (
      <button type="button" style={toggleStyle} onClick={() => setOpen(true)}>
        + Candidate facts
      </button>
    );
  }

  // Orphans are human-saved refs whose graph node vanished — surfaced only under
  // the filters where a confirmed fact is expected (included / all), per the status
  // gate.
  const showOrphans = orphansVisibleUnder(statusFilter) && orphans.length > 0;
  // Exclude the error state: on a failed load `candidates` stays null, which
  // would otherwise render the "no matches" empty-state text UNDER the error
  // banner — making a load failure read as an empty filter result. The banner
  // is the observable; the empty-state message must yield to it (Standing Rule 1
  // — distinct states, distinct observables).
  const nothingToShow = !loading && !error && visible.length === 0 && !showOrphans;

  // Bug 1: distinguish the FIRST load (no pool yet — a full-panel loader is the
  // right observable) from a post-ruling REFRESH (pool already on screen). On a
  // refresh we keep the list mounted so its scroll node survives (and the restore
  // effect can put the viewport back); the in-flight state stays observable via a
  // small non-blocking "Updating…" note instead of tearing the list down — the two
  // states remain distinct (Standing Rule 1), just no longer at the cost of scroll.
  const initialLoading = loading && candidates === null;
  const refreshing = loading && candidates !== null;

  // Counts are derived from state already on screen (the gather pool + orphans),
  // re-derived each render — they cannot drift from the rendered list. Orphans
  // (statusless saved refs missing from the pool) are folded into `included`
  // HERE at the call site: a conservative over-approximation matching where 1a.6
  // renders them (under the included/all filters), so the count agrees with the
  // list. It never under-counts a confirmed fact (the ratified guarantee).
  const counts = countByStatus(candidates ?? []);
  const includedShown = counts.included + orphans.length;
  const totalShown = counts.total + orphans.length;
  // "x of y" numerator: rows visible under the current filter — orphans add in
  // only when the filter actually shows them (so it equals the rendered count).
  const shownCount = visible.length + (showOrphans ? orphans.length : 0);

  return (
    <div>
      <button type="button" style={toggleStyle} onClick={() => setOpen(false)}>
        − Hide candidate facts
      </button>
      <div style={panelStyle}>
        <div style={controlRow}>
          <label style={{ fontSize: "0.82rem", color: "var(--text-secondary)" }}>
            Status:{" "}
            <select
              style={selectStyle}
              value={statusFilter}
              onChange={(e) => setStatusFilter(e.target.value as StatusFilter)}
            >
              {STATUS_FILTERS.map((s) => (
                <option key={s} value={s}>
                  {STATUS_FILTER_LABEL[s]}
                </option>
              ))}
            </select>
          </label>
          {candidates !== null && (
            <span style={xOfYStyle}>
              {shownCount} of {totalShown}
            </span>
          )}
        </div>

        {/* Status summary — all three totals at once, so a ruling is watchable
            (drop one → undecided ticks down, dropped ticks up, in one glance).
            Shown once the pool has loaded; hidden during the first load / on an
            error so it never displays misleading zeros (Standing Rule 1). */}
        {candidates !== null && (
          <div style={summaryRowStyle}>
            <span style={undecidedCountStyle}>{counts.undecided} undecided</span>
            <span style={includedCountStyle}>{includedShown} included</span>
            <span style={droppedCountStyle}>{counts.dropped} dropped</span>
          </div>
        )}

        {error && <div style={errorStyle}>{error}</div>}

        {/* In-flight observable for a post-ruling refresh — the list stays put
            (scroll preserved) while this note marks that persisted state is being
            re-read, so "refreshing" is never a silent state (Standing Rule 1). */}
        {refreshing && <div style={refreshNoteStyle}>Updating…</div>}

        {/* List ↔ viewer split (Bug 2). The list column always renders; the PDF
            side-panel appears beside it only when a reviewer opens a source, so
            the candidate list stays visible for card ↔ source comparison and no
            filter/scroll is lost to a navigation. */}
        <div style={bodyRowStyle}>
          <div style={listColStyle}>
        {initialLoading ? (
          <div style={messageStyle}>Loading candidate facts…</div>
        ) : nothingToShow ? (
          <div style={messageStyle}>No candidate facts match this filter.</div>
        ) : (
          // Bounded scroll region (~60vh, min ~360px) so the ~94-row list does not
          // own the whole page — it scrolls within its own window while the scan
          // card and page chrome stay put. `scrollRegionRef` lets the restore
          // effect return the viewport to where the reviewer was after a ruling
          // refetch (Bug 1); the list is kept mounted across a refresh so this
          // node — and its `scrollTop` — survives.
          <div ref={scrollRegionRef} style={scrollRegionStyle}>
            {visible.map((c) => {
              return (
                <div key={c.content.evidence_id}>
                  <EvidenceCard
                    // Full content (tags NOT stripped) so the shared card renders
                    // its colored pattern-tag pills — the workbench color restore
                    // (§2). The card is unchanged; we simply stopped hiding its tags.
                    instance={c.content}
                    // Lead strip bound INSIDE the card (§2): the id chip always, plus
                    // the "role · NN%" judgment badge ONLY when scored — its absence
                    // is the human-added / unscored signal (no "unscored" text).
                    leadBadge={
                      <span style={leadStripStyle}>
                        {/* The C-chip is the human's handle. Absent (not a
                            placeholder) when the candidate has no ordinal yet —
                            "C-0"/"C-?" would read as real ids. The full graph node
                            id stays on hover and on click-to-copy: the ordinal is
                            the human handle, the node id remains the machine
                            identity. */}
                        {candidateChip(c.ordinal) && (
                          <button
                            type="button"
                            style={idChipStyle}
                            title={`${c.content.evidence_id} (click to copy)`}
                            onClick={() => copyChip(c.content.evidence_id)}
                          >
                            {candidateChip(c.ordinal)}
                          </button>
                        )}
                        {c.confidence != null && (
                          <span style={roleBadgeStyle}>
                            {roleConfidenceLabel(c.role, c.confidence)}
                          </span>
                        )}
                        {copyNotice?.id === c.content.evidence_id && (
                          <span
                            style={copyNotice.ok ? copyOkStyle : copyFailStyle}
                            role="status"
                          >
                            {copyNotice.ok
                              ? "copied ✓"
                              : "copy failed — full id is in the tooltip"}
                          </span>
                        )}
                      </span>
                    }
                    // Open the source in THIS panel's side-panel viewer instead of
                    // navigating away (Bug 2). Only the workbench passes this; the
                    // card's other consumers keep their default <Link>.
                    onViewPdf={setViewerTarget}
                    action={
                      <span style={{ display: "flex", gap: "0.4rem" }}>
                        {actionsForStatus(c.status).map((action) => (
                          <button
                            key={action}
                            type="button"
                            style={actionBtnStyle[action]}
                            onClick={() =>
                              handleAction(c.content.evidence_id, action)
                            }
                          >
                            {ACTION_LABEL[action]}
                          </button>
                        ))}
                      </span>
                    }
                  />
                  {actionError?.id === c.content.evidence_id && (
                    <div style={errorStyle}>{actionError.message}</div>
                  )}
                </div>
              );
            })}

            {/* Orphan guarantee: saved facts whose graph node has vanished are
                absent from the pool-driven gather response, so they are surfaced
                here (never silently dropped — Standing Rule 1). Informational in
                1a.6; a ruling affordance on a stale card is out of scope. */}
            {showOrphans &&
              orphans.map((o) => (
                <div key={o.graph_node_id} style={staleCardStyle}>
                  Saved fact <code>{o.graph_node_id}</code> — content unavailable
                  (the source node may have been removed).
                </div>
              ))}
          </div>
        )}
          </div>

          {/* Right-hand PDF side-panel (Bug 2). Uses the PUBLIC file endpoint —
              `/api/documents/:id/file`, the documented "any authenticated user may
              view" route — so a non-admin reviewer is never 403'd by binding to the
              incidentally-open admin route. `PdfViewer` renders its OWN error UI on
              a failed load (a 403/404 shows the message + URL), so a load failure is
              observable here without extra handling (Standing Rule 1); it also owns
              its own fetch, so this panel adds no untimed `fetch` (Rule 13). */}
          {viewerTarget && (
            <div style={viewerColStyle}>
              <div style={viewerHeaderStyle}>
                <span style={viewerTitleStyle} title={viewerTarget.documentTitle}>
                  {viewerTarget.documentTitle}
                </span>
                {viewerTarget.page != null && (
                  <span style={xOfYStyle}>p.{viewerTarget.page}</span>
                )}
                <button
                  type="button"
                  style={viewerCloseStyle}
                  onClick={() => setViewerTarget(null)}
                >
                  Close
                </button>
              </div>
              <div style={viewerFrameStyle}>
                <PdfViewer
                  src={`${API_BASE_URL}/api/documents/${encodeURIComponent(
                    viewerTarget.documentId,
                  )}/file`}
                  page={viewerTarget.page ?? 1}
                  highlightText={viewerTarget.highlightText}
                  highlightPage={viewerTarget.page}
                />
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default CandidateFactsPanel;
