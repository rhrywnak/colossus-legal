import {
  Document, Packer, Paragraph, TextRun, HeadingLevel,
  AlignmentType, BorderStyle, Header, Footer,
  convertInchesToTwip,
} from "docx";
import { AskResponse } from "../services/ask";

// ─── Inline formatting parser ────────────────────────────────────────────────

/** Split a line into TextRun segments handling **bold** and *italic* markers. */
function parseInlineFormatting(text: string, baseItalic = false): TextRun[] {
  const runs: TextRun[] = [];
  // Match **bold** or *italic* segments
  const regex = /(\*\*(.+?)\*\*|\*(.+?)\*)/g;
  let lastIndex = 0;
  let match: RegExpExecArray | null;

  while ((match = regex.exec(text)) !== null) {
    // Text before this match
    if (match.index > lastIndex) {
      runs.push(new TextRun({ text: text.slice(lastIndex, match.index), italics: baseItalic, font: "Arial", size: 22 }));
    }
    if (match[2]) {
      // **bold**
      runs.push(new TextRun({ text: match[2], bold: true, italics: baseItalic, font: "Arial", size: 22 }));
    } else if (match[3]) {
      // *italic*
      runs.push(new TextRun({ text: match[3], italics: true, font: "Arial", size: 22 }));
    }
    lastIndex = match.index + match[0].length;
  }

  if (lastIndex < text.length) {
    runs.push(new TextRun({ text: text.slice(lastIndex), italics: baseItalic, font: "Arial", size: 22 }));
  }
  if (runs.length === 0) {
    runs.push(new TextRun({ text: text || " ", italics: baseItalic, font: "Arial", size: 22 }));
  }
  return runs;
}

// ─── Line-by-line markdown to Paragraph converter ────────────────────────────

function markdownToParagraphs(markdown: string): Paragraph[] {
  const paragraphs: Paragraph[] = [];
  const lines = markdown.split("\n");

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    // Skip empty lines
    if (line.trim() === "") {
      paragraphs.push(new Paragraph({ spacing: { after: 100 } }));
      continue;
    }

    // Horizontal rule
    if (/^---+$/.test(line.trim())) {
      paragraphs.push(new Paragraph({
        border: { bottom: { style: BorderStyle.SINGLE, size: 1, color: "CCCCCC" } },
        spacing: { after: 200 },
      }));
      continue;
    }

    // Headings
    const headingMatch = line.match(/^(#{1,3})\s+(.+)/);
    if (headingMatch) {
      const level = headingMatch[1].length;
      const heading = level === 1 ? HeadingLevel.HEADING_1
        : level === 2 ? HeadingLevel.HEADING_2 : HeadingLevel.HEADING_3;
      paragraphs.push(new Paragraph({
        heading,
        children: [new TextRun({ text: headingMatch[2], bold: true, font: "Arial", size: level === 1 ? 28 : level === 2 ? 24 : 22 })],
        spacing: { before: 240, after: 120 },
      }));
      continue;
    }

    // Blockquote
    if (line.startsWith(">")) {
      const text = line.replace(/^>\s*/, "");
      paragraphs.push(new Paragraph({
        children: parseInlineFormatting(text, true),
        indent: { left: 720 },
        border: { left: { style: BorderStyle.SINGLE, size: 6, color: "93C5FD" } },
        spacing: { after: 80 },
      }));
      continue;
    }

    // Bullet list
    const bulletMatch = line.match(/^\s*[-*]\s+(.+)/);
    if (bulletMatch) {
      paragraphs.push(new Paragraph({
        children: parseInlineFormatting(bulletMatch[1]),
        bullet: { level: 0 },
        spacing: { after: 60 },
      }));
      continue;
    }

    // Numbered list
    const numberedMatch = line.match(/^\s*\d+\.\s+(.+)/);
    if (numberedMatch) {
      paragraphs.push(new Paragraph({
        children: parseInlineFormatting(numberedMatch[1]),
        numbering: { reference: "default-numbering", level: 0 },
        spacing: { after: 60 },
      }));
      continue;
    }

    // Plain paragraph
    paragraphs.push(new Paragraph({
      children: parseInlineFormatting(line),
      spacing: { after: 120 },
    }));
  }

  return paragraphs;
}

// ─── Document generator ──────────────────────────────────────────────────────

export async function generateAnswerDocx(response: AskResponse): Promise<Blob> {
  const stats = response.retrieval_stats;
  const totalSec = (stats.total_ms / 1000).toFixed(1);
  const dateStr = new Date().toLocaleDateString("en-US", { year: "numeric", month: "long", day: "numeric" });

  const doc = new Document({
    numbering: {
      config: [{ reference: "default-numbering", levels: [{ level: 0, format: "decimal", text: "%1.", alignment: AlignmentType.START }] }],
    },
    sections: [{
      properties: {
        page: {
          size: { width: 12240, height: 15840 },
          margin: { top: convertInchesToTwip(1), bottom: convertInchesToTwip(1), left: convertInchesToTwip(1), right: convertInchesToTwip(1) },
        },
      },
      headers: {
        default: new Header({
          children: [new Paragraph({
            children: [
              new TextRun({ text: "COLOSSUS LEGAL — Case Analysis Report", font: "Arial", size: 16, color: "6B7280" }),
              new TextRun({ text: `    Generated: ${dateStr}`, font: "Arial", size: 16, color: "9CA3AF" }),
            ],
            alignment: AlignmentType.LEFT,
          })],
        }),
      },
      footers: {
        default: new Footer({
          children: [new Paragraph({
            children: [
              new TextRun({ text: "Awad v. Catholic Family Services", font: "Arial", size: 16, color: "6B7280", italics: true }),
              new TextRun({ text: "  |  Generated by Colossus Legal", font: "Arial", size: 16, color: "9CA3AF" }),
            ],
            alignment: AlignmentType.CENTER,
          })],
        }),
      },
      children: [
        // Question section
        new Paragraph({
          children: [new TextRun({ text: "Question:", bold: true, font: "Arial", size: 22, color: "374151" })],
          spacing: { after: 80 },
        }),
        new Paragraph({
          children: [new TextRun({ text: response.question, font: "Arial", size: 24, color: "1E293B" })],
          spacing: { after: 200 },
        }),
        new Paragraph({ border: { bottom: { style: BorderStyle.SINGLE, size: 1, color: "E5E7EB" } }, spacing: { after: 300 } }),

        // Answer content
        ...markdownToParagraphs(response.answer),

        // Metadata section
        new Paragraph({ border: { bottom: { style: BorderStyle.SINGLE, size: 1, color: "E5E7EB" } }, spacing: { before: 400, after: 200 } }),
        new Paragraph({
          children: [new TextRun({ text: "Analysis Metadata", bold: true, font: "Arial", size: 22, color: "374151" })],
          spacing: { after: 120 },
        }),
        ...[
          `Model: ${response.provider}`,
          `Evidence hits: ${stats.qdrant_hits}`,
          `Graph nodes expanded: ${stats.graph_nodes_expanded}`,
          `Response time: ${totalSec}s`,
          `Context tokens: ~${stats.context_tokens.toLocaleString()}`,
        ].map((line) => new Paragraph({
          children: [new TextRun({ text: line, font: "Arial", size: 20, color: "6B7280" })],
          spacing: { after: 40 },
        })),
      ],
    }],
  });

  return Packer.toBlob(doc);
}
