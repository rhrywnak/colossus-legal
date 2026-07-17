// =============================================================================
// ThemeScanPanel.tsx — the background Theme Scan driver on the scenario page.
// -----------------------------------------------------------------------------
// Pick a model (Opus / Qwen-14B), toggle benchmark mode (dry-run), Run. The POST
// returns a run_id immediately; we POLL the GET every 3s (the DocumentsPage
// idiom) and render three states:
//   SETUP    — model radio-cards + benchmark toggle + Run button.
//   RUNNING  — live "X of N judged" + mono elapsed timer + progress bar + tiles.
//   COMPLETE — Complete pill + duration; counts; top relevant findings; and, when
//              a second model has been run, a HERO agreement block comparing them.
//
// Two runs (e.g. Opus + Qwen-14B) accumulate in session state so they sit side by
// side. Every color comes from tokens.css (no hardcoded hex — "elevation via
// borders", not shadows). Reuses PipelineProgressBar (the bar) and EvidenceCard
// (findings); the status pills are purpose-built for the scan states.
// =============================================================================

import React, { useCallback, useEffect, useRef, useState } from "react";

import EvidenceCard from "../pages/BiasExplorer/EvidenceCard";
import PipelineProgressBar from "./pipeline/PipelineProgressBar";
import RunHistoryList from "./RunHistoryList";
import { computeAgreement, costLabel, formatElapsed } from "./themeScanFormat";
import { gatherCandidates } from "../services/scenarioGather";
import {
  deleteScanRun,
  fetchScanModels,
  fetchScanRuns,
  getScanRun,
  startThemeScan,
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

interface Props {
  slug: string;
  scenarioId: string;
  scenarioTitle: string;
}

const ThemeScanPanel: React.FC<Props> = ({ slug, scenarioId, scenarioTitle }) => {
  const [models, setModels] = useState<ScanModel[]>([]);
  const [selectedModel, setSelectedModel] = useState<string | null>(null);
  const [benchmarkMode, setBenchmarkMode] = useState(true); // dry-run default ON
  const [candidateCount, setCandidateCount] = useState<number | null>(null);

  const [activeRun, setActiveRun] = useState<{ runId: string; modelId: string } | null>(null);
  const [poll, setPoll] = useState<ScanRunStatus | null>(null);
  const [elapsedMs, setElapsedMs] = useState(0);
  const startedAtRef = useRef<number>(0);

  const [startError, setStartError] = useState<string | null>(null);
  // A model-catalog load failure gets its OWN observable state, distinct from a
  // genuinely-empty registry (Standing Rule 1 — the two states must look different).
  const [modelError, setModelError] = useState<string | null>(null);

  // ── Run history is the SOURCE OF TRUTH, hydrated from the DB (not session) ──
  // `runs` are the persisted headers (newest first) — they survive navigation and
  // reloads, replacing the old ephemeral `completed` map. `summaries` is a LAZY
  // cache of each run's full result, filled by clicking a row (getScanRun).
  // `selectedRunIds` (0 or 1 — single-select) drives which run renders.
  const [runs, setRuns] = useState<ScanRunHeader[]>([]);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const [summaries, setSummaries] = useState<Record<string, ThemeScanSummary>>({});
  const [selectedRunIds, setSelectedRunIds] = useState<string[]>([]);
  // A per-run detail-load failure is distinct from the list-load failure above.
  const [detailError, setDetailError] = useState<string | null>(null);

  // Re-read the persisted history (after a scan finishes, or on mount).
  const refreshRuns = useCallback(() => {
    fetchScanRuns(slug, scenarioId)
      .then((rs) => {
        setRuns(rs);
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
      .then((ms) => {
        setModels(ms);
        setSelectedModel((cur) => cur ?? ms.find((m) => m.is_default)?.model_id ?? ms[0]?.model_id ?? null);
      })
      .catch((e: unknown) => {
        // A load failure is NOT an empty registry — surface it so the operator
        // can tell "backend/auth problem" from "no models configured".
        setModelError(e instanceof Error ? e.message : "Failed to load the model catalog.");
      });
    gatherCandidates(slug, scenarioId)
      .then((g) => setCandidateCount(g.pool.length + g.dropped.length))
      .catch((e: unknown) => {
        // The pre-scan count is cosmetic (candidates_total arrives with the run),
        // so it degrades to "no count shown" — but the failure is still logged so
        // it is observable in the console during diagnostics (Standing Rule 1).
        console.warn("Theme Scan: candidate-count fetch failed:", e);
        setCandidateCount(null);
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
        dry_run: benchmarkMode,
      });
      setCandidateCount(started.candidates_total);
      setPoll(null);
      setActiveRun({ runId: started.run_id, modelId: selectedModel });
    } catch (e) {
      // Verbatim backend message (names the endpoint / both models on a 503 gate).
      setStartError(e instanceof Error ? e.message : "Failed to start the scan.");
    }
  }, [selectedModel, benchmarkMode, slug, scenarioId]);

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

  // ── Delete a history run ────────────────────────────────────────────────────
  // The row owns the confirm; the panel owns the network call, its error UI, and
  // the post-delete state cleanup (Standing Rule 1 — a failed delete is surfaced
  // in the history error box, never swallowed). On success: re-hydrate the history
  // from the DB (the run is now gone), and if the deleted run was the one open
  // below, clear the selection and drop its cached summary so the results area
  // does not render a run that no longer exists.
  const onDeleteRun = useCallback(
    async (runId: string) => {
      try {
        await deleteScanRun(slug, scenarioId, runId);
      } catch (e) {
        setHistoryError(e instanceof Error ? e.message : "Failed to delete the run.");
        return;
      }
      refreshRuns();
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

  return (
    <section style={S.card}>
      {/* Keyframes for the "Scanning" pulse dot — inlined like ProcessingPanel's
          colossus-spin, so the animation ships with the component. */}
      <style>{`@keyframes colossus-pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.3; } }`}</style>
      <header style={S.header}>
        <div>
          <div style={S.title}>Theme Scan</div>
          <div style={S.subtitle}>
            {scenarioTitle}
            {candidateCount != null && ` · ${candidateCount} candidates gathered`}
          </div>
        </div>
      </header>

      {running ? (
        <RunningView poll={poll} modelName={modelName(activeRun.modelId)} elapsedMs={elapsedMs} />
      ) : (
        <SetupView
          models={models}
          modelError={modelError}
          selectedModel={selectedModel}
          onSelect={setSelectedModel}
          benchmarkMode={benchmarkMode}
          onToggleBenchmark={() => setBenchmarkMode((b) => !b)}
          onRun={onRun}
        />
      )}

      {startError && (
        <div style={S.errorBox} role="alert">
          {startError}
        </div>
      )}

      {/* Run history hydrated from the DB — survives navigation and reloads. */}
      <RunHistoryList
        runs={runs}
        selectedRunIds={selectedRunIds}
        onToggle={onSelectRun}
        onDelete={onDeleteRun}
        modelName={modelName}
      />
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

      {/* The EXISTING results display + comparison hero, fed by the selected runs
          (one → a single result; two → the hero). */}
      {hasSelectedResults && (
        <ResultsArea completed={selectedSummaries} modelName={modelName} />
      )}
    </section>
  );
};

// ─── SETUP ────────────────────────────────────────────────────────────────────

const SetupView: React.FC<{
  models: ScanModel[];
  modelError: string | null;
  selectedModel: string | null;
  onSelect: (id: string) => void;
  benchmarkMode: boolean;
  onToggleBenchmark: () => void;
  onRun: () => void;
}> = ({ models, modelError, selectedModel, onSelect, benchmarkMode, onToggleBenchmark, onRun }) => (
  <div style={S.setup}>
    <div style={S.sectionLabel}>Model</div>
    {modelError && (
      <div style={S.errorBox} role="alert">
        Could not load models — {modelError}
      </div>
    )}
    <div style={S.modelGrid}>
      {models.length === 0 && !modelError && (
        <div style={S.muted}>No active models available.</div>
      )}
      {models.map((m) => {
        const selected = m.model_id === selectedModel;
        return (
          <button
            key={m.model_id}
            type="button"
            onClick={() => onSelect(m.model_id)}
            style={{ ...S.radioCard, ...(selected ? S.radioCardSelected : {}) }}
          >
            <span style={{ ...S.radioDot, ...(selected ? S.radioDotSelected : {}) }} />
            <span style={S.radioName}>{m.display_name}</span>
            {m.is_default && <span style={S.radioBadge}>default</span>}
          </button>
        );
      })}
    </div>

    <label style={S.toggleRow}>
      <input type="checkbox" checked={benchmarkMode} onChange={onToggleBenchmark} />
      <span style={S.toggleLabel}>
        Benchmark mode
        <span style={S.muted}> — record verdicts without saving suggestions to the scenario</span>
      </span>
    </label>

    <button type="button" onClick={onRun} disabled={!selectedModel} style={S.runButton}>
      Run Theme Scan
    </button>
  </div>
);

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

const ResultsArea: React.FC<{
  // Keyed by run_id (a run's stable identity — the same model can appear twice in
  // history). The display name comes from each summary's own `model_id`, NOT the
  // record key, so a run still labels correctly.
  completed: Record<string, ThemeScanSummary>;
  modelName: (id: string) => string;
}> = ({ completed, modelName }) => {
  const runs = Object.entries(completed);
  const showHero = runs.length >= 2;
  const [a, b] = runs.map(([, s]) => s);

  return (
    <div style={S.results}>
      {showHero && (
        <div style={S.sticky}>
          <ComparisonHero a={a} b={b} modelName={modelName} />
        </div>
      )}
      {runs.map(([runId, summary]) => (
        <RunResult key={runId} summary={summary} modelName={modelName(summary.model_id)} />
      ))}
    </div>
  );
};

const ComparisonHero: React.FC<{
  a: ThemeScanSummary;
  b: ThemeScanSummary;
  modelName: (id: string) => string;
}> = ({ a, b, modelName }) => {
  const { relevantPct, rolePct, sharedCount } = computeAgreement(a, b);
  return (
    <div style={S.hero}>
      <div style={S.heroEyebrow}>Relevant-set overlap · at-a-glance estimate</div>
      <div style={S.heroPct}>{relevantPct}%</div>
      <div style={S.heroLabel}>
        {rolePct}% role agreement on {sharedCount} shared quotes
      </div>
      {/* Flag 3: this client-side Jaccard is an ESTIMATE, NOT the promotion
          number. The authoritative agreement (relevant AND role over the full
          ~94-verdict set) is the scan_run_verdicts SQL join at L3 — never this. */}
      <div style={S.heroCaption}>
        Estimate only — not the promotion number. The authoritative agreement is the
        full-verdict-set SQL comparison (scan_run_verdicts), run separately.
      </div>
      <div style={S.heroCompare}>
        <HeroSide summary={a} modelName={modelName(a.model_id)} />
        <span style={S.heroVs}>vs</span>
        <HeroSide summary={b} modelName={modelName(b.model_id)} />
      </div>
    </div>
  );
};

const HeroSide: React.FC<{ summary: ThemeScanSummary; modelName: string }> = ({
  summary,
  modelName,
}) => (
  <div style={S.heroSide}>
    <div style={S.heroSideName}>{modelName}</div>
    <div style={S.heroSideMeta}>
      {costLabel(summary)} · {formatElapsed(summary.duration_ms)}
    </div>
  </div>
);

const RunResult: React.FC<{ summary: ThemeScanSummary; modelName: string }> = ({
  summary,
  modelName,
}) => {
  // The judge fans out with `buffer_unordered`, so `suggestions` arrives in
  // completion order, not ranked. Present the STRONGEST findings first: sort a
  // COPY (spread before sort — Array.prototype.sort mutates in place, and the
  // source array lives in the cached summary) by confidence descending. Every
  // suggestion is a RELEVANT verdict, so each carries a confidence — no
  // null-guard is needed here (unlike a nullable field, this is always present).
  const rankedSuggestions = [...summary.suggestions].sort((a, b) => b.confidence - a.confidence);

  return (
    <div style={S.runResult}>
      <div style={S.runResultHead}>
        <span style={S.modelChip}>{modelName}</span>
        <span style={S.completePill}>Complete</span>
        <span style={S.muted}>{formatElapsed(summary.duration_ms)}</span>
      </div>

      <div style={{ ...S.tileRow, ...S.sticky }}>
        <LiveTile label="Read" value={summary.candidates_read} tone="muted" />
        <LiveTile label="Relevant" value={summary.relevant_written} tone="success" />
        <LiveTile label="Not relevant" value={summary.irrelevant} tone="muted" />
        <LiveTile label="Failed" value={summary.failed} tone="danger" />
      </div>

      <div style={S.findingsHead}>Top relevant findings</div>
      {rankedSuggestions.length === 0 && <div style={S.muted}>No relevant quotes found.</div>}
      {rankedSuggestions.map((sug) => (
        <div key={sug.graph_node_id} style={S.finding}>
          <EvidenceCard
            instance={sug.content}
            action={
              <span style={S.roleBadge}>
                {sug.proposed_role} · {Math.round(sug.confidence * 100)}%
              </span>
            }
          />
        </div>
      ))}
    </div>
  );
};

// ─── styling (tokens.css only) ────────────────────────────────────────────────

function toneColor(tone: "success" | "muted" | "danger"): string {
  if (tone === "success") return "var(--state-success-strong)";
  if (tone === "danger") return "var(--state-danger-strong)";
  return "var(--text-secondary)";
}

const S: Record<string, React.CSSProperties> = {
  card: {
    fontFamily: "var(--font-sans)",
    background: "var(--bg-surface)",
    border: "1px solid var(--border-default)",
    borderRadius: "12px",
    padding: "20px",
    marginBottom: "1.5rem",
  },
  header: { display: "flex", justifyContent: "space-between", marginBottom: "16px" },
  title: { fontSize: "1.05rem", fontWeight: 600, color: "var(--text-primary)" },
  subtitle: { fontSize: "0.85rem", color: "var(--text-muted)", marginTop: "2px" },
  muted: { color: "var(--text-muted)", fontSize: "0.82rem" },

  setup: { display: "flex", flexDirection: "column", gap: "14px" },
  sectionLabel: {
    fontSize: "0.72rem",
    fontWeight: 600,
    textTransform: "uppercase",
    letterSpacing: "0.04em",
    color: "var(--text-muted)",
  },
  modelGrid: { display: "grid", gridTemplateColumns: "1fr 1fr", gap: "10px" },
  radioCard: {
    display: "flex",
    alignItems: "center",
    gap: "10px",
    padding: "14px 16px",
    background: "var(--bg-page)",
    border: "1px solid var(--border-default)",
    borderRadius: "10px",
    cursor: "pointer",
    textAlign: "left",
    fontFamily: "var(--font-sans)",
  },
  radioCardSelected: {
    borderColor: "var(--accent-primary)",
    boxShadow: "inset 0 0 0 1px var(--accent-primary)",
  },
  radioDot: {
    width: "14px",
    height: "14px",
    borderRadius: "50%",
    border: "2px solid var(--border-default)",
    flexShrink: 0,
  },
  radioDotSelected: {
    borderColor: "var(--accent-primary)",
    background:
      "radial-gradient(circle, var(--accent-primary) 0 40%, transparent 45%)",
  },
  radioName: { fontSize: "0.9rem", fontWeight: 500, color: "var(--text-primary)", flex: 1 },
  radioBadge: {
    fontSize: "0.68rem",
    color: "var(--text-muted)",
    border: "1px solid var(--border-default)",
    borderRadius: "6px",
    padding: "1px 6px",
  },
  toggleRow: { display: "flex", alignItems: "flex-start", gap: "9px", cursor: "pointer" },
  toggleLabel: { fontSize: "0.86rem", color: "var(--text-primary)" },
  runButton: {
    alignSelf: "flex-start",
    background: "var(--accent-primary)",
    color: "var(--bg-surface)", // #fff per the design token mapping (on-accent text)
    border: "none",
    borderRadius: "8px",
    padding: "10px 20px",
    fontSize: "0.9rem",
    fontWeight: 600,
    cursor: "pointer",
    fontFamily: "var(--font-sans)",
  },

  running: { display: "flex", flexDirection: "column", gap: "12px" },
  runningTop: { display: "flex", alignItems: "center", gap: "10px" },
  modelChip: {
    fontSize: "0.8rem",
    fontWeight: 600,
    color: "var(--text-secondary)",
    background: "var(--bg-page)",
    border: "1px solid var(--border-default)",
    borderRadius: "6px",
    padding: "3px 9px",
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
    background: "var(--bg-page)",
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
    background: "var(--bg-page)",
    border: "1px solid var(--state-danger-strong)",
    borderRadius: "8px",
    color: "var(--state-danger-strong)",
    fontSize: "0.85rem",
  },

  results: { marginTop: "18px", display: "flex", flexDirection: "column", gap: "16px" },
  sticky: {
    position: "sticky",
    top: 0,
    background: "var(--bg-surface)",
    zIndex: 1,
    paddingTop: "4px",
    paddingBottom: "4px",
  },
  hero: {
    background: "var(--bg-page)",
    border: "1px solid var(--border-default)",
    borderRadius: "12px",
    padding: "18px 20px",
    textAlign: "center",
  },
  heroEyebrow: {
    fontSize: "0.72rem",
    fontWeight: 600,
    textTransform: "uppercase",
    letterSpacing: "0.05em",
    color: "var(--text-muted)",
  },
  heroPct: { fontSize: "2.4rem", fontWeight: 800, color: "var(--accent-primary)" },
  heroLabel: { fontSize: "0.82rem", color: "var(--text-secondary)" },
  heroCaption: {
    fontSize: "0.72rem",
    fontStyle: "italic",
    color: "var(--text-muted)",
    maxWidth: "42ch",
    margin: "6px auto 12px",
  },
  heroCompare: { display: "flex", alignItems: "center", justifyContent: "center", gap: "16px" },
  heroSide: { textAlign: "center" },
  heroSideName: { fontSize: "0.85rem", fontWeight: 600, color: "var(--text-primary)" },
  heroSideMeta: { fontSize: "0.8rem", color: "var(--text-secondary)" },
  heroVs: { fontSize: "0.8rem", color: "var(--text-muted)" },

  runResult: {
    border: "1px solid var(--border-default)",
    borderRadius: "12px",
    padding: "16px",
    background: "var(--bg-surface)",
  },
  runResultHead: { display: "flex", alignItems: "center", gap: "10px", marginBottom: "12px" },
  completePill: {
    fontSize: "0.78rem",
    fontWeight: 600,
    color: "var(--state-success-strong)",
    background: "var(--bg-page)",
    border: "1px solid var(--state-success-strong)",
    borderRadius: "999px",
    padding: "2px 10px",
  },
  findingsHead: {
    fontSize: "0.78rem",
    fontWeight: 600,
    textTransform: "uppercase",
    letterSpacing: "0.04em",
    color: "var(--text-muted)",
    margin: "14px 0 8px",
  },
  finding: { marginBottom: "8px" },
  roleBadge: {
    fontSize: "0.72rem",
    fontWeight: 600,
    color: "var(--accent-primary)",
    background: "var(--bg-page)",
    border: "1px solid var(--border-default)",
    borderRadius: "6px",
    padding: "2px 8px",
    whiteSpace: "nowrap",
  },
};

export default ThemeScanPanel;
