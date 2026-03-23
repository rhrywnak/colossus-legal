import React, { useEffect, useState } from "react";
import {
  AdminDocument,
  getAdminDocuments,
  registerDocument,
  importEvidence,
  RegisterDocumentRequest,
  ImportEvidenceRequest,
} from "../../services/admin";

// ── Styles ────────────────────────────────────────────────────────────────────

const cardStyle: React.CSSProperties = {
  backgroundColor: "#ffffff", border: "1px solid #e2e8f0", borderRadius: "10px",
  padding: "1.25rem 1.5rem",
};

const btnPrimary: React.CSSProperties = {
  backgroundColor: "#2563eb", color: "#fff", border: "none", borderRadius: "6px",
  padding: "0.45rem 1rem", fontSize: "0.82rem", fontWeight: 600, cursor: "pointer",
  fontFamily: "inherit",
};

const btnSecondary: React.CSSProperties = {
  backgroundColor: "#f1f5f9", color: "#334155", border: "1px solid #e2e8f0",
  borderRadius: "6px", padding: "0.45rem 1rem", fontSize: "0.82rem", fontWeight: 500,
  cursor: "pointer", fontFamily: "inherit",
};

const inputStyle: React.CSSProperties = {
  width: "100%", padding: "0.45rem 0.65rem", border: "1px solid #e2e8f0",
  borderRadius: "6px", fontSize: "0.84rem", fontFamily: "inherit",
  boxSizing: "border-box",
};

const labelStyle: React.CSSProperties = {
  display: "block", fontSize: "0.76rem", fontWeight: 600, color: "#475569",
  marginBottom: "0.25rem",
};

const msgSuccess: React.CSSProperties = {
  padding: "0.65rem 1rem", backgroundColor: "#ecfdf5", border: "1px solid #a7f3d0",
  borderRadius: "6px", fontSize: "0.84rem", color: "#047857", marginBottom: "1rem",
};

const msgError: React.CSSProperties = {
  padding: "0.65rem 1rem", backgroundColor: "#fef2f2", border: "1px solid #fecaca",
  borderRadius: "6px", fontSize: "0.84rem", color: "#dc2626", marginBottom: "1rem",
};

// ── Component ─────────────────────────────────────────────────────────────────

