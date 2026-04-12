/**
 * ProcessingPanel — Displays content based on doc.status_group.
 *
 * Five statuses: new, processing, completed, failed, cancelled.
 * Fetches execution history from the actions endpoint on mount
 * and when the document status changes.
 */
import React, { useCallback, useEffect, useRef, useState } from "react";
import ExecutionHistory from "./ExecutionHistory";
import ReprocessDialog from "./ReprocessDialog";
import {
  fetchDocumentActions,
  processDocument,
  cancelProcessing,
  PipelineDocument,
  DocumentActions,
  ExecutionHistoryEntry,
  PipelineStep,
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
const bodyStyle: React.CSSProperties = {
  padding: "1rem 0.85rem",
};
const errorBox: React.CSSProperties = {
  padding: "0.6rem 1rem", backgroundColor: "#fef2f2", border: "1px solid #fecaca",
  borderRadius: "6px", color: "#991b1b", fontSize: "0.84rem", marginBottom: "1rem",
};
const suggestionBox: React.CSSProperties = {
  padding: "0.6rem 1rem", backgroundColor: "#fffbeb", border: "1px solid #fde68a",
  borderRadius: "6px", color: "#92400e", fontSize: "0.84rem", marginTop: "0.75rem",
};
const summaryLine: React.CSSProperties = {
  fontSize: "0.84rem", color: "#334155", marginBottom: "0.35rem",
};
const mutedText: React.CSSProperties = {
  fontSize: "0.84rem", color: "#64748b",
};
const progressBarOuter: React.CSSProperties = {
  width: "100%", height: "10px", backgroundColor: "#e2e8f0",
  borderRadius: "5px", overflow: "hidden", marginTop: "0.5rem",
};
const btnPrimary = (enabled: boolean): React.CSSProperties => ({
  padding: "0.35rem 0.85rem", fontSize: "0.8rem", fontWeight: 600,
  border: "1px solid #2563eb", borderRadius: "6px",
  cursor: enabled ? "pointer" : "default",
  backgroundColor: enabled ? "#2563eb" : "#e2e8f0",
  color: enabled ? "#ffffff" : "#94a3b8",
  fontFamily: "inherit",
});
const btnDanger = (enabled: boolean): React.CSSProperties => ({
  padding: "0.35rem 0.85rem", fontSize: "0.8rem", fontWeight: 600,
  border: "1px solid #dc2626", borderRadius: "6px",
  cursor: enabled ? "pointer" : "default",
  backgroundColor: enabled ? "#dc2626" : "#e2e8f0",
  color: enabled ? "#ffffff" : "#94a3b8",
  fontFamily: "inherit",
});

// ── Helpers ─────────────────────────────────────────────────────

/** Map ExecutionHistoryEntry[] to PipelineStep[] for the ExecutionHistory component. */
function toSteps(entries: ExecutionHistoryEntry[]): PipelineStep[] {
  return entries.map((e, i) => ({
    id: i,
    document_id: "",
    step_name: e.label || e.step_name,
    status: e.status,
    started_at: e.started_at,
    completed_at: null,
    duration_secs: e.duration_secs,
    triggered_by: e.triggered_by,
    input_params: {},
    result_summary: e.summary ?? {},
    error_message: e.error_message,
  }));
}

// ── Component ───────────────────────────────────────────────────

interface ProcessingPanelProps {
  document: PipelineDocument;
  onRefresh: () => void;
}

const ProcessingPanel: React.FC<ProcessingPanelProps> = ({
  document: doc, onRefresh,
}) => {
  const [busy, setBusy] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [actions, setActions] = useState<DocumentActions | null>(null);
  const [showReprocess, setShowReprocess] = useState(false);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Fetch execution history from actions endpoint
  const loadActions = useCallback(async () => {
    try {
      const data = await fetchDocumentActions(doc.id);
      setActions(data);
    } catch { /* non-fatal */ }
  }, [doc.id]);

  useEffect(() => { loadActions(); }, [loadActions, doc.status]);

  // Polling: refresh every 3s while processing
  useEffect(() => {
    if (doc.status_group === "processing") {
      pollRef.current = setInterval(() => { onRefresh(); }, 3000);
    }
    return () => {
      if (pollRef.current) {
        clearInterval(pollRef.current);
        pollRef.current = null;
      }
    };
  }, [doc.status_group, onRefresh]);

  // Button handlers
  const handleProcess = async () => {
    if (busy) return;
    setBusy(true);
    setActionError(null);
    try {
      await processDocument(doc.id);
      onRefresh();
    } catch (e) {
      setActionError(e instanceof Error ? e.message : "Failed to start processing");
    } finally {
      setBusy(false);
    }
  };

  const handleReprocess = () => {
    setShowReprocess(true);
  };

  const handleCancel = async () => {
    if (busy) return;
    setBusy(true);
    setActionError(null);
    try {
      await cancelProcessing(doc.id);
      onRefresh();
    } catch (e) {
      setActionError(e instanceof Error ? e.message : "Failed to cancel processing");
    } finally {
      setBusy(false);
    }
  };

  const historySteps = toSteps(actions?.execution_history ?? []);
  const statusGroup = doc.status_group ?? "new";

  // ── Render per status_group ────────────────────────────────────

  const renderProcessing = () => (
    <div style={bodyStyle}>
      <div style={{ fontSize: "0.95rem", fontWeight: 600, color: "#2563eb", marginBottom: "0.75rem" }}>
        Processing...
      </div>
      <div style={summaryLine}>
        Current step: <strong>{doc.processing_step_label ?? doc.processing_step ?? "—"}</strong>
      </div>
      {doc.entities_found != null && (
        <div style={summaryLine}>Entities found: {doc.entities_found}</div>
      )}
      <div style={{ ...mutedText, marginTop: "0.25rem" }}>
        Progress: {doc.percent_complete ?? 0}%
      </div>
      <div style={progressBarOuter}>
        <div style={{
          width: `${doc.percent_complete ?? 0}%`,
          height: "100%",
          backgroundColor: "#2563eb",
          borderRadius: "5px",
          transition: "width 0.3s ease",
        }} />
      </div>
      <div style={{ marginTop: "1rem" }}>
        <button style={btnDanger(!busy)} disabled={busy} onClick={handleCancel}>
          {busy ? "Cancelling..." : "Cancel Processing"}
        </button>
      </div>
    </div>
  );

  const renderCompleted = () => (
    <div style={bodyStyle}>
      <div style={{ fontSize: "0.95rem", fontWeight: 600, color: "#22c55e", marginBottom: "0.75rem" }}>
        Processing Complete
      </div>
      {doc.model_name && (
        <div style={summaryLine}>Model: {doc.model_name}</div>
      )}
      {doc.total_cost_usd != null && (
        <div style={summaryLine}>Cost: ${doc.total_cost_usd.toFixed(2)}</div>
      )}
      {doc.run_chunk_count != null && (
        <div style={summaryLine}>
          Chunks: {doc.run_chunk_count} total
          {doc.run_chunks_succeeded != null && <>, {doc.run_chunks_succeeded} succeeded</>}
          {(doc.run_chunks_failed ?? 0) > 0 && <>, <span style={{ color: "#dc2626" }}>{doc.run_chunks_failed} failed</span></>}
        </div>
      )}
      {(doc.entities_written ?? 0) > 0 && (() => {
        const written = doc.entities_written ?? 0;
        const flagged = doc.entities_flagged ?? 0;
        const total = written + flagged;
        const rate = total > 0 ? Math.round((written / total) * 100) : 0;
        return (
          <div style={summaryLine}>
            Grounding: {rate}% ({written} grounded, {flagged} ungrounded)
          </div>
        );
      })()}
      <div style={{ ...summaryLine, color: "#16a34a" }}>
        {doc.entities_written ?? 0} entities written to graph
      </div>
      <div style={summaryLine}>
        {doc.relationships_written ?? 0} relationships written
      </div>
      {(doc.entities_flagged ?? 0) > 0 && (
        <div style={{ ...summaryLine, color: "#d97706" }}>
          {doc.entities_flagged} entities flagged (ungrounded)
        </div>
      )}
      <div style={{ marginTop: "1rem" }}>
        <button style={btnPrimary(!busy)} disabled={busy} onClick={() => setShowReprocess(true)}>
          {busy ? "Starting..." : "Re-process"}
        </button>
      </div>
    </div>
  );

  const renderFailed = () => (
    <div style={bodyStyle}>
      <div style={{ fontSize: "0.95rem", fontWeight: 600, color: "#dc2626", marginBottom: "0.75rem" }}>
        Processing Failed
      </div>
      {doc.failed_step && (
        <div style={summaryLine}>
          Failed at: <strong>{doc.failed_step}</strong>
        </div>
      )}
      {doc.error_message && (
        <div style={{ ...summaryLine, color: "#dc2626" }}>
          Error: {doc.error_message}
        </div>
      )}
      {doc.error_suggestion && (
        <div style={suggestionBox}>
          Suggestion: {doc.error_suggestion}
        </div>
      )}
      <div style={{ marginTop: "1rem" }}>
        <button style={btnPrimary(!busy)} disabled={busy} onClick={() => setShowReprocess(true)}>
          {busy ? "Starting..." : "Re-process"}
        </button>
      </div>
    </div>
  );

  const renderCancelled = () => (
    <div style={bodyStyle}>
      <div style={{ fontSize: "0.95rem", fontWeight: 600, color: "#64748b", marginBottom: "0.75rem" }}>
        Processing Cancelled
      </div>
      {doc.error_message && (
        <div style={summaryLine}>{doc.error_message}</div>
      )}
      <div style={mutedText}>
        No data was written to the knowledge graph.
      </div>
      <div style={{ marginTop: "1rem" }}>
        <button style={btnPrimary(!busy)} disabled={busy} onClick={() => setShowReprocess(true)}>
          {busy ? "Starting..." : "Re-process"}
        </button>
      </div>
    </div>
  );

  const renderNew = () => (
    <div style={bodyStyle}>
      <div style={mutedText}>
        This document has not been processed yet.
      </div>
      <div style={{ ...mutedText, marginTop: "0.5rem" }}>
        Click "Process Document" to start extraction.
      </div>
      <div style={{ marginTop: "1rem" }}>
        <button style={btnPrimary(!busy)} disabled={busy} onClick={handleProcess}>
          {busy ? "Starting..." : "Process Document"}
        </button>
      </div>
    </div>
  );

  const contentRenderers: Record<string, () => React.ReactNode> = {
    processing: renderProcessing,
    completed: renderCompleted,
    failed: renderFailed,
    cancelled: renderCancelled,
    new: renderNew,
  };

  const renderContent = contentRenderers[statusGroup] ?? renderNew;

  return (
    <div>
      {actionError && <div style={errorBox}>{actionError}</div>}

      <div style={containerStyle}>
        <div style={headerStyle}>Processing</div>
        {renderContent()}
      </div>

      {/* Collapsible execution history */}
      {historySteps.length > 0 && <ExecutionHistory steps={historySteps} />}

      {showReprocess && (
        <ReprocessDialog
          open={showReprocess}
          documentId={doc.id}
          onClose={() => setShowReprocess(false)}
          onSuccess={() => { setShowReprocess(false); onRefresh(); }}
        />
      )}
    </div>
  );
};

export default ProcessingPanel;
