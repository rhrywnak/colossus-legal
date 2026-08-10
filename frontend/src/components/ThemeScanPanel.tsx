// =============================================================================
// ThemeScanPanel.tsx — the background Theme Scan driver on the scenario page.
// -----------------------------------------------------------------------------
// Pick a model (Opus / Qwen-14B), Run. The POST returns a run_id immediately; we
// POLL the GET every 3s (the DocumentsPage idiom) and render three states:
//   SETUP    — model radio-cards + Run button.
//   RUNNING  — live "X of N judged" + mono elapsed timer + progress bar + tiles.
//   COMPLETE — the scan REPORT: five tiles, the reconciliation line, and the live
//              proposed count. Numbers only.
//
// SCANNING IS SCORING, NEVER COMMITTING. A scan writes nothing to the scenario.
//
// ## The findings list and the Merge button are GONE (2026-08-08)
//
// This panel used to be a work surface: every admitted finding rendered inline
// with a checkbox, and "Merge selected (N)" was the one write path into Candidate
// Facts. That made a human select each candidate TWICE — once as a checkbox here,
// once as a card in the queue — which is the defect this build removes.
//
// A completed run's admitted verdicts now reach the queue as a READ-TIME
// PROJECTION, so they are simply THERE, marked "Proposed by the … scan", already
// carrying quote-in-context, pinpoint, stance, bears-on, grounding and the three
// ruling buttons the finding rows never had. What is left here is a RECEIPT: what
// was gathered, what was folded, what was set aside, what was judged, and how many
// are waiting below. The unbounded findings list — measured as the panel's scroll
// defect — dies with it.
//
// ## The card collapses, and this component still mounts (architect ruling R3)
//
// Once a run has completed the card folds to one line, because the work has moved
// to the queue underneath it. The collapse is INSIDE this component and hides its
// BODY: the mount effect below calls `gatherCandidates`, and gather is the one
// place candidate ordinals are minted. Unmounting the panel would stop new
// candidates being numbered and a proposed card would arrive with `code: null`.
//
// Every color comes from tokens.css (no hardcoded hex — "elevation via borders",
// not shadows). Reuses PipelineProgressBar for the running bar.
// =============================================================================

import React, { useCallback, useEffect, useRef, useState } from "react";

import PipelineProgressBar from "./pipeline/PipelineProgressBar";
import ScanHistoryTable from "./ScanHistoryTable";
import ScenarioDeleteConfirm from "./ScenarioDeleteConfirm";
import ScanControlLine from "./ScanControlLine";
import {
  collapsedFailedSummary,
  collapsedScanSummary,
  formatElapsed,
  lastRunSummary,
} from "./themeScanFormat";
import { gatherCandidates } from "../services/scenarioGather";
import type { ProposalSource } from "../services/scenarioCards";
import {
  deleteScanRun,
  fetchScanModels,
  fetchScanRuns,
  getScanRun,
  startThemeScan,
  type ScanWording,
  type ScanModel,
  type ScanRunHeader,
  type ScanRunStatus,
  type ThemeScanSummary,
} from "../services/themeScan";

// CONST: frontend poll/tick cadences are not runtime-configurable (there is no
// frontend config endpoint); POLL_INTERVAL_MS matches the DocumentsPage
// processing-poll cadence so the two polling surfaces stay consistent, and
// ELAPSED_TICK_MS is a one-second UI refresh for the running timer. Change the
// poll value in both surfaces together if the cadence ever changes.
const POLL_INTERVAL_MS = 3000;
const ELAPSED_TICK_MS = 1000;

// REMOVED in task R1: the per-scenario collapse PREFERENCE and its localStorage
// helpers (`COLLAPSE_KEY_PREFIX`, `readCollapsed`, `writeCollapsed`, and the
// `collapsed` state that read them).
//
// They were kept dormant in 1.7D on the reasoning that "a dead FUNCTION is debt;
// a dormant PREFERENCE is a decision already made", waiting for task 3.14 to give
// this panel a collapse affordance again. Ruling R7 then decided the opposite for
// this whole family — collapse is deliberately NOT remembered, on the queue and
// here, because "a card that remembers 'folded' through a scan the human then
// cannot find is a silent failure wearing a preference's clothes" (see `expanded`
// below, which states the live rule).
//
// So the dormant preference was not waiting for a feature, it was contradicting a
// ruling. `expandOverride` is the live mechanism and it is per-session by design.

interface Props {
  slug: string;
  scenarioId: string;
  /** Which completed run is proposing candidates below, or `null` (2026-08-08).
   *  Served with the cards; the panel uses it for the collapsed one-liner and to
   *  decide which history row may show a proposed count. */
  proposalSource: ProposalSource | null;
  /**
   * Tell the page that what it is showing about this scan is now out of date.
   *
   * Called when a run COMPLETES and when a run is DELETED — the two events that
   * change which run projects candidates, and therefore change the queue's pool,
   * its proposal attribution, and the served never-scanned notice. The page
   * responds by re-reading all four of its payloads.
   *
   * ## Why the name outlived its original caller
   *
   * It was written for the merge that used to live in this panel and has since
   * moved out (see the scan REPORT below: numbers only, no Merge). Between those
   * two facts the prop was declared, destructured, passed down from
   * `ScenarioDetailPage` — and never called, by anything, for two releases.
   * `noUnusedLocals` does not catch a destructured parameter; `noUnusedParameters`
   * does, and .390 turns both on so this cannot recur silently.
   *
   * Optional because the panel still works standalone — it simply cannot refresh
   * a sibling it does not own.
   */
  onFactsChanged?: () => void;
}

