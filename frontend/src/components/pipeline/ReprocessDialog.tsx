/**
 * ReprocessDialog — re-extract a document, with a guard proportional to what it
 * carries.
 *
 * REEXTRACT_PATH (2026-08-17). This dialog used to offer three radio buttons.
 * None of them did what it said:
 *
 *   - the backend handler took no body, so the choice was parsed by nothing;
 *   - both LLM passes short-circuit on the existing COMPLETED extraction run, so
 *     every option was a re-INGEST, not a re-extraction — measured on Morris:
 *     eight pipeline steps in 2.4 seconds, zero tokens;
 *   - "Delete graph data and re-extract" was identical to "same settings"
 *     regardless, because ingest already cleans the document's graph
 *     unconditionally before writing.
 *
 * It is now ONE action that really re-extracts. Different settings are chosen in
 * the Configuration panel, which is the surface that can actually collect them —
 * this dialog never could, and a radio button promising it was the third lie.
 */
import React, { useEffect, useState } from "react";
import { processDocument, fetchCuratedRows, CuratedRowsResponse } from "../../services/pipelineApi";
import { evaluateGuard, CuratedState } from "./reprocessDialogHelpers";

interface ReprocessDialogProps {
  open: boolean;
  documentId: string;
  onClose: () => void;
  onSuccess: () => void;
}

const overlay: React.CSSProperties = {
  position: "fixed", inset: 0, backgroundColor: "rgba(0,0,0,0.4)",
  display: "flex", alignItems: "center", justifyContent: "center", zIndex: 1000,
};
const card: React.CSSProperties = {
  backgroundColor: "var(--bg-surface)", borderRadius: "12px", padding: "1.75rem",
  maxWidth: "520px", width: "90%", boxShadow: "0 20px 60px rgba(0,0,0,0.15)",
};
const body: React.CSSProperties = {
  fontSize: "0.84rem", color: "var(--text-secondary)", lineHeight: 1.5,
};
const note: React.CSSProperties = {
  fontSize: "0.76rem", color: "var(--text-muted)", marginTop: "0.75rem", lineHeight: 1.5,
};
const guardBox: React.CSSProperties = {
  marginTop: "1rem", padding: "0.75rem 0.9rem",
  backgroundColor: "var(--state-danger-bg-soft)", border: "1px solid var(--state-danger-border)",
  borderRadius: "8px", fontSize: "0.8rem", color: "var(--text-primary)", lineHeight: 1.5,
};
const input: React.CSSProperties = {
  width: "100%", marginTop: "0.5rem", padding: "0.45rem 0.6rem", fontSize: "0.82rem",
  border: "1px solid var(--border-default)", borderRadius: "6px",
  fontFamily: "inherit", boxSizing: "border-box", backgroundColor: "var(--bg-surface)",
};
const errorBox: React.CSSProperties = {
  padding: "0.5rem 0.75rem", backgroundColor: "var(--state-danger-bg-soft)",
  border: "1px solid var(--state-danger-border)", borderRadius: "6px",
  color: "var(--status-dropped-text)", fontSize: "0.76rem", marginTop: "0.75rem",
};
const btnRow: React.CSSProperties = {
  display: "flex", justifyContent: "flex-end", gap: "0.5rem", marginTop: "1.25rem",
};
const btnCancel: React.CSSProperties = {
  padding: "0.45rem 1rem", fontSize: "0.84rem", fontWeight: 500,
  border: "1px solid var(--border-default)", borderRadius: "6px",
  backgroundColor: "var(--bg-surface)", color: "var(--text-muted)",
  cursor: "pointer", fontFamily: "inherit",
};
const btnGo = (enabled: boolean): React.CSSProperties => ({
  padding: "0.45rem 1rem", fontSize: "0.84rem", fontWeight: 600, border: "none",
  borderRadius: "6px",
  backgroundColor: enabled ? "var(--accent-primary)" : "var(--text-disabled)",
  color: "var(--bg-surface)", cursor: enabled ? "pointer" : "default", fontFamily: "inherit",
});

