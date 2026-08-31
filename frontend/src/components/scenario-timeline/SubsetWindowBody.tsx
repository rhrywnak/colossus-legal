// =============================================================================
// SubsetWindowBody.tsx — the window's contents (mockup v2 Screen 2, inside `.fw`)
// =============================================================================
//
// Split from the window shell for Rule 17 and because these are two different
// concerns: the shell is a draggable box, this is a subset rendered as the
// timeline renders it. Description strip, year dividers, event rows, footer.
//
// It is ARRANGEMENT ONLY. Every decision a row makes — which divider goes
// above it, whether it carries the ⚑, what the caption under its date says,
// how the two lines of the date split — is a pure function in `subsetRows.ts`
// or `timelineFilters.ts`, where a test can reach it. This project has no
// component-testing tier, so anything decided inside the `.map()` below would
// be decided where nothing can check it.
//
// ## ⚑ THE DATE IS THE FIRST AND BOLDEST THING IN EVERY ROW
//
// Defect D3, 2026-08-31: this shipped with the date as a grey caption beside a
// black title. Design §11 item 2 and the v2 mockup put it in its own 96-px
// column, at full ink, weight 800, with a rule in the tag's colour down its
// right edge and the year small and muted underneath. Nothing else in the row
// is grey. If a future change makes the date quieter than the title, it has
// re-introduced the defect.
//
// ## ⚑ CLICKING AN EVENT OPENS A NEW TAB, ALWAYS
//
// Design §5C is explicit and §5D says why: Marie is reading this beside a
// cross-examination question. Navigating the page UNDER the window would take
// away the question she is answering to show her the answer's evidence — she
// would have to find her way back to a place she never chose to leave.
//
// ## The gap rows are the point, not an error state
//
// A `removed` event is one the chronology soft-deleted. The row is MARKED and
// kept — struck, amber, badged with the stored `subsets_gap_badge_label` — never
// dropped. Dropping it would silently shorten a story somebody counted, and the
// design calls the visible gap half a subset's value: the story saying "this
// happened and it is not on our timeline yet".

import React from "react";

import type {
  ChronologyWording,
  TimelinePhase,
  TimelineTag,
} from "../../services/caseTimeline";
import { cw } from "../../services/caseTimeline";
import type { SubsetDetail } from "../../services/caseTimelineSubsets";
import { dotColor, splitEventDate } from "../timeline/timelineFilters";
import {
  crossesPhases,
  dateCaption,
  dividerFor,
  footerLine,
  isDateToConfirm,
} from "./subsetRows";
import * as ws from "./windowStyles";

type Props = {
  subset: SubsetDetail;
  phases: TimelinePhase[];
  /** The case's tag vocabulary — the source of every row's rule colour. */
  tags: TimelineTag[];
  wording: ChronologyWording;
  onOpenTimeline: () => void;
  onEditSubset: () => void;
  onOpenEvent: (eventId: string) => void;
};

const SubsetWindowBody: React.FC<Props> = ({
  subset,
  phases,
  tags,
  wording,
  onOpenTimeline,
  onEditSubset,
  onOpenEvent,
}) => {
  // Asked once for the whole story, because "does this story cross phases" is a
  // fact about the story and not about any one row.
  const spansPhases = crossesPhases(subset.events);

  return (
    <>
      {subset.description !== "" && <div style={ws.description}>{subset.description}</div>}

      <div style={ws.body}>
        {subset.events.map((row, index) => {
          const event = row.event;
          const previous = index === 0 ? null : subset.events[index - 1].event;
          const divider = dividerFor(event, previous, spansPhases, phases, wording);
          const date = splitEventDate(event.event_date, event.approximate, event.date_precision);
          const caption = dateCaption(event, date.year, wording);
          // The tag's OWN stored colour. `--border-default` is the fallback for
          // an event whose tags the vocabulary has not caught up with: a rule
          // in the hairline colour, which reads as "no tag" rather than as some
          // other tag's green.
          const rule = dotColor(tags, event, "var(--border-default)");
          const flagged = isDateToConfirm(event);
          return (
            <React.Fragment key={event.id}>
              {divider !== null && (
                <div style={ws.yearDivider}>
                  <span style={ws.dividerRule} />
                  <span>{divider}</span>
                  <span style={ws.dividerRule} />
                </div>
              )}
              <button
                type="button"
                style={ws.eventRow(row.removed)}
                onClick={() => onOpenEvent(event.id)}
              >
                <span style={ws.eventDate(rule, event.approximate)}>
                  {date.lead}
                  {caption !== "" && <small style={ws.eventDateCaption}>{caption}</small>}
                </span>
                <span>
                  <h4 style={row.removed ? ws.removedTitle : ws.eventTitle}>{event.title}</h4>
                  {event.tags.map((id) => {
                    const tag = tags.find((t) => t.id === id);
                    // A tag the vocabulary does not carry gets no pill rather
                    // than a pill reading its raw slug: the slug is not a word
                    // anybody chose to show, and the rule colour already
                    // degraded for the same event.
                    return tag === undefined ? null : (
                      <span key={id} style={ws.tagPill(tag.color)}>
                        {tag.label}
                      </span>
                    );
                  })}
                  {/* ⚑ The GLYPH is in code and the WORDS are a row, which is
                      the same split the title bar's ⧉ ⇲ – × already use: a
                      glyph is furniture with no language in it, and the
                      sentence beside it is the thing an editor would want to
                      change. The mockup draws "⚑ date to confirm"; the stored
                      row carries the four words. */}
                  {flagged && (
                    <span style={ws.dateFlag}>
                      ⚑ {cw(wording, "subsets_date_to_confirm_badge")}
                    </span>
                  )}
                  {row.removed && (
                    <span style={ws.gapBadge}>{cw(wording, "subsets_gap_badge_label")}</span>
                  )}
                  {row.removed && (
                    <p style={ws.eventFact}>{cw(wording, "subsets_removed_event_line")}</p>
                  )}
                  {!row.removed && event.fact !== undefined && event.fact !== "" && (
                    <p style={ws.eventFact}>{event.fact}</p>
                  )}
                  {row.subset_note !== "" && <div style={ws.storyNote}>{row.subset_note}</div>}
                </span>
              </button>
            </React.Fragment>
          );
        })}
      </div>

      <div style={ws.foot}>
        <button type="button" style={ws.footLink} onClick={onOpenTimeline}>
          {cw(wording, "subsets_window_open_timeline")}
        </button>
        <button type="button" style={ws.footLink} onClick={onEditSubset}>
          {cw(wording, "subsets_window_edit")}
        </button>
        {/* "15 events · 2 ⚑", exactly as Screen 2 draws it.
            This shipped as "15 on the chronology · 0 gaps" and was rejected on
            2026-08-31: it is a DIFFERENT NUMBER wearing the same clothes. The
            gap count answers "how many of these were deleted off the
            chronology"; the ⚑ answers "how many of these dates are unsettled".
            Gaps are still marked, one badge per row, where a reader can act on
            one. `footerLine` is where the composition lives and is tested. */}
        <span style={ws.footCount}>{footerLine(subset.events, wording)}</span>
      </div>
    </>
  );
};

export default SubsetWindowBody;
