// =============================================================================
// FactCardBody.tsx — the card Marie reads (FACT_CARD_v2 §2)
// =============================================================================
//
// The title bar, the source line, and the four authored rows. Split from
// `FactRow`, which keeps what may be DONE to a fact — drag, remove — while this
// keeps what the fact SAYS.
//
// ## What §2 changed, and why
//
// The weight control is gone from the title bar and a POSITION NUMBER stands
// where it was: a witness reading a deck needs to know which card she is on, and
// a three-way weight picker is a curator's control on a page a witness reads.
// The "extracted" chip is gone from the source line for the same reason — it says
// something about provenance that means nothing to her, on the line whose job is
// to say WHEN, WHO and WHERE.
//
// ## Nothing is hidden for lacking a field
//
// Every one of the four rows renders, always, with the stored em dash when it is
// empty. §2 says so and it is the one place this instruction overrules the mockup
// of record: a card with no Answer is work somebody still owes, and hiding it
// hides the work rather than the gap.
//
// ## Every visible word is served
//
// There is not one user-facing literal in this file. The row labels, the draft
// mark, the em dash and the three editing controls all come from
// `FactCardWording`; the layout constants are CSS tokens.

import React, { useState } from "react";

import type { ScenarioCard } from "../services/scenarioCards";
import type { FactCardWording } from "../services/evidenceLinks";
import { saveCardField, type CardFieldUpdate } from "../services/factCards";
import { cardRows, cardTitle, rowText, type CardFieldName } from "./factCard";
// `pdfHref` moved out of `ElementAllegationList` into `evidenceLocator` on
// PROOF_MATRIX_v2, which FACT_CARD_v2 was branched beside rather than after.
// Both merges applied cleanly and the result did not compile — the integration's
// one semantic conflict, caught by `tsc` and not by git.
import { pdfHref } from "./evidenceLocator";
import {
  BUTTON_STYLE,
  CARD_STYLE,
  CODE_STYLE,
  COLLAPSED_TITLE_STYLE,
  COUNT_TAG_STYLE,
  DRAFT_STYLE,
  ERROR_STYLE,
  FIELD_STYLE,
  INLINE_EDIT_STYLE,
  KIND_CHIP_STYLE,
  LINK_STYLE,
  POSITION_STYLE,
  QUOTE_STYLE,
  ROWS_STYLE,
  ROW_LABEL_STYLE,
  ROW_VALUE_STYLE,
  SOURCE_STYLE,
  TITLE_BAR_STYLE,
  TITLE_STYLE,
} from "./factCardStyles";

export interface FactCardBodyProps {
  card: ScenarioCard;
  wording: FactCardWording;
  slug: string;
  scenarioId: string;
  /** The number in the title bar — the human's order (`display_ordinal`). */
  position: number | null;
  /** Collapsed cards show the title bar and source line only (§2). */
  collapsed: boolean;
  /** Opens or closes this card. */
  onToggle: () => void;
  /** Re-reads the deck after a successful edit, so the screen matches the store. */
  onEdited: () => void;
}

/**
 * The title bar: position, title, count tag(s).
 *
 * The count tags come from the backend already composed — "Count 1 — Breach of
 * Fiduciary Duty" — because joining a number to a title is a presentation
 * decision about case vocabulary, which this payload exists to keep server-side.
 */
const TitleBar: React.FC<{
  card: ScenarioCard;
  wording: FactCardWording;
  position: number | null;
  collapsed: boolean;
  onToggle: () => void;
}> = ({ card, wording, position, collapsed, onToggle }) => {
  const title = cardTitle(card, wording);
  return (
    <div
      style={TITLE_BAR_STYLE}
      role="button"
      tabIndex={0}
      aria-expanded={!collapsed}
      onClick={onToggle}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onToggle();
        }
      }}
    >
      {/* The position REPLACES the weight control (§2). A card with no stored
          position shows none rather than a zero — "not placed" and "first" are
          different states. */}
      <span style={POSITION_STYLE}>{position ?? ""}</span>
      <span style={collapsed ? COLLAPSED_TITLE_STYLE : TITLE_STYLE}>
        {title.text}
        {title.draft && <DraftMark wording={wording} />}
      </span>
      {card.card?.count_tags.map((tag) => (
        <span key={tag} style={COUNT_TAG_STYLE}>
          {tag}
        </span>
      ))}
    </div>
  );
};

