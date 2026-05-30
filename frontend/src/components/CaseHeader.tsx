// =============================================================================
// CaseHeader.tsx — the case caption block at the top of the Home page
// -----------------------------------------------------------------------------
// A compact 5-line caption (top to bottom):
//   1. Title          — serif H1 (display_title_full), the ONLY serif element
//   2. Metadata strip — court · Case No. · Filed … · Status (+ transfer note)
//   3. Column headers — PLAINTIFF | DEFENDANTS
//   4. Names row      — plaintiff(s) left; defendants right, active + dropped on
//                       ONE line, the dropped names muted with "(Dropped)"
//   5. Counsel        — one self-labeled line per counsel-of-record
//
// Typography is deliberately minimal: ONE serif (the H1 title) + ONE 14px sans
// size for everything else. Hierarchy comes from weight (the uppercase column
// headers are 600) and color tokens (--text-primary for names, --text-secondary
// for metadata/counsel, --text-muted for dropped parties), NOT from many sizes.
//
// PRESENTATIONAL: it receives the API payload via props and never fetches. The
// fetch + loading/error states live in Home.tsx. All colors come from tokens
// (var(--…)); no hardcoded color hex.
// =============================================================================

import React from "react";
import {
  CaseHeaderResponse,
  CounselContact,
  CourtInfo,
  DroppedDefendant,
  HeaderParty,
} from "../services/caseHeader";

// ─── Pure helpers (exported for unit testing — no DOM, no React) ─────────────

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
 * "— firm" suffix when firm_name is null/empty (rule: omit, don't print empty
 * parentheses or a dangling dash).
 */
export function formatCounselLine(c: CounselContact): string {
  const bar = c.bar_number && c.bar_number.trim() !== "" ? ` (${c.bar_number})` : "";
  const firm = c.firm_name && c.firm_name.trim() !== "" ? ` — ${c.firm_name}` : "";
  return `${c.represents_role}'s Counsel: ${c.attorney_name}${bar}${firm}`;
}

/**
 * Label for a dropped defendant on the shared defendants line: "{name} (Dropped)".
 * The "(Dropped)" marker is a fixed literal for every non-active defendant
 * regardless of the party's specific lifecycle word (dropped / dismissed /
 * settled) — the caller renders the whole label in the muted color token.
 */
export function formatDroppedDefendant(name: string): string {
  return `${name} (Dropped)`;
}

/**
 * Resolve the case title to render: prefer `display_title_full`, fall back to
 * `display_title`, then to a clear placeholder. Trims so a whitespace-only
 * value is treated as absent (Rule 1: never render a blank title).
 */
export function resolveTitle(data: CaseHeaderResponse): string {
  if (data.display_title_full && data.display_title_full.trim() !== "") {
    return data.display_title_full;
  }
  if (data.display_title && data.display_title.trim() !== "") {
    return data.display_title;
  }
  return "Case title unavailable";
}

// ─── Shared inline styles ────────────────────────────────────────────────────
// ONE sans size (14px) for the whole header below the H1. Names use the primary
// text color; metadata/counsel use the secondary color. No dedicated token class
// exists for these exact combinations, so the color tokens are referenced inline.

/** Party-name typography: 14px / 400 / primary. Dropped names override the color. */
const NAME_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "14px",
  fontWeight: 400,
  color: "var(--text-primary)",
  lineHeight: 1.5,
};

/** Metadata-strip and counsel typography: 14px / 400 / secondary. */
const META_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "14px",
  fontWeight: 400,
  color: "var(--text-secondary)",
  lineHeight: 1.5,
};

// ─── Sub-components ──────────────────────────────────────────────────────────

/** A single labeled party column (header + one name per line). */
const PartyColumn: React.FC<{ label: string; names: string[] }> = ({
  label,
  names,
}) => (
  <div style={{ flex: 1, minWidth: 0 }}>
    <div className="h2-section-header" style={{ marginBottom: "4px" }}>
      {label}
    </div>
    {names.map((name, i) => (
      <div key={i} style={NAME_STYLE}>
        {name}
      </div>
    ))}
  </div>
);

/**
 * The defendants column: a DEFENDANTS header over a SINGLE names line — active
 * defendants comma-joined, then each dropped defendant (after a " · ") shown
 * muted with the "(Dropped)" marker. Collapses what used to be a separate
 * DROPPED sub-block into one line.
 */
const DefendantsColumn: React.FC<{
  active: HeaderParty[];
  dropped: DroppedDefendant[];
}> = ({ active, dropped }) => (
  <div style={{ flex: 1, minWidth: 0 }}>
    <div className="h2-section-header" style={{ marginBottom: "4px" }}>
      {pluralizePartyLabel("DEFENDANT", active.length)}
    </div>
    <div style={NAME_STYLE}>
      {active.map((d) => d.name).join(", ")}
      {dropped.map((d, i) => (
        <React.Fragment key={d.party_id}>
          {/* Separator before each dropped name when anything precedes it. */}
          {active.length > 0 || i > 0 ? " · " : ""}
          <span style={{ color: "var(--text-muted)" }}>
            {formatDroppedDefendant(d.name)}
          </span>
        </React.Fragment>
      ))}
    </div>
  </div>
);

/**
 * The metadata strip: "court · Case No. {n|[pending]} · Filed {date} · Status:
 * {label}" with an optional "(transferred from …)" tail. Built as discrete
 * nodes so the [pending] and active-status values can carry their own token
 * colors, then interleaved with " · ".
 */
const MetadataStrip: React.FC<{ court: CourtInfo; status: string }> = ({
  court,
  status,
}) => {
  const isActive = status.toLowerCase() === "active";
  const statusLabel = status.charAt(0).toUpperCase() + status.slice(1);
  const filed = formatFiledDate(court.filed_date);

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

  return (
    <div style={{ ...META_STYLE, marginTop: "8px" }}>
      {segments.map((seg, i) => (
        <React.Fragment key={i}>
          {i > 0 ? " · " : ""}
          {seg}
        </React.Fragment>
      ))}
      {court.transferred_from && <> (transferred from {court.transferred_from})</>}
    </div>
  );
};

// ─── Component ───────────────────────────────────────────────────────────────

/**
 * CaseHeader — the read-only, compact case caption for the Home page.
 *
 * @param data the payload from GET /api/cases/:slug (see services/caseHeader.ts)
 */
const CaseHeader: React.FC<{ data: CaseHeaderResponse }> = ({ data }) => {
  const { court, parties, counsel } = data;

  return (
    <header>
      {/* 1. Title — full caption, the only serif element */}
      <h1 className="h1-case-title">{resolveTitle(data)}</h1>

      {/* 2. Metadata strip */}
      <MetadataStrip court={court} status={data.status} />

      {/* 3 + 4. Column headers + names: plaintiffs left, defendants right */}
      <div style={{ display: "flex", gap: "32px", marginTop: "12px" }}>
        <PartyColumn
          label={pluralizePartyLabel("PLAINTIFF", parties.plaintiffs.length)}
          names={parties.plaintiffs.map((p) => p.name)}
        />
        <DefendantsColumn
          active={parties.active_defendants}
          dropped={parties.dropped_defendants}
        />
      </div>

      {/* 5. Counsel — one self-labeled line per record, directly beneath names */}
      {counsel.length > 0 && (
        <div style={{ marginTop: "8px" }}>
          {counsel.map((c) => (
            <div key={c.counsel_id} style={META_STYLE}>
              {formatCounselLine(c)}
            </div>
          ))}
        </div>
      )}
    </header>
  );
};

export default CaseHeader;
