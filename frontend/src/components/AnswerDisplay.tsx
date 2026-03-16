import React from "react";
import { AskResponse } from "../services/ask";
import { API_BASE_URL } from "../services/api";
import MarkdownAnswer from "./MarkdownAnswer";
import ExportButtons from "./ExportButtons";

interface Props {
  response: AskResponse;
}

const AnswerDisplay: React.FC<Props> = ({ response }) => {
  return (
    <div>
      {/* Answer text with markdown rendering */}
      <div style={{
        padding: "1.5rem", backgroundColor: "#ffffff", borderRadius: "8px",
        border: "1px solid #e5e7eb", marginBottom: "1rem",
      }}>
        <MarkdownAnswer content={response.answer} />

        {/* Source Documents — clickable links to PDFs */}
        {response.sources && response.sources.length > 0 && (
          <div style={{
            marginTop: "1rem",
            paddingTop: "0.75rem",
            borderTop: "1px solid #e5e7eb",
          }}>
            <div
              style={{
                fontSize: "0.8rem",
                fontWeight: 600,
                color: "#6b7280",
                textTransform: "uppercase",
                letterSpacing: "0.05em",
                marginBottom: "0.5rem",
              }}
            >
              Sources ({response.sources.length})
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem" }}>
              {response.sources.map((source, idx) => (
                <div key={idx} style={{ fontSize: "0.85rem", display: "flex", alignItems: "baseline", gap: "0.5rem" }}>
                  <a
                    href={`${API_BASE_URL}/documents/${encodeURIComponent(source.document_id)}/file${
                      source.page_number !== undefined ? `#page=${source.page_number}` : ""
                    }`}
                    target="_blank"
                    rel="noopener noreferrer"
                    style={{ color: "#2563eb", textDecoration: "none" }}
                    onMouseEnter={(e) => { (e.target as HTMLElement).style.textDecoration = "underline"; }}
                    onMouseLeave={(e) => { (e.target as HTMLElement).style.textDecoration = "none"; }}
                  >
                    {source.document_title}
                    {source.page_number !== undefined && (
                      <span style={{ color: "#6b7280", fontWeight: 400 }}> (p. {source.page_number})</span>
                    )}
                  </a>
                  <span style={{ color: "#9ca3af", fontSize: "0.8rem" }}>— {source.evidence_title}</span>
                </div>
              ))}
            </div>
          </div>
        )}

        <div style={{ marginTop: "1rem", paddingTop: "0.75rem", borderTop: "1px solid #f3f4f6" }}>
          <ExportButtons markdown={response.answer} question={response.question} response={response} />
        </div>
      </div>
    </div>
  );
};

export default AnswerDisplay;
