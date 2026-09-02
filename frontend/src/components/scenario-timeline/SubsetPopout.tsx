// =============================================================================
// SubsetPopout.tsx — the story as its own always-on-top desktop window
// =============================================================================
//
// TIMELINE_SUBSET_MOCKUP_v2_2026-08-31.html Screen 5, approved as drawn ("Screen
// 5 awesome" — Roman, 08-31), and design §11 item 5.
//
// A module of its own and NOT another branch inside `ScenarioTimelineDock`, for
// two reasons. Rule 17: the dock was already the longest file in this directory
// before this landed. And the work here is genuinely different — dressing a
// second `Document` that inherits nothing from the page and portalling into it.
//
// ## ⚑ WHO OPENS THE WINDOW, AND WHY IT IS NOT THIS COMPONENT
//
// The dock opens it, in the click handler, and hands the opened `Window` here as
// a prop. That is not a style preference — it is the fix for a defect this
// component HAD, found by clicking the button on 2026-08-31:
//
//   NotAllowedError: Failed to execute 'requestWindow' on
//   'DocumentPictureInPicture': Document PiP requires user activation
//
// The first draft called `requestWindow` from a `useEffect` that ran after
// React committed the state change from the click. Two things go wrong there.
// The activation is TRANSIENT — the browser grants it to the click's own call
// stack, and an effect is a later turn. And React 18's StrictMode invokes a
// mount effect twice in development, so the second call arrives with the
// activation already spent by the first. Neither is visible in a unit test:
// both are properties of the browser and of React's scheduler.
//
// Calling it in the handler is also what MDN and developer.chrome.com show, for
// exactly this reason. So this component receives a window that is already
// open, and never opens or closes one.
//
// ## What it DOES do, in order
//
//  1. Re-creates the page's stylesheets inside the new document — a new
//     `Document` inherits NOTHING, so without this the subset renders as
//     unstyled black-on-white (`stylesheetsToClone`, and the MDN pattern).
//  2. Mirrors the page root's theme attributes onto it (`rootAttributesToMirror`).
//  3. Listens for `pagehide`, which is how the OS window closing reaches this
//     program when the reader used the window's own close button rather than
//     the ⇲ in the bar. After T7 that is a DIFFERENT callback from ⇲'s: one
//     docks the story, the other puts it away. See `onWindowClosed`.
//  4. Portals the SAME children the in-page window renders into its body. One
//     row design, two containers — the mockup's own words for Screen 5.
//
// Step 1 is guarded by a marker attribute so StrictMode's double mount dresses
// the document once rather than appending every stylesheet twice.

import React, { useEffect, useState } from "react";
import { createPortal } from "react-dom";

import { cw } from "../../services/caseTimeline";
import type { ChronologyWording } from "../../services/caseTimeline";
import { rootAttributesToMirror, stylesheetsToClone, unclonableCount } from "./popout";
import * as ws from "./windowStyles";

type Props = {
  /** The window the DOCK opened, in the click handler. Never opened here. */
  pipWindow: Window;
  /** The subset's name and count, for the popped-out bar. */
  title: string;
  count: string;
  wording: ChronologyWording;
  /** ⇲ `Back into the page` — dock it: this window closes, the in-page one opens. */
  onPopIn: () => void;
  /**
   * The OS window's own close button, heard through `pagehide`.
   *
   * ## ⚑ A SEPARATE PROP FROM `onPopIn`, AND T7 IS WHY
   *
   * Until T7 these were the same callback, and that was right: the floating
   * window was an extra the reader had opted into with ⧉, so taking it away
   * returned them to the in-page window they came from. T7.2 changes what the
   * event MEANS. `View Timeline` now opens this window, so closing it is the
   * reader putting the story away — not asking for a docked copy of it. One
   * event, two possible intentions, and the caller is the one entitled to say
   * which; this component only reports what happened.
   */
  onWindowClosed: () => void;
  /** × — close both. */
  onClose: () => void;
  children: React.ReactNode;
};