const ThemeScanPanel: React.FC<Props> = ({
  slug,
  scenarioId,
  onFactsChanged,
  proposalSource,
}) => {
  const [models, setModels] = useState<ScanModel[]>([]);
  const [selectedModel, setSelectedModel] = useState<string | null>(null);
  const [candidateCount, setCandidateCount] = useState<number | null>(null);
  // Whether the pre-scan candidate-count fetch FAILED (distinct from "loaded, count
  // is 0" and from "still loading"). A failed `authFetch` is a data read, so Rule 1's
  // cosmetic best-effort carve-out does NOT apply — it must stay user-observable.
  //
  // Task 1.7D moved where it is observed. Both this and `candidateCount` used to
  // render in the panel's own subtitle; suppressing that header for mockup parity
  // deleted their only render site and left them set-but-never-read, with this
  // comment still claiming they were visible. They now render on the SCAN ROW,
  // beside the last-run meta, which is where the mockup puts scan facts anyway.
  const [countError, setCountError] = useState(false);

  const [activeRun, setActiveRun] = useState<{ runId: string; modelId: string } | null>(null);
  const [poll, setPoll] = useState<ScanRunStatus | null>(null);
  const [elapsedMs, setElapsedMs] = useState(0);
  const startedAtRef = useRef<number>(0);

  const [startError, setStartError] = useState<string | null>(null);
  // A model-catalog load failure gets its OWN observable state, distinct from a
  // genuinely-empty registry (Standing Rule 1 — the two states must look different).
  const [modelError, setModelError] = useState<string | null>(null);
  // Models the backend could not offer, one sentence each — distinct from a load
  // failure (`modelError`) and from an empty registry. All three states look
  // different on screen (Standing Rule 1).
  const [modelWarnings, setModelWarnings] = useState<string[]>([]);

  // ── Run history is the SOURCE OF TRUTH, hydrated from the DB (not session) ──
  // `runs` are the persisted headers (newest first) — they survive navigation and
  // reloads, replacing the old ephemeral `completed` map. `summaries` is a LAZY
  // cache of each run's full result, filled by clicking a row (getScanRun).
  // `selectedRunIds` (0 or 1 — single-select) drives which run renders.
  const [runs, setRuns] = useState<ScanRunHeader[]>([]);
  // The words the history's own controls speak, served with the list. `null`
  // until it loads — the table renders no control it has no words for, rather
  // than falling back to a literal (the configuration law).
  const [historyWording, setHistoryWording] = useState<ScanWording | null>(null);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const [summaries, setSummaries] = useState<Record<string, ThemeScanSummary>>({});
  const [selectedRunIds, setSelectedRunIds] = useState<string[]>([]);
  // A per-run detail-load failure is distinct from the list-load failure above.
  const [detailError, setDetailError] = useState<string | null>(null);
  /** The human's own collapse choice, or `null` while the computed default
   *  stands. See `expanded` below for why it is not persisted. */
  const [expandOverride, setExpandOverride] = useState<boolean | null>(null);
  // Re-read the persisted history (after a scan finishes, or on mount).
  const refreshRuns = useCallback(() => {
    fetchScanRuns(slug, scenarioId)
      .then((list) => {
        setRuns(list.runs);
        setHistoryWording(list.wording);
        setHistoryError(null);
      })
      .catch((e: unknown) => {
        // A history-load failure is observable and distinct from "no runs yet".
        setHistoryError(e instanceof Error ? e.message : "Failed to load scan history.");
      });
  }, [slug, scenarioId]);

  // ── Load the model catalog + the pre-scan candidate count on mount ──────────
  useEffect(() => {
    fetchScanModels()
      .then((catalog) => {
        setModels(catalog.models);
        // Rows the backend refused to list, in its own words. Empty on a healthy
        // deployment; shown when not, because a picker one row short looks
        // exactly like a complete one (task 1.7B).
        setModelWarnings(catalog.warnings);
        setSelectedModel(
          (cur) =>
            cur ??
            catalog.models.find((m) => m.is_default)?.model_id ??
            catalog.models[0]?.model_id ??
            null,
        );
      })
      .catch((e: unknown) => {
        // A load failure is NOT an empty registry — surface it so the operator
        // can tell "backend/auth problem" from "no models configured".
        setModelError(e instanceof Error ? e.message : "Failed to load the model catalog.");
      });
    gatherCandidates(slug, scenarioId)
      .then((g) => {
        setCandidateCount(g.pool.length + g.dropped.length);
        setCountError(false);
      })
      .catch((e: unknown) => {
        // A failed count fetch is a DATA read failure, so it is SURFACED (a muted
        // "(candidate count unavailable)" beside the subtitle), not silently dropped
        // — Rule 9's best-effort carve-out is limited to cosmetic browser-storage and
        // does not cover an `authFetch`. It is non-blocking: the scan still runs and
        // `candidates_total` arrives with the run, so a small inline notice (not a
        // page banner) is the proportionate surface. Also logged for diagnostics.
        console.warn("Theme Scan: candidate-count fetch failed:", e);
        setCandidateCount(null);
        setCountError(true);
      });
    // Hydrate the run history from the DB — the thing that survives navigation.
    refreshRuns();
  }, [slug, scenarioId, refreshRuns]);

  // ── Poll the active run every 3s while it is running ────────────────────────
  useEffect(() => {
    if (!activeRun) return;
    let cancelled = false;
    const tick = async () => {
      try {
        const status = await getScanRun(slug, scenarioId, activeRun.runId);
        if (cancelled) return;
        setPoll(status);
        if (status.status === "completed" && status.summary) {
          const summary = status.summary;
          // Seed the lazy cache with the just-finished result, auto-select it so
          // it renders immediately, and re-read the history so the new run appears.
          setSummaries((m) => ({ ...m, [status.run_id]: summary }));
          setSelectedRunIds([status.run_id]);
          refreshRuns();
          // THE DEAD WIRE, reconnected (audit defects 7-8). `refreshRuns` reloads
          // this panel's own history list and nothing else — but a completed run
          // changes what the PAGE is showing: a new run now projects, so the
          // queue's pool, its `proposal_source` attribution and the served
          // "no scan has run yet" notice are all stale the instant this fires.
          // Without this call they stayed stale until a manual reload, which is
          // why the banner went on claiming nothing had scanned a scenario that
          // had just been scanned.
          onFactsChanged?.();
          setActiveRun(null);
        } else if (status.status === "failed") {
          setStartError(status.error ?? "The scan failed.");
          // A failed run is also part of the history — surface it in the list.
          refreshRuns();
          setActiveRun(null);
        }
      } catch (e) {
        if (!cancelled) setStartError(e instanceof Error ? e.message : "Failed to poll the scan.");
      }
    };
    tick(); // immediate first poll, then interval
    const id = setInterval(tick, POLL_INTERVAL_MS);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [activeRun, slug, scenarioId, refreshRuns]);

  // ── Tick the elapsed timer client-side while running ────────────────────────
  useEffect(() => {
    if (!activeRun) return;
    const id = setInterval(() => setElapsedMs(Date.now() - startedAtRef.current), ELAPSED_TICK_MS);
    return () => clearInterval(id);
  }, [activeRun]);

  const onRun = useCallback(async () => {
    if (!selectedModel) return;
    setStartError(null);
    startedAtRef.current = Date.now();
    setElapsedMs(0);
    try {
      const started = await startThemeScan(slug, scenarioId, {
        model_id: selectedModel,
      });
      setCandidateCount(started.candidates_total);
      setPoll(null);
      setActiveRun({ runId: started.run_id, modelId: selectedModel });
    } catch (e) {
      // Verbatim backend message (names the endpoint / both models on a 503 gate,
      // and the missing path when the judging prompt is not deployed).
      setStartError(e instanceof Error ? e.message : "Failed to start the scan.");
      // A start that got far enough to record a run leaves a FAILED row behind
      // (the backend writes the stub before it prepares), so re-read the history
      // to surface it. Without this the row exists but stays invisible until the
      // next mount — the toast is dismissed, nothing is on screen, and the scan
      // looks like it never happened. That was the eleven-day symptom.
      refreshRuns();
    }
  }, [selectedModel, slug, scenarioId, refreshRuns]);

  // ── Select a history run for display ────────────────────────────────────────
  // Single-select: click a row to VIEW that run (replaces any prior selection);
  // click the already-selected row to collapse it. No multi-select/comparison
  // (a deliberate future opt-in, not the default). Read state DIRECTLY so the
  // fetch decision doesn't depend on the async setState updater (the old race).
  const onSelectRun = useCallback(
    async (runId: string) => {
      if (selectedRunIds.length === 1 && selectedRunIds[0] === runId) {
        setSelectedRunIds([]);
        return;
      }
      setSelectedRunIds([runId]);
      if (summaries[runId]) return;
      setDetailError(null);
      try {
        const status = await getScanRun(slug, scenarioId, runId);
        if (status.summary) {
          setSummaries((m) => ({ ...m, [runId]: status.summary as ThemeScanSummary }));
        } else {
          // A running/failed run has no stored result — say so, don't render blank.
          setDetailError(
            status.status === "failed"
              ? `That run failed: ${status.error ?? "no reason recorded"}.`
              : "That run has no stored result to display yet.",
          );
        }
      } catch (e) {
        setDetailError(e instanceof Error ? e.message : "Failed to load the run.");
      }
    },
    [selectedRunIds, summaries, slug, scenarioId],
  );

  // ── A completed run's results survive a page refresh (task 2.15, piece 4a) ──
  //
  // The measured defect, 2026-08-07 night: after a reload the RELEVANT FINDINGS
  // panel was simply gone. The wiring to bring it back was already here — clicking
  // a history row loads and renders that run — but the history disclosure starts
  // closed and the rows did not look clickable, so "the verdicts are unreachable"
  // was the honest reading of the screen.
  //
  // So the panel now opens its most recent COMPLETED run by itself. That is also
  // piece 3c's law from the other side: when a scan HAS run, the section leads
  // with the scan's results.
  //
  // ## Why a ref rather than "select when nothing is selected"
  //
  // Clicking the open row collapses it (`onSelectRun` toggles). A condition on an
  // empty selection would re-open it on the very next render, and the control
  // would look broken. The ref makes this a once-per-scenario arrival, after which
  // the human's clicks are the only thing that moves it.
  const autoOpenedFor = useRef<string | null>(null);
  useEffect(() => {
    if (autoOpenedFor.current === scenarioId) return;
    // Server order is newest-first, so the first completed row IS the latest one.
    // A failed or running run is deliberately skipped: it has no stored result,
    // and auto-opening it would put "that run failed" on screen unprompted.
    const latest = runs.find((run) => run.status === "completed");
    if (!latest) return;
    autoOpenedFor.current = scenarioId;
    void onSelectRun(latest.run_id);
  }, [runs, scenarioId, onSelectRun]);

  // ── Delete a history run ────────────────────────────────────────────────────
  // The row owns the confirm; the panel owns the network call, its error UI, and
  // the post-delete state cleanup (Standing Rule 1 — a failed delete is surfaced
  // in the history error box, never swallowed). On success: re-hydrate the history
  // from the DB (the run is now gone), and if the deleted run was the one open
  // below, clear the selection and drop its cached summary so the results area
  // does not render a run that no longer exists.
  // THE RUN-DELETE DIALOG (task R1 Piece 10c).
  //
  // Held HERE rather than in the history table for the same reason the dashboard
  // holds its delete dialog on the page rather than on the card: one dialog
  // serves every row, and whoever owns the refresh that follows a delete has to
  // own the question that starts it.
  //
  // `pending` carries the run AND its already-filled sentence, so the dialog
  // names the run it is about. `null` means no dialog.
  const [pendingDelete, setPendingDelete] = useState<{
    runId: string;
    message: string;
  } | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);

  const onDeleteRun = useCallback(
    async (runId: string) => {
      setDeleting(true);
      setDeleteError(null);
      try {
        await deleteScanRun(slug, scenarioId, runId);
      } catch (e) {
        // The dialog STAYS OPEN with the failure on it (Standing Rule 1, and the
        // contract `ScenarioDeleteConfirm` is built around). Closing here would
        // tell the human a run was deleted that is still in the table behind
        // them — which is what the native `confirm` could not avoid, having no
        // way to report anything at all.
        setDeleting(false);
        // Names WHAT failed, WHY (the backend's own message), and WHAT TO DO.
        // The dialog stays open with both buttons live, so "try again" is a real
        // instruction here rather than a platitude — the retry is one click away
        // and the run is still named on screen.
        //
        // These two sentences are code literals on a surface whose words are
        // otherwise stored rows. They are already on Piece 9's migration
        // inventory (.391) and are not new debt; the alternative was leaving a
        // failure that says only that it happened.
        setDeleteError(
          e instanceof Error
            ? `${e.message} The run has NOT been deleted — try again, or reload the page if this keeps happening.`
            : "Failed to delete the run. It has NOT been deleted — try again, or reload the page if this keeps happening.",
        );
        return;
      }
      setDeleting(false);
      // Closed only AFTER the DELETE resolved. The dialog closing is not proof
      // the run is gone; the re-read below is.
      setPendingDelete(null);
      refreshRuns();
      // Same wire, other direction (audit defect 9). Deleting a run can change
      // WHICH run projects — the next-newest completed one, or none at all — and
      // the page owns `proposal_source`. Without this the history table and the
      // collapsed card would go on attributing proposals to a run that no longer
      // exists.
      //
      // This is the heavier of the two refreshes: it re-runs all four page reads
      // and reloads the queue's pool. Deliberate and safe here — a deletion does
      // not change pool MEMBERSHIP, so `cards_loaded`'s clamp lands the human on
      // the same card they were on. Said out loud because the page's own comment
      // block warns at length against exactly this kind of over-refresh, and the
      // next reader deserves to know this one was weighed rather than missed.
      onFactsChanged?.();
      setSelectedRunIds((sel) => sel.filter((id) => id !== runId));
      setSummaries((m) => {
        if (!(runId in m)) return m;
        const next = { ...m };
        delete next[runId];
        return next;
      });
    },
    [slug, scenarioId, refreshRuns],
  );

  // The selected runs whose full summaries are loaded, keyed by run_id — this is
  // what feeds the EXISTING results display + comparison hero (one entry → a
  // single result; two → the hero). Order follows selection.
  const selectedSummaries: Record<string, ThemeScanSummary> = {};
  for (const id of selectedRunIds) {
    const s = summaries[id];
    if (s) selectedSummaries[id] = s;
  }

  const modelName = (id: string) => models.find((m) => m.model_id === id)?.display_name ?? id;
  const running = activeRun !== null;
  const hasSelectedResults = Object.keys(selectedSummaries).length > 0;

  // ── The collapsed card (piece 4a) ──────────────────────────────────────────
  //
  // The one line a folded card shows, or `null` when there is nothing to fold —
  // no completed run yet, or the words have not loaded. `null` means the card has
  // no collapse control at all and renders expanded, which is exactly right for a
  // never-scanned scenario: running the first scan IS the work there.
  //
  // Composed from the STORED template. A card that invented its own summary would
  // be the one sentence on this screen the configuration law does not reach.
  const latestCompleted = runs.find((r) => r.status === "completed") ?? null;
  // The most recent SETTLED run, which is not always the same row. A run whose
  // every judged call failed now records `failed` (ruling R3), so it is invisible
  // to the line above — correctly, because it projects nothing — but it is the run
  // the human just watched, and the folded card has to say what happened to it
  // rather than quietly describing the one before it.
  const latestSettled = runs.find((r) => r.status !== "running") ?? null;
  const latestFailed = latestSettled?.status === "failed" ? latestSettled : null;
  const collapsedSummary =
    historyWording && latestFailed
      ? collapsedFailedSummary(
          historyWording.card_collapsed_failed_template,
          formatRunDate(latestFailed.started_at),
          modelName(latestFailed.model_id),
          latestFailed.failed_count,
        )
      : historyWording && latestCompleted
        ? collapsedScanSummary(
            historyWording.card_collapsed_summary_template,
            formatRunDate(latestCompleted.started_at),
            modelName(latestCompleted.model_id),
            proposalSource?.proposed_count ?? null,
          )
        : null;

  // Default: COLLAPSED once a run has completed, EXPANDED before that. The
  // human's own click wins from then on, and is deliberately NOT persisted — the
  // same ruling (R7) that keeps the queue's own collapse un-remembered, for the
  // same reason: a card that remembers "folded" through a scan the human then
  // cannot find is a silent failure wearing a preference's clothes.
  const expanded = expandOverride ?? collapsedSummary === null;

  return (
    <section style={S.card}>
      {/* Keyframes for the "Scanning" pulse dot — inlined like ProcessingPanel's
          colossus-spin, so the animation ships with the component. */}
      <style>{`@keyframes colossus-pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.3; } }`}</style>
      {/* THE COLLAPSED CARD (piece 4a). Once a run has completed, the scan is no
          longer where the work happens — the queue below is — so the card folds to
          one line naming the run, the model and how many candidates are waiting.
          A never-scanned scenario opens expanded, because running the first scan
          IS the work there.

          The control is here rather than in `ScanSection` for a mechanical reason
          worth stating: the summary is composed from this component's own state
          (the run history and the scan wording it fetched), and lifting it would
          mean a second fetch of both. What must NOT move is the component itself —
          see the module header on why unmounting breaks candidate codes. */}
      {collapsedSummary !== null && (
        <button
          type="button"
          style={S.collapseRow}
          onClick={() => setExpandOverride(!expanded)}
          aria-expanded={expanded}
        >
          <span style={S.collapseChevron}>{expanded ? "▾" : "▸"}</span>
          <span style={S.collapseSummary}>{collapsedSummary}</span>
        </button>
      )}

      {expanded && (
        <>
          {running ? (
            <RunningView poll={poll} modelName={modelName(activeRun.modelId)} elapsedMs={elapsedMs} />
          ) : (
            <ScanControlLine
              models={models}
              modelError={modelError}
              modelWarnings={modelWarnings}
              selectedModel={selectedModel}
              lastRun={lastRunSummary(runs, modelName)}
              candidateCount={candidateCount}
              countError={countError}
              historySlot={
                /* Run history from the DB, inline on the control line (v3). */
                <ScanHistoryTable
                  runs={runs}
                  wording={historyWording}
                  selectedRunIds={selectedRunIds}
                  onToggle={onSelectRun}
                  onRequestDelete={setPendingDelete}
                  modelName={modelName}
                  proposingRunId={proposalSource?.run_id ?? null}
                  proposedCount={proposalSource?.proposed_count ?? null}
                />
              }
              onSelect={setSelectedModel}
              onRun={onRun}
            />
          )}

          {startError && (
            <div style={S.errorBox} role="alert">
              {startError}
            </div>
          )}
          {historyError && (
            <div style={S.errorBox} role="alert">
              {historyError}
            </div>
          )}
          {detailError && (
            <div style={S.errorBox} role="alert">
              {detailError}
            </div>
          )}

          {/* The scan REPORT — numbers only (piece 4b). No findings list, no
              checkboxes, no Merge. The proposed candidates are in the queue below;
              this says where they came from and proves nothing was lost. */}
          {hasSelectedResults && historyWording && (
            <div style={S.results}>
              {Object.entries(selectedSummaries).map(([runId, summary]) => (
                <RunReport
                  key={runId}
                  summary={summary}
                  modelName={modelName(summary.model_id)}
                  wording={historyWording}
                  // The pill reads the RUN's status, not the summary's — a summary
                  // is what the run produced and has no opinion about whether
                  // producing it counted as success.
                  status={runs.find((r) => r.run_id === runId)?.status ?? null}
                  // The live count belongs to THIS run only when this run is the
                  // one projecting. Reopening an older run must not borrow the
                  // current run's number (R-b).
                  proposedCount={
                    proposalSource && proposalSource.run_id === runId
                      ? proposalSource.proposed_count
                      : null
                  }
                />
              ))}
            </div>
          )}
        </>
      )}

      {/* The run-delete confirmation (task R1 Piece 10c), replacing the native
          `window.confirm` that froze the browser walk on 2026-08-09.

          Rendered at the panel's root rather than inside the collapsible body:
          the dialog must survive whatever the human does to the card behind it,
          and a confirmation that can be unmounted by a collapse is a
          confirmation that can strand a delete mid-flight.

          Its `message` is the stored row, filled by the table that had both
          halves. No `title`: the sentence IS the question, and inventing a
          heading in code would put a literal on the one surface whose words are
          all configuration. */}
      {pendingDelete && (
        <ScenarioDeleteConfirm
          message={pendingDelete.message}
          busy={deleting}
          error={deleteError}
          onConfirm={() => void onDeleteRun(pendingDelete.runId)}
          onCancel={() => {
            setPendingDelete(null);
            setDeleteError(null);
          }}
        />
      )}
    </section>
  );
};