/**
 * The source line: date · speaker · kind · document (linked, with page) · C-code.
 *
 * ## Why the date is formatted HERE
 *
 * It is the one value on this card the backend does not compose. A date is
 * rendered in the READER's locale, which the server does not know — the same
 * division of labour the scan-history line already makes. Everything else on this
 * line arrives finished.
 */
const SourceLine: React.FC<{ card: ScenarioCard }> = ({ card }) => (
  <div style={SOURCE_STYLE}>
    {card.source_date && <span>{formatDate(card.source_date)}</span>}
    {/* Documentary evidence genuinely has no speaker; the line then leads with
        the document, rather than the card inventing "Unknown". */}
    {card.speaker.name && <span>{card.speaker.name}</span>}
    {card.statement_kind && <span style={KIND_CHIP_STYLE}>{card.statement_kind}</span>}
    <a
      href={pdfHref(card.pinpoint.document_id, card.pinpoint.page)}
      target="_blank"
      rel="noopener noreferrer"
      style={LINK_STYLE}
      onClick={(e) => e.stopPropagation()}
    >
      {card.pinpoint.label}
    </a>
    {card.code && <span style={CODE_STYLE}>{card.code}</span>}
  </div>
);

/**
 * A date in the reader's locale, or the raw value when it is not a full date.
 *
 * The graph holds `YYYY-MM` on some statements. `new Date("2009-04")` parses as
 * the first of that month and would print "1 April 2009" — a day nobody
 * recorded. So only a full `YYYY-MM-DD` is formatted; anything else prints as
 * stored, which is visibly partial rather than falsely precise.
 */
