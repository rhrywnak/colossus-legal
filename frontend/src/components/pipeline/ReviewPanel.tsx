/**
 * ReviewPanel — Side-by-side extraction item review with PDF viewer.
 *
 * Left pane: scrollable items list with filters, category-aware actions.
 * Right pane: PdfViewer showing the document, scrolled to the selected item's page.
 * Uses useResizablePanes for a draggable divider.
 */
import React, { useCallback, useEffect, useMemo, useState } from "react";
import PdfViewer from "../shared/PdfViewer";
import { useResizablePanes } from "../../hooks/useResizablePanes";
import {
  fetchDocumentItems, approveItem, rejectItem, editItem, bulkApprove,
  unapproveItem, unrejectItem, ingestDelta, reverifySync,
  ExtractionItem, ReviewSummary, ReverifySyncResponse,
} from "../../services/pipelineApi";
import { getColor } from "../../hooks/useSchema";
import { formatItemProperties } from "../../utils/itemProperties";

interface ReviewPanelProps {
  documentId: string;
  pdfUrl: string;
  documentStatus?: string;
}

// ── Styles ──────────────────────────────────────────────────────

const badge = (bg: string, fg: string): React.CSSProperties => ({
  display: "inline-block", padding: "0.1rem 0.4rem", borderRadius: "9999px",
  fontSize: "0.68rem", fontWeight: 600, backgroundColor: bg, color: fg,
});
const REVIEW_BADGE: Record<string, React.CSSProperties> = {
  approved: badge("var(--state-success-bg-soft)", "var(--status-active-text)"),
  rejected: badge("var(--state-danger-bg-soft)", "var(--status-dropped-text)"),
  pending: badge("var(--bg-page)", "var(--text-muted)"),
  edited: badge("var(--state-info-bg-soft)", "var(--bias-indigo-text)"),
};
const CATEGORY_BADGE: Record<string, React.CSSProperties> = {
  foundation: badge("var(--accent-bg-soft)", "var(--accent-primary-hover)"),
  structural: badge("var(--burden-warning-bg)", "var(--burden-warning-text)"),
  evidence: badge("var(--state-success-bg-soft)", "var(--status-active-text)"),
  reference: badge("var(--bg-page)", "var(--text-secondary)"),
};
const filterSel: React.CSSProperties = {
  padding: "0.3rem 0.5rem", fontSize: "0.76rem", borderRadius: "4px",
  border: "1px solid var(--border-default)", fontFamily: "inherit", color: "var(--text-secondary)",
};
const actionBtn = (bg: string, fg: string, border: string): React.CSSProperties => ({
  padding: "0.2rem 0.5rem", fontSize: "0.72rem", fontWeight: 500,
  border: `1px solid ${border}`, borderRadius: "4px", backgroundColor: bg,
  color: fg, cursor: "pointer", fontFamily: "inherit",
});
const secondaryBtn: React.CSSProperties = {
  ...actionBtn("var(--bg-page)", "var(--text-muted)", "var(--border-default)"),
  fontSize: "0.66rem", padding: "0.15rem 0.35rem",
};
const cardBase: React.CSSProperties = {
  padding: "0.6rem 0.75rem", borderRadius: "6px", border: "1px solid var(--border-default)",
  backgroundColor: "var(--bg-surface)", cursor: "pointer", marginBottom: "0.4rem",
  transition: "border-color 0.15s",
};

// Grounding statuses considered "verified" for filter purposes.
// Mirrors backend GROUNDED_STATUSES in review.rs — items with these
// statuses are approved/ingested and should not appear under "Not
// verified". Any new status NOT in this set defaults to "Not verified"
// (visible, not hidden).
const VERIFIED_STATUSES = new Set(["exact", "normalized", "derived", "unverified", "manual"]);

// ── Grounding indicator ─────────────────────────────────────────