// ─── RUNNING ──────────────────────────────────────────────────────────────────

const RunningView: React.FC<{
  poll: ScanRunStatus | null;
  modelName: string;
  elapsedMs: number;
}> = ({ poll, modelName, elapsedMs }) => {
  const judged = poll?.candidates_judged ?? 0;
  const total = poll?.candidates_total ?? 0;
  const pct = total > 0 ? Math.round((judged / total) * 100) : 0;
  return (
    <div style={S.running}>
      <div style={S.runningTop}>
        <span style={S.modelChip}>{modelName}</span>
        <span style={S.scanningPill}>
          <span style={S.pulseDot} /> Scanning
        </span>
        <span style={S.timer}>{formatElapsed(elapsedMs)}</span>
      </div>

      <div style={S.judged}>
        {judged} <span style={S.judgedOf}>of {total || "…"} judged</span>
      </div>

      <PipelineProgressBar status="PROCESSING" percentComplete={pct} />

      <div style={S.tileRow}>
        <LiveTile label="Relevant" value={poll?.relevant_count ?? 0} tone="success" />
        <LiveTile label="Not relevant" value={poll?.irrelevant_count ?? 0} tone="muted" />
        <LiveTile label="Failed" value={poll?.failed_count ?? 0} tone="danger" />
      </div>
      <div style={S.soFar}>counts so far — in progress</div>
    </div>
  );
};

