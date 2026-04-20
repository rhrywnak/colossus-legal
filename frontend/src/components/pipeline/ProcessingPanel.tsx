/**
 * ProcessingPanel — Displays content based on doc.status_group.
 *
 * Five statuses: new, processing, completed, failed, cancelled.
 * Execution history is passed from the parent (DocumentWorkspaceTabs)
 * which already fetches it in loadData(). Polling is also owned by the
 * parent — this component does not poll independently.
 *
 * ## Contract: onStepTriggered
 *
 * onStepTriggered must be called after every user action (process, cancel,
 * reprocess) to ensure the parent reloads document state. Without this
 * call, the UI shows stale status indefinitely.
 */
import React, { useState } from "react";
import ConfigurationPanel from "./ConfigurationPanel";
import ExecutionHistory from "./ExecutionHistory";
import ReprocessDialog from "./ReprocessDialog";
import {
  processDocument,
  cancelProcessing,
  PipelineDocument,
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

// ── Component ───────────────────────────────────────────────────

interface ProcessingPanelProps {
  document: PipelineDocument;
  // Called after any action (process, cancel, reprocess) to reload the document.
  // Named onStepTriggered to match DocumentWorkspaceTabs.
  onStepTriggered: () => void;
  // Called when the panel wants the parent to switch tabs (e.g., after
  // reprocess starts, switch to the processing view). Optional.
  onSwitchTab?: (tabId: string) => void;
  // History is passed from the parent — no need to fetch it again here.
  // The parent already has it from loadData(). Kept as optional so the
  // component works standalone if needed.
  history?: PipelineStep[];
}

const ProcessingPanel: React.FC<ProcessingPanelProps> = ({
  document: doc, onStepTriggered, onSwitchTab, history,
}) => {
  const [busy, setBusy] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [showReprocess, setShowReprocess] = useState(false);

  // No internal polling — the parent (DocumentWorkspaceTabs) owns polling
  // and passes updated document props every 3s during PROCESSING.

  // Button handlers
  const handleProcess = async () => {
    if (busy) return;
    setBusy(true);
    setActionError(null);
    try {
      await processDocument(doc.id);
      // Small delay before refresh to allow the backend to transition
      // the document status from NEW to PROCESSING. Without this delay,
      // the first poll may return the document still in NEW status because
      // the status update hasn't committed yet. 500ms is enough.
      await new Promise(resolve => setTimeout(resolve, 500));
      onStepTriggered();
      // Switch to processing tab so user sees live progress immediately
      if (onSwitchTab) onSwitchTab("processing");
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
      onStepTriggered();
    } catch (e) {
      setActionError(e instanceof Error ? e.message : "Failed to cancel processing");
    } finally {
      setBusy(false);
    }
  };

  const historySteps = history ?? [];
  const statusGroup = doc.status_group ?? "new";

  // ── Render per status_group ────────────────────────────────────

  // Fallback render used when a pipeline step has already failed but the
  // document row hasn't yet transitioned to FAILED status. In that gap,
  // status_group is still "processing" but showing a Cancel button is
  // wrong — the job is already dead. A proper full fix would have the
  // documents list query LEFT JOIN the most-recent failed step so we can
  // show its error message here too; for now we render whatever doc-level
  // error fields exist plus a note that details may still be settling.
  const renderProcessingButFailed = () => (
    <div style={bodyStyle}>
      <div style={{ fontSize: "0.95rem", fontWeight: 600, color: "#dc2626", marginBottom: "0.75rem" }}>
        Processing Failed
      </div>
      <div style={suggestionBox}>
        A pipeline step failed. Full error details will appear once the job
        finalizes — refresh in a moment, or click Re-process to try again.
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
      <div style={{ marginTop: "1rem" }}>
        <button style={btnPrimary(!busy)} disabled={busy} onClick={() => setShowReprocess(true)}>
          {busy ? "Starting..." : "Re-process"}
        </button>
      </div>
    </div>
  );

  const renderProcessing = () => {
    // Detect the failure-transition gap described on renderProcessingButFailed.
    if (doc.has_failed_steps) {
      return renderProcessingButFailed();
    }
    return (
    <div style={bodyStyle}>
      <div style={{ fontSize: "0.95rem", fontWeight: 600, color: "#2563eb", marginBottom: "0.75rem" }}>
        Processing...
      </div>

      {/* Current step label — updated after each chunk */}
      <div style={summaryLine}>
        <strong>{doc.processing_step_label ?? doc.processing_step ?? "Starting..."}</strong>
      </div>

      {/* Chunk progress — only shown during extraction step when chunks are known.
          chunks_total > 0 means the extraction step has started and we know
          how many chunks there are. Without this, the user has no way to tell
          if the pipeline is progressing normally or stuck on a slow chunk. */}
      {(doc.chunks_total ?? 0) > 0 && (
        <div style={{ ...summaryLine, marginTop: "0.4rem" }}>
          Chunk {doc.chunks_processed ?? 0} of {doc.chunks_total} analyzed
          {(doc.entities_found ?? 0) > 0 && (
            <span style={{ color: "#64748b" }}> — {doc.entities_found} entities found so far</span>
          )}
        </div>
      )}

      {/* Overall percent complete bar */}
      <div style={{ ...mutedText, marginTop: "0.5rem" }}>
        Overall: {doc.percent_complete ?? 0}%
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
  };

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
        This document has not been processed yet. Review the processing
        configuration above, preview the assembled prompt if needed, then
        click Process Document to start extraction.
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

      {statusGroup === "new" && (
        <ConfigurationPanel
          documentId={doc.id}
          documentType={doc.document_type}
          documentStatus={doc.status}
          contentType={doc.content_type}
          pageCount={doc.page_count}
          textPages={doc.text_pages}
          scannedPages={doc.scanned_pages}
          onProcess={handleProcess}
          busy={busy}
        />
      )}

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
          onSuccess={() => { setShowReprocess(false); onStepTriggered(); }}
        />
      )}
    </div>
  );
};

export default ProcessingPanel;
