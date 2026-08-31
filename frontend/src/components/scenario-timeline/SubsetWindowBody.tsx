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
                  {/* ⚑ THE "date to confirm" BADGE WAS HERE AND IS RETIRED
                      (Roman's ruling, 2026-08-31, reversing his own T4 call).
                      It could only read `approximate`, so it claimed four of
                      the case's thirty-one events needed a date confirmed —
                      including two nobody has ever flagged. The ⚑ carries a
                      specific meaning in this case and spreading it that
                      thinly destroyed the signal. What stays is what the data
                      supports: the date is amber and its caption says
                      "month · approx.". See the T6 round-two migration. */}
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
        {/* "15 events" — and only that, since T6 round two.
            Two rulings, in order. It first shipped as "15 on the chronology · 0
            gaps" and was rejected on 2026-08-31: a DIFFERENT NUMBER wearing the
            same clothes, since the gap count answers "how many of these were
            deleted off the chronology". It became "15 events · 2 ⚑", and the ⚑
            half was retired later the same day with the badge it counted — see
            `isDateToConfirm`. Gaps are still marked, one badge per row, where a
            reader can act on one. `footerLine` composes this and is tested. */}
        <span style={ws.footCount}>{footerLine(subset.events, wording)}</span>
      </div>
    </>
  );
};

export default SubsetWindowBody;
