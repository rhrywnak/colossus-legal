import React, { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { fetchSchemas, uploadDocument, SchemaInfo } from "../../services/pipelineApi";

interface Props {
  open: boolean;
  onClose: () => void;
  onSuccess: () => void;
}

const MAX_SIZE = 50 * 1024 * 1024;

const overlay: React.CSSProperties = {
  position: "fixed", inset: 0, backgroundColor: "rgba(0,0,0,0.4)",
  display: "flex", alignItems: "center", justifyContent: "center", zIndex: 1000,
};
const card: React.CSSProperties = {
  backgroundColor: "#ffffff", borderRadius: "12px", padding: "1.75rem",
  maxWidth: "480px", width: "90%", boxShadow: "0 20px 60px rgba(0,0,0,0.15)",
};
const titleRow: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "1.25rem",
};
const dropZone = (dragging: boolean): React.CSSProperties => ({
  border: `2px dashed ${dragging ? "#2563eb" : "#cbd5e1"}`,
  borderRadius: "8px", padding: "1.5rem", textAlign: "center", cursor: "pointer",
  backgroundColor: dragging ? "#eff6ff" : "#f8fafc", marginBottom: "1rem",
  transition: "all 0.15s ease",
});
const labelStyle: React.CSSProperties = {
  fontSize: "0.76rem", fontWeight: 600, color: "#334155", marginBottom: "0.3rem",
};
const selectStyle: React.CSSProperties = {
  width: "100%", padding: "0.5rem", fontSize: "0.84rem", borderRadius: "6px",
  border: "1px solid #e2e8f0", fontFamily: "inherit", marginBottom: "1rem",
};
const btnRow: React.CSSProperties = {
  display: "flex", justifyContent: "flex-end", gap: "0.5rem",
};
const btnCancel: React.CSSProperties = {
  padding: "0.45rem 1rem", fontSize: "0.84rem", fontWeight: 500, border: "1px solid #e2e8f0",
  borderRadius: "6px", backgroundColor: "#ffffff", color: "#64748b", cursor: "pointer", fontFamily: "inherit",
};
const btnUpload = (enabled: boolean): React.CSSProperties => ({
  padding: "0.45rem 1rem", fontSize: "0.84rem", fontWeight: 600, border: "none",
  borderRadius: "6px", backgroundColor: enabled ? "#2563eb" : "#94a3b8",
  color: "#ffffff", cursor: enabled ? "pointer" : "default", fontFamily: "inherit",
});

function slugify(name: string): string {
  return name.toLowerCase().replace(/\.pdf$/i, "").replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
}
function titleize(name: string): string {
  return name.replace(/\.pdf$/i, "").replace(/[-_]+/g, " ").replace(/\b\w/g, c => c.toUpperCase());
}

const UploadDialog: React.FC<Props> = ({ open, onClose, onSuccess }) => {
  const navigate = useNavigate();
  const fileRef = useRef<HTMLInputElement>(null);
  const [schemas, setSchemas] = useState<SchemaInfo[]>([]);
  const [file, setFile] = useState<File | null>(null);
  const [schema, setSchema] = useState("");
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);

  useEffect(() => {
    if (open) {
      fetchSchemas().then(setSchemas).catch(() => setSchemas([]));
      setFile(null);
      setSchema("");
      setError(null);
    }
  }, [open]);

  useEffect(() => {
    if (schemas.length === 1 && !schema) setSchema(schemas[0].name);
  }, [schemas, schema]);

  if (!open) return null;

  const handleFile = (f: File) => {
    if (!f.name.toLowerCase().endsWith(".pdf")) { setError("Only PDF files accepted"); return; }
    if (f.size > MAX_SIZE) { setError(`File too large (${(f.size / 1024 / 1024).toFixed(1)} MB, max 50 MB)`); return; }
    setFile(f);
    setError(null);
  };

  const handleUpload = async () => {
    if (!file || !schema || uploading) return;
    setUploading(true);
    setError(null);
    try {
      const id = `doc-${slugify(file.name)}`;
      const title = titleize(file.name);
      const doc = await uploadDocument(file, {
        id, title, documentType: schema, schemaFile: `${schema}.yaml`,
      });
      onSuccess();
      navigate(`/pipeline/${doc.id}`);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Upload failed");
    } finally {
      setUploading(false);
    }
  };

  const canUpload = !!file && !!schema && !uploading;

  return (
    <div style={overlay} onClick={onClose}>
      <div style={card} onClick={(e) => e.stopPropagation()}>
        <div style={titleRow}>
          <h2 style={{ margin: 0, fontSize: "1.1rem", fontWeight: 700, color: "#0f172a" }}>Upload Document</h2>
          <button onClick={onClose} style={{ background: "none", border: "none", fontSize: "1.25rem", color: "#94a3b8", cursor: "pointer" }}>{"\u2715"}</button>
        </div>

        <div
          style={dropZone(dragging)}
          onClick={() => fileRef.current?.click()}
          onDragOver={(e) => { e.preventDefault(); setDragging(true); }}
          onDragLeave={() => setDragging(false)}
          onDrop={(e) => { e.preventDefault(); setDragging(false); if (e.dataTransfer.files[0]) handleFile(e.dataTransfer.files[0]); }}
        >
          <input ref={fileRef} type="file" accept=".pdf" style={{ display: "none" }}
            onChange={(e) => { if (e.target.files?.[0]) handleFile(e.target.files[0]); }} />
          {file ? (
            <div>
              <div style={{ fontWeight: 600, color: "#0f172a", fontSize: "0.84rem" }}>{file.name}</div>
              <div style={{ fontSize: "0.76rem", color: "#64748b" }}>{(file.size / 1024 / 1024).toFixed(1)} MB</div>
            </div>
          ) : (
            <div style={{ color: "#64748b", fontSize: "0.84rem" }}>Drop PDF here or click to browse</div>
          )}
        </div>

        <div style={labelStyle}>Document Type</div>
        <select style={selectStyle} value={schema} onChange={(e) => setSchema(e.target.value)}>
          <option value="">Select type...</option>
          {schemas.map((s) => <option key={s.name} value={s.name}>{s.label}</option>)}
        </select>

        {error && (
          <div style={{ padding: "0.5rem 0.75rem", backgroundColor: "#fef2f2", border: "1px solid #fecaca", borderRadius: "6px", color: "#991b1b", fontSize: "0.76rem", marginBottom: "0.75rem" }}>
            {error}
          </div>
        )}

        <div style={btnRow}>
          <button style={btnCancel} onClick={onClose}>Cancel</button>
          <button style={btnUpload(canUpload)} onClick={handleUpload} disabled={!canUpload}>
            {uploading ? "Uploading..." : "Upload & Process"}
          </button>
        </div>
      </div>
    </div>
  );
};

export default UploadDialog;
