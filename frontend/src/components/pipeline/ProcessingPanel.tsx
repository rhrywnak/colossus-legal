/**
 * ProcessingPanel — Pipeline step cards and execution controls.
 *
 * Fetches available actions from the backend state machine
 * (GET /documents/:id/actions). The frontend renders what the backend
 * provides — zero status string checks for decision-making.
 */
import React, { useCallback, useEffect, useState } from "react";
import StepCard from "./StepCard";
import ExecutionHistory from "./ExecutionHistory";
import {
  triggerExtractText, triggerExtract, triggerVerify,
  triggerIngest, triggerIndex, fetchCompleteness,
  fetchDocumentActions,
  PipelineDocument, PipelineStep, DocumentActions,
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

const stepsContainer: React.CSSProperties = {
  backgroundColor: "#ffffff", borderRadius: "8px", border: "1px solid #e2e8f0",
  overflow: "hidden",
};
const stepsHeader: React.CSSProperties = {
  padding: "0.6rem 0.85rem", fontWeight: 600, fontSize: "0.84rem", color: "#334155",
  backgroundColor: "#f8fafc", borderBottom: "1px solid #e2e8f0",
};
const errorBox: React.CSSProperties = {
  padding: "0.6rem 1rem", backgroundColor: "#fef2f2", border: "1px solid #fecaca",
  borderRadius: "6px", color: "#991b1b", fontSize: "0.84rem", marginBottom: "1rem",
};
const actionBtnStyle = (enabled: boolean): React.CSSProperties => ({
  padding: "0.4rem 0.9rem", fontSize: "0.8rem", fontWeight: 600,
  border: "1px solid #2563eb", borderRadius: "6px", cursor: enabled ? "pointer" : "default",
  backgroundColor: enabled ? "#2563eb" : "#e2e8f0",
  color: enabled ? "#ffffff" : "#94a3b8",
  fontFamily: "inherit", opacity: enabled ? 1 : 0.6,
});
const actionsRow: React.CSSProperties = {
  display: "flex", gap: "0.5rem", padding: "0.75rem 0.85rem",
  borderTop: "1px solid #e2e8f0", backgroundColor: "#f8fafc",
};

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
    } catch {
      // Non-fatal — fall back to showing history only
    }
  }, [doc.id]);

  // Load actions on mount and when document status changes
  useEffect(() => { loadActions(); }, [loadActions, doc.status]);

  const handleAction = async (actionName: string, isNavigation: boolean) => {
    if (isNavigation) {
      if (actionName === "review" && onSwitchTab) {
        onSwitchTab("review");
      }
      return;
    }

    const triggerFn = TRIGGER_MAP[actionName];
    if (!triggerFn || running) return;
    setRunning(true);
    setActionError(null);
    try {
      await triggerFn(doc.id);
      onStepTriggered();
      // Refresh actions after step completes
      await loadActions();
    } catch (e) {
      setActionError(e instanceof Error ? e.message : `Step '${actionName}' failed`);
    } finally {
      setRunning(false);
    }
  };

  // Build completed steps list from history for display
  // The upload step is synthetic — always shown as completed
  const uploadStep = {
    step_name: "upload", label: "Upload", status: "completed",
    duration_secs: null, result_summary: null, error_message: null,
  };
  const completedSteps = actions?.completed_steps ?? [];
  const allSteps = [uploadStep, ...completedSteps];

  return (
    <div>
      {actionError && <div style={errorBox}>{actionError}</div>}

      <div style={stepsContainer}>
        <div style={stepsHeader}>Pipeline Steps</div>

        {/* Completed steps */}
        {allSteps.map((step, i) => (
          <StepCard key={`${step.step_name}-${i}`} step={step} index={i} />
        ))}

        {/* Available actions from backend */}
        {actions && actions.available_actions.length > 0 && (
          <div style={actionsRow}>
            {actions.available_actions.map((action) => (
              <button
                key={action.action}
                style={actionBtnStyle(!running)}
                disabled={running}
                onClick={() => handleAction(action.action, action.is_navigation)}
                title={action.description}
              >
                {running ? "Running..." : action.label}
                {action.is_navigation ? " →" : ""}
              </button>
            ))}
          </div>
        )}
      </div>

      <ExecutionHistory steps={history} />
    </div>
  );
};

export default ProcessingPanel;
