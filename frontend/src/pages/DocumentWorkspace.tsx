/**
 * DocumentWorkspace — Side-by-side PDF viewer + evidence audit panel.
 *
 * Split-pane layout:
 * - Left (60%): PdfViewer showing the document's PDF
 * - Right (40%): Scrollable evidence cards with verify/flag actions
 * - Top bar: Document title, evidence stats, back button
 *
 * Clicking an evidence card navigates the PDF to the cited page.
 * The PdfViewer is controlled — we own the page state and pass it down.
 */
import React, { useCallback, useEffect, useState } from "react";
import { useParams, useLocation, Link } from "react-router-dom";

import PdfViewer from "../components/shared/PdfViewer";
import EvidenceCard from "../components/admin/EvidenceCard";
import { API_BASE_URL } from "../services/api";
import {
  DocumentEvidence,
  DocumentEvidenceResponse,
  fetchDocumentEvidence,
  verifyEvidence,
  flagEvidence,
} from "../services/documentEvidence";
import { cardStyle, btnPrimary, btnSecondary, inputStyle, labelStyle } from "../components/admin/adminStyles";

// ── Types for inline forms ──────────────────────────────────────

type ModalMode =
  | { kind: "none" }
  | { kind: "verify"; evidence: DocumentEvidence }
  | { kind: "flag"; evidence: DocumentEvidence };

// ── Component ───────────────────────────────────────────────────

