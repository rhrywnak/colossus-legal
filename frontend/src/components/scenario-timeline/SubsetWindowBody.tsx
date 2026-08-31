// =============================================================================
// SubsetWindowBody.tsx — the window's contents (mockup Screen 1, inside `.fw`)
// =============================================================================
//
// Split from the window shell for Rule 17 and because these are two different
// concerns: the shell is a draggable box, this is a subset rendered as the
// timeline renders it. Description strip, phase dividers, event rows, footer.
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

import type { ChronologyWording, TimelinePhase } from "../../services/caseTimeline";
import { cw, fill } from "../../services/caseTimeline";
import type { SubsetDetail } from "../../services/caseTimelineSubsets";
import * as ws from "./windowStyles";

type Props = {
  subset: SubsetDetail;
  phases: TimelinePhase[];
  wording: ChronologyWording;
  onOpenTimeline: () => void;
  onEditSubset: () => void;
  onOpenEvent: (eventId: string) => void;
};

/** One phase's colour and label, or a muted fallback for a phase with no row. */
function phaseOf(phases: TimelinePhase[], id: string): { label: string; color: string } {
  const found = phases.find((p) => p.id === id);
  // A phase the payload does not carry still gets a divider rather than none:
  // an event silently filed under nothing is the defect the timeline's own
  // unknown-phase row exists to prevent.
  return found === undefined
    ? { label: id, color: "var(--text-muted)" }
    : { label: found.label, color: found.color };
}

const SubsetWindowBody: React.FC<Props> = ({
  subset,
  phases,
  wording,
  onOpenTimeline,
  onEditSubset,
  onOpenEvent,
}) => {
  const live = subset.events.filter((e) => !e.removed).length;
  const gaps = subset.events.length - live;

  let lastPhase: string | null = null;

  return (
    <>
      {subset.description !== "" && <div style={ws.description}>{subset.description}</div>}

      <div style={ws.body}>
        {subset.events.map((row) => {
          const event = row.event;
          const phase = phaseOf(phases, event.phase);
          // A divider whenever the phase changes, so "estate → probate →
          // appeals" stays visible without leaving the list (design §5C).
          const divider = event.phase !== lastPhase;
          lastPhase = event.phase;
          return (
            <React.Fragment key={event.id}>
              {divider && (
                <div style={ws.phaseDivider(phase.color)}>
                  <span style={ws.dividerRule} />
                  <span>{phase.label}</span>
                  <span style={ws.dividerRule} />
                </div>
              )}
              <button
                type="button"
                style={ws.eventRow(row.removed)}
                onClick={() => onOpenEvent(event.id)}
              >
                <span style={ws.eventDate}>{event.event_date}</span>
                <span style={ws.eventDot(phase.color)} />
                <span>
                  <h4 style={row.removed ? ws.removedTitle : ws.eventTitle}>
                    {event.title}
                    {row.removed && (
                      <span style={ws.gapBadge}>{cw(wording, "subsets_gap_badge_label")}</span>
                    )}
                  </h4>
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
        {/* TWO numbers and not one total, for the reason the stored template's
            own note gives: "15 events" over a list showing twelve live lines
            and three struck ones is the sentence that makes a reader distrust
            the count. */}
        <span style={ws.footCount}>
          {fill(cw(wording, "subsets_window_footer_template"), {
            on_chronology: live,
            gaps,
          })}
        </span>
      </div>
    </>
  );
};

export default SubsetWindowBody;
