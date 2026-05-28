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
import React, { useEffect, useState } from "react";
import ConfigurationPanel from "./ConfigurationPanel";
import ExecutionHistory from "./ExecutionHistory";
import ReprocessDialog from "./ReprocessDialog";
import {
  processDocument,
  cancelProcessing,
  PipelineDocument,
  PipelineStep,
} from "../../services/pipelineApi";
import { getDocumentConfig, PatchConfigInput } from "../../services/configApi";

// ── Styles ──────────────────────────────────────────────────────

const containerStyle: React.CSSProperties = {
  backgroundColor: "var(--bg-surface)", borderRadius: "8px", border: "1px solid var(--border-default)",
  overflow: "hidden",
};
const headerStyle: React.CSSProperties = {
  padding: "0.6rem 0.85rem", fontWeight: 600, fontSize: "0.84rem", color: "var(--text-secondary)",
  backgroundColor: "var(--bg-page)", borderBottom: "1px solid var(--border-default)",
  display: "flex", alignItems: "center", gap: "0.5rem",
};
const spinnerStyle: React.CSSProperties = {
  width: "14px", height: "14px",
  border: "2px solid var(--border-default)",
  borderTopColor: "var(--accent-primary)",
  borderRadius: "50%",
  animation: "colossus-spin 0.8s linear infinite",
  display: "inline-block",
};
const configBlock: React.CSSProperties = {
  marginTop: "1rem", padding: "0.6rem 0.75rem",
  backgroundColor: "var(--bg-page)", border: "1px solid var(--border-default)", borderRadius: "6px",
};
const configLabel: React.CSSProperties = {
  fontSize: "0.72rem", fontWeight: 600, color: "var(--text-muted)",
  textTransform: "uppercase", letterSpacing: "0.03em", marginBottom: "0.4rem",
};
const reprocessSubtitle: React.CSSProperties = {
  marginTop: "0.3rem", fontSize: "0.72rem", color: "var(--text-muted)",
};
const bodyStyle: React.CSSProperties = {
  padding: "1rem 0.85rem",
};
const errorBox: React.CSSProperties = {
  padding: "0.6rem 1rem", backgroundColor: "var(--state-danger-bg-soft)", border: "1px solid var(--state-danger-border)",
  borderRadius: "6px", color: "var(--status-dropped-text)", fontSize: "0.84rem", marginBottom: "1rem",
};
const suggestionBox: React.CSSProperties = {
  padding: "0.6rem 1rem", backgroundColor: "var(--burden-warning-bg)", border: "1px solid var(--burden-warning-bg)",
  borderRadius: "6px", color: "var(--burden-warning-text)", fontSize: "0.84rem", marginTop: "0.75rem",
};
const summaryLine: React.CSSProperties = {
  fontSize: "0.84rem", color: "var(--text-secondary)", marginBottom: "0.35rem",
};
const mutedText: React.CSSProperties = {
  fontSize: "0.84rem", color: "var(--text-muted)",
};
const progressBarOuter: React.CSSProperties = {
  width: "100%", height: "10px", backgroundColor: "var(--border-default)",
  borderRadius: "5px", overflow: "hidden", marginTop: "0.5rem",
};
const btnPrimary = (enabled: boolean): React.CSSProperties => ({
  padding: "0.35rem 0.85rem", fontSize: "0.8rem", fontWeight: 600,
  border: "1px solid var(--accent-primary)", borderRadius: "6px",
  cursor: enabled ? "pointer" : "default",
  backgroundColor: enabled ? "var(--accent-primary)" : "var(--border-default)",
  color: enabled ? "var(--bg-surface)" : "var(--text-disabled)",
  fontFamily: "inherit",
});
const btnDanger = (enabled: boolean): React.CSSProperties => ({
  padding: "0.35rem 0.85rem", fontSize: "0.8rem", fontWeight: 600,
  border: "1px solid var(--state-danger-strong)", borderRadius: "6px",
  cursor: enabled ? "pointer" : "default",
  backgroundColor: enabled ? "var(--state-danger-strong)" : "var(--border-default)",
  color: enabled ? "var(--bg-surface)" : "var(--text-disabled)",
  fontFamily: "inherit",
});

// ── Helpers ─────────────────────────────────────────────────────

/** Convert a model id like "claude-sonnet-4-6" into "Claude Sonnet 4.6". */
function humanizeModelName(model: string): string {
  return model
    .replace(/-/g, " ")
    .replace(/\b\w/g, (c) => c.toUpperCase())
    .replace(/(\d+) (\d+)/g, "$1.$2");
}

