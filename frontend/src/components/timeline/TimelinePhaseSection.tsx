// =============================================================================
// TimelinePhaseSection.tsx — one phase, its subtitle, and its scroll window
// =============================================================================
//
// Mockup v2 Screen 1's `.phase`. Three things arrived with Phase B:
//
//  · the phase DESCRIPTION renders, as the muted italic line (design R14). It
//    has been stored since the first timeline and rendered by nothing.
//  · the body is a SCROLL WINDOW whose size is a stored number (design R6).
//  · every header carries an always-visible Expand control (design R16, R17),
//    and expanding applies this phase as the page's filter rather than opening
//    anything — which is what keeps the product two levels deep.

import React from "react";

import type {
  ChronologyWording,
  TimelineEvent,
  TimelinePhase,
  TimelineTag,
} from "../../services/caseTimeline";
import { cw, fill } from "../../services/caseTimeline";
import TimelineEventCard from "./TimelineEventCard";
import TimelineUndoLine from "./TimelineUndoLine";
import * as s from "./timelineStyles";
import { rowsForPhase } from "./timelineWriteRules";

type Props = {
  phase: TimelinePhase;
  events: TimelineEvent[];
  tags: TimelineTag[];
  wording: ChronologyWording;
  /** How many events the window shows before it scrolls. */
  windowEvents: number;
  /** True when this phase owns the page — no window, normal page scrolling. */
  expanded: boolean;
  /**
   * Events deleted in this visit, across every phase.
   *
   * This section draws the undo line only for its OWN — see `rowsForPhase`. The
   * whole list is passed rather than a filtered one so the filtering decision
   * stays in the tested pure function rather than in the page above.
   */
  undoable: TimelineEvent[];
  onToggleExpand: () => void;
  onOpenEvent: (id: string) => void;
  onEditEvent?: (event: TimelineEvent) => void;
  onDeleteEvent?: (event: TimelineEvent) => void;
  onUndoDelete?: (event: TimelineEvent) => void;
};

const TimelinePhaseSection: React.FC<Props> = ({
  phase,
  events,
  tags,
  wording,
  windowEvents,
  expanded,
  undoable,
  onToggleExpand,
  onOpenEvent,
  onEditEvent,
  onDeleteEvent,
  onUndoDelete,
}) => {
  // The window only earns its scroll hint when there is something below the
  // fold. A hint over four events in a four-event window would be a lie.
  const scrolls = !expanded && events.length > windowEvents;
  // A deleted event's undo line sits where its card was, so the window's own
  // height is unchanged by a delete — the row is replaced, not removed.
  const rows = rowsForPhase(events, undoable, phase.id);
  const body = (
    <>
      {rows.map((row) =>
        row.kind === "event" ? (
          <TimelineEventCard
            key={row.event.id}
            event={row.event}
            tags={tags}
            wording={wording}
            onOpen={onOpenEvent}
            onEdit={onEditEvent}
            onDelete={onDeleteEvent}
          />
        ) : (
          <TimelineUndoLine
            key={`undo:${row.event.id}`}
            event={row.event}
            wording={wording}
            onUndo={onUndoDelete}
          />
        ),
      )}
    </>
  );

  return (
    <section style={s.phase} id={`phase-${phase.id}`}>
      <div style={s.phaseHead(phase.color)}>
        <span style={s.phaseLabel}>{phase.label}</span>
        <span style={s.phaseMeta}>
          {fill(cw(wording, "phase_count_template"), {
            range: phase.date_range,
            count: events.length,
          })}
        </span>
        <button type="button" style={s.expandControl} onClick={onToggleExpand}>
          {/* Two calls and not one with a ternary inside: the reach guard reads
              the FIRST literal of a call, so a conditional key would leave the
              other invisible to it — declared, requested, and unguarded. */}
          {expanded ? cw(wording, "show_all_phases_label") : cw(wording, "expand_label")}
        </button>
      </div>

      {phase.description && <div style={s.phaseDesc}>{phase.description}</div>}

      {scrolls && (
        <div style={s.scrollHint}>
          {fill(cw(wording, "scroll_hint_template"), { count: windowEvents })}
        </div>
      )}

      {expanded ? (
        // No window: the phase owns the page and the page scrolls normally
        // (mockup Screen 1b).
        <div>{body}</div>
      ) : (
        <div style={s.scrollWindow(windowEvents)}>{body}</div>
      )}
    </section>
  );
};

export default TimelinePhaseSection;
