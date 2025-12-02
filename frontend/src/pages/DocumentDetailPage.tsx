import React, { useEffect, useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";
import {
  DocumentDetail,
  updateDocument,
  getDocument,
  DocumentUpdateRequest,
} from "../services/documents";

const ALLOWED_DOC_TYPES = ["pdf", "motion", "ruling", "evidence", "filing"] as const;

type ServiceError =
  | { kind: "validation"; field?: string; message: string }
  | { kind: "not_found"; message: string }
  | { kind: "network"; message: string };

const DocumentDetailPage: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const [document, setDocument] = useState<DocumentDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notFound, setNotFound] = useState(false);

  const [title, setTitle] = useState("");
  const [docType, setDocType] = useState("");
  const [createdAt, setCreatedAt] = useState("");
  const [fieldError, setFieldError] = useState<string | null>(null);
  const [saveSuccess, setSaveSuccess] = useState(false);
  const [saving, setSaving] = useState(false);

  const docId = id ?? "";

  useEffect(() => {
    if (!docId) return;
    let active = true;
    setLoading(true);
    setError(null);
    setNotFound(false);

    getDocument(docId)
      .then((data) => {
        if (!active) return;
        setDocument(data);
        setTitle(data.title ?? "");
        setDocType(data.doc_type ?? "");
        setCreatedAt(data.created_at ?? "");
      })
      .catch((err: ServiceError) => {
        if (!active) return;
        if (err.kind === "not_found") {
          setNotFound(true);
        } else {
          setError(err.message || "Failed to load document");
        }
      })
      .finally(() => {
        if (active) {
          setLoading(false);
        }
      });

    return () => {
      active = false;
    };
  }, [docId]);

  const onSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!document) return;
    setFieldError(null);
    setSaveSuccess(false);
    setError(null);
    setSaving(true);

    const payload: DocumentUpdateRequest = {
      title: title.trim(),
      doc_type: docType.trim(),
      created_at: createdAt.trim() || undefined,
    };

    try {
      const updated = await updateDocument(docId, payload);
      setDocument(updated);
      setTitle(updated.title ?? "");
      setDocType(updated.doc_type ?? "");
      setCreatedAt(updated.created_at ?? "");
      setSaveSuccess(true);
    } catch (err: any) {
      if (err?.kind === "validation") {
        setFieldError(err.field || null);
        setError(err.message || "Validation error");
      } else if (err?.kind === "not_found") {
        setNotFound(true);
      } else {
        setError(err?.message || "Failed to update document");
      }
    } finally {
      setSaving(false);
    }
  };

  const docTypeOptions = useMemo(() => ALLOWED_DOC_TYPES, []);

  if (!docId) {
    return <div>Document ID is required.</div>;
  }

  if (loading) {
    return <div>Loading document...</div>;
  }

  if (notFound) {
    return (
      <div>
        <p>Document not found.</p>
        <Link to="/documents">Back to documents</Link>
      </div>
    );
  }

  if (error) {
    return (
      <div role="alert">
        <p>Error: {error}</p>
        <Link to="/documents">Back to documents</Link>
      </div>
    );
  }

  if (!document) {
    return <div>Document not available.</div>;
  }

  return (
    <div>
      <h2>Document Detail</h2>
      <div style={{ marginBottom: "1rem" }}>
        <div><strong>ID:</strong> {document.id}</div>
        <div><strong>Current Type:</strong> {document.doc_type}</div>
        <div><strong>Created At:</strong> {document.created_at ?? "—"}</div>
      </div>

      <form onSubmit={onSubmit} style={{ display: "grid", gap: "0.75rem", maxWidth: 480 }}>
        <div>
          <label htmlFor="title" style={{ display: "block", marginBottom: "0.25rem" }}>
            Title
          </label>
          <input
            id="title"
            type="text"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            style={{ width: "100%", padding: "0.4rem" }}
          />
          {fieldError === "title" ? (
            <div style={{ color: "#c00", marginTop: "0.25rem" }}>Title is required.</div>
          ) : null}
        </div>

        <div>
          <label htmlFor="docType" style={{ display: "block", marginBottom: "0.25rem" }}>
            Document Type
          </label>
          <select
            id="docType"
            value={docType}
            onChange={(e) => setDocType(e.target.value)}
            style={{ width: "100%", padding: "0.4rem" }}
          >
            <option value="">Select a type</option>
            {docTypeOptions.map((opt) => (
              <option key={opt} value={opt}>
                {opt}
              </option>
            ))}
          </select>
          {fieldError === "doc_type" ? (
            <div style={{ color: "#c00", marginTop: "0.25rem" }}>
              Document type must be one of: {docTypeOptions.join(", ")}
            </div>
          ) : null}
        </div>

        <div>
          <label htmlFor="createdAt" style={{ display: "block", marginBottom: "0.25rem" }}>
            Created At (ISO-8601)
          </label>
          <input
            id="createdAt"
            type="text"
            value={createdAt}
            onChange={(e) => setCreatedAt(e.target.value)}
            placeholder="2025-12-02T10:00:00Z"
            style={{ width: "100%", padding: "0.4rem" }}
          />
          {fieldError === "created_at" ? (
            <div style={{ color: "#c00", marginTop: "0.25rem" }}>
              Created at must be ISO-8601.
            </div>
          ) : null}
        </div>

        {error && fieldError === null ? (
          <div style={{ color: "#c00" }}>{error}</div>
        ) : null}
        {saveSuccess ? (
          <div style={{ color: "#0a0" }}>Saved successfully.</div>
        ) : null}

        <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
          <button type="submit" disabled={saving} style={{ padding: "0.5rem 0.75rem" }}>
            {saving ? "Saving..." : "Save"}
          </button>
          <Link to="/documents">Back to documents</Link>
        </div>
      </form>
    </div>
  );
};

export default DocumentDetailPage;