const LiveTile: React.FC<{ label: string; value: number; tone: "success" | "muted" | "danger" }> = ({
  label,
  value,
  tone,
}) => (
  <div style={S.tile}>
    <div style={{ ...S.tileValue, color: toneColor(tone) }}>{value}</div>
    <div style={S.tileLabel}>{label}</div>
  </div>
);

// ─── COMPLETE / COMPARISON ────────────────────────────────────────────────────

/**
 * One completed run's REPORT: five tiles, the reconciliation line, and the live
 * proposed count. Numbers only (piece 4b).
 *
 * ## What this replaced, and why nothing was lost
 *
 * It used to render every admitted finding inline with a merge checkbox. Those
 * findings ARE the proposed cards in the queue below — richer there than they
 * ever were here (quote in context, pinpoint, stance with its object, bears-on,
 * grounding, and the three ruling buttons) — so listing them again would be the
 * same candidates twice on one screen, which is the select-twice defect wearing a
 * read-only coat. What a human needs from a finished run is proof that nothing
 * was lost, and that is arithmetic.
 *
 * Every string is served. The tile captions, the advisory note and the live line
 * come from the settings store; the reconciliation sentence is composed by the
 * BACKEND from this run's own frozen counts.
 */
const RunReport: React.FC<{
  summary: ThemeScanSummary;
  modelName: string;
  wording: ScanWording;
  /** The LIVE count, or `null` when this run is not the one projecting. */
  proposedCount: number | null;
  /** This run's own status, or `null` when its history row is not loaded. */
  status: ScanRunHeader["status"] | null;
}> = ({ summary, modelName, wording, proposedCount, status }) => (
  <div style={S.runResult}>
    <div style={S.runResultHead}>
      <span style={S.modelChip}>{modelName}</span>
      {/* Both words are served (ruling R4). "Complete" used to be a literal here,
          and it was the literal that said "Complete" over a run whose 104 judge
          calls had all returned 400. A pill that can be wrong about which of two
          things happened has to be able to render either one. */}
      {status === "failed" ? (
        <span style={S.failedPill}>{wording.status_failed_label}</span>
      ) : (
        <span style={S.completePill}>{wording.status_complete_label}</span>
      )}
      <span style={S.muted}>{formatElapsed(summary.duration_ms)}</span>
    </div>
    <div style={S.advisory}>{wording.report_advisory_note}</div>

    {/* The conservation tiles. Rendered only when the run MEASURED conservation:
        runs recorded before task 2.15 carry no such block, and zeroed tiles would
        claim they counted something they never did (Standing Rule 1). Those runs
        show their own frozen four instead. */}
    {summary.conservation ? (
      <div style={S.tileRow}>
        <LiveTile label={wording.report_tile_gathered} value={summary.conservation.pool} tone="muted" />
        <LiveTile
          label={wording.report_tile_folded}
          value={summary.conservation.duplicates_collapsed}
          tone="muted"
        />
        <LiveTile
          label={wording.report_tile_set_aside}
          value={setAsideTotal(summary.conservation)}
          tone="muted"
        />
        <LiveTile label={wording.report_tile_judged} value={summary.conservation.judged} tone="muted" />
        {/* Shown only when it is nonzero. A permanent zero tile is decoration the
            eye learns to skip, and the one run where this number matters is the
            run where it stops being zero. */}
        {summary.conservation.failed > 0 && (
          <LiveTile
            label={wording.report_tile_failed}
            value={summary.conservation.failed}
            tone="danger"
          />
        )}
        {/* The one live tile. An em dash rather than a zero when this run is not
            the projecting one: "not the current run" and "current run proposing
            nothing" are different facts. */}
        <ReportTile
          label={wording.report_tile_proposed}
          value={proposedCount === null ? "—" : String(proposedCount)}
        />
      </div>
    ) : (
      <div style={S.tileRow}>
        <LiveTile label="Judged" value={summary.candidates_read} tone="muted" />
        <LiveTile label="Relevant" value={summary.relevant} tone="success" />
        <LiveTile label="Not relevant" value={summary.irrelevant} tone="muted" />
        <LiveTile label="Failed" value={summary.failed} tone="danger" />
      </div>
    )}

    {/* Composed BY THE BACKEND from this run's own FROZEN counts and the stored
        template. Absent on runs recorded before 2.15 — which is honest: those runs
        never measured it, and a zeroed line would say they did. */}
    {summary.conservation_line && <div style={S.conservationLine}>{summary.conservation_line}</div>}

    {/* The live number, in its own sentence and labelled as live (architect ruling
        R5). Kept OUT of the reconciliation line above because that line describes
        what the run DID and must never appear to move, while this falls every time
        the human rules a card. */}
    {proposedCount !== null && (
      <div style={S.proposedLine}>
        {wording.report_proposed_line_template.replace("{count}", String(proposedCount))}
      </div>
    )}
  </div>
);

