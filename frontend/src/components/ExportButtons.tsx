import React, { useState } from "react";
import { saveAs } from "file-saver";
import { AskResponse } from "../services/ask";
import { generateAnswerDocx } from "../utils/generateDocx";

interface ExportButtonsProps {
  markdown: string;
  question: string;
  askedBy?: string;
  askedAt?: string;
  response?: AskResponse;
}

const ExportButtons: React.FC<ExportButtonsProps> = ({
  markdown, question, askedBy, askedAt, response,
}) => {
  const [copied, setCopied] = useState<string | null>(null);
  const [downloading, setDownloading] = useState(false);

  const copyToClipboard = async (text: string, label: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(label);
      setTimeout(() => setCopied(null), 2000);
    } catch {
      const textarea = document.createElement("textarea");
      textarea.value = text;
      document.body.appendChild(textarea);
      textarea.select();
      document.execCommand("copy");
      document.body.removeChild(textarea);
      setCopied(label);
      setTimeout(() => setCopied(null), 2000);
    }
  };

  const handleCopyMarkdown = () => {
    const header = `# Q: ${question}\n` +
      (askedBy ? `*Asked by ${askedBy}` : "") +
      (askedAt ? ` on ${new Date(askedAt).toLocaleDateString()}` : "") +
      (askedBy || askedAt ? "*\n\n" : "\n") +
      "---\n\n";
    copyToClipboard(header + markdown, "markdown");
  };

  const handleCopyPlainText = () => {
    const plain = markdown
      .replace(/#{1,6}\s+/g, "")
      .replace(/\*\*(.+?)\*\*/g, "$1")
      .replace(/\*(.+?)\*/g, "$1")
      .replace(/`(.+?)`/g, "$1")
      .replace(/^\s*[-*]\s+/gm, "\u2022 ")
      .replace(/^\s*\d+\.\s+/gm, "")
      .replace(/\[(.+?)\]\(.+?\)/g, "$1")
      .replace(/---+/g, "")
      .replace(/\n{3,}/g, "\n\n");
    copyToClipboard(`Q: ${question}\n\n${plain}`, "plain");
  };

  const handleDownloadDocx = async () => {
    if (!response || downloading) return;
    setDownloading(true);
    try {
      const blob = await generateAnswerDocx(response);
      const dateStr = new Date().toISOString().slice(0, 10);
      const slug = question.slice(0, 30).replace(/[^a-zA-Z0-9]/g, "_");
      saveAs(blob, `minerva_${slug}_${dateStr}.docx`);
    } catch (err) {
      console.error("Failed to generate docx:", err);
    } finally {
      setDownloading(false);
    }
  };

  const buttonStyle: React.CSSProperties = {
    padding: "0.35rem 0.75rem",
    fontSize: "0.8rem",
    border: "1px solid #d1d5db",
    borderRadius: "5px",
    backgroundColor: "#fff",
    color: "#374151",
    cursor: "pointer",
    display: "inline-flex",
    alignItems: "center",
    gap: "0.3rem",
  };

  return (
    <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
      <button onClick={handleCopyMarkdown} style={buttonStyle}>
        {copied === "markdown" ? "\u2713 Copied!" : "\uD83D\uDCCB Copy Markdown"}
      </button>
      <button onClick={handleCopyPlainText} style={buttonStyle}>
        {copied === "plain" ? "\u2713 Copied!" : "\uD83D\uDCC4 Copy Text"}
      </button>
      {response && (
        <button onClick={handleDownloadDocx} disabled={downloading} style={buttonStyle}>
          {downloading ? "Generating..." : "\u2B07 Download Docx"}
        </button>
      )}
    </div>
  );
};

export default ExportButtons;