const AdminDocuments: React.FC = () => {
  const [docs, setDocs] = useState<AdminDocument[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [success, setSuccess] = useState("");
  const [showRegister, setShowRegister] = useState(false);
  const [showImport, setShowImport] = useState(false);
  const [submitting, setSubmitting] = useState(false);

  // Register form
  const [regForm, setRegForm] = useState<RegisterDocumentRequest>({
    id: "", title: "", doc_type: "discovery", file_path: "",
  });

  // Import evidence
  const [importJson, setImportJson] = useState("");
  const [importDocId, setImportDocId] = useState("");

  const loadDocs = async () => {
    setLoading(true);
    try {
      const data = await getAdminDocuments();
      setDocs(data.documents);
      setError("");
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { loadDocs(); }, []);

  const handleRegister = async () => {
    setSubmitting(true);
    setError("");
    setSuccess("");
    try {
      const res = await registerDocument(regForm);
      setSuccess(`Registered "${res.title}" (hash: ${res.content_hash.slice(0, 12)}...)`);
      setShowRegister(false);
      setRegForm({ id: "", title: "", doc_type: "discovery", file_path: "" });
      loadDocs();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSubmitting(false);
    }
  };

  const handleImport = async () => {
    setSubmitting(true);
    setError("");
    setSuccess("");
    try {
      const parsed = JSON.parse(importJson) as ImportEvidenceRequest["evidence"];
      const res = await importEvidence({ document_id: importDocId, evidence: parsed });
      setSuccess(`Imported ${res.created} evidence items`);
      setShowImport(false);
      setImportJson("");
      setImportDocId("");
      loadDocs();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSubmitting(false);
    }
  };

  const totalEvidence = docs.reduce((sum, d) => sum + d.evidence_count, 0);

  return (
    <div>
      {/* Stats bar */}
      <div style={{ display: "flex", gap: "1rem", marginBottom: "1rem" }}>
        <div style={{ ...cardStyle, flex: 1, textAlign: "center" }}>
          <div style={{ fontSize: "1.5rem", fontWeight: 700, color: "#0f172a" }}>{docs.length}</div>
          <div style={{ fontSize: "0.76rem", color: "#64748b" }}>Documents</div>
        </div>
        <div style={{ ...cardStyle, flex: 1, textAlign: "center" }}>
          <div style={{ fontSize: "1.5rem", fontWeight: 700, color: "#0f172a" }}>{totalEvidence}</div>
          <div style={{ fontSize: "0.76rem", color: "#64748b" }}>Evidence Items</div>
        </div>
      </div>

      {success && <div style={msgSuccess}>{success}</div>}
      {error && <div style={msgError}>{error}</div>}

      {/* Action buttons */}
      <div style={{ display: "flex", gap: "0.5rem", marginBottom: "1rem" }}>
        <button style={btnPrimary} onClick={() => { setShowRegister(!showRegister); setShowImport(false); }}>
          {showRegister ? "Cancel" : "Register Document"}
        </button>
        <button style={btnSecondary} onClick={() => { setShowImport(!showImport); setShowRegister(false); }}>
          {showImport ? "Cancel" : "Import Evidence"}
        </button>
      </div>

      {/* Register form */}
      {showRegister && (
        <div style={{ ...cardStyle, marginBottom: "1rem" }}>
          <div style={{ fontSize: "0.9rem", fontWeight: 600, color: "#0f172a", marginBottom: "0.75rem" }}>
            Register New Document
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "0.75rem" }}>
            <div>
              <label style={labelStyle}>Document ID</label>
              <input style={inputStyle} value={regForm.id} placeholder="doc-my-document"
                onChange={(e) => setRegForm({ ...regForm, id: e.target.value })} />
            </div>
            <div>
              <label style={labelStyle}>Title</label>
              <input style={inputStyle} value={regForm.title}
                onChange={(e) => setRegForm({ ...regForm, title: e.target.value })} />
            </div>
            <div>
              <label style={labelStyle}>Type</label>
              <select style={inputStyle} value={regForm.doc_type}
                onChange={(e) => setRegForm({ ...regForm, doc_type: e.target.value })}>
                <option value="complaint">Complaint</option>
                <option value="discovery">Discovery</option>
                <option value="motion">Motion</option>
                <option value="court_ruling">Court Ruling</option>
                <option value="appellate_brief">Appellate Brief</option>
                <option value="affidavit">Affidavit</option>
              </select>
            </div>
            <div>
              <label style={labelStyle}>PDF Filename</label>
              <input style={inputStyle} value={regForm.file_path} placeholder="filename.pdf"
                onChange={(e) => setRegForm({ ...regForm, file_path: e.target.value })} />
            </div>
          </div>
          <div style={{ marginTop: "0.75rem" }}>
            <button style={btnPrimary} onClick={handleRegister} disabled={submitting}>
              {submitting ? "Registering..." : "Register"}
            </button>
          </div>
        </div>
      )}

      {/* Import evidence form */}
      {showImport && (
        <div style={{ ...cardStyle, marginBottom: "1rem" }}>
          <div style={{ fontSize: "0.9rem", fontWeight: 600, color: "#0f172a", marginBottom: "0.75rem" }}>
            Import Evidence JSON
          </div>
          <div style={{ marginBottom: "0.5rem" }}>
            <label style={labelStyle}>Document ID</label>
            <select style={inputStyle} value={importDocId}
              onChange={(e) => setImportDocId(e.target.value)}>
              <option value="">Select document...</option>
              {docs.map((d) => <option key={d.id} value={d.id}>{d.title}</option>)}
            </select>
          </div>
          <div style={{ marginBottom: "0.5rem" }}>
            <label style={labelStyle}>Evidence JSON Array</label>
            <textarea style={{ ...inputStyle, minHeight: "120px", fontFamily: "monospace", fontSize: "0.78rem" }}
              value={importJson} placeholder='[{"id": "...", "title": "...", ...}]'
              onChange={(e) => setImportJson(e.target.value)} />
          </div>
          <button style={btnPrimary} onClick={handleImport} disabled={submitting || !importDocId}>
            {submitting ? "Importing..." : "Import"}
          </button>
        </div>
      )}

      {/* Document table */}
      <div style={cardStyle}>
        {loading ? (
          <div style={{ textAlign: "center", padding: "2rem", color: "#64748b" }}>Loading...</div>
        ) : (
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "0.84rem" }}>
            <thead>
              <tr style={{ borderBottom: "2px solid #e2e8f0" }}>
                <th style={{ textAlign: "left", padding: "0.5rem 0.5rem 0.5rem 0", color: "#64748b", fontWeight: 600 }}>Title</th>
                <th style={{ textAlign: "left", padding: "0.5rem", color: "#64748b", fontWeight: 600 }}>Type</th>
                <th style={{ textAlign: "right", padding: "0.5rem", color: "#64748b", fontWeight: 600 }}>Evidence</th>
                <th style={{ textAlign: "center", padding: "0.5rem", color: "#64748b", fontWeight: 600 }}>PDF</th>
              </tr>
            </thead>
            <tbody>
              {docs.map((d) => (
                <tr key={d.id} style={{ borderBottom: "1px solid #f1f5f9" }}>
                  <td style={{ padding: "0.5rem 0.5rem 0.5rem 0", color: "#0f172a", fontWeight: 500 }}>{d.title}</td>
                  <td style={{ padding: "0.5rem", color: "#475569", textTransform: "capitalize" }}>{d.doc_type ?? "-"}</td>
                  <td style={{ padding: "0.5rem", textAlign: "right", color: "#0f172a", fontWeight: 500 }}>{d.evidence_count}</td>
                  <td style={{ padding: "0.5rem", textAlign: "center" }}>
                    {d.has_pdf ? <span style={{ color: "#047857" }}>Yes</span> : <span style={{ color: "#dc2626" }}>No</span>}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
};

export default AdminDocuments;
