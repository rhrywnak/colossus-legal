// =============================================================================
// TimelineEventCard.tsx — one dated fact, as the list draws it
// =============================================================================
//
// Mockup v2 Screen 1's `.ev`: date · dot · title · tag chip(s) · fact in serif,
// then the row of links and the note badge. Every decision it renders — the
// date's shape, the dot's colour, which of three states a link is in, whether
// there is a badge at all — is made by a pure function in `timelineFilters`,
// where a test can reach it.

import React from "react";

import { API_BASE_URL } from "../../services/api";
import type {
  ChronologyWording,
  TimelineEvent,
  TimelineTag,
} from "../../services/caseTimeline";
import { dotColor, formatEventDate, linkRendering, noteBadge, tagOf } from "./timelineFilters";
import * as s from "./timelineStyles";

type Props = {
  event: TimelineEvent;
  tags: TimelineTag[];
  wording: ChronologyWording;
  /** Open this event's page. */
  onOpen: (id: string) => void;
};

/** The colour an event's dot falls back to when no tag of its is known. */
const NEUTRAL_DOT = "var(--text-disabled)";

const TimelineEventCard: React.FC<Props> = ({ event, tags, wording, onOpen }) => {
  const badge = noteBadge(event.note_count, wording);

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

        {(event.links.length > 0 || badge) && (
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
          </div>
        )}
      </div>
    </div>
  );
};

/**
 * The pinpoint marker, read defensively.
 *
 * `cw` throws by design when a key is missing — correct at the page boundary,
 * wrong inside a list where it would take the whole timeline down over one
 * absent marker. The card is already rendering; a missing marker degrades to
 * nothing rather than to a blank screen.
 */
function cwSafe(wording: ChronologyWording, key: string): string {
  const value = wording[key];
  return typeof value === "string" && value.trim() !== "" ? value : "";
}

export default TimelineEventCard;
