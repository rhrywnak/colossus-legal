// =============================================================================
// TimelineEventCard.tsx — one dated fact, as the list draws it
// =============================================================================
//
// Mockup v2 Screen 1's `.ev`: date · dot · title · tag chip(s) · fact in serif,
// then the row of links, the note badge, and — since Phase C — the ✎ and 🗑
// controls. Every decision it renders — the date's shape, the dot's colour,
// which of three states a link is in, whether there is a badge at all — is made
// by a pure function in `timelineFilters`, where a test can reach it.
//
// ## ⚑ THE CONTROLS ARE ALWAYS VISIBLE, AND MUTED (design R17)
//
// Ruled 2026-08-25: "Hover-only controls are a named anti-pattern and
// CaseFleet's rows carry a visible pencil. Small, gray, never competing with
// content." So they are drawn on every card at all times, in the meta row where
// the links and the note badge already live, and they stop the click from
// reaching the card underneath.
//
// There is NO confirm dialog behind 🗑, by ruling R10. The delete happens and
// the card is replaced in place by the undo line, which the PAGE draws — see
// `TimelinePage`. That is the safety.

import React from "react";

import { API_BASE_URL } from "../../services/api";
import type {
  ChronologyWording,
  TimelineEvent,
  TimelineTag,
} from "../../services/caseTimeline";
import { dotColor, formatEventDate, linkRendering, noteBadge, tagOf } from "./timelineFilters";
import * as s from "./timelineStyles";
import * as w from "./timelineWriteStyles";

type Props = {
  event: TimelineEvent;
  tags: TimelineTag[];
  wording: ChronologyWording;
  /** Open this event's page. */
  onOpen: (id: string) => void;
  /**
   * Open the edit form for this event, pre-filled.
   *
   * Optional so the card can be drawn on a surface with no writes behind it.
   * Absent means the control is not DRAWN — never drawn-and-inert, which is the
   * dead button the standing rules forbid.
   */
  onEdit?: (event: TimelineEvent) => void;
  /** Delete this event. No confirm dialog follows (R10). */
  onDelete?: (event: TimelineEvent) => void;
};

/** The colour an event's dot falls back to when no tag of its is known. */
const NEUTRAL_DOT = "var(--text-disabled)";

const TimelineEventCard: React.FC<Props> = ({
  event,
  tags,
  wording,
  onOpen,
  onEdit,
  onDelete,
}) => {
  const badge = noteBadge(event.note_count, wording);
  const hasControls = onEdit !== undefined || onDelete !== undefined;

  return (
    <div
      // A div with a role rather than a <button>: the card CONTAINS links (a
      // document opens in its own tab), and an interactive element inside a
      // button is invalid and unreachable by keyboard. Enter and Space are
      // wired below so the card is still operable without a mouse.
      role="button"
      tabIndex={0}
      style={s.card}
      onClick={() => onOpen(event.id)}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onOpen(event.id);
        }
      }}
    >
      <div style={s.cardDate}>
        {formatEventDate(event.event_date, event.approximate, event.date_precision)}
      </div>
      <div style={s.dot(dotColor(tags, event, NEUTRAL_DOT))} />
      <div style={{ minWidth: 0 }}>
        <h3 style={s.cardTitle}>{event.title}</h3>
        {event.tags.map((id) => {
          const tag = tagOf(tags, id);
          // An unknown tag still renders — as its raw token, in a neutral chip.
          // Hiding it would make a vocabulary drift invisible on the one screen
          // where somebody could notice it.
          return (
            <span key={id} style={s.tagChip(tag?.color ?? NEUTRAL_DOT)}>
              {tag?.label ?? id}
            </span>
          );
        })}
        {event.fact && <p style={s.fact}>{event.fact}</p>}

        {(event.links.length > 0 || badge || hasControls) && (
          <div style={s.rowMeta}>
            {event.links.map((link) => {
              const rendered = linkRendering(link, wording);
              if (rendered.kind === "link") {
                return (
                  <a
                    key={`${link.target_type}:${link.target_id}`}
                    href={`${API_BASE_URL}/api/documents/${encodeURIComponent(link.target_id)}/file`}
                    target="_blank"
                    rel="noopener noreferrer"
                    style={s.docLink}
                    // The card opens the event page; this opens a document.
                    // Without this the click would do both.
                    onClick={(e) => e.stopPropagation()}
                  >
                    {rendered.label}{" "}
                    <span style={s.pinpoint}>
                      {rendered.pinpoint ?? cwSafe(wording, "no_pinpoint_label")}
                    </span>
                  </a>
                );
              }
              return (
                <span
                  key={`${link.target_type}:${link.target_id}`}
                  style={rendered.kind === "missing" ? s.noDoc : s.unchecked}
                  title={link.target_id}
                >
                  {rendered.label}
                </span>
              );
            })}
            {badge && <span style={s.noteCount}>{badge}</span>}
            {hasControls && (
              <div style={w.cardActions}>
                {onEdit && (
                  <button
                    type="button"
                    style={w.cardAction}
                    // The card opens the event page; this opens the form. Without
                    // stopping propagation the click would do both, and the form
                    // would open on a page the reader was already leaving.
                    onClick={(e) => {
                      e.stopPropagation();
                      onEdit(event);
                    }}
                  >
                    {cwSafe(wording, "edit_label")}
                  </button>
                )}
                {onDelete && (
                  <button
                    type="button"
                    style={w.cardAction}
                    onClick={(e) => {
                      e.stopPropagation();
                      onDelete(event);
                    }}
                  >
                    {cwSafe(wording, "delete_label")}
                  </button>
                )}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

/**
 * A card marker or control label, read defensively.
 *
 * `cw` throws by design when a key is missing — correct at the page boundary,
 * wrong inside a list where it would take the whole timeline down over one
 * absent marker. The card is already rendering; a missing word degrades to
 * nothing rather than to a blank screen.
 *
 * ## ⚑ Which is worse: a blank button, or no page?
 *
 * For the two CONTROLS this is a real trade, and it is decided in their favour
 * for one reason: they are drawn beside the words they act on, in a row that
 * already names the event. A ✎ with no glyph is still a control a reader can
 * find and press; a thrown error is twenty-two events nobody can read. The
 * boot loader refuses to start when a declared key has no row, so this
 * degradation is a second line of defence, not the plan.
 */
function cwSafe(wording: ChronologyWording, key: string): string {
  const value = wording[key];
  return typeof value === "string" && value.trim() !== "" ? value : "";
}

export default TimelineEventCard;
