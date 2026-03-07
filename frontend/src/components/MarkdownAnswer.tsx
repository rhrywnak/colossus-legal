import React from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

interface MarkdownAnswerProps {
  content: string;
}

const MarkdownAnswer: React.FC<MarkdownAnswerProps> = ({ content }) => {
  return (
    <div className="markdown-answer" style={containerStyle}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          h1: ({ children }) => <h1 style={h1Style}>{children}</h1>,
          h2: ({ children }) => <h2 style={h2Style}>{children}</h2>,
          h3: ({ children }) => <h3 style={h3Style}>{children}</h3>,
          p: ({ children }) => <p style={pStyle}>{children}</p>,
          ul: ({ children }) => <ul style={ulStyle}>{children}</ul>,
          ol: ({ children }) => <ol style={olStyle}>{children}</ol>,
          li: ({ children }) => <li style={liStyle}>{children}</li>,
          strong: ({ children }) => <strong style={strongStyle}>{children}</strong>,
          blockquote: ({ children }) => <blockquote style={blockquoteStyle}>{children}</blockquote>,
          code: ({ className, children }) => {
            const isBlock = className?.includes("language-");
            if (isBlock) {
              return (
                <pre style={preStyle}>
                  <code style={codeBlockStyle}>{children}</code>
                </pre>
              );
            }
            return <code style={inlineCodeStyle}>{children}</code>;
          },
          table: ({ children }) => (
            <div style={{ overflowX: "auto", marginBottom: "1rem" }}>
              <table style={tableStyle}>{children}</table>
            </div>
          ),
          th: ({ children }) => <th style={thStyle}>{children}</th>,
          td: ({ children }) => <td style={tdStyle}>{children}</td>,
        }}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
};

// ── Typography styles matching Claude.ai ──

const containerStyle: React.CSSProperties = {
  fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif',
  fontSize: "16px",
  lineHeight: 1.7,
  color: "#1a1a1a",
  maxWidth: "none",
};

const h1Style: React.CSSProperties = {
  fontSize: "1.5rem",
  fontWeight: 700,
  color: "#0f172a",
  marginTop: "1.5rem",
  marginBottom: "0.75rem",
  lineHeight: 1.3,
};

const h2Style: React.CSSProperties = {
  fontSize: "1.25rem",
  fontWeight: 600,
  color: "#0f172a",
  marginTop: "1.5rem",
  marginBottom: "0.6rem",
  paddingBottom: "0.3rem",
  borderBottom: "1px solid #e5e7eb",
  lineHeight: 1.3,
};

const h3Style: React.CSSProperties = {
  fontSize: "1.1rem",
  fontWeight: 600,
  color: "#1e293b",
  marginTop: "1.25rem",
  marginBottom: "0.5rem",
  lineHeight: 1.3,
};

const pStyle: React.CSSProperties = {
  marginTop: 0,
  marginBottom: "1em",
};

const ulStyle: React.CSSProperties = {
  paddingLeft: "1.5rem",
  marginBottom: "1em",
};

const olStyle: React.CSSProperties = {
  paddingLeft: "1.5rem",
  marginBottom: "1em",
};

const liStyle: React.CSSProperties = {
  marginBottom: "0.4em",
};

const strongStyle: React.CSSProperties = {
  fontWeight: 600,
  color: "#0f172a",
};

const blockquoteStyle: React.CSSProperties = {
  borderLeft: "3px solid #d1d5db",
  paddingLeft: "1rem",
  margin: "1em 0",
  color: "#4b5563",
  fontStyle: "italic",
};

const preStyle: React.CSSProperties = {
  backgroundColor: "#f8f9fa",
  borderRadius: "6px",
  padding: "1rem",
  overflowX: "auto",
  marginBottom: "1em",
  border: "1px solid #e5e7eb",
};

const codeBlockStyle: React.CSSProperties = {
  fontFamily: '"Fira Code", "Cascadia Code", "JetBrains Mono", "Consolas", monospace',
  fontSize: "0.875rem",
  lineHeight: 1.6,
};

const inlineCodeStyle: React.CSSProperties = {
  fontFamily: '"Fira Code", "Cascadia Code", "Consolas", monospace',
  fontSize: "0.875em",
  backgroundColor: "#f1f3f5",
  padding: "0.15em 0.4em",
  borderRadius: "3px",
};

const tableStyle: React.CSSProperties = {
  borderCollapse: "collapse",
  width: "100%",
  fontSize: "0.9rem",
  marginBottom: "1rem",
};

const thStyle: React.CSSProperties = {
  backgroundColor: "#f8f9fa",
  border: "1px solid #dee2e6",
  padding: "0.5rem 0.75rem",
  textAlign: "left",
  fontWeight: 600,
};

const tdStyle: React.CSSProperties = {
  border: "1px solid #dee2e6",
  padding: "0.5rem 0.75rem",
};

export default MarkdownAnswer;
