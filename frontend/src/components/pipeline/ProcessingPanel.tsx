/**
 * ProcessingPanel — Fixed 8-stage pipeline display with inline actions.
 *
 * Fetches pipeline stages from the backend state machine. Always shows
 * exactly 8 stages in order. Action buttons appear inline on the
 * actionable stage. Execution history is a collapsible section below.
 */
import React, { useCallback, useEffect, useState } from "react";
import ExecutionHistory from "./ExecutionHistory";
import ActionConfirmDialog from "./ActionConfirmDialog";
import ExtractionConfigDialog from "./ExtractionConfigDialog";
import {
  triggerPipelineAction,
  fetchDocumentActions,
  PipelineDocument, PipelineStep, DocumentActions, PipelineStage, AvailableAction,
} from "../../services/pipelineApi";

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
const docActionBtn = (enabled: boolean): React.CSSProperties => ({
  padding: "0.3rem 0.75rem", fontSize: "0.76rem", fontWeight: 600,
  border: "1px solid #d97706", borderRadius: "6px",
  cursor: enabled ? "pointer" : "default",
  backgroundColor: enabled ? "#fff" : "#e2e8f0",
  color: enabled ? "#d97706" : "#94a3b8",
  fontFamily: "inherit",
});
const docActionsRow: React.CSSProperties = {
  padding: "0.6rem 0.85rem", display: "flex", gap: "0.5rem", flexWrap: "wrap",
  borderTop: "1px solid #f1f5f9", backgroundColor: "#fafafa",
};

// ── Helpers ─────────────────────────────────────────────────────

const ICONS: Record<string, string> = {
  completed: "\u2705", available: "\uD83D\uDD35", failed: "\u274C",
};
const ICON_COLORS: Record<string, string> = {
  completed: "#22c55e", available: "#2563eb", failed: "#ef4444",
};
const stageIcon = (s: string) => ICONS[s] ?? "\u2B1C";
const iconColor = (s: string) => ICON_COLORS[s] ?? "#cbd5e1";

function formatDuration(secs: number | null): string {
  if (secs == null) return "\u2014";
  if (secs < 60) return `${secs.toFixed(1)}s`;
  return `${Math.floor(secs / 60)}m ${Math.round(secs % 60)}s`;
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
  const [confirmAction, setConfirmAction] = useState<AvailableAction | null>(null);
  const [showExtractConfig, setShowExtractConfig] = useState(false);

  const loadActions = useCallback(async () => {
    try {
      const data = await fetchDocumentActions(doc.id);
      setActions(data);
    } catch { /* non-fatal */ }
  }, [doc.id]);

  useEffect(() => { loadActions(); }, [loadActions, doc.status]);

  const runAction = async (
    action: AvailableAction,
    body?: Record<string, unknown>,
  ) => {
    if (running) return;
    setRunning(true);
    setActionError(null);
    setConfirmAction(null);
    try {
      await triggerPipelineAction(doc.id, action.endpoint, action.method, body);
      onStepTriggered();
      await loadActions();
    } catch (e) {
      setActionError(e instanceof Error ? e.message : `Action '${action.label}' failed`);
      await loadActions();
    } finally {
      setRunning(false);
    }
  };

  const handleAction = (stage: PipelineStage) => {
    if (!stage.action) return;
    if (stage.action.is_navigation) {
      if (stage.action.action === "review" && onSwitchTab) onSwitchTab("review");
      return;
    }
    if (stage.action.action === "extract") {
      setShowExtractConfig(true);
      return;
    }
    if (stage.action.requires_confirmation) {
      setConfirmAction(stage.action);
      return;
    }
    void runAction(stage.action);
  };

  const handleDocAction = (action: AvailableAction) => {
    if (action.requires_confirmation) {
      setConfirmAction(action);
      return;
    }
    void runAction(action);
  };

  const stages = actions?.pipeline_stages ?? [];
  const stageActionNames = new Set(
    stages.filter((s) => s.action).map((s) => s.action!.action)
  );
  const docActions = (actions?.available_actions ?? []).filter(
    (a) => !stageActionNames.has(a.action)
  );

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

        {docActions.length > 0 && (
          <div style={docActionsRow}>
            {docActions.map((a) => (
              <button
                key={a.action}
                style={docActionBtn(!running)}
                disabled={running}
                onClick={() => handleDocAction(a)}
                title={a.description}
              >
                {running ? "Running..." : a.label}
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Collapsible execution history — single instance (ISS-010) */}
      {history.length > 0 && <ExecutionHistory steps={history} />}
      {confirmAction && (
        <ActionConfirmDialog
          action={confirmAction}
          onCancel={() => setConfirmAction(null)}
          onConfirm={() => void runAction(confirmAction)}
        />
      )}
      {showExtractConfig && (
        <ExtractionConfigDialog
          documentId={doc.id}
          onCancel={() => setShowExtractConfig(false)}
          onSubmit={async (config) => {
            setShowExtractConfig(false);
            const extractStage = stages.find((s) => s.action?.action === "extract");
            if (extractStage?.action) await runAction(extractStage.action, config);
          }}
        />
      )}
    </div>
  );
};

export default ProcessingPanel;
