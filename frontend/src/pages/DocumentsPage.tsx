import React, { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { API_BASE_URL } from "../services/api";
import { DocumentItem, getDocuments } from "../services/documents";

// Document type color mapping
const DOC_TYPE_COLORS: Record<string, string> = {
  complaint: "#dc2626",
  motion: "#2563eb",
  affidavit: "#7c3aed",
  ruling: "#059669",
  response: "#d97706",
  opinion: "#0891b2",
  filing: "#4b5563",
  evidence: "#0d9488",
  pdf: "#6b7280",
};

const getTypeColor = (docType: string): string => {
  return DOC_TYPE_COLORS[docType.toLowerCase()] ?? DOC_TYPE_COLORS.pdf ?? "#6b7280";
};

// Styles
const pageStyle: React.CSSProperties = {
  backgroundColor: "#f9fafb",
  minHeight: "calc(100vh - 100px)",
  margin: "-1.5rem",
  padding: "1.5rem",
};

const headerStyle: React.CSSProperties = {
  marginBottom: "1.5rem",
};

const titleStyle: React.CSSProperties = {
  fontSize: "1.5rem",
  fontWeight: 700,
  color: "#1f2937",
  margin: 0,
};

const subtitleStyle: React.CSSProperties = {
  fontSize: "0.95rem",
  color: "#6b7280",
  marginTop: "0.25rem",
};

const groupStyle: React.CSSProperties = {
  marginBottom: "2rem",
};

const gridStyle: React.CSSProperties = {
  display: "grid",
  gridTemplateColumns: "repeat(auto-fit, minmax(320px, 1fr))",
  gap: "1rem",
};

const cardStyle: React.CSSProperties = {
  backgroundColor: "#ffffff",
  border: "1px solid #e5e7eb",
  borderRadius: "8px",
  padding: "1.25rem",
  transition: "box-shadow 0.2s ease",
  display: "flex",
  flexDirection: "column",
};

const cardTitleStyle: React.CSSProperties = {
  fontSize: "1rem",
  fontWeight: 600,
  color: "#1f2937",
  textDecoration: "none",
  display: "block",
};

const filenameStyle: React.CSSProperties = {
  fontSize: "0.75rem",
  color: "#6b7280",
  marginTop: "0.25rem",
  fontFamily: "monospace",
};

const dateStyle: React.CSSProperties = {
  fontSize: "0.8rem",
  color: "#6b7280",
  marginTop: "0.5rem",
};

const cardFooterStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  marginTop: "auto",
  paddingTop: "1rem",
};

const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block",
  padding: "0.25rem 0.625rem",
  backgroundColor: color,
  color: "#ffffff",
  borderRadius: "9999px",
  fontSize: "0.75rem",
  fontWeight: 500,
  textTransform: "capitalize",
});

const viewPdfButtonStyle: React.CSSProperties = {
  display: "inline-block",
  padding: "0.375rem 0.75rem",
  backgroundColor: "#2563eb",
  color: "#ffffff",
  borderRadius: "6px",
  textDecoration: "none",
  fontSize: "0.8rem",
  fontWeight: 500,
};

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
    return (
      <div style={{ padding: "2rem", textAlign: "center", color: "#6b7280" }}>
        Loading documents...
      </div>
    );
  }

  if (error) {
    return (
      <div
        style={{
          padding: "1rem",
          backgroundColor: "#fef2f2",
          border: "1px solid #fecaca",
          borderRadius: "6px",
          color: "#dc2626",
        }}
      >
        Error loading documents: {error}
      </div>
    );
  }

  if (documents.length === 0) {
    return (
      <div style={{ padding: "2rem", textAlign: "center", color: "#6b7280" }}>
        No documents found.
      </div>
    );
  }

  // Group documents by type
  const groupedDocs = documents.reduce(
    (acc, doc) => {
      const type = doc.docType || "other";
      if (!acc[type]) acc[type] = [];
      acc[type].push(doc);
      return acc;
    },
    {} as Record<string, DocumentItem[]>
  );

  // Sort group keys alphabetically
  const sortedTypes = Object.keys(groupedDocs).sort();

  return (
    <div style={pageStyle}>
      {/* Header */}
      <div style={headerStyle}>
        <h1 style={titleStyle}>Documents</h1>
        <p style={subtitleStyle}>
          {documents.length} document{documents.length !== 1 ? "s" : ""}
        </p>
      </div>

      {/* Grouped Cards */}
      {sortedTypes.map((type) => (
        <div key={type} style={groupStyle}>
          <h3
            style={{
              fontSize: "1.1rem",
              fontWeight: 600,
              color: "#374151",
              marginBottom: "1rem",
              paddingBottom: "0.5rem",
              borderBottom: "2px solid #e5e7eb",
              display: "flex",
              alignItems: "center",
              gap: "0.5rem",
            }}
          >
            <span
              style={{
                width: "4px",
                height: "1.25rem",
                backgroundColor: getTypeColor(type),
                borderRadius: "2px",
              }}
            ></span>
            {type.charAt(0).toUpperCase() + type.slice(1)}
            {type.endsWith("s") ? "" : "s"}
          </h3>
          <div style={gridStyle}>
            {groupedDocs[type].map((doc) => (
              <div
                key={doc.id}
                style={cardStyle}
                onMouseEnter={(e) => {
                  e.currentTarget.style.boxShadow = "0 4px 12px rgba(0,0,0,0.1)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.boxShadow = "none";
                }}
              >
                {/* Title */}
                <Link to={`/documents/${doc.id}`} style={cardTitleStyle}>
                  {doc.title}
                </Link>

                {/* Filename */}
                {doc.filePath && (
                  <div style={filenameStyle}>{doc.filePath}</div>
                )}

                {/* Date */}
                {doc.createdAt && (
                  <div style={dateStyle}>Created: {doc.createdAt}</div>
                )}

                {/* Footer: Type badge + View PDF button */}
                <div style={cardFooterStyle}>
                  <span style={badgeStyle(getTypeColor(doc.docType))}>
                    {doc.docType}
                  </span>
                  {doc.filePath && (
                    <a
                      href={`${API_BASE_URL}/documents/${encodeURIComponent(doc.id)}/file`}
                      target="_blank"
                      rel="noopener noreferrer"
                      style={viewPdfButtonStyle}
                    >
                      View PDF
                    </a>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
};

export default DocumentsPage;