export function formatDate(value: string): string {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return value;
  const parsed = new Date(`${value}T00:00:00`);
  if (Number.isNaN(parsed.getTime())) return value;
  return parsed.toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

/** The grey mark on a field the machine wrote and no human has edited. */
const DraftMark: React.FC<{ wording: FactCardWording }> = ({ wording }) => (
  <span style={DRAFT_STYLE}>{wording.draft_mark}</span>
);

/**
 * One authored row, with its inline editor.
 *
 * The editor follows the `SentenceEditor` pattern — click to open, seeded from
 * what is STORED every time, Save and Cancel — rather than reusing that component
 * itself: this row edits four different shapes (three sentences, a position and
 * an accusation list) and carries a draft mark, and bending one component to both
 * would fork the behaviour the next time either changed.
 */
const CardRowView: React.FC<{
  field: CardFieldName;
  label: string;
  lines: string[];
  draft: boolean;
  wording: FactCardWording;
  onSave: (field: CardFieldName, raw: string) => void;
  busy: boolean;
  editable: boolean;
}> = ({ field, label, lines, draft, wording, onSave, busy, editable }) => {
  const [editing, setEditing] = useState(false);
  const [value, setValue] = useState("");

  const open = () => {
    // Seeded from what is STORED every time, never kept from a previous edit: an
    // editor holding a ten-minute-old draft lets a human overwrite a change made
    // since without ever seeing it (the `SentenceEditor` rule).
    setValue(lines.join("\n"));
    setEditing(true);
  };

  return (
    <>
      <div style={ROW_LABEL_STYLE}>{label}</div>
      <div style={ROW_VALUE_STYLE}>
        {editing ? (
          <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
            <textarea
              style={FIELD_STYLE}
              value={value}
              onChange={(e) => setValue(e.target.value)}
              aria-label={label}
            />
            <div style={{ display: "flex", gap: "6px" }}>
              <button
                type="button"
                style={BUTTON_STYLE}
                disabled={busy}
                onClick={() => {
                  onSave(field, value);
                  setEditing(false);
                }}
              >
                {wording.save_label}
              </button>
              <button type="button" style={BUTTON_STYLE} onClick={() => setEditing(false)}>
                {wording.cancel_label}
              </button>
            </div>
          </div>
        ) : (
          <>
            {lines.map((line, index) => (
              <div key={index}>{line}</div>
            ))}
            {draft && <DraftMark wording={wording} />}
            {editable && (
              <button type="button" style={INLINE_EDIT_STYLE} onClick={open}>
                {wording.edit_label}
              </button>
            )}
          </>
        )}
      </div>
    </>
  );
};

/** One fact card. */
const FactCardBody: React.FC<FactCardBodyProps> = ({
  card,
  wording,
  slug,
  scenarioId,
  position,
  collapsed,
  onToggle,
  onEdited,
}) => {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const save = async (field: CardFieldName, raw: string) => {
    const update = buildUpdate(field, raw);
    if (!update) {
      // The accusation list is not editable as free text — it is a picker, and
      // §2's Include prefill reads it. Offering a text box would let a human type
      // an id nothing matches.
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await saveCardField(slug, scenarioId, card.graph_node_id, update);
      onEdited();
    } catch (err: unknown) {
      const detail = err instanceof Error ? err.message : "unknown error";
      setError(wording.save_failed_template.replace("{detail}", detail));
    } finally {
      setBusy(false);
    }
  };

  const block = card.card;
  return (
    <div style={CARD_STYLE}>
      <TitleBar
        card={card}
        wording={wording}
        position={position}
        collapsed={collapsed}
        onToggle={onToggle}
      />
      <SourceLine card={card} />
      {!collapsed && (
        <div style={ROWS_STYLE}>
          {/* Proof is the record's own words: no human writes it, so it carries no
              draft mark and no editor. The RFA form, when the card is a Q&A, is
              composed by the backend and arrives in the quote. */}
          <div style={ROW_LABEL_STYLE}>{wording.proof_label}</div>
          <div style={ROW_VALUE_STYLE}>
            <span style={QUOTE_STYLE}>{card.quote.text}</span>
          </div>
          {block ? (
            cardRows(block, wording).map((row) => (
              <CardRowView
                key={row.field}
                field={row.field}
                label={row.label}
                lines={rowText(row, wording)}
                draft={row.draft}
                wording={wording}
                onSave={save}
                busy={busy}
                // The accusation list is a picker, not a text box — see `save`.
                editable={row.field !== "supports"}
              />
            ))
          ) : (
            // No card drafted at all. The rows are still shown, empty, because a
            // statement nobody has written about is exactly what a reader needs
            // to see on a deck they are preparing.
            <>
              {[wording.backs_label, wording.supports_label, wording.watch_out_label, wording.answer_label].map(
                (label) => (
                  <React.Fragment key={label}>
                    <div style={ROW_LABEL_STYLE}>{label}</div>
                    <div style={ROW_VALUE_STYLE}>{wording.empty_value}</div>
                  </React.Fragment>
                ),
              )}
            </>
          )}
        </div>
      )}
      {error && <div style={ERROR_STYLE}>{error}</div>}
    </div>
  );
};

/**
 * The typed update one edited row becomes.
 *
 * Returns `null` for `supports`, which is a picker rather than a text box: a free
 * text box there would let a human type an accusation id nothing matches, and the
 * Include prefill reads that list.
 *
 * A blank box clears the field — the same act as pressing a Clear control would
 * be, and the card renders both as the em dash.
 */
export function buildUpdate(field: CardFieldName, raw: string): CardFieldUpdate | null {
  const trimmed = raw.trim();
  switch (field) {
    case "title":
      return { field: "title", value: trimmed === "" ? null : trimmed };
    case "watch_out":
      return { field: "watch_out", value: trimmed === "" ? null : trimmed };
    case "answer":
      return { field: "answer", value: trimmed === "" ? null : trimmed };
    case "backs_position": {
      if (trimmed === "") return { field: "backs_position", value: null };
      const parsed = Number.parseInt(trimmed, 10);
      // A value that is not a whole number CLEARS rather than sending NaN: the
      // backend would refuse it, and refusing a typo by storing nothing is a
      // worse answer than the em dash the human can see and correct.
      return {
        field: "backs_position",
        value: Number.isFinite(parsed) ? parsed : null,
      };
    }
    case "supports":
      return null;
  }
}

export default FactCardBody;
