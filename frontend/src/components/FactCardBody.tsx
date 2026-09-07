// =============================================================================
// FactCardBody.tsx — the card Marie reads (FACT_CARD_v2 §2)
// =============================================================================
//
// The title bar, the source line, the proof, and the one authored row left.
// Split from `FactRow`, which keeps what may be DONE to a fact — drag, remove —
// while this keeps what the fact SAYS.
//
// ## ⚑ v2.1 (ruling R37): EVIDENCE ONLY
//
// Three rows and a number left this card, and the reason is the same for all
// four: this page is Roman's and Chuck's working file, not a witness's deck.
//
//   BACKS      was a sentence the machine composed ("Point 2 — …") with a free
//              text editor under it. The thing a human actually sets from here
//              is WHICH POINT, so it is a picker now (`BacksPicker`) and the
//              composed sentence is gone. A text box let a human type a position
//              no talking point occupies, which the payload then rendered as an
//              em dash with no explanation.
//   WATCH OUT  the machine's prose about the evidence.
//   ANSWER     the machine's suggested words in Marie's mouth.
//   the ordinal  a raw `sort_ordinal` — `20480` — printed in the title bar. It
//              was `display_ordinal` when §2 put it there and a witness needed
//              "which card am I on"; on a list nobody reads in sequence it is a
//              five-digit number with no meaning to anybody.
//
// Nothing was DELETED: `answer`, `watch_out` and `backs_position` are still
// columns, still on the wire, still writable through `saveCardField`. See
// `factCard.cardRows`.
//
// ## Proof is a Q&A pair when the record has one
//
// The comment that stood here claimed "the RFA form, when the card is a Q&A, is
// composed by the backend and arrives in the quote". It does not — measured on
// DEV. `card.quote.question` is its own field and `EvidenceCardBody` has rendered
// it as a folded `Q:` line since ONE_CARD_GRAMMAR. This card renders the same two
// lines, folded by the same served threshold, so one piece of evidence reads the
// same under every wrapper.
//
// ## Every visible word is served
//
// There is still not one user-facing literal in this file. The row labels, the
// draft mark, the em dash, the question's show/hide pair and the editing controls
// all come from the stored wording; the layout constants are CSS tokens.

import React, { useState } from "react";

import type { ScenarioCard } from "../services/scenarioCards";
import type { CardGrammarWording, FactCardWording } from "../services/evidenceLinks";
import { saveCardField, type CardFieldUpdate } from "../services/factCards";
import { cardRows, cardTitle, rowText, type CardFieldName } from "./factCard";
// The two rows this card renders live next door — see that module's header for
// why the split fell where it did.
import { BacksPicker, ProofRows } from "./FactCardRows";
import type { TalkingPointDto } from "../services/scenarioAugmentation";
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
  /** The shared card grammar — the question fold's show/hide words (v2.1). */
  grammar: CardGrammarWording;
  /** `card_question_truncate_chars`, served. Where the `Q:` line folds. */
  questionChars: number;
  slug: string;
  scenarioId: string;
  /** Collapsed cards show the title bar and source line only (§2). */
  collapsed: boolean;
  /** Opens or closes this card. */
  onToggle: () => void;
  /** Re-reads the deck after a successful edit, so the screen matches the store. */
  onEdited: () => void;
  /**
   * The scenario's live talking points, which turn the Backs picker on (v2.1).
   *
   * ## Why this is optional, and what its absence MEANS
   *
   * Absent ⇒ no picker. That is not a fallback, it is the second surface this
   * card renders on: under a talking point, in `TalkingPointsSection`, where the
   * card is already there BECAUSE it backs that point. A control offering to move
   * it out from under the heading it is filed beneath is a control that would be
   * clicked by accident, and the facts list twenty lines down is where that
   * choice belongs.
   *
   * Passed as the whole points list rather than a count so the options can carry
   * each point's own sentence in their tooltip — the position number alone says
   * nothing about which argument it is.
   */
  points?: TalkingPointDto[];
}

