// =============================================================================
// SubsetPopoutPage.tsx — one subset, alone, at its own address
// =============================================================================
//
// The FALLBACK half of Pop out (design §11 item 5, mockup Screen 5). Where the
// Document Picture-in-Picture API exists — Chrome and Edge — the dock portals
// the same React tree into a real always-on-top window and this page is never
// reached. Where it does not — Safari, Firefox — the button opens a plain
// `window.open` popup, and a popup needs a URL. This is that URL.
//
// ## ⚑ NO APP CHROME, AND THAT IS ARRANGED IN `App.tsx`
//
// This route is matched ABOVE the shell that draws the header and the 1080-px
// `<main>`, so nothing here has to hide anything. A page that rendered inside
// the shell and then hid its surroundings with CSS would still be laying them
// out — the popup would open showing a nav bar for a frame, and its content
// would be inset by a page margin meant for a 1440-px screen.
//
// ## Its own reads, like the dock's
//
// Two, and they are the two the in-page window already makes: the subset, and
// the timeline payload that carries the phases, the tag vocabulary and every
// word this surface speaks. Nothing is passed in, because nothing CAN be —
// a popup is a new document with no props from its opener.
//
// ## Live data: refetch on focus, and that is enough
//
// Design §11 wants a change made on the timeline page to show here. It does
// NOT poll: a window that re-reads on a timer burns a request a minute for a
// panel somebody left open beside a question they are thinking about. It
// re-reads when the reader gives the window focus, which is exactly when they
// have come back to look at it — the same moment a change would matter.

import React, { useCallback, useEffect, useState } from "react";
import { useParams } from "react-router-dom";

import {
  BOOTSTRAP_TEXT,
  cw,
  getCaseTimeline,
  type ChronologyWording,
  type TimelinePhase,
  type TimelineTag,
} from "../services/caseTimeline";
import { getSubset, type SubsetDetail } from "../services/caseTimelineSubsets";
import SubsetWindowBody from "../components/scenario-timeline/SubsetWindowBody";
import * as ws from "../components/scenario-timeline/windowStyles";
import { timelineEventPath, timelinePath } from "../utils/routePaths";

/** What the timeline payload gives this page: the furniture and the words. */
type Frame = {
  phases: TimelinePhase[];
  tags: TimelineTag[];
  wording: ChronologyWording;
};

const SubsetPopoutPage: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const [frame, setFrame] = useState<Frame | null>(null);
  const [subset, setSubset] = useState<SubsetDetail | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    if (id === undefined) {
      // Not a silent return. The route pattern guarantees a segment when it
      // MATCHES, so reaching this means the URL was hand-edited or the route
      // was renamed — and a popup sitting for ever on its loading line, with no
      // nav to leave by, is the worst possible way to say so.
      setError(BOOTSTRAP_TEXT.timelineFailed("no subset id in the address"));
      return;
    }
    setError(null);
    // Both reads, every time. Not swallowed: either failure names itself and
    // reaches the surface, because a popped-out window showing a stale story
    // with no sign of the failure is the one outcome worse than showing none.
    Promise.all([getCaseTimeline(), getSubset(id)])
      .then(([timeline, full]) => {
        setFrame({ phases: timeline.phases, tags: timeline.tags, wording: timeline.wording });
        setSubset(full);
      })
      .catch((err: unknown) => {
        setError(err instanceof Error ? err.message : "unknown error");
      });
  }, [id]);

  useEffect(() => {
    load();
  }, [load]);

  // Refetch on focus — the live-data half, stated in the module header.
  useEffect(() => {
    window.addEventListener("focus", load);
    return () => window.removeEventListener("focus", load);
  }, [load]);

  if (error !== null) return <div style={ws.errorState}>{error}</div>;
  // ⚑ The one English string on this surface, and it is the documented
  // bootstrap: the wording block arrives on the very request whose pending
  // state this describes, so there is no stored word to say it with yet. Every
  // other line below comes from `frame.wording`.
  if (frame === null || subset === null) return <div style={ws.state}>{BOOTSTRAP_TEXT.loading}</div>;

  return (
    <div style={ws.popoutShell}>
      <div style={ws.popoutBar}>
        <span style={ws.barTitle}>{subset.name}</span>
        <span style={ws.barCount}>
          {cw(frame.wording, "subsets_window_events_count_template").replace(
            "{count}",
            String(subset.event_count),
          )}
        </span>
      </div>
      <SubsetWindowBody
        subset={subset}
        phases={frame.phases}
        tags={frame.tags}
        wording={frame.wording}
        // A popup is already a second window; a THIRD one per click would be
        // the reader losing track of which window holds what. These go to the
        // opener's tab, which is where the timeline belongs.
        onOpenTimeline={() => window.open(timelinePath(), "_blank", "noopener")}
        onEditSubset={() => window.open(timelinePath(), "_blank", "noopener")}
        onOpenEvent={(eventId) => window.open(timelineEventPath(eventId), "_blank", "noopener")}
      />
    </div>
  );
};

export default SubsetPopoutPage;
