// =============================================================================
// DocumentDateEditor.tsx — the post-hoc edit path (task P4b, B2 §3)
// =============================================================================
//
// The nine already-ingested documents were never asked for their date, so intake
// alone cannot fill them in. This is where Roman backfills them, from the
// documents themselves — minutes of work, not a project, and deliberately not a
// script: nothing parses "041212" out of a title, and nothing seeds a date.
//
// It reads as a line of text until clicked, because on the document workspace the
// date is a fact about the document, not a form. Clicking it opens the SAME
// control the upload dialog uses, so the mandatory-with-override rule is one
// implementation and cannot differ between the two screens.
//
// ## Three states, three readings (Standing Rule 1)
//
//   "Date not set"        — nobody has been asked. All nine start here.
//   "No date on document" — someone looked, and it carries none. An ANSWER.
//   "5 November 2009"     — dated, with its precision shown when it is not a day.
//
// Collapsing the first two would make every unasked document look answered.

import React, { useCallback, useEffect, useState } from "react";

import {
  DatePrecisionOption,
  fetchDatePrecisions,
  fetchDocumentDate,
  setDocumentDate,
} from "../../services/documentDate";
import DocumentDateField, {
  DocumentDateValue,
  isDocumentDateComplete,
} from "./DocumentDateField";

interface Props {
  documentId: string;
}

const UNSET: DocumentDateValue = { date: null, precision: "" };

const summaryStyle: React.CSSProperties = {
  background: "none",
  border: "none",
  padding: 0,
  cursor: "pointer",
  fontSize: "0.74rem",
  color: "var(--accent-primary)",
  textDecoration: "underline",
};

const panelStyle: React.CSSProperties = {
  marginTop: "0.5rem",
  padding: "0.75rem",
  border: "1px solid var(--border-default)",
  borderRadius: "6px",
  background: "var(--bg-surface)",
  minWidth: "16rem",
};

const errorStyle: React.CSSProperties = {
  padding: "0.4rem 0.6rem",
  backgroundColor: "var(--state-danger-bg-soft)",
  border: "1px solid var(--state-danger-border)",
  borderRadius: "6px",
  color: "var(--status-dropped-text)",
  fontSize: "0.74rem",
  marginBottom: "0.5rem",
};

/**
 * The one-line summary of a stored date.
 *
 * Exported and pure so it can be tested without rendering — this repo has no
 * component-test infrastructure, so the readable behaviour has to live in a
 * helper if it is to be covered at all.
 */
export function summarise(
  date: string | null | undefined,
  precisionLabel: string | null | undefined,
  precision: string | null | undefined,
): string {
  if (!precision) return "Date not set";
  if (!date) return precisionLabel ?? "No date on the document";
  if (precision === "day") return date;
  // For a coarser precision the stored day is padding, so it is not shown as
  // though the document stated it.
  if (precision === "month") return `${date.slice(0, 7)} (month only)`;
  if (precision === "year") return `${date.slice(0, 4)} (year only)`;
  return `${date} (${precisionLabel ?? precision})`;
}

const DocumentDateEditor: React.FC<Props> = ({ documentId }) => {
  const [summary, setSummary] = useState<string>("Date not set");
  const [editing, setEditing] = useState(false);
  const [value, setValue] = useState<DocumentDateValue>(UNSET);
  const [precisions, setPrecisions] = useState<DatePrecisionOption[]>([]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    fetchDocumentDate(documentId)
      .then((stored) => {
        setSummary(
          summarise(
            stored.document_date,
            stored.date_precision_label,
            stored.date_precision,
          ),
        );
        setValue({
          date: stored.document_date ?? null,
          precision: stored.date_precision ?? "",
        });
      })
      .catch((e: unknown) => {
        setError(
          e instanceof Error
            ? `Could not read the document date: ${e.message}`
            : "Could not read the document date",
        );
      });
  }, [documentId]);

  useEffect(() => {
    load();
    fetchDatePrecisions()
      .then(setPrecisions)
      .catch((e: unknown) => {
        setPrecisions([]);
        setError(
          e instanceof Error
            ? `Could not load the date options: ${e.message}`
            : "Could not load the date options",
        );
      });
  }, [load]);

  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      await setDocumentDate(documentId, value.date, value.precision);
      setEditing(false);
      load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Could not save the date");
    } finally {
      setSaving(false);
    }
  };

  if (!editing) {
    return (
      <div style={{ fontSize: "0.74rem", color: "var(--text-muted)" }}>
        Document date:{" "}
        <button style={summaryStyle} onClick={() => setEditing(true)}>
          {summary}
        </button>
        {error && <div style={errorStyle}>{error}</div>}
      </div>
    );
  }

  return (
    <div style={panelStyle}>
      {error && <div style={errorStyle}>{error}</div>}
      <DocumentDateField value={value} onChange={setValue} disabled={saving} />
      <div style={{ display: "flex", gap: "0.5rem" }}>
        <button
          onClick={save}
          disabled={saving || !isDocumentDateComplete(value, precisions)}
          style={{
            padding: "0.35rem 0.75rem",
            borderRadius: "6px",
            border: "none",
            background: "var(--accent-primary)",
            color: "var(--bg-surface)",
            fontSize: "0.76rem",
            cursor: saving ? "default" : "pointer",
          }}
        >
          {saving ? "Saving..." : "Save"}
        </button>
        <button
          onClick={() => {
            setEditing(false);
            setError(null);
            load();
          }}
          disabled={saving}
          style={{
            padding: "0.35rem 0.75rem",
            borderRadius: "6px",
            border: "1px solid var(--border-default)",
            background: "none",
            color: "var(--text-secondary)",
            fontSize: "0.76rem",
            cursor: "pointer",
          }}
        >
          Cancel
        </button>
      </div>
    </div>
  );
};

export default DocumentDateEditor;