const DocumentWorkspace: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const location = useLocation();
  const mode = location.pathname.endsWith("/audit") ? "audit" : "view";
  const docId = id ?? "";

  // Data state
  const [data, setData] = useState<DocumentEvidenceResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  // PDF page state (controlled)
  const [pdfPage, setPdfPage] = useState(1);

  // Selected evidence card
  const [selectedId, setSelectedId] = useState<string | null>(null);

  // Inline form modal
  const [modal, setModal] = useState<ModalMode>({ kind: "none" });
  const [formStatus, setFormStatus] = useState("verified");
  const [formNotes, setFormNotes] = useState("");
  const [formSeverity, setFormSeverity] = useState("medium");
  const [formDescription, setFormDescription] = useState("");
  const [submitting, setSubmitting] = useState(false);

  // ── Data fetching ────────────────────────────────────────────

  const loadEvidence = useCallback(async () => {
    if (!docId) return;
    setLoading(true);
    try {
      const result = await fetchDocumentEvidence(docId);
      setData(result);
      setError("");
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }, [docId]);

  useEffect(() => { loadEvidence(); }, [loadEvidence]);

  // ── Handlers ─────────────────────────────────────────────────

  const handleSelect = (ev: DocumentEvidence) => {
    setSelectedId(ev.id);
    if (ev.page_number != null && ev.page_number > 0) {
      setPdfPage(ev.page_number);
    }
  };

  const handleOpenVerify = (ev: DocumentEvidence) => {
    setModal({ kind: "verify", evidence: ev });
    setFormStatus(ev.verification?.status || "verified");
    setFormNotes(ev.verification?.notes || "");
  };

  const handleOpenFlag = (ev: DocumentEvidence) => {
    setModal({ kind: "flag", evidence: ev });
    setFormSeverity("medium");
    setFormDescription("");
  };

  const handleSubmitVerify = async () => {
    if (modal.kind !== "verify") return;
    setSubmitting(true);
    try {
      await verifyEvidence(docId, modal.evidence.id, formStatus, formNotes);
      setModal({ kind: "none" });
      await loadEvidence();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSubmitting(false);
    }
  };

  const handleSubmitFlag = async () => {
    if (modal.kind !== "flag") return;
    if (!formDescription.trim()) {
      setError("Description is required");
      return;
    }
    setSubmitting(true);
    try {
      await flagEvidence(docId, modal.evidence.id, formSeverity, formDescription);
      setModal({ kind: "none" });
      await loadEvidence();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSubmitting(false);
    }
  };

  // ── PDF URL ──────────────────────────────────────────────────

  const pdfUrl = `${API_BASE_URL}/api/documents/${encodeURIComponent(docId)}/file`;

  // ── Render ───────────────────────────────────────────────────

  if (loading && !data) {
    return (
      <div style={{ padding: "3rem", textAlign: "center", color: "#64748b" }}>
        Loading workspace...
      </div>
    );
  }

  if (error && !data) {
    return (
      <div style={{ padding: "3rem", textAlign: "center" }}>
        <div style={{ color: "#dc2626", marginBottom: "1rem" }}>{error}</div>
        <Link to="/admin" style={{ color: "#2563eb", fontSize: "0.84rem" }}>
          Back to Admin
        </Link>
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "calc(100vh - 60px)" }}>
      {/* Top bar */}
      <div style={{
        display: "flex", alignItems: "center", justifyContent: "space-between",
        padding: "0.6rem 1.25rem", borderBottom: "1px solid #e2e8f0",
        backgroundColor: "#f8fafc",
      }}>
        <div style={{ display: "flex", alignItems: "center", gap: "1rem" }}>
          <Link to="/admin" style={{ color: "#2563eb", fontSize: "0.82rem", textDecoration: "none" }}>
            Back
          </Link>
          <div>
            <div style={{ fontSize: "0.95rem", fontWeight: 600, color: "#0f172a" }}>
              {data?.document_title || docId}
            </div>
            <div style={{ fontSize: "0.74rem", color: "#64748b" }}>
              {mode} mode
            </div>
          </div>
        </div>
        <div style={{ display: "flex", gap: "1.25rem", fontSize: "0.78rem" }}>
          <StatBadge label="Evidence" value={data?.evidence_count ?? 0} color="#334155" />
          <StatBadge label="Verified" value={data?.verified_count ?? 0} color="#047857" />
          <StatBadge label="Flagged" value={data?.flagged_count ?? 0} color="#dc2626" />
        </div>
      </div>

      {/* Error banner */}
      {error && (
        <div style={{
          padding: "0.5rem 1.25rem", backgroundColor: "#fef2f2",
          borderBottom: "1px solid #fecaca", fontSize: "0.82rem", color: "#dc2626",
        }}>
          {error}
          <button onClick={() => setError("")} style={{
            marginLeft: "0.5rem", background: "none", border: "none",
            color: "#dc2626", cursor: "pointer", fontFamily: "inherit",
          }}>dismiss</button>
        </div>
      )}

      {/* Split pane */}
      <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
        {/* Left: PDF viewer (60%) */}
        <div style={{ width: "60%", overflow: "auto", borderRight: "1px solid #e2e8f0" }}>
          <PdfViewer
            src={pdfUrl}
            page={pdfPage}
            onPageChange={setPdfPage}
          />
        </div>

        {/* Right: Evidence panel (40%) */}
        <div style={{
          width: "40%", overflow: "auto", padding: "0.75rem",
          backgroundColor: "#fafbfc",
        }}>
          {/* Inline verify form */}
          {modal.kind === "verify" && (
            <div style={{ ...cardStyle, marginBottom: "0.75rem", border: "2px solid #a7f3d0" }}>
              <div style={{ fontSize: "0.84rem", fontWeight: 600, color: "#0f172a", marginBottom: "0.5rem" }}>
                Verify: {modal.evidence.title || modal.evidence.id}
              </div>
              <div style={{ marginBottom: "0.5rem" }}>
                <label style={labelStyle}>Status</label>
                <select style={inputStyle} value={formStatus} onChange={(e) => setFormStatus(e.target.value)}>
                  <option value="verified">Verified</option>
                  <option value="rejected">Rejected</option>
                  <option value="pending">Pending (undo)</option>
                </select>
              </div>
              <div style={{ marginBottom: "0.5rem" }}>
                <label style={labelStyle}>Notes</label>
                <textarea
                  style={{ ...inputStyle, minHeight: "60px", resize: "vertical" }}
                  value={formNotes}
                  onChange={(e) => setFormNotes(e.target.value)}
                  placeholder="e.g. Confirmed on page 5, paragraph 2"
                />
              </div>
              <div style={{ display: "flex", gap: "0.4rem" }}>
                <button style={btnPrimary} onClick={handleSubmitVerify} disabled={submitting}>
                  {submitting ? "Saving..." : "Save"}
                </button>
                <button style={btnSecondary} onClick={() => setModal({ kind: "none" })}>
                  Cancel
                </button>
              </div>
            </div>
          )}

          {/* Inline flag form */}
          {modal.kind === "flag" && (
            <div style={{ ...cardStyle, marginBottom: "0.75rem", border: "2px solid #fecaca" }}>
              <div style={{ fontSize: "0.84rem", fontWeight: 600, color: "#0f172a", marginBottom: "0.5rem" }}>
                Flag: {modal.evidence.title || modal.evidence.id}
              </div>
              <div style={{ marginBottom: "0.5rem" }}>
                <label style={labelStyle}>Severity</label>
                <select style={inputStyle} value={formSeverity} onChange={(e) => setFormSeverity(e.target.value)}>
                  <option value="low">Low</option>
                  <option value="medium">Medium</option>
                  <option value="high">High</option>
                  <option value="critical">Critical</option>
                </select>
              </div>
              <div style={{ marginBottom: "0.5rem" }}>
                <label style={labelStyle}>Description</label>
                <textarea
                  style={{ ...inputStyle, minHeight: "60px", resize: "vertical" }}
                  value={formDescription}
                  onChange={(e) => setFormDescription(e.target.value)}
                  placeholder="Describe the issue..."
                />
              </div>
              <div style={{ display: "flex", gap: "0.4rem" }}>
                <button style={{ ...btnPrimary, backgroundColor: "#dc2626" }} onClick={handleSubmitFlag} disabled={submitting}>
                  {submitting ? "Saving..." : "Submit Flag"}
                </button>
                <button style={btnSecondary} onClick={() => setModal({ kind: "none" })}>
                  Cancel
                </button>
              </div>
            </div>
          )}

          {/* Evidence cards */}
          {data?.evidence.map((ev) => (
            <EvidenceCard
              key={ev.id}
              evidence={ev}
              isSelected={selectedId === ev.id}
              onSelect={handleSelect}
              onVerify={handleOpenVerify}
              onFlag={handleOpenFlag}
            />
          ))}

          {data?.evidence.length === 0 && (
            <div style={{ textAlign: "center", padding: "2rem", color: "#64748b", fontSize: "0.84rem" }}>
              No evidence linked to this document.
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

// ── Small helper component ──────────────────────────────────────

const StatBadge: React.FC<{ label: string; value: number; color: string }> = ({
  label, value, color,
}) => (
  <div style={{ textAlign: "center" }}>
    <div style={{ fontSize: "1.1rem", fontWeight: 700, color }}>{value}</div>
    <div style={{ fontSize: "0.68rem", color: "#64748b" }}>{label}</div>
  </div>
);

export default DocumentWorkspace;
