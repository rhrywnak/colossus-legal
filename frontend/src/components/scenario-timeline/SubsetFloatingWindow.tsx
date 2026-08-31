// =============================================================================
// SubsetFloatingWindow.tsx — the in-page window: title bar, drag, resize
// =============================================================================
//
// TIMELINE_SUBSET_MOCKUP_v2_2026-08-31.html Screen 2's `.fw`, approved as drawn,
// and design §5C / §11 item 2.
//
// Split out of `ScenarioTimelineDock` in T4, when the dock crossed Rule 17's
// 300 code lines. It is also the honest boundary: the dock is a BUTTON that
// fetches a scenario's subsets and decides whether to draw anything, and this
// is a draggable box. The dock still owns every piece of state — this component
// holds none, and reports what the reader did through its callbacks.
//
// Its sibling is `SubsetPopout`, which is the same story in a real OS window.
// Both are handed the SAME `children` by the dock, which is what makes Screen
// 5's promise true: one row design, two containers.
//
// ## ⚑ A PORTAL TO `body`, and it is not decoration
//
// `Rnd` places its element relative to the nearest POSITIONED ancestor, and
// every one of the five scenario surfaces wraps the dock in its own laid-out
// page. The first-open rule computes VIEWPORT coordinates — "the right edge,
// 20px below the header strip" is a statement about the screen, not about a
// card — so the two disagreed and the window landed in the middle of the page
// instead of in the right margin. Portalling to `body` makes the coordinate
// space the one the rule is written in, and it also puts the window above every
// page's own stacking context rather than inside one. Found by opening it, not
// by reading it.

import React from "react";
import { createPortal } from "react-dom";
import { Rnd } from "react-rnd";

import { cw, type ChronologyWording } from "../../services/caseTimeline";
import type { AttachedSubset } from "./scenarioTimeline";
import { MIN_HEIGHT, MIN_WIDTH, minimizedPosition, type WindowState } from "./subsetWindow";
import * as ws from "./windowStyles";

type Props = {
  win: WindowState;
  /**
   * The subset the bar NAMES — id, name and count, and nothing more.
   *
   * Narrower than `AttachedSubset` on purpose: a PREVIEWED subset is not
   * attached, so it has no `position` and its `gap_count` is a fact about the
   * subset rather than about this scenario's link to it. The bar renders three
   * fields; asking for five would have forced the preview path to invent two.
   */
  current: Pick<AttachedSubset, "id" | "name" | "event_count">;
  /** Every attached subset, in selector order — the selector draws on >1. */
  ordered: AttachedSubset[];
  /** The already-filled "{count} events" line. */
  countLine: string;
  wording: ChronologyWording;
  /** ⧉ draws only once there is a story to send. See the note on the button. */
  canPopOut: boolean;
  onPersist: (next: WindowState) => void;
  onPopOut: () => void;
  onClose: () => void;
  children: React.ReactNode;
};

const SubsetFloatingWindow: React.FC<Props> = ({
  win,
  current,
  ordered,
  countLine,
  wording,
  canPopOut,
  onPersist,
  onPopOut,
  onClose,
  children,
}) => {
  return createPortal(
    <Rnd
      size={{ width: win.width, height: win.minimized ? MIN_HEIGHT : win.height }}
      // Minimized, the bar is PINNED bottom-right (§5C), and `win.x/y` go on
      // meaning where the OPEN window belongs — so restoring returns it to the
      // place the reader put it, not to wherever its bar happened to sit.
      position={
        win.minimized
          ? minimizedPosition(window.innerWidth, window.innerHeight)
          : { x: win.x, y: win.y }
      }
      minWidth={MIN_WIDTH}
      minHeight={MIN_HEIGHT}
      bounds="window"
      // Drag by the title bar only (mockup, and §5C): a window draggable by its
      // body cannot have a scrolling body.
      dragHandleClassName="subset-window-bar"
      enableResizing={!win.minimized}
      style={{ zIndex: ws.WINDOW_Z_INDEX }}
      onDragStop={(_e, dd) => onPersist({ ...win, x: dd.x, y: dd.y })}
      onResizeStop={(_e, _dir, ref, _delta, pos) =>
        onPersist({
          ...win,
          width: ref.offsetWidth,
          height: ref.offsetHeight,
          x: pos.x,
          y: pos.y,
        })
      }
    >
      <div style={win.minimized ? ws.minimizedBar : ws.shell}>
        {/* Title bar order, mockup v2 Screen 2: name · count · selector (only
            when the scenario carries more than one) · ⧉ · – · ×. */}
        <div style={win.minimized ? undefined : ws.bar} className="subset-window-bar">
          <span style={ws.barTitle}>{current.name}</span>
          <span style={ws.barCount}>{countLine}</span>
          {ordered.length > 1 && (
            <select
              style={ws.barSelect}
              value={current.id}
              onChange={(e) => onPersist({ ...win, subsetId: e.target.value })}
            >
              {ordered.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.name}
                </option>
              ))}
            </select>
          )}
          <span style={ws.barActions}>
            {/* ⚑ HIDDEN WHILE THE STORY IS STILL LOADING, and that is the point
                of the condition. Pop out hands the new window the CONTENTS as
                they stand; clicked a beat early it would open a real OS window
                holding a loading line, which is a worse thing to have on your
                desktop than a button that waited half a second. Hidden while
                MINIMIZED for the same reason the other controls are: there is
                nothing but a bar to pop out. */}
            {!win.minimized && canPopOut && (
              <button
                type="button"
                style={ws.barPopButton}
                aria-label={cw(wording, "subsets_window_popout_label")}
                title={cw(wording, "subsets_window_popout_label")}
                onClick={onPopOut}
              >
                ⧉
              </button>
            )}
            <button
              type="button"
              style={ws.barButton}
              aria-label={cw(wording, "subsets_window_minimize_label")}
              title={cw(wording, "subsets_window_minimize_label")}
              onClick={() => onPersist({ ...win, minimized: !win.minimized })}
            >
              {win.minimized ? "▢" : "–"}
            </button>
            <button
              type="button"
              style={ws.barButton}
              aria-label={cw(wording, "subsets_window_close_label")}
              title={cw(wording, "subsets_window_close_label")}
              onClick={onClose}
            >
              ×
            </button>
          </span>
        </div>

        {/* A failed phases read is shown, not swallowed; the loading line has a
            word of its own. Both are in the tree the dock builds — see its
            `contents`, which is the SAME value handed to `SubsetPopout`. */}
        {!win.minimized && children}
      </div>
    </Rnd>,
    document.body,
  );
};

export default SubsetFloatingWindow;