/**
 * The popped-out document's own body rules — a new `Document` has none.
 *
 * The colours are custom properties because the cloned stylesheets define them.
 * The FONT cannot be, and that is the one line here worth a note.
 */
const POPOUT_BODY_STYLE = [
  "margin:0",
  "background:var(--bg-surface)",
  "color:var(--text-primary)",
  // STRUCTURAL: the typeface name, written out rather than read from a token.
  // This app has no font custom property — `App.tsx` sets `fontFamily: "'Inter',
  // sans-serif"` inline on the shell for the same reason — and this attribute is
  // applied to the new document BEFORE its cloned stylesheets have parsed, so a
  // `var()` here would resolve to nothing and the window would flash in Times
  // New Roman. It must match the app's declared typeface; if that ever changes,
  // it changes in `App.tsx` and here together.
  "font-family:'Inter',sans-serif",
  "height:100%",
  "overflow:hidden",
].join(";");

/** Marks a document this component has already dressed. See the header. */
const DRESSED_MARKER = "data-colossus-subset-popout";

const SubsetPopout: React.FC<Props> = ({
  pipWindow,
  title,
  count,
  wording,
  onPopIn,
  onWindowClosed,
  onClose,
  children,
}) => {
  const [host, setHost] = useState<HTMLElement | null>(null);

  useEffect(() => {
    const doc = pipWindow.document;

    if (doc.documentElement.getAttribute(DRESSED_MARKER) === null) {
      doc.documentElement.setAttribute(DRESSED_MARKER, "1");

      // ── the stylesheets, or the window renders unstyled ────────────────────
      const clones = stylesheetsToClone(document.styleSheets);
      const lost = unclonableCount(document.styleSheets);
      if (lost > 0) {
        // Not silent: a sheet with neither readable rules nor an href cannot be
        // reproduced by any means, and the visible result is a window styled
        // differently from the page it came from.
        console.warn(
          `The popped-out timeline window could not reproduce ${lost} of ` +
            `${document.styleSheets.length} stylesheets; it may not match the page.`,
        );
      }
      for (const clone of clones) {
        if (clone.kind === "inline") {
          const style = doc.createElement("style");
          style.textContent = clone.css;
          doc.head.appendChild(style);
        } else {
          const link = doc.createElement("link");
          link.rel = "stylesheet";
          link.href = clone.href;
          if (clone.media !== "") link.media = clone.media;
          doc.head.appendChild(link);
        }
      }

      // ── the theme, mirrored rather than assumed ────────────────────────────
      for (const attribute of rootAttributesToMirror(document.documentElement)) {
        doc.documentElement.setAttribute(attribute.name, attribute.value);
      }
      doc.body.setAttribute("style", POPOUT_BODY_STYLE);
    }

    // The OS window's own close button. React never hears about it, so without
    // this the dock would still believe the story is floating, and the reader
    // would be left with a View Timeline button that appears to do nothing.
    // What it MEANS is the dock's ruling, not this component's — see the prop.
    pipWindow.addEventListener("pagehide", onWindowClosed);
    setHost(doc.body);

    return () => {
      // Detach the listener and NOTHING else. Closing the window here would be
      // this component destroying something it does not own — and under
      // StrictMode's mount/unmount/mount it would close the reader's window
      // half a second after they opened it. The dock closes it, because the
      // dock opened it.
      pipWindow.removeEventListener("pagehide", onWindowClosed);
    };
  }, [pipWindow, onWindowClosed]);

  if (host === null) return null;

  return createPortal(
    <div style={ws.popoutShell}>
      <div style={ws.popoutBar}>
        <span style={ws.barTitle}>{title}</span>
        <span style={ws.barCount}>{count}</span>
        <span style={ws.barActions}>
          <button
            type="button"
            style={ws.barPopButton}
            aria-label={cw(wording, "subsets_window_popin_label")}
            title={cw(wording, "subsets_window_popin_label")}
            onClick={onPopIn}
          >
            ⇲
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
      {children}
    </div>,
    host,
  );
};

export default SubsetPopout;
