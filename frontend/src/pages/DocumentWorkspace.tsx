/** DocumentWorkspace — Side-by-side PDF viewer + evidence audit panel. */
import React, { useCallback, useEffect, useState } from "react";
import { useParams, useLocation, Link } from "react-router-dom";
import { useResizablePanes } from "../hooks/useResizablePanes";

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
import { getSourceTypeDisplay } from "../utils/nodeTypeDisplay";
import { InlineVerifyForm, InlineFlagForm } from "../components/admin/InlineAuditForms";
import NodeTypeFilter from "../components/admin/NodeTypeFilter";

type ModalMode =
  | { kind: "none" }
  | { kind: "verify"; evidence: DocumentEvidence }
  | { kind: "flag"; evidence: DocumentEvidence };

const DocumentWorkspace: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const location = useLocation();
  const mode = location.pathname.endsWith("/audit") ? "audit" : "view";
  const docId = id ?? "";

  const [data, setData] = useState<DocumentEvidenceResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [pdfPage, setPdfPage] = useState(1);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [modal, setModal] = useState<ModalMode>({ kind: "none" });
  const [filterType, setFilterType] = useState("all");
  const [formStatus, setFormStatus] = useState("verified");
  const [formNotes, setFormNotes] = useState("");
  const [formSeverity, setFormSeverity] = useState("medium");
  const [formDescription, setFormDescription] = useState("");
  const [submitting, setSubmitting] = useState(false);

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

  const handleSelect = (ev: DocumentEvidence) => {
    setSelectedId(ev.id);
    if (ev.node_type === "ComplaintAllegation") return; // paragraph numbers, not PDF pages
    const pageNum = parseInt(String(ev.page_number), 10);
    if (!isNaN(pageNum) && pageNum > 0) {
      setPdfPage(pageNum);
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

  const { splitPercent, containerRef: splitRef, dividerProps, isDragging } = useResizablePanes();
  const pdfUrl = `${API_BASE_URL}/api/documents/${encodeURIComponent(docId)}/file`;

  const allItems = data?.evidence ?? [];
  const filteredItems = filterType === "all"
    ? allItems
    : allItems.filter((ev) => (ev.node_type || "Evidence") === filterType);
  useEffect(() => {
    if (selectedId && !filteredItems.some((ev) => ev.id === selectedId)) {
      setSelectedId(null);
    }
  }, [filterType, selectedId, filteredItems]);
  const selectedItem = allItems.find((ev) => ev.id === selectedId) ?? null;
  const highlightText = selectedItem?.verbatim_quote ?? selectedItem?.title ?? null;
  const highlightPage = selectedItem ? parseInt(String(selectedItem.page_number), 10) || null : null;

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
            <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
              <span style={{ fontSize: "0.95rem", fontWeight: 600, color: "#0f172a" }}>
                {data?.document_title || docId}
              </span>
              {(() => {
                const st = getSourceTypeDisplay(data?.source_type);
                if (!st) return null;
                return (
                  <span
                    title={st.tooltip}
                    style={{
                      backgroundColor: st.color, color: "#fff",
                      padding: "0.1rem 0.45rem", borderRadius: "4px",
                      fontSize: "0.65rem", fontWeight: 600,
                    }}
                  >
                    {st.label}
                  </span>
                );
              })()}
            </div>
            <div style={{ fontSize: "0.74rem", color: "#64748b" }}>
              {mode} mode
            </div>
          </div>
        </div>
        <div style={{ display: "flex", gap: "1.25rem", fontSize: "0.78rem" }}>
          <StatBadge
            label="Items"
            value={filteredItems.length}
            color="#334155"
            suffix={filterType !== "all" ? ` / ${allItems.length}` : undefined}
          />
          <StatBadge label="Verified" value={data?.verified_count ?? 0} color="#047857" />
          <StatBadge label="Flagged" value={data?.flagged_count ?? 0} color="#dc2626" />
        </div>
      </div>

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

      <div ref={splitRef} style={{
        display: "flex", flex: 1, overflow: "hidden",
        userSelect: isDragging ? "none" : "auto",
      }}>
        <div style={{ width: `${splitPercent}%`, overflow: "hidden", display: "flex", flexDirection: "column" }}>
          <PdfViewer
            src={pdfUrl}
            page={pdfPage}
            onPageChange={setPdfPage}
            highlightText={highlightText}
            highlightPage={highlightPage}
          />
        </div>

        <div {...dividerProps}>
          <div style={{ width: "2px", height: "24px", borderRadius: "1px", backgroundColor: "#94a3b8" }} />
        </div>

        <div style={{
          width: `${100 - splitPercent}%`, overflow: "auto", padding: "0.75rem",
          backgroundColor: "#fafbfc",
        }}>
          {modal.kind === "verify" && (
            <InlineVerifyForm
              evidence={modal.evidence}
              status={formStatus}
              notes={formNotes}
              submitting={submitting}
              onStatusChange={setFormStatus}
              onNotesChange={setFormNotes}
              onSubmit={handleSubmitVerify}
              onCancel={() => setModal({ kind: "none" })}
            />
          )}

          {modal.kind === "flag" && (
            <InlineFlagForm
              evidence={modal.evidence}
              severity={formSeverity}
              description={formDescription}
              submitting={submitting}
              onSeverityChange={setFormSeverity}
              onDescriptionChange={setFormDescription}
              onSubmit={handleSubmitFlag}
              onCancel={() => setModal({ kind: "none" })}
            />
          )}

          <NodeTypeFilter
            items={allItems}
            filterType={filterType}
            onFilterChange={setFilterType}
          />

          {filteredItems.map((ev) => (
            <EvidenceCard
              key={ev.id}
              evidence={ev}
              isSelected={selectedId === ev.id}
              onSelect={handleSelect}
              onVerify={handleOpenVerify}
              onFlag={handleOpenFlag}
            />
          ))}

          {filteredItems.length === 0 && (
            <div style={{ textAlign: "center", padding: "2rem", color: "#64748b", fontSize: "0.84rem" }}>
              No content linked to this document.
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

const StatBadge: React.FC<{ label: string; value: number; color: string; suffix?: string }> = ({
  label, value, color, suffix,
}) => (
  <div style={{ textAlign: "center" }}>
    <div style={{ fontSize: "1.1rem", fontWeight: 700, color }}>
      {value}{suffix && <span style={{ fontSize: "0.78rem", fontWeight: 500, color: "#94a3b8" }}>{suffix}</span>}
    </div>
    <div style={{ fontSize: "0.68rem", color: "#64748b" }}>{label}</div>
  </div>
);

export default DocumentWorkspace;
