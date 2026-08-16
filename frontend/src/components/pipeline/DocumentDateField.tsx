// =============================================================================
// DocumentDateField.tsx — the document's own date, asked once at intake and
// correctable afterwards (task P4b, B2 §3)
// =============================================================================
//
// ONE control, used by two screens: the upload dialog asks for it before the
// file is sent, and the document page shows the same pair for correction. The
// two screens differ only in what they do with the value, so the control itself
// is shared and the rule it enforces cannot diverge between them.
//
// ## Mandatory-with-override, in the UI
//
// The precision select has NO pre-selected value: the user must answer. Choosing
// a real precision reveals the date input and the parent's submit stays disabled
// until it is filled; choosing "No date on the document" hides the input and
// enables submit. A blank date is never accepted silently — which is the whole
// ruling — and the escape hatch is a visible, one-click answer rather than
// something the user has to defeat the form to express.
//
// ## Why the precisions are fetched
//
// Standing Rule 12. The vocabulary and the "does this need a date?" flag are the
// backend's, and a hardcoded copy here would drift from the Rust lookup the
// first time one changed.

import React, { useEffect, useState } from "react";

import {
  DatePrecisionOption,
  fetchDatePrecisions,
} from "../../services/documentDate";

export interface DocumentDateValue {
  /** `YYYY-MM-DD`, or null when the precision is the override. */
  date: string | null;
  /** The chosen precision token, or "" when nothing has been chosen yet. */
  precision: string;
}

interface Props {
  value: DocumentDateValue;
  onChange: (next: DocumentDateValue) => void;
  /** Disables both controls while the parent is submitting. */
  disabled?: boolean;
}

const labelStyle: React.CSSProperties = {
  fontSize: "0.76rem",
  fontWeight: 600,
  color: "var(--text-muted)",
  marginBottom: "0.25rem",
};

const controlStyle: React.CSSProperties = {
  width: "100%",
  padding: "0.45rem 0.6rem",
  borderRadius: "6px",
  border: "1px solid var(--border-default)",
  background: "var(--surface-default)",
  color: "var(--text-primary)",
  fontSize: "0.84rem",
  marginBottom: "0.75rem",
};

const noteStyle: React.CSSProperties = {
  fontSize: "0.72rem",
  color: "var(--text-muted)",
  marginTop: "-0.5rem",
  marginBottom: "0.75rem",
};

const errorStyle: React.CSSProperties = {
  padding: "0.5rem 0.75rem",
  backgroundColor: "var(--state-danger-bg-soft)",
  border: "1px solid var(--state-danger-border)",
  borderRadius: "6px",
  color: "var(--status-dropped-text)",
  fontSize: "0.76rem",
  marginBottom: "0.75rem",
};

/**
 * Whether a value is complete enough to submit.
 *
 * Exported because the PARENT owns its submit button, and both screens need the
 * identical answer. A pure function of the value, so it is testable without
 * rendering anything — which matters here, because this repo has no component
 * test infrastructure and a helper is the only part that can be covered.
 */
export function isDocumentDateComplete(
  value: DocumentDateValue,
  precisions: DatePrecisionOption[],
): boolean {
  const chosen = precisions.find((p) => p.value === value.precision);
  if (!chosen) return false;
  if (!chosen.requires_date) return true;
  return !!value.date && value.date.length > 0;
}

/**
 * The value to send for a given precision.
 *
 * Clearing the date when the override is chosen is what stops a stale date being
 * submitted alongside "this document has no date" — a contradiction the backend
 * refuses, and one the user would have no idea they had created.
 */
export function valueForPrecision(
  current: DocumentDateValue,
  precision: string,
  precisions: DatePrecisionOption[],
): DocumentDateValue {
  const chosen = precisions.find((p) => p.value === precision);
  const keepsDate = !!chosen?.requires_date;
  return { precision, date: keepsDate ? current.date : null };
}

const DocumentDateField: React.FC<Props> = ({ value, onChange, disabled }) => {
  const [precisions, setPrecisions] = useState<DatePrecisionOption[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    fetchDatePrecisions()
      .then((options) => {
        if (live) setPrecisions(options);
      })
      .catch((e: unknown) => {
        if (!live) return;
        setPrecisions([]);
        setError(
          e instanceof Error
            ? `Could not load the date options: ${e.message}`
            : "Could not load the date options",
        );
      });
    return () => {
      live = false;
    };
  }, []);

  const chosen = precisions.find((p) => p.value === value.precision);
  const needsDate = !!chosen?.requires_date;

  return (
    <>
      <div style={labelStyle}>Document date</div>
      <select
        style={controlStyle}
        value={value.precision}
        disabled={disabled || precisions.length === 0}
        onChange={(e) =>
          onChange(valueForPrecision(value, e.target.value, precisions))
        }
      >
        <option value="" disabled>
          How is this document dated?
        </option>
        {precisions.map((p) => (
          <option key={p.value} value={p.value}>
            {p.label}
          </option>
        ))}
      </select>

      {needsDate && (
        <>
          <input
            type="date"
            style={controlStyle}
            value={value.date ?? ""}
            disabled={disabled}
            onChange={(e) =>
              onChange({ ...value, date: e.target.value || null })
            }
          />
          {value.precision !== "day" && (
            <div style={noteStyle}>
              Only the part the document actually states is kept. Pick any day in
              the right {value.precision === "month" ? "month" : "year"}.
            </div>
          )}
        </>
      )}

      {error && <div style={errorStyle}>{error}</div>}
    </>
  );
};

export default DocumentDateField;
