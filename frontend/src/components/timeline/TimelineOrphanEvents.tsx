// =============================================================================
// TimelineOrphanEvents.tsx — events naming a phase that has no row
// =============================================================================
//
// Extracted from `TimelinePage` when the Subsets section mounted there (T2):
// the page was already over Rule 17's 300 code lines, and this block is the one
// piece of it that is a self-contained screen rather than page wiring.
//
// ## Why the loud row exists at all
//
// Before Phase B an event whose `phase` matched no row fell out of the render
// entirely — it belonged to no section, so nothing drew it. It now renders in a
// row naming its id and the phase it asked for. An event nobody can see is an
// event nobody can fix.
//
// And it is EDITABLE, deliberately: this row exists so the event can be
// corrected, and a row you can see but not fix is only half the fix.

import React from "react";

import type {
  ChronologyWording,
  TimelineEvent,
  TimelineTag,
} from "../../services/caseTimeline";
import { cw, fill } from "../../services/caseTimeline";
import TimelineEventCard from "./TimelineEventCard";
import * as s from "./timelineStyles";

type Props = {
  events: TimelineEvent[];
  tags: TimelineTag[];
  wording: ChronologyWording;
  onOpen: (id: string) => void;
  onEdit: (event: TimelineEvent) => void;
  onDelete: (event: TimelineEvent) => void;
};

const TimelineOrphanEvents: React.FC<Props> = ({
  events,
  tags,
  wording,
  onOpen,
  onEdit,
  onDelete,
}) => {
  if (events.length === 0) return null;
  return (
    <div>
      {events.map((event) => (
        <div key={event.id} style={s.unknownPhase}>
          {fill(cw(wording, "unknown_phase_template"), {
            id: event.id,
            phase: event.phase,
          })}
          <TimelineEventCard
            event={event}
            tags={tags}
            wording={wording}
            onOpen={onOpen}
            onEdit={onEdit}
            onDelete={onDelete}
          />
        </div>
      ))}
    </div>
  );
};

export default TimelineOrphanEvents;