/** Derive the card header title + color from the current status group. */
function cardTitle(statusGroup: string, hasFailedSteps: boolean):
  { text: string; color: string; showSpinner: boolean }
{
  if (statusGroup === "completed") return { text: "Processing Complete", color: "var(--status-active-text)", showSpinner: false };
  if (statusGroup === "failed") return { text: "Processing Failed", color: "var(--state-danger-strong)", showSpinner: false };
  if (statusGroup === "cancelled") return { text: "Processing Cancelled", color: "var(--text-muted)", showSpinner: false };
  if (statusGroup === "processing") {
    // Failure-transition gap: step already failed but doc row not yet FAILED.
    if (hasFailedSteps) return { text: "Processing Failed", color: "var(--state-danger-strong)", showSpinner: false };
    return { text: "Processing...", color: "var(--accent-primary)", showSpinner: true };
  }
  return { text: "Processing", color: "var(--text-secondary)", showSpinner: false };
}

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
  const [completedConfig, setCompletedConfig] = useState<PatchConfigInput | null>(null);

  // Pull the effective profile / template / schema for the Completed card
  // once the document has a config row. Fetch only after processing is
  // past the point where pipeline_config is written; 404s for legacy docs
  // are swallowed. Re-fetch if the document id changes.
  useEffect(() => {
    const statusGroup = doc.status_group ?? "new";
    if (statusGroup !== "completed" && statusGroup !== "failed") {
      setCompletedConfig(null);
      return;
    }
    let cancelled = false;
    getDocumentConfig(doc.id)
      .then((cfg) => { if (!cancelled) setCompletedConfig(cfg); })
      .catch(() => { if (!cancelled) setCompletedConfig(null); });
    return () => { cancelled = true; };
  }, [doc.id, doc.status_group]);

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
  const title = cardTitle(statusGroup, doc.has_failed_steps);

  // Shared Re-process button + subtitle. Used in four render paths.
  const reprocessButton = (label = "Re-process") => (
    <div style={{ marginTop: "1rem" }}>
      <button style={btnPrimary(!busy)} disabled={busy} onClick={() => setShowReprocess(true)}>
        {busy ? "Starting..." : label}
      </button>
      <div style={reprocessSubtitle}>
        Delete extracted data and run the pipeline again with current settings.
      </div>
    </div>
  );

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
        <div style={{ ...summaryLine, color: "var(--state-danger-strong)" }}>
          Error: {doc.error_message}
        </div>
      )}
      {reprocessButton()}
    </div>
  );

  const renderProcessing = () => {
    // Detect the failure-transition gap described on renderProcessingButFailed.
    if (doc.has_failed_steps) {
      return renderProcessingButFailed();
    }
    return (
    <div style={bodyStyle}>
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
            <span style={{ color: "var(--text-muted)" }}> — {doc.entities_found} entities found so far</span>
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
          backgroundColor: "var(--accent-primary)",
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

  const renderCompleted = () => {
    const entitiesCreated = doc.entities_written ?? 0;
    const relationshipsCreated = doc.relationships_written ?? 0;
    const modelDisplay = doc.model_name ? humanizeModelName(doc.model_name) : null;

    return (
      <div style={bodyStyle}>
        {doc.run_chunk_count != null && (
          <div style={summaryLine}>
            Chunks: {doc.run_chunk_count} total
            {doc.run_chunks_succeeded != null && <>, {doc.run_chunks_succeeded} succeeded</>}
            {(doc.run_chunks_failed ?? 0) > 0 && <>, <span style={{ color: "var(--state-danger-strong)" }}>{doc.run_chunks_failed} failed</span></>}
          </div>
        )}
        {entitiesCreated > 0 && (() => {
          const flagged = doc.entities_flagged ?? 0;
          const total = entitiesCreated + flagged;
          const rate = total > 0 ? Math.round((entitiesCreated / total) * 100) : 0;
          return (
            <div style={summaryLine}>
              Grounding: {rate}% ({entitiesCreated} grounded, {flagged} ungrounded)
            </div>
          );
        })()}
        {entitiesCreated > 0 ? (
          <>
            <div style={{ ...summaryLine, color: "var(--state-success-strong)" }}>
              {entitiesCreated} entities created
            </div>
            <div style={summaryLine}>
              {relationshipsCreated} relationships created
            </div>
          </>
        ) : (
          <div style={{ ...summaryLine, color: "var(--text-muted)", fontStyle: "italic" }}>
            See Review tab for extracted items.
          </div>
        )}
        {(doc.entities_flagged ?? 0) > 0 && (
          <div style={{ ...summaryLine, color: "var(--state-warning-strong)" }}>
            {doc.entities_flagged} entities flagged (ungrounded)
          </div>
        )}

        <div style={configBlock}>
          <div style={configLabel}>Configuration</div>
          {modelDisplay && (
            <div style={summaryLine}>Model: {modelDisplay}</div>
          )}
          {completedConfig?.profile_name && (
            <div style={summaryLine}>Profile: {completedConfig.profile_name}</div>
          )}
          {completedConfig?.template_file && (
            <div style={summaryLine}>Template: {completedConfig.template_file}</div>
          )}
          {completedConfig?.schema_file && (
            <div style={summaryLine}>Schema: {completedConfig.schema_file}</div>
          )}
          {completedConfig?.extraction_model &&
            completedConfig.extraction_model !== doc.model_name && (
              <div style={summaryLine}>
                Extraction model: {completedConfig.extraction_model}
              </div>
            )}
        </div>

        {reprocessButton()}
      </div>
    );
  };

  const renderFailed = () => (
    <div style={bodyStyle}>
      {doc.failed_step && (
        <div style={summaryLine}>
          Failed at: <strong>{doc.failed_step}</strong>
        </div>
      )}
      {doc.error_message && (
        <div style={{ ...summaryLine, color: "var(--state-danger-strong)" }}>
          Error: {doc.error_message}
        </div>
      )}
      {doc.error_suggestion && (
        <div style={suggestionBox}>
          Suggestion: {doc.error_suggestion}
        </div>
      )}
      {reprocessButton()}
    </div>
  );

  const renderCancelled = () => (
    <div style={bodyStyle}>
      {doc.error_message && (
        <div style={summaryLine}>{doc.error_message}</div>
      )}
      <div style={mutedText}>
        No data was written to the knowledge graph.
      </div>
      {reprocessButton()}
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
      {/* Spinner keyframes — scoped by a unique animation name to avoid
          collisions with any other "spin" animation in the app. */}
      <style>{`@keyframes colossus-spin { to { transform: rotate(360deg); } }`}</style>

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
        <div style={{ ...headerStyle, color: title.color }}>
          {title.showSpinner && <span style={spinnerStyle} aria-hidden="true" />}
          <span>{title.text}</span>
        </div>
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