/** One tile whose value is not a plain count — the live proposed number, which
 *  renders an em dash when this run is not the one projecting. */
const ReportTile: React.FC<{ label: string; value: string }> = ({ label, value }) => (
  <div style={S.tile}>
    <div style={{ ...S.tileValue, color: "var(--text-primary)" }}>{value}</div>
    <div style={S.tileLabel}>{label}</div>
  </div>
);

/**
 * Everything the pre-filter kept from the judge, as ONE number.
 *
 * The three exclusion reasons are counted separately in the record (empty quote,
 * content-free statement kind, unanchored fragment) and the backend's
 * reconciliation sentence names each with its own count. The tile is the total,
 * because a row of three near-identical tiles would spend the report's width on a
 * breakdown the sentence beneath it already gives.
 */
function setAsideTotal(c: NonNullable<ThemeScanSummary["conservation"]>): number {
  return c.excluded_empty + c.excluded_statement_type + c.excluded_too_short;
}

// ─── styling (tokens.css only) ────────────────────────────────────────────────

/**
 * The run's start time, as the collapsed summary says it: "Aug 7, 09:37 PM".
 *
 * Formatted by the BROWSER because the server does not know the reader's locale —
 * the same split the delete confirmation's `{run}` already makes, so both dates on
 * this card read alike. An unparseable timestamp falls back to the raw string:
 * a summary naming an ugly date still identifies its run, which is the job.
 */
