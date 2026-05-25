// =============================================================================
// CaseHeader.tsx — the case caption block at the top of the Home page
// -----------------------------------------------------------------------------
// Renders (top to bottom, per HOME_PAGE_REDESIGN_v2.md §6):
//   1. Title          — serif H1 with an italicized "v." (legal convention)
//   2. Metadata strip — court · Case No. · Filed … · Status (+ transfer note)
//   3. Parties block  — plaintiffs (left) / defendants (right, + DROPPED)
//   4. Counsel block  — one self-labeled line per counsel-of-record
//
// This is a PRESENTATIONAL component: it receives the API payload via props and
// never fetches. The fetch + loading/error states live in Home.tsx (Phase 2C
// instruction rule 1). All colors come from Phase 2A tokens (var(--…)); all
// typography uses the token utility classes from styles/tokens.css.
// =============================================================================

import React from "react";
import {
  CaseHeaderResponse,
  CounselContact,
} from "../services/caseHeader";

// ─── Pure helpers (exported for unit testing — no DOM, no React) ─────────────

/**
 * Split a case title on the legal " v. " separator so the caller can italicize
 * the "v." per convention. Returns `null` when there is no " v. " — the caller
 * then renders the title verbatim (defensive: not every title is adversarial,
 * e.g. "In re Estate of …").
 *
 * Splits on the FIRST occurrence only: "A v. B v. C" → left "A", right "B v. C".
 */
export function splitOnVersus(
  title: string,
): { left: string; right: string } | null {
  const sep = " v. ";
  const i = title.indexOf(sep);
  if (i === -1) return null;
  return {
    left: title.slice(0, i),
    right: title.slice(i + sep.length),
  };
}

/**
 * Format an ISO "YYYY-MM-DD" date as a long US date ("November 1, 2013").
 * Returns `null` for null/empty input so the caller can omit the "Filed …"
 * segment rather than print "Filed null".
 *
 * ## React/TS Learning: why parse to a UTC date explicitly
 * `new Date("2013-11-01")` parses as UTC midnight, but `Intl.DateTimeFormat`
 * formats in the *local* zone — west of UTC that midnight rolls back to
 * "October 31". We build the date from its parts and format with
 * `timeZone: "UTC"` so the displayed day always matches the stored day,
 * regardless of where the browser is.
 */
export function formatFiledDate(iso: string | null | undefined): string | null {
  if (!iso) return null;
  const parts = iso.split("-");
  if (parts.length !== 3) return null;
  const [year, month, day] = parts.map(Number);
  if (!year || !month || !day) return null;
  const date = new Date(Date.UTC(year, month - 1, day));
  return new Intl.DateTimeFormat("en-US", {
    month: "long",
    day: "numeric",
    year: "numeric",
    timeZone: "UTC",
  }).format(date);
}

/**
 * Pluralize an already-uppercase party label by count. 1 → singular;
 * everything else (including 0) → singular + "S". English treats zero as plural
 * ("0 defendants"), so 0 pluralizes too.
 */
export function pluralizePartyLabel(singular: string, count: number): string {
  return count === 1 ? singular : `${singular}S`;
}

/**
 * Whether the case number should render as "[pending]". True when null,
 * undefined, empty, or whitespace-only. The backend already collapses a NULL or
 * empty docket number to `null`, but we also guard against a whitespace string
 * so the indicator never shows a blank.
 */
export function isCaseNumberPending(value: string | null | undefined): boolean {
  return value == null || value.trim() === "";
}

/**
 * Build one counsel line: "{role}'s Counsel: {attorney} ({bar}) — {firm}".
 * Omits the "(bar)" parenthetical when bar_number is null/empty and the
 * "— firm" suffix when firm_name is null/empty (Phase 2C rule: omit, don't
 * print empty parentheses or a dangling dash).
 */
export function formatCounselLine(c: CounselContact): string {
  const bar = c.bar_number && c.bar_number.trim() !== "" ? ` (${c.bar_number})` : "";
  const firm = c.firm_name && c.firm_name.trim() !== "" ? ` — ${c.firm_name}` : "";
  return `${c.represents_role}'s Counsel: ${c.attorney_name}${bar}${firm}`;
}

// ─── Shared inline styles ────────────────────────────────────────────────────
// Party-name typography (§6: 14px, 400, --text-primary). No dedicated token
// class exists for it, so we reference the color token inline. Dropped names
// override only the color.

const NAME_STYLE: React.CSSProperties = {
  fontSize: "14px",
  fontWeight: 400,
  color: "var(--text-primary)",
  lineHeight: 1.5,
};

// ─── Sub-components ──────────────────────────────────────────────────────────