const ReprocessDialog: React.FC<ReprocessDialogProps> = ({
  open, documentId, onClose, onSuccess,
}) => {
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [curated, setCurated] = useState<CuratedRowsResponse | null>(null);
  const [curatedError, setCuratedError] = useState<string | null>(null);
  const [typed, setTyped] = useState("");

  // Read what the document carries before offering to re-extract it. An explicit
  // error state, never a silent zero: a failed count must not read as "nothing
  // at stake" — that is the exact failure the guard exists to prevent.
  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    setCurated(null);
    setCuratedError(null);
    fetchCuratedRows(documentId)
      .then((r) => { if (!cancelled) setCurated(r); })
      .catch((e: unknown) => {
        if (cancelled) return;
        setCuratedError(e instanceof Error ? e.message : "Could not read the curated-row count");
      });
    return () => { cancelled = true; };
  }, [open, documentId]);

  if (!open) return null;

  // The decision lives in a pure helper so it can be tested — see
  // reprocessDialogHelpers.ts for why, and for the failed-count rule.
  const curatedState: CuratedState =
    curatedError !== null
      ? { kind: "failed" }
      : curated === null
        ? { kind: "loading" }
        : { kind: "loaded", total: curated.total };
  const loading = curatedState.kind === "loading";
  const atRisk = curatedState.kind === "loaded" && curatedState.total > 0;
  const { canRun } = evaluateGuard({ curated: curatedState, typed, documentId, running });

  const handleRun = async () => {
    if (!canRun) return;
    setRunning(true);
    setError(null);
    try {
      await processDocument(documentId, "same_settings");
      onSuccess();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Re-extraction failed to start");
    } finally {
      setRunning(false);
    }
  };

  return (
    <div style={overlay} onClick={onClose}>
      <div style={card} onClick={(e) => e.stopPropagation()}>
        <h2 style={{ margin: "0 0 0.75rem", fontSize: "1.1rem", fontWeight: 700, color: "var(--text-primary)" }}>
          Re-extract Document
        </h2>

        <div style={body}>
          This runs the model again over the document's text, then re-verifies,
          re-ingests and re-indexes it. The extracted text itself is not
          re-read — only the extraction is redone.
        </div>

        <div style={note}>
          To change the model, schema or template first, close this and use
          <strong> Processing Configuration</strong> — its <em>Save &amp; Process</em>
          button re-extracts with the new settings.
        </div>

        {loading && (
          <div style={note}>Checking what this document carries…</div>
        )}

        {curatedError !== null && (
          <div style={guardBox}>
            The curated-row count could not be read ({curatedError}). Proceeding
            without knowing what this document carries — type the document id to
            confirm.
            <input
              style={input}
              value={typed}
              onChange={(e) => setTyped(e.target.value)}
              placeholder={documentId}
              aria-label="Type the document id to confirm"
            />
          </div>
        )}

        {atRisk && curated !== null && (
          <div style={guardBox}>
            This document carries {curated.total} rulings. Take a{" "}
            <code>remap_evidence snapshot</code> first; re-extraction can move ids.
            <div style={{ ...note, marginTop: "0.5rem" }}>
              {curated.by_column.map((c) => `${c.reference}: ${c.rows}`).join(" · ")}
            </div>
            <input
              style={input}
              value={typed}
              onChange={(e) => setTyped(e.target.value)}
              placeholder={documentId}
              aria-label="Type the document id to confirm"
            />
          </div>
        )}

        {curated !== null && curated.total === 0 && (
          <div style={note}>
            This document carries no rulings — nothing is at risk if ids move.
          </div>
        )}

        {error && <div style={errorBox}>{error}</div>}

        <div style={btnRow}>
          <button style={btnCancel} onClick={onClose}>Cancel</button>
          <button style={btnGo(canRun)} onClick={() => { void handleRun(); }} disabled={!canRun}>
            {running ? "Starting…" : "Re-extract"}
          </button>
        </div>
      </div>
    </div>
  );
};

export default ReprocessDialog;
