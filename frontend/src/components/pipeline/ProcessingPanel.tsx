/**
 * ProcessingPanel — Pipeline step cards and execution history.
 *
 * Extracted from PipelineDocumentDetail so it can be used as a tab
 * inside DocumentWorkspaceTabs. Contains the step definitions, trigger
 * map, and next-action logic that were previously inline.
 */
import React, { useState } from "react";
import { useNavigate } from "react-router-dom";
import StepCard, { StepDef } from "./StepCard";
import ExecutionHistory from "./ExecutionHistory";
import {
  triggerExtractText, triggerExtract, triggerVerify,
  triggerIngest, triggerIndex, fetchCompleteness,
  PipelineDocument, PipelineStep,
} from "../../services/pipelineApi";

// ── Step definitions ────────────────────────────────────────────

const PIPELINE_STEPS: StepDef[] = [
  { name: "upload", label: "Upload", statusRequired: null },
  { name: "extract_text", label: "Extract Text", statusRequired: "UPLOADED" },
  { name: "extract", label: "LLM Extract", statusRequired: "TEXT_EXTRACTED" },
  { name: "verify", label: "Verify", statusRequired: "EXTRACTED" },
  { name: "review", label: "Review", statusRequired: "VERIFIED", isManual: true },
  { name: "ingest", label: "Ingest", statusRequired: "VERIFIED" },
  { name: "index", label: "Index", statusRequired: "INGESTED" },
  { name: "completeness", label: "Completeness", statusRequired: "INDEXED" },
];

const TRIGGER_MAP: Record<string, (id: string) => Promise<unknown>> = {
  extract_text: triggerExtractText,
  extract: triggerExtract,
  verify: triggerVerify,
  ingest: triggerIngest,
  index: triggerIndex,
  completeness: fetchCompleteness,
};

// ── Helpers ─────────────────────────────────────────────────────

function latestEntry(history: PipelineStep[], stepName: string): PipelineStep | undefined {
  return history.find((h) => h.step_name === stepName);
}

function findNextAction(docStatus: string, history: PipelineStep[]): string | null {
  for (const step of PIPELINE_STEPS) {
    if (step.statusRequired === null) continue;
    const entry = latestEntry(history, step.name);
    if (entry && entry.status === "completed") continue;
    if (step.statusRequired === docStatus) return step.name;
  }
  return null;
}

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
  const navigate = useNavigate();
  const [running, setRunning] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);

  const handleTrigger = async (stepName: string) => {
    if (running) return;
    const triggerFn = TRIGGER_MAP[stepName];
    if (!triggerFn) return;
    setRunning(true);
    setActionError(null);
    try {
      await triggerFn(doc.id);
      onStepTriggered();
    } catch (e) {
      setActionError(e instanceof Error ? e.message : `Step '${stepName}' failed`);
    } finally {
      setRunning(false);
    }
  };

  const handleNavigate = (stepName: string) => {
    if (stepName === "review" && onSwitchTab) {
      onSwitchTab("review");
    }
  };

  const nextAction = findNextAction(doc.status, history);

  return (
    <div>
      {actionError && <div style={errorBox}>{actionError}</div>}

      <div style={stepsContainer}>
        <div style={stepsHeader}>Pipeline Steps</div>
        {PIPELINE_STEPS.map((step, i) => {
          const entry = step.name === "upload"
            ? { id: 0, document_id: doc.id, step_name: "upload", status: "completed",
                started_at: doc.created_at, completed_at: doc.created_at,
                duration_secs: null, triggered_by: null, input_params: {},
                result_summary: {}, error_message: null } as PipelineStep
            : latestEntry(history, step.name);

          return (
            <StepCard
              key={step.name}
              step={step}
              index={i}
              historyEntry={entry}
              documentStatus={doc.status}
              isNextAction={step.name === nextAction ||
                (step.name === "review" && doc.status === "VERIFIED" && nextAction === "ingest")}
              running={running}
              onTrigger={handleTrigger}
              onNavigate={handleNavigate}
            />
          );
        })}
      </div>

      <ExecutionHistory steps={history} />
    </div>
  );
};

export default ProcessingPanel;