function GroundingIndicator({ status }: { status: string | null }) {
  switch (status) {
    case "exact":
    case "normalized":
      return <span style={{ fontSize: "0.66rem", color: "var(--status-active-text)" }} title="Verified in document">&#10003; Verified</span>;
    case "not_found":
      return <span style={{ fontSize: "0.66rem", color: "var(--state-warning-strong)" }} title="Not verified">&#9888; Not verified</span>;
    case "derived":
      return <span style={{ fontSize: "0.66rem", color: "var(--accent-primary)" }} title="Derived from other entities">&var(--status-active-text); Derived</span>;
    case "unverified":
      return <span style={{ fontSize: "0.66rem", color: "var(--text-disabled)" }} title="Unverified">&mdash; Unverified</span>;
    case "missing_quote":
      return <span style={{ fontSize: "0.66rem", color: "var(--state-danger-strong)" }} title="Missing quote">&#10007; Missing quote</span>;
    default:
      return null;
  }
}

// ── Provenance display ──────────────────────────────────────────

function ProvenanceLinks({ item }: { item: ExtractionItem }) {
  const provenance = (item.properties?.provenance as Array<{ ref_type?: string; ref?: string; quote_snippet?: string }>) ?? null;
  if (!provenance || !Array.isArray(provenance) || provenance.length === 0) return null;

  return (
    <div style={{ fontSize: "0.7rem", color: "var(--text-secondary)", marginTop: "0.25rem", paddingLeft: "0.5rem",
      borderLeft: "2px solid var(--accent-bg-soft)" }}>
      <div style={{ fontWeight: 600, marginBottom: "0.15rem" }}>Derived from:</div>
      {provenance.map((p, i) => (
        <div key={i} style={{ marginBottom: "0.1rem" }}>
          &rarr; &para;{p.ref}: <span style={{ fontStyle: "italic" }}>"{p.quote_snippet}"</span>
        </div>
      ))}
    </div>
  );
}

// ── Helper: resolve actions from backend fields ─────────────────

function getActions(item: ExtractionItem): string[] {
  if (item.available_actions && item.available_actions.length > 0) {
    return item.available_actions;
  }
  // Fallback to legacy booleans
  const actions: string[] = [];
  if (item.can_approve) actions.push("approve");
  if (item.can_reject) actions.push("reject");
  if (item.can_edit) actions.push("edit");
  return actions;
}

// ── Component ───────────────────────────────────────────────────

const REVERIFY_ALLOWED_STATUSES = new Set(["INGESTED", "INDEXED", "PUBLISHED", "COMPLETED"]);