/** A single labeled party column (header + one name per line). */
const PartyColumn: React.FC<{ label: string; names: string[] }> = ({
  label,
  names,
}) => (
  <div style={{ flex: 1, minWidth: 0 }}>
    <div className="h2-section-header" style={{ marginBottom: "8px" }}>
      {label}
    </div>
    {names.map((name, i) => (
      <div key={i} style={NAME_STYLE}>
        {name}
      </div>
    ))}
  </div>
);

// ─── Component ───────────────────────────────────────────────────────────────

/**
 * CaseHeader — the read-only case caption for the Home page.
 *
 * @param data the payload from GET /api/cases/:slug (see services/caseHeader.ts)
 */
const CaseHeader: React.FC<{ data: CaseHeaderResponse }> = ({ data }) => {
  const { court, parties, counsel } = data;
  const isActive = data.status.toLowerCase() === "active";
  const statusLabel =
    data.status.charAt(0).toUpperCase() + data.status.slice(1);
  const filed = formatFiledDate(court.filed_date);

  // Build the metadata strip as discrete nodes so the [pending] and status
  // values can carry their own token colors, then interleave " · " between them.
  const segments: React.ReactNode[] = [];
  if (court.name) segments.push(<span key="court">{court.name}</span>);
  segments.push(
    <span key="caseno">
      Case No.{" "}
      {isCaseNumberPending(court.case_number) ? (
        <span style={{ color: "var(--status-pending-text)" }}>[pending]</span>
      ) : (
        court.case_number
      )}
    </span>,
  );
  if (filed) segments.push(<span key="filed">Filed {filed}</span>);
  segments.push(
    <span key="status">
      Status:{" "}
      <span style={{ color: isActive ? "var(--status-active-text)" : undefined }}>
        {statusLabel}
      </span>
    </span>,
  );

  // Title: split on " v. " to italicize the "v."; fall back to verbatim (or a
  // clear placeholder if the title is somehow missing — Rule 1, no blank render).
  const titleParts = data.display_title ? splitOnVersus(data.display_title) : null;

  return (
    // No bottom margin: the Causes of Action placeholder below supplies the
    // inter-section gap via its own top padding (it's out of scope to edit).
    <header>
      {/* 1. Title */}
      {!data.display_title ? (
        <h1 className="h1-case-title">Case title unavailable</h1>
      ) : titleParts ? (
        <h1 className="h1-case-title">
          {titleParts.left} <em>v.</em> {titleParts.right}
        </h1>
      ) : (
        <h1 className="h1-case-title">{data.display_title}</h1>
      )}

      {/* 2. Court / metadata strip (24px below the title) */}
      <div className="metadata" style={{ marginTop: "24px" }}>
        {segments.map((seg, i) => (
          <React.Fragment key={i}>
            {i > 0 ? " · " : ""}
            {seg}
          </React.Fragment>
        ))}
        {court.transferred_from && <> (transferred from {court.transferred_from})</>}
      </div>

      {/* 3. Parties block (16px below the strip): plaintiffs left, defendants right */}
      <div style={{ display: "flex", gap: "32px", marginTop: "16px" }}>
        <PartyColumn
          label={pluralizePartyLabel("PLAINTIFF", parties.plaintiffs.length)}
          names={parties.plaintiffs.map((p) => p.name)}
        />
        <div style={{ flex: 1, minWidth: 0 }}>
          <div className="h2-section-header" style={{ marginBottom: "8px" }}>
            {pluralizePartyLabel("DEFENDANT", parties.active_defendants.length)}
          </div>
          {parties.active_defendants.map((d) => (
            <div key={d.party_id} style={NAME_STYLE}>
              {d.name}
            </div>
          ))}
          {parties.dropped_defendants.length > 0 && (
            <>
              <div
                style={{
                  borderTop: "1px solid var(--border-default)",
                  margin: "12px 0 8px",
                }}
              />
              <div className="h2-section-header" style={{ marginBottom: "8px" }}>
                DROPPED
              </div>
              {parties.dropped_defendants.map((d) => (
                <div
                  key={d.party_id}
                  style={{ ...NAME_STYLE, color: "var(--status-dropped-text)" }}
                >
                  {d.name}
                </div>
              ))}
            </>
          )}
        </div>
      </div>

      {/* 4. Counsel block (16px below parties): one self-labeled line per record */}
      {counsel.length > 0 && (
        <div style={{ marginTop: "16px" }}>
          {counsel.map((c) => (
            <div key={c.counsel_id} className="metadata" style={{ lineHeight: 1.6 }}>
              {formatCounselLine(c)}
            </div>
          ))}
        </div>
      )}
    </header>
  );
};

export default CaseHeader;
