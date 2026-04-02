import React, { useCallback, useEffect, useState } from "react";
import { Link, useParams, useNavigate } from "react-router-dom";
import { useAuth } from "../context/AuthContext";
import DocumentStatusBadge from "../components/pipeline/DocumentStatusBadge";
import StepCard, { StepDef } from "../components/pipeline/StepCard";
import ExecutionHistory from "../components/pipeline/ExecutionHistory";
import {
  fetchPipelineDocuments, fetchDocumentHistory, triggerExtractText, triggerExtract,
  triggerVerify, triggerIngest, triggerIndex, fetchCompleteness,
  PipelineDocument, PipelineStep,
} from "../services/pipelineApi";

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

// ── Styles ──────────────────────────────────────────────────────

const backLink: React.CSSProperties = {
  fontSize: "0.84rem", color: "#2563eb", textDecoration: "none", fontWeight: 500,
};
const pageTitle: React.CSSProperties = {
  fontSize: "1.35rem", fontWeight: 700, color: "#0f172a", marginBottom: "0.15rem",
};
const metaRow: React.CSSProperties = {
  display: "flex", gap: "1.25rem", fontSize: "0.84rem", color: "#64748b",
  marginBottom: "1.25rem", alignItems: "center", flexWrap: "wrap",
};
const stepsContainer: React.CSSProperties = {
  backgroundColor: "#ffffff", borderRadius: "8px", border: "1px solid #e2e8f0",
  overflow: "hidden",
};
const stepsHeader: React.CSSProperties = {
  padding: "0.6rem 0.85rem", fontWeight: 600, fontSize: "0.84rem", color: "#334155",
  backgroundColor: "#f8fafc", borderBottom: "1px solid #e2e8f0",
};
const emptyState: React.CSSProperties = {
  padding: "3rem", textAlign: "center", color: "#94a3b8", fontSize: "0.9rem",
};
const errorBox: React.CSSProperties = {
  padding: "0.6rem 1rem", backgroundColor: "#fef2f2", border: "1px solid #fecaca",
  borderRadius: "6px", color: "#991b1b", fontSize: "0.84rem", marginBottom: "1rem",
};

// ── Helpers ─────────────────────────────────────────────────────

function latestEntry(history: PipelineStep[], stepName: string): PipelineStep | undefined {
  return history.find((h) => h.step_name === stepName);
}

function findNextAction(docStatus: string, history: PipelineStep[]): string | null {
  for (const step of PIPELINE_STEPS) {
    if (step.statusRequired === null) continue; // upload is always done
    const entry = latestEntry(history, step.name);
    if (entry && entry.status === "completed") continue;
    if (step.statusRequired === docStatus) return step.name;
  }
  return null;
}

// ── Component ───────────────────────────────────────────────────

const PipelineDocumentDetail: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { user } = useAuth();
  const [doc, setDoc] = useState<PipelineDocument | null>(null);
  const [history, setHistory] = useState<PipelineStep[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);

  const loadData = useCallback(async () => {
    if (!id) return;
    try {
      const [docs, hist] = await Promise.all([
        fetchPipelineDocuments(),
        fetchDocumentHistory(id),
      ]);
      const found = docs.find((d) => d.id === id);
      if (!found) {
        setError(`Document '${id}' not found`);
        return;
      }
      setDoc(found);
      setHistory(hist.steps);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load document");
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => { loadData(); }, [loadData]);

  const handleTrigger = async (stepName: string) => {
    if (!id || running) return;
    const triggerFn = TRIGGER_MAP[stepName];
    if (!triggerFn) return;
    setRunning(true);
    setActionError(null);
    try {
      await triggerFn(id);
      await loadData();
    } catch (e) {
      setActionError(e instanceof Error ? e.message : `Step '${stepName}' failed`);
    } finally {
      setRunning(false);
    }
  };

  const handleNavigate = (stepName: string) => {
    if (stepName === "review") navigate(`/pipeline/${id}/review`);
  };

  if (!user?.permissions.is_admin) return <div style={emptyState}>Admin access required.</div>;
  if (loading) return <div style={emptyState}>Loading...</div>;
  if (error) return <div style={{ ...emptyState, color: "#dc2626" }}>{error}</div>;
  if (!doc) return <div style={emptyState}>Document not found.</div>;

  const nextAction = findNextAction(doc.status, history);

  return (
    <div style={{ paddingTop: "1.5rem", paddingBottom: "2rem" }}>
      <Link to="/documents" style={backLink}>&larr; Back to Documents</Link>

      <h1 style={{ ...pageTitle, marginTop: "0.75rem" }}>{doc.title}</h1>
      <div style={metaRow}>
        <DocumentStatusBadge status={doc.status} />
        <span>Type: {doc.document_type}</span>
        <span>ID: {doc.id}</span>
        <span>Updated: {new Date(doc.updated_at).toLocaleDateString()}</span>
      </div>

      {actionError && <div style={errorBox}>{actionError}</div>}

      <div style={stepsContainer}>
        <div style={stepsHeader}>Pipeline Steps</div>
        {PIPELINE_STEPS.map((step, i) => {
          // Upload is always completed if the doc exists
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

export default PipelineDocumentDetail;