function formatRunDate(iso: string): string {
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return iso;
  return at.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}

function toneColor(tone: "success" | "muted" | "danger"): string {
  if (tone === "success") return "var(--state-success-strong)";
  if (tone === "danger") return "var(--state-danger-strong)";
  return "var(--text-secondary)";
}

const S: Record<string, React.CSSProperties> = {
  /**
   * The Run-scan button. Mockup `.btn.ghost`: white, NO border, its own
   * `--shadow-card`, radius 9, 13.5px/500.
   *
   * ## This style was MISSING until task 1.7D
   *
   * `SetupView` has referenced `S.runButton` and `S.runButtonDisabled` since 1.7B,
   * and neither key existed in this object — so the lookup returned `undefined` and
   * the primary control of the whole scan flow rendered as an unstyled browser
   * button. TypeScript could not catch it: `Record<string, React.CSSProperties>`
   * types every key as present, and `style={undefined}` is legal JSX.
   *
   * It is part of why beta.367 shipped below the signed mockup, and it is the kind
   * of gap a word-checklist review cannot see — which is the gap this task's
   * screenshot comparison exists to close.
   */
  runButton: {
    fontFamily: "inherit",
    fontSize: "13.5px",
    fontWeight: 500,
    padding: "8px 15px",
    borderRadius: "9px",
    border: "none",
    cursor: "pointer",
    background: "var(--bg-surface)",
    color: "var(--text-primary)",
    boxShadow: "var(--shadow-card)",
  },
  runButtonDisabled: {
    // No model selected yet. Dimmed and not-allowed rather than hidden: the control
    // is the point of the section, and hiding it would leave the row looking broken.
    opacity: 0.55,
    cursor: "not-allowed",
    boxShadow: "none",
  },
  card: {
    fontFamily: "var(--font-sans)",
    background: "var(--bg-surface)",
    // v3: no card borders — the shadow is the edge (tokens.css).
    boxShadow: "var(--shadow-card)",
    borderRadius: "12px",
    padding: "20px",
    marginBottom: "1.5rem",
  },
  header: { display: "flex", justifyContent: "space-between", marginBottom: "16px" },
  // The header is a single full-width toggle button; the chevron + title/subtitle
  // stack sit inside it. Reset the native button chrome so it reads as the header.
  collapseToggle: {
    display: "flex",
    alignItems: "center",
    gap: "10px",
    width: "100%",
    padding: 0,
    background: "none",
    border: "none",
    textAlign: "left",
    cursor: "pointer",
    color: "inherit",
    font: "inherit",
  },
  // Enlarged from 0.8rem (§6.1) — the old chevron was too small a click/touch
  // target. A fixed square keeps it centered as the header toggle's affordance.
  collapseChevron: {
    fontSize: "1.15rem",
    color: "var(--text-muted)",
    lineHeight: 1,
    width: "1.4rem",
    textAlign: "center",
    flexShrink: 0,
  },
  title: { display: "block", fontSize: "1.05rem", fontWeight: 600, color: "var(--text-primary)" },
  subtitle: { display: "block", fontSize: "0.85rem", color: "var(--text-muted)", marginTop: "2px" },
  // The inline "count unavailable" note — danger-tinted so a failed data fetch reads
  // as a problem (not just muted chrome), but small/inline since it is non-blocking.
  countUnavailable: { color: "var(--state-danger-strong)", fontStyle: "italic" },
  // "Merged ✓ · merged N× · last …" — a durable-state chip (muted, success-tinted),
  // distinct from the actionable Merge/Re-merge button beside it.
  muted: { color: "var(--text-muted)", fontSize: "0.82rem" },

  /** The conservation line: quiet, full-width, directly under the tiles it
   *  reconciles. Sized between the tile labels and the body text — it is a
   *  receipt, not a headline. */
  conservationLine: {
    color: "var(--text-secondary)",
    fontSize: "0.78rem",
    padding: "0.35rem 0 0.6rem",
  },

  /**
   * The collapsed card's one-line row (piece 4a).
   *
   * A `<button>` rather than a styled `<div>`: it is the only control on a folded
   * card, and the keyboard has to reach it. Styled flat so it reads as a heading
   * line rather than as a second call to action beside "Run scan" — §2c's colour
   * budget is one accent plus chips, and this is neither.
   */
  collapseRow: {
    display: "flex",
    alignItems: "center",
    gap: "10px",
    width: "100%",
    padding: "10px 0",
    background: "none",
    border: "none",
    cursor: "pointer",
    font: "inherit",
    textAlign: "left",
    color: "var(--text-primary)",
  },
  /** Reuses the existing `collapseChevron` above — one chevron style on this
   *  card, whatever is being folded. */
  collapseSummary: { fontSize: "0.85rem", color: "var(--text-secondary)" },

  /** The report's "advisory only" line — quiet, because it is telling the reader
   *  there is nothing to do here. */
  advisory: {
    fontSize: "0.78rem",
    color: "var(--text-muted)",
    padding: "0 0 0.7rem",
  },

  /** The LIVE proposed sentence, set apart from the frozen reconciliation line
   *  above it so the two are not read as one claim (architect ruling R5). */
  proposedLine: {
    fontSize: "0.8rem",
    color: "var(--text-primary)",
    padding: "0.2rem 0 0",
  },

  setup: { display: "flex", flexDirection: "column", gap: "14px" },
  sectionLabel: {
    fontSize: "0.72rem",
    fontWeight: 600,
    textTransform: "uppercase",
    letterSpacing: "0.04em",
    color: "var(--text-muted)",
  },
  // ── The scan control line (task 1.7B) ──────────────────────────────────────
  //
  // The radio grid it replaced is gone: a two-column card grid for a choice most
  // people make once, in the vertical space of a small form. §2c — hairline
  // borders, no tint, regular weight.
  // Mockup `.scan`: padding 14px 24px, 14px gaps, centred, wrapping.
  controlLine: {
    display: "flex",
    alignItems: "center",
    gap: "14px",
    flexWrap: "wrap",
    padding: "14px 24px",
  },
  // Mockup `select`: BORDERLESS on the chrome fill, radius 8, 13.5px (task 1.7D).
  modelSelect: {
    fontSize: "13.5px",
    fontFamily: "inherit",
    fontWeight: 400,
    color: "var(--text-primary)",
    background: "var(--v3-chrome)",
    border: "none",
    borderRadius: "8px",
    padding: "7px 10px",
    maxWidth: "22rem",
  },
  lastRun: { fontSize: "0.8rem", color: "var(--text-muted)" },

  running: { display: "flex", flexDirection: "column", gap: "12px" },
  runningTop: { display: "flex", alignItems: "center", gap: "10px" },
  // The four remaining `--bg-page` fills are CHIPS, PILLS and BADGES — small
  // decorative elements, which §2 explicitly leaves as where colour lives
  // ("all other colour lives in chips and status dots"). The panels and boxes
  // that used to share the tint are white as of task 1.7B: a surface holding
  // content sits on white and is carried by its hairline border.
  modelChip: {
    fontSize: "0.8rem",
    fontWeight: 600,
    color: "var(--text-secondary)",
    background: "var(--bg-page)",
    border: "1px solid var(--border-default)",
    borderRadius: "6px",
    padding: "3px 9px",
  },
  // The two settled-run pills. `completePill` was REFERENCED here but never
  // defined — `S` is a `Record<string, CSSProperties>`, so `S.completePill`
  // resolved to `undefined` and the word rendered unstyled beside a styled model
  // chip. Defining both now, because a FAILED pill that looks identical to a
  // Complete one would be the honesty fix defeated by its own styling.
  completePill: {
    fontSize: "0.78rem",
    fontWeight: 600,
    color: "var(--text-secondary)",
    background: "var(--bg-canvas)",
    border: "1px solid var(--border-default)",
    borderRadius: "999px",
    padding: "2px 10px",
  },
  failedPill: {
    fontSize: "0.78rem",
    fontWeight: 700,
    color: "var(--state-danger-strong)",
    background: "var(--state-danger-bg-soft)",
    border: "1px solid var(--state-danger-strong)",
    borderRadius: "999px",
    padding: "2px 10px",
  },
  scanningPill: {
    display: "inline-flex",
    alignItems: "center",
    gap: "6px",
    fontSize: "0.78rem",
    fontWeight: 600,
    color: "var(--accent-primary)",
    background: "var(--accent-bg-soft)",
    border: "1px solid var(--accent-primary)",
    borderRadius: "999px",
    padding: "2px 10px",
  },
  pulseDot: {
    width: "7px",
    height: "7px",
    borderRadius: "50%",
    background: "var(--accent-primary)",
    animation: "colossus-pulse 1s ease-in-out infinite",
  },
  timer: {
    marginLeft: "auto",
    fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
    fontSize: "0.95rem",
    color: "var(--text-secondary)",
  },
  judged: { fontSize: "1.9rem", fontWeight: 700, color: "var(--text-primary)" },
  judgedOf: { fontSize: "1rem", fontWeight: 400, color: "var(--text-muted)" },
  tileRow: { display: "flex", gap: "10px" },
  tile: {
    flex: 1,
    background: "var(--bg-surface)",
    border: "1px solid var(--border-default)",
    borderRadius: "10px",
    padding: "12px 14px",
    textAlign: "center",
  },
  tileValue: { fontSize: "1.4rem", fontWeight: 700 },
  tileLabel: { fontSize: "0.74rem", color: "var(--text-muted)", marginTop: "2px" },
  soFar: { fontSize: "0.72rem", color: "var(--text-muted)", fontStyle: "italic" },

  errorBox: {
    marginTop: "12px",
    padding: "12px 14px",
    background: "var(--bg-surface)",
    border: "1px solid var(--state-danger-strong)",
    borderRadius: "8px",
    color: "var(--state-danger-strong)",
    fontSize: "0.85rem",
  },


  results: { marginTop: "18px", display: "flex", flexDirection: "column", gap: "16px" },
};

export default ThemeScanPanel;