/**
 * The title bar: title, count tag(s).
 *
 * The count tags come from the backend already composed — "Count 1 — Breach of
 * Fiduciary Duty" — because joining a number to a title is a presentation
 * decision about case vocabulary, which this payload exists to keep server-side.
 */
const TitleBar: React.FC<{
  card: ScenarioCard;
  wording: FactCardWording;
  collapsed: boolean;
  onToggle: () => void;
}> = ({ card, wording, collapsed, onToggle }) => {
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
      {/* The ordinal that stood here is GONE (v2.1). It was `display_ordinal`,
          which is a sparse sort key — the live values on S-7 run to five digits
          — and §2 put it here for a witness working through a deck in sequence.
          Nobody reads this list in sequence, so it printed `20480` beside every
          title and meant nothing to anyone. The order it encodes is still the
          order the list renders in; it simply is not narrated any more. */}
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
  grammar,
  questionChars,
  slug,
  scenarioId,
  collapsed,
  onToggle,
  onEdited,
  points,
}) => {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  /**
   * Store one field, then re-read the deck.
   *
   * ## Rust Learning: `async fn` and the `finally` that has no `?`
   *
   * TypeScript's `try / catch / finally` is doing what a Rust caller would write
   * as a `match` on the `Result` plus a guard: `busy` MUST be cleared on both
   * arms, and putting the clear on the success path alone is how a control ends
   * up permanently disabled after one refused write. Rust would push you toward a
   * drop guard; here `finally` is the equivalent, and it is not optional.
   */
  const save = async (update: CardFieldUpdate) => {
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

  /** The text-editor path: one row's raw string becomes one typed update. */
  const saveField = (field: CardFieldName, raw: string) => {
    const update = buildUpdate(field, raw);
    if (!update) {
      // The accusation list is not editable as free text — it is a picker, and
      // §2's Include prefill reads it. Offering a text box would let a human type
      // an id nothing matches.
      return;
    }
    void save(update);
  };

  const block = card.card;
  // The picker is drawn when the caller supplied points AND there is something to
  // choose between — or when this card already points at one, which must stay
  // clearable even on a scenario whose points have since been deleted. A stale
  // pointer with no way to clear it is the defect the picker replaced, wearing a
  // different hat.
  const showsBacks =
    points !== undefined && (points.length > 0 || block?.backs_position != null);

  return (
    <div style={CARD_STYLE}>
      <TitleBar
        card={card}
        wording={wording}
        collapsed={collapsed}
        onToggle={onToggle}
      />
      <SourceLine card={card} />
      {!collapsed && (
        <div style={ROWS_STYLE}>
          {/* Proof is the record's own words: no human writes it, so it carries
              no draft mark and no editor. It is TWO lines when the record has a
              question — see `ProofRows`. */}
          <ProofRows
            card={card}
            wording={wording}
            grammar={grammar}
            questionChars={questionChars}
          />

          {/* SUPPORTS, and nothing else the machine drafted (v2.1, ruling R37).
              `block` is `null` on a statement nobody has drafted a card for; the
              row is still rendered, empty, because "no card yet" is work
              somebody owes and hiding it hides the work. That reasoning stood
              for four rows and still stands for the one that is left. */}
          {block ? (
            cardRows(block, wording).map((row) => (
              <CardRowView
                key={row.field}
                field={row.field}
                label={row.label}
                lines={rowText(row, wording)}
                draft={row.draft}
                wording={wording}
                onSave={saveField}
                busy={busy}
                // The accusation list is a picker, not a text box — see `save`.
                editable={row.field !== "supports"}
              />
            ))
          ) : (
            <>
              <div style={ROW_LABEL_STYLE}>{wording.supports_label}</div>
              <div style={ROW_VALUE_STYLE}>{wording.empty_value}</div>
            </>
          )}

          {/* The one judgment a human makes from this card. Placed LAST, under
              the evidence it is a judgment about — the removed BACKS row led the
              rows, which put an editable field above the quote it referred to. */}
          {showsBacks && (
            <BacksPicker
              wording={wording}
              points={points}
              current={block?.backs_position ?? null}
              busy={busy}
              onPick={(value) => void save({ field: "backs_position", value })}
            />
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
