import React, { useEffect, useState } from "react";
import { DocumentItem, getDocuments } from "../services/documents";

const DocumentsPage: React.FC = () => {
  const [documents, setDocuments] = useState<DocumentItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;

    const loadDocuments = async () => {
      try {
        const data = await getDocuments();
        if (!active) return;
        setDocuments(data);
        setError(null);
      } catch (err) {
        if (!active) return;
        const message = err instanceof Error ? err.message : "Unknown error";
        setError(message);
      } finally {
        if (active) {
          setLoading(false);
        }
      }
    };

    loadDocuments();

    return () => {
      active = false;
    };
  }, []);

  if (loading) {
    return <div>Loading documents...</div>;
  }

  if (error) {
    return <div role="alert">Error loading documents: {error}</div>;
  }

  if (documents.length === 0) {
    return <div>No documents found.</div>;
  }

  return (
    <div>
      <h2>Documents</h2>
      <p>Showing documents from backend.</p>
      <ul style={{ paddingLeft: "1.25rem" }}>
        {documents.map((doc) => (
          <li key={doc.id} style={{ marginBottom: "0.75rem" }}>
            <strong>{doc.title}</strong> <em>({doc.docType})</em>
            {doc.createdAt ? (
              <div style={{ color: "#555", marginTop: "0.15rem" }}>
                Created: {doc.createdAt}
              </div>
            ) : null}
          </li>
        ))}
      </ul>
    </div>
  );
};

export default DocumentsPage;
