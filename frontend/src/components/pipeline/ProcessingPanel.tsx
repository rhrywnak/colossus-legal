/**
 * ProcessingPanel — Fixed 8-stage pipeline display with inline actions.
 *
 * Fetches pipeline stages from the backend state machine. Always shows
 * exactly 8 stages in order. Action buttons appear inline on the
 * actionable stage. Execution history is a collapsible section below.
 */
import React, { useCallback, useEffect, useState } from "react";
import ExecutionHistory from "./ExecutionHistory";
import {
  triggerExtractText, triggerExtract, triggerVerify,
  triggerIngest, triggerIndex, fetchCompleteness,
  fetchDocumentActions,
  PipelineDocument, PipelineStep, DocumentActions, PipelineStage,
} from "../../services/pipelineApi";

// Map action names to API trigger functions
const TRIGGER_MAP: Record<string, (id: string) => Promise<unknown>> = {
  extract_text: triggerExtractText,
  extract: triggerExtract,
  verify: triggerVerify,
  ingest: triggerIngest,
  index: triggerIndex,
  completeness: fetchCompleteness,
};

// ── Styles ──────────────────────────────────────────────────────

const containerStyle: React.CSSProperties = {
  backgroundColor: "#ffffff", borderRadius: "8px", border: "1px solid #e2e8f0",
  overflow: "hidden",
};
const headerStyle: React.CSSProperties = {
  padding: "0.6rem 0.85rem", fontWeight: 600, fontSize: "0.84rem", color: "#334155",
  backgroundColor: "#f8fafc", borderBottom: "1px solid #e2e8f0",
};
const errorBox: React.CSSProperties = {
  padding: "0.6rem 1rem", backgroundColor: "#fef2f2", border: "1px solid #fecaca",
  borderRadius: "6px", color: "#991b1b", fontSize: "0.84rem", marginBottom: "1rem",
};
const rowStyle: React.CSSProperties = {
  display: "flex", alignItems: "center", gap: "0.75rem",
  padding: "0.55rem 0.85rem", borderBottom: "1px solid #f1f5f9",
};
const iconCol: React.CSSProperties = {
  width: "24px", textAlign: "center", fontSize: "0.9rem", flexShrink: 0,
};
const labelCol: React.CSSProperties = {
  fontWeight: 600, fontSize: "0.84rem", color: "#0f172a", minWidth: "140px",
};
const durationCol: React.CSSProperties = {
  fontSize: "0.76rem", color: "#64748b", minWidth: "55px",
};
const summaryCol: React.CSSProperties = {
  flex: 1, fontSize: "0.76rem", color: "#64748b",
};
const actionBtn = (enabled: boolean): React.CSSProperties => ({
  padding: "0.25rem 0.65rem", fontSize: "0.76rem", fontWeight: 600,
  border: "1px solid #2563eb", borderRadius: "6px", cursor: enabled ? "pointer" : "default",
  backgroundColor: enabled ? "#2563eb" : "#e2e8f0",
  color: enabled ? "#ffffff" : "#94a3b8",
  fontFamily: "inherit", flexShrink: 0, whiteSpace: "nowrap",
});
const historySection: React.CSSProperties = {
  marginTop: "1rem", backgroundColor: "#ffffff", borderRadius: "8px",
  border: "1px solid #e2e8f0", overflow: "hidden",
};
const historySummary: React.CSSProperties = {
  padding: "0.6rem 0.85rem", fontSize: "0.84rem", fontWeight: 600,
  color: "#334155", cursor: "pointer", backgroundColor: "#f8fafc",
};

// ── Helpers ─────────────────────────────────────────────────────

function stageIcon(status: string): string {
  switch (status) {
    case "completed": return "\u2705";
    case "available": return "\uD83D\uDD35";
    case "failed": return "\u274C";
    default: return "\u2B1C";
  }
}

function iconColor(status: string): string {
  switch (status) {
    case "completed": return "#22c55e";
    case "available": return "#2563eb";
    case "failed": return "#ef4444";
    default: return "#cbd5e1";
  }
}

function formatDuration(secs: number | null): string {
  if (secs == null) return "\u2014";
  if (secs < 60) return `${secs.toFixed(1)}s`;
  const mins = Math.floor(secs / 60);
  const s = Math.round(secs % 60);
  return `${mins}m ${s}s`;
}

// ── Component ───────────────────────────────────────────────────

interface ProcessingPanelProps {
  document: PipelineDocument;
  history: PipelineStep[];
  onStepTriggered: () => void;
  onSwitchTab?: (tabId: string) => void;
}

const ProcessingPanel: React.FC<ProcessingPanelProps> = ({
  document: doc, history, onStepTriggered, onSwitchTab,
}) => {
  const [running, setRunning] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [actions, setActions] = useState<DocumentActions | null>(null);

  const loadActions = useCallback(async () => {
    try {
      const data = await fetchDocumentActions(doc.id);
      setActions(data);
    } catch { /* non-fatal */ }
  }, [doc.id]);

  useEffect(() => { loadActions(); }, [loadActions, doc.status]);

  const handleAction = async (stage: PipelineStage) => {
    if (!stage.action) return;

    if (stage.action.is_navigation) {
      if (stage.action.action === "review" && onSwitchTab) {
        onSwitchTab("review");
      }
      return;
    }

    const triggerFn = TRIGGER_MAP[stage.action.action];
    if (!triggerFn || running) return;
    setRunning(true);
    setActionError(null);
    try {
      await triggerFn(doc.id);
      onStepTriggered();
      await loadActions();
    } catch (e) {
      setActionError(e instanceof Error ? e.message : `Step '${stage.action.action}' failed`);
      await loadActions();
    } finally {
      setRunning(false);
    }
  };

  const stages = actions?.pipeline_stages ?? [];
  const execHistory = actions?.execution_history ?? [];

  return (
    <div>
      {actionError && <div style={errorBox}>{actionError}</div>}

      <div style={containerStyle}>
        <div style={headerStyle}>Pipeline Steps</div>

        {stages.map((stage) => (
          <div key={stage.name} style={rowStyle}>
            <span style={{ ...iconCol, color: iconColor(stage.status) }}>
              {stageIcon(stage.status)}
            </span>
            <span style={{
              ...labelCol,
              color: stage.status === "pending" ? "#94a3b8" : "#0f172a",
            }}>
              {stage.order}. {stage.label}
            </span>
            <span style={durationCol}>
              {stage.status === "completed" ? formatDuration(stage.duration_secs) : "\u2014"}
            </span>
            <span style={{
              ...summaryCol,
              color: stage.status === "failed" ? "#ef4444" : "#64748b",
            }}>
              {stage.summary ?? ""}
            </span>
            {stage.action && (
              <button
                style={actionBtn(!running)}
                disabled={running}
                onClick={() => handleAction(stage)}
              >
                {running ? "Running..." : stage.action.label}
                {stage.action.is_navigation ? " \u2192" : ""}
              </button>
            )}
          </div>
        ))}
      </div>

      {/* Collapsible execution history */}
      {execHistory.length > 0 && (
        <details style={historySection}>
          <summary style={historySummary}>
            Execution History ({execHistory.length} entries)
          </summary>
          <ExecutionHistory steps={history} />
        </details>
      )}
    </div>
  );
};

export default ProcessingPanel;