const ReviewPanel: React.FC<ReviewPanelProps> = ({ documentId, pdfUrl, documentStatus }) => {
  const [items, setItems] = useState<ExtractionItem[]>([]);
  const [summary, setSummary] = useState<ReviewSummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [pdfPage, setPdfPage] = useState(1);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editPage, setEditPage] = useState("");
  const [editQuote, setEditQuote] = useState("");
  const [reverifyBusy, setReverifyBusy] = useState(false);
  const [reverifyResult, setReverifyResult] = useState<ReverifySyncResponse | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);

  // Filters
  const [typeFilter, setTypeFilter] = useState("all");
  const [reviewFilter, setReviewFilter] = useState("all");
  const [groundFilter, setGroundFilter] = useState("all");

  const { splitPercent, containerRef, dividerProps, isDragging } = useResizablePanes();

  const loadItems = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetchDocumentItems(documentId, { per_page: 500 });
      setItems(res.items);
      setSummary(res.summary ?? null);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load items");
    } finally {
      setLoading(false);
    }
  }, [documentId]);

  useEffect(() => { loadItems(); }, [loadItems]);

  const filtered = useMemo(() => {
    return items.filter((it) => {
      if (typeFilter !== "all" && it.entity_type !== typeFilter) return false;
      if (reviewFilter !== "all" && (it.review_status || "pending").toLowerCase() !== reviewFilter) return false;
      if (groundFilter === "grounded") {
        const gs = it.grounding_status || "";
        if (!VERIFIED_STATUSES.has(gs)) return false;
      } else if (groundFilter === "ungrounded") {
        const gs = it.grounding_status || "";
        if (VERIFIED_STATUSES.has(gs)) return false;
      }
      return true;
    });
  }, [items, typeFilter, reviewFilter, groundFilter]);

  const entityTypes = useMemo(() => {
    return Array.from(new Set(items.map((it) => it.entity_type))).sort();
  }, [items]);

  // Group foundation items for the summary header
  const foundationItems = useMemo(() => {
    return items.filter((it) => it.category === "foundation");
  }, [items]);
  const hasFoundation = foundationItems.length > 0;

  // Any locked items? Drives the "some items are read-only" context note.
  // Unlike the old allLocked flag, this does NOT hide the bulk-approve button
  // or the per-item actions — those are gated by the backend's per-item
  // `locked` field so pending items stay actionable on PUBLISHED docs.
  const anyLocked = items.some((it) => it.locked === true);

  const selectedItem = items.find((it) => it.id === selectedId) ?? null;
  const highlightText = selectedItem?.verbatim_quote ?? null;
  const highlightPage = selectedItem?.grounded_page ?? null;

  const pending = summary?.pending ?? 0;
  const approved = summary?.approved ?? 0;
  const rejected = summary?.rejected ?? 0;

  const handleSelect = (item: ExtractionItem) => {
    setSelectedId(item.id);
    if (item.grounded_page) setPdfPage(item.grounded_page);
  };

  const handleApprove = async (id: number) => {
    setActionError(null);
    try { await approveItem(id); await loadItems(); }
    catch (e) { setActionError(e instanceof Error ? e.message : "Approve failed"); }
  };

  const handleReject = async (id: number) => {
    const reason = window.prompt("Rejection reason:");
    if (reason === null) return;
    setActionError(null);
    try { await rejectItem(id, reason); await loadItems(); }
    catch (e) { setActionError(e instanceof Error ? e.message : "Reject failed"); }
  };

  const handleUnapprove = async (id: number) => {
    setActionError(null);
    try { await unapproveItem(id); await loadItems(); }
    catch (e) { setActionError(e instanceof Error ? e.message : "Unapprove failed"); }
  };

  const handleUnreject = async (id: number) => {
    setActionError(null);
    try { await unrejectItem(id); await loadItems(); }
    catch (e) { setActionError(e instanceof Error ? e.message : "Unreject failed"); }
  };

  const startEdit = (item: ExtractionItem) => {
    setEditingId(item.id);
    setEditPage(item.grounded_page?.toString() ?? "");
    setEditQuote(item.verbatim_quote ?? "");
  };

  const saveEdit = async () => {
    if (editingId === null) return;
    const updates: { grounded_page?: number; verbatim_quote?: string } = {};
    const pg = parseInt(editPage, 10);
    if (!isNaN(pg) && pg > 0) updates.grounded_page = pg;
    if (editQuote.trim()) updates.verbatim_quote = editQuote.trim();
    try { await editItem(editingId, updates); setEditingId(null); await loadItems(); } catch { /* silent */ }
  };

  const [bulkMsg, setBulkMsg] = useState<string | null>(null);
  const [deltaBusy, setDeltaBusy] = useState(false);
  const [deltaMsg, setDeltaMsg] = useState<string | null>(null);

  const handleBulkApprove = async () => {
    setBulkMsg(null);
    try {
      const result = await bulkApprove(documentId, "grounded") as {
        approved_count: number; skipped_ungrounded: number;
      };
      await loadItems();
      if (result.skipped_ungrounded > 0) {
        setBulkMsg(`Approved ${result.approved_count} items. ${result.skipped_ungrounded} ungrounded items skipped.`);
      } else {
        setBulkMsg(`Approved all ${result.approved_count} items.`);
      }
    } catch { /* silent */ }
  };

  const handleIngestDelta = async () => {
    setDeltaBusy(true);
    setDeltaMsg(null);
    try {
      const result = await ingestDelta(documentId);
      await loadItems();
      const nodes = result.nodes_written.total;
      const rels = result.relationships_written.total;
      const skipped = result.skipped_relationships;
      const base = `Wrote ${nodes} node${nodes === 1 ? "" : "s"} and ${rels} relationship${rels === 1 ? "" : "s"} to graph.`;
      const extra = skipped > 0 ? ` ${skipped} relationship${skipped === 1 ? "" : "s"} deferred (endpoint still pending).` : "";
      setDeltaMsg(base + extra);
    } catch (e) {
      setDeltaMsg(e instanceof Error ? e.message : "Delta ingest failed");
    } finally {
      setDeltaBusy(false);
    }
  };

  const handleReverifySync = async () => {
    setReverifyBusy(true);
    setReverifyResult(null);
    setActionError(null);
    try {
      const result = await reverifySync(documentId);
      setReverifyResult(result);
      await loadItems();
    } catch (e) {
      setActionError(e instanceof Error ? e.message : "Re-verify & Sync failed");
    } finally {
      setReverifyBusy(false);
    }
  };

  if (loading && items.length === 0) {
    return <div style={{ padding: "2rem", textAlign: "center", color: "var(--text-disabled)" }}>Loading review items...</div>;
  }
  if (error && items.length === 0) {
    return <div style={{ padding: "2rem", textAlign: "center", color: "var(--state-danger-strong)" }}>{error}</div>;
  }

  return (
    <div ref={containerRef} style={{
      display: "flex", height: "calc(100vh - 300px)", minHeight: "400px",
      border: "1px solid var(--border-default)", borderRadius: "8px", overflow: "hidden",
      userSelect: isDragging ? "none" : "auto",
    }}>
      {/* Left pane: items list */}
      <div style={{ width: `${splitPercent}%`, overflow: "auto", padding: "0.75rem", backgroundColor: "var(--bg-surface)" }}>
        {/* Soft context note — shows when some items are already in the graph.
            Pending items remain actionable; only approved/rejected items are
            read-only post-ingest. */}
        {anyLocked && (
          <div style={{ padding: "0.5rem 0.75rem", marginBottom: "0.5rem", borderRadius: "6px",
            backgroundColor: "var(--accent-bg-soft)", border: "1px solid var(--accent-bg-soft)", fontSize: "0.76rem", color: "var(--accent-primary-hover)" }}>
            Items already written to the graph are read-only. Pending items can still be approved, rejected, or edited. To modify ingested items, revert ingest from the Processing tab.
          </div>
        )}

        {/* Foundation summary */}
        {hasFoundation && (
          <div style={{ padding: "0.4rem 0.6rem", marginBottom: "0.5rem", borderRadius: "6px",
            backgroundColor: "var(--accent-bg-soft)", border: "1px solid var(--accent-bg-soft)", fontSize: "0.72rem", color: "var(--accent-primary-hover)" }}>
            <span style={{ fontWeight: 600 }}>Foundation Entities: </span>
            {(() => {
              const byType: Record<string, number> = {};
              foundationItems.forEach((it) => { byType[it.entity_type] = (byType[it.entity_type] || 0) + 1; });
              return Object.entries(byType).map(([t, c]) => `${c} ${t}`).join(", ");
            })()}
          </div>
        )}

        {actionError && (
          <div style={{ padding: "0.4rem 0.6rem", marginBottom: "0.5rem", borderRadius: "6px",
            backgroundColor: "var(--state-danger-bg-soft)", border: "1px solid var(--state-danger-border)", fontSize: "0.76rem", color: "var(--status-dropped-text)",
            display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span>{actionError}</span>
            <button onClick={() => setActionError(null)}
              style={{ background: "none", border: "none", cursor: "pointer", color: "var(--status-dropped-text)",
                fontWeight: 600, fontSize: "0.8rem", fontFamily: "inherit" }}>&times;</button>
          </div>
        )}

        {/* Summary + Bulk actions */}
        <div style={{ display: "flex", gap: "0.75rem", alignItems: "center", marginBottom: "0.5rem", flexWrap: "wrap" }}>
          <span style={{ fontSize: "0.76rem", color: "var(--text-secondary)", fontWeight: 600 }}>
            {pending} pending
          </span>
          <span style={{ fontSize: "0.76rem", color: "var(--status-active-text)" }}>{approved} approved</span>
          <span style={{ fontSize: "0.76rem", color: "var(--status-dropped-text)" }}>{rejected} rejected</span>
          {/* Bulk approve is always available — the backend filters the
              update to pending items only, so on PUBLISHED docs this only
              affects the still-reviewable set, not the ingested ones. */}
          <button style={actionBtn("var(--state-success-bg-soft)", "var(--status-active-text)", "var(--state-success-bg-soft)")} onClick={handleBulkApprove}>
            Approve All Grounded
          </button>
          {bulkMsg && (
            <span style={{ fontSize: "0.72rem", color: "var(--status-active-text)", fontStyle: "italic" }}>{bulkMsg}</span>
          )}
          {/* Delta ingest — visible only when there are approved/edited
              items still missing from Neo4j. Backend returns this count
              as summary.pending_graph_write. */}
          {(summary?.pending_graph_write ?? 0) > 0 && (
            <button
              style={{ ...actionBtn("var(--accent-bg-soft)", "var(--accent-primary)", "var(--accent-bg-soft)"), opacity: deltaBusy ? 0.6 : 1 }}
              onClick={handleIngestDelta}
              disabled={deltaBusy}
              title="Write approved items that aren't yet in the graph"
            >
              {deltaBusy
                ? "Writing to graph…"
                : `Write ${summary?.pending_graph_write} approved item${(summary?.pending_graph_write ?? 0) === 1 ? "" : "s"} to graph`}
            </button>
          )}
          {deltaMsg && (
            <span style={{ fontSize: "0.72rem", color: "var(--accent-primary)", fontStyle: "italic" }}>{deltaMsg}</span>
          )}
          {documentStatus && REVERIFY_ALLOWED_STATUSES.has(documentStatus) && (
            <button
              style={{ ...actionBtn("var(--bias-purple-bg-soft)", "var(--bias-purple-text)", "var(--bias-purple-bg-soft)"), opacity: reverifyBusy ? 0.6 : 1 }}
              onClick={handleReverifySync}
              disabled={reverifyBusy}
              title="Re-verify all items, auto-approve grounded items, and write to graph"
            >
              {reverifyBusy ? "Re-verifying…" : "Re-verify & Sync"}
            </button>
          )}
        </div>

        {reverifyResult && (
          <div style={{ padding: "0.5rem 0.75rem", marginBottom: "0.5rem", borderRadius: "6px",
            backgroundColor: "var(--bias-purple-bg-soft)", border: "1px solid var(--bias-purple-bg-soft)", fontSize: "0.76rem", color: "var(--bias-purple-text)" }}>
            <div style={{ fontWeight: 600, marginBottom: "0.2rem" }}>Re-verify & Sync complete</div>
            <div>
              Verified {reverifyResult.verify_results.total_items} items:
              {" "}{reverifyResult.verify_results.exact} exact,
              {" "}{reverifyResult.verify_results.normalized} normalized,
              {" "}{reverifyResult.verify_results.derived} derived,
              {" "}{reverifyResult.verify_results.not_found} not found.
              {" "}<strong>{reverifyResult.verify_results.changed} changed</strong> from previous.
            </div>
            {reverifyResult.auto_approve_results.newly_approved > 0 && (
              <div>{reverifyResult.auto_approve_results.newly_approved} newly approved.</div>
            )}
            {reverifyResult.ingest_delta_results && reverifyResult.ingest_delta_results.written_to_graph > 0 && (
              <div>{reverifyResult.ingest_delta_results.written_to_graph} written to graph.</div>
            )}
            {reverifyResult.partial_error && (
              <div style={{ color: "var(--status-dropped-text)", marginTop: "0.2rem" }}>Partial error: {reverifyResult.partial_error}</div>
            )}
            <button onClick={() => setReverifyResult(null)}
              style={{ background: "none", border: "none", cursor: "pointer", color: "var(--bias-purple-text)",
                fontWeight: 600, fontSize: "0.72rem", fontFamily: "inherit", padding: 0, marginTop: "0.2rem" }}>Dismiss</button>
          </div>
        )}

        {/* Filters */}
        <div style={{ display: "flex", gap: "0.5rem", marginBottom: "0.75rem", flexWrap: "wrap" }}>
          <select style={filterSel} value={typeFilter} onChange={(e) => setTypeFilter(e.target.value)}>
            <option value="all">All types</option>
            {entityTypes.map((t) => <option key={t} value={t}>{t}</option>)}
          </select>
          <select style={filterSel} value={reviewFilter} onChange={(e) => setReviewFilter(e.target.value)}>
            <option value="all">All review</option>
            <option value="pending">Pending</option>
            <option value="approved">Approved</option>
            <option value="rejected">Rejected</option>
          </select>
          <select style={filterSel} value={groundFilter} onChange={(e) => setGroundFilter(e.target.value)}>
            <option value="all">All items</option>
            <option value="grounded">Verified in document</option>
            <option value="ungrounded">Not verified</option>
          </select>
          <span style={{ fontSize: "0.72rem", color: "var(--text-muted)", alignSelf: "center" }}>
            {filtered.length} / {items.length}
          </span>
        </div>

        {/* Items */}
        {filtered.map((item) => {
          const actions = getActions(item);
          const isLocked = item.locked === true;

          return (
            <div key={item.id}
              style={{ ...cardBase, borderColor: selectedId === item.id ? "var(--accent-primary)" : "var(--border-default)",
                backgroundColor: selectedId === item.id ? "var(--accent-bg-soft)" : "var(--bg-surface)",
                opacity: isLocked ? 0.75 : 1 }}
              onClick={() => handleSelect(item)}
            >
              <div style={{ display: "flex", alignItems: "center", gap: "0.4rem", marginBottom: "0.25rem", flexWrap: "wrap" }}>
                {(() => {
                  // Show the post-ingest label ("Person"/"Organization") when
                  // Ingest has resolved a Party. Schema-driven category lookup
                  // still runs on `entity_type` (the LLM label) upstream.
                  const displayType = item.resolved_entity_type ?? item.entity_type;
                  return (
                    <span style={{
                      display: "inline-block", padding: "0.1rem 0.4rem", borderRadius: "4px",
                      fontSize: "0.66rem", fontWeight: 600, color: "var(--bg-surface)",
                      backgroundColor: getColor(displayType),
                    }}>{displayType}</span>
                  );
                })()}
                {item.category && CATEGORY_BADGE[item.category] && (
                  <span style={CATEGORY_BADGE[item.category]}>{item.category}</span>
                )}
                <span style={{ fontSize: "0.82rem", fontWeight: 600, color: "var(--text-primary)" }}>{item.label}</span>
                <span style={REVIEW_BADGE[(item.review_status || "pending").toLowerCase()] ?? REVIEW_BADGE.pending}>
                  {(item.review_status || "pending").toLowerCase()}
                </span>
                {item.grounded_page && (
                  <span style={{ fontSize: "0.68rem", color: "var(--text-muted)" }}>p.{item.grounded_page}</span>
                )}
                <GroundingIndicator status={item.grounding_status} />
              </div>

              {(() => {
                const props = formatItemProperties(item);
                if (props.length === 0) return null;
                return (
                  <div style={{
                    display: "flex", flexWrap: "wrap", gap: "0.7rem",
                    fontSize: "0.72rem", color: "var(--text-secondary)",
                    marginBottom: "0.35rem", lineHeight: 1.4,
                  }}>
                    {props.map((p) => (
                      <span key={p.label}>
                        <strong style={{ color: "var(--text-secondary)", fontWeight: 600 }}>{p.label}:</strong>{" "}
                        {p.value}
                      </span>
                    ))}
                  </div>
                );
              })()}

              {item.verbatim_quote && (
                <div style={{ fontSize: "0.74rem", color: "var(--text-muted)", fontStyle: "italic", lineHeight: 1.4,
                  maxHeight: "2.8em", overflow: "hidden", marginBottom: "0.35rem" }}>
                  "{item.verbatim_quote.length > 120 ? item.verbatim_quote.slice(0, 120) + "..." : item.verbatim_quote}"
                </div>
              )}

              {/* Provenance links for derived entities */}
              {item.grounding_status === "derived" && <ProvenanceLinks item={item} />}

              {/* Locked message */}
              {isLocked && (
                <div style={{ fontSize: "0.66rem", color: "var(--burden-warning-text)", marginTop: "0.2rem" }}>
                  Post-ingest &mdash; revert ingest to modify
                </div>
              )}

              {/* Edit form */}
              {editingId === item.id ? (
                <div style={{ display: "flex", gap: "0.4rem", alignItems: "center", flexWrap: "wrap", marginTop: "0.3rem" }}
                  onClick={(e) => e.stopPropagation()}>
                  <input value={editPage} onChange={(e) => setEditPage(e.target.value)}
                    placeholder="Page" style={{ width: "3.5rem", padding: "0.2rem 0.3rem", fontSize: "0.72rem",
                    border: "1px solid var(--border-default)", borderRadius: "4px" }} />
                  <input value={editQuote} onChange={(e) => setEditQuote(e.target.value)}
                    placeholder="Quote" style={{ flex: 1, minWidth: "120px", padding: "0.2rem 0.3rem",
                    fontSize: "0.72rem", border: "1px solid var(--border-default)", borderRadius: "4px" }} />
                  <button style={actionBtn("var(--state-success-bg-soft)", "var(--status-active-text)", "var(--state-success-bg-soft)")} onClick={saveEdit}>Save</button>
                  <button style={actionBtn("var(--bg-surface)", "var(--text-muted)", "var(--border-default)")} onClick={() => setEditingId(null)}>Cancel</button>
                </div>
              ) : !isLocked && actions.length > 0 ? (
                <div style={{ display: "flex", gap: "0.3rem", marginTop: "0.2rem", flexWrap: "wrap" }}
                  onClick={(e) => e.stopPropagation()}>
                  {actions.includes("confirm") && (
                    <button style={actionBtn("var(--state-info-bg-soft)", "var(--bias-indigo-text)", "var(--state-info-bg-soft)")} onClick={() => handleApprove(item.id)}>Confirm</button>
                  )}
                  {actions.includes("approve") && (
                    <button style={actionBtn("var(--state-success-bg-soft)", "var(--status-active-text)", "var(--state-success-bg-soft)")} onClick={() => handleApprove(item.id)}>Approve</button>
                  )}
                  {actions.includes("reject") && (
                    <button style={actionBtn("var(--state-danger-bg-soft)", "var(--state-danger-strong)", "var(--state-danger-border)")} onClick={() => handleReject(item.id)}>Reject</button>
                  )}
                  {actions.includes("edit") && (
                    <button style={actionBtn("var(--bg-surface)", "var(--text-muted)", "var(--border-default)")} onClick={() => startEdit(item)}>Edit</button>
                  )}
                  {actions.includes("unapprove") && (
                    <button style={secondaryBtn} onClick={() => handleUnapprove(item.id)}>Unapprove</button>
                  )}
                  {actions.includes("unreject") && (
                    <button style={secondaryBtn} onClick={() => handleUnreject(item.id)}>Unreject</button>
                  )}
                </div>
              ) : null}
            </div>
          );
        })}
      </div>

      {/* Divider */}
      <div {...dividerProps}>
        <div style={{ width: "2px", height: "24px", borderRadius: "1px", backgroundColor: "var(--text-disabled)" }} />
      </div>

      {/* Right pane: PDF viewer */}
      <div style={{ width: `${100 - splitPercent}%`, overflow: "hidden", display: "flex", flexDirection: "column" }}>
        <PdfViewer
          src={pdfUrl}
          page={pdfPage}
          onPageChange={setPdfPage}
          highlightText={highlightText}
          highlightPage={highlightPage}
        />
      </div>
    </div>
  );
};

export default ReviewPanel;
