// =============================================================================
// ScenarioTimelineDock.tsx — the View Timeline button and its floating window
// =============================================================================
//
// Mockup Screen 1, approved as drawn. Design §5B, §5C, §5D and §10.
//
// ## ⚑ SELF-CONTAINED BY RULING, AND WHY THAT WAS THE ONLY WAY
//
// It takes a case slug and a scenario id and NOTHING else. It fetches its own
// data, holds its own state, and speaks its own words — which arrive on the very
// read that tells it whether to draw anything at all.
//
// That shape is not preference. The five scenario surfaces this mounts on share
// no header component and no read between them: `ScenarioIdentityBlock` and
// `ScenarioHeaderTiers` belong to the detail page alone, `RehearsalPageHeader`
// to the rehearsal page, the practice page builds an inline `<h1>` from its own
// deck payload, and the dashboard has no per-scenario header at all. A component
// that needed data passed in would have needed five different pages to learn
// about it, and five payloads to grow a field. This one needs a one-line mount.
//
// ## The window never steals the page
//
// No autofocus, ever: the reader may be typing an answer underneath. Clicking an
// event opens a new tab rather than navigating the page beneath (§5C). The
// window's body is the only thing that scrolls.
//
// ## What is remembered, and what is not
//
// Position, size, minimized and the selected subset, per scenario, in
// `localStorage` — the .415 collapsed-sections pattern, and the same Standing
// Rule 1 carve-out: this is a COSMETIC preference, so a private window or a
// blocked store degrades to the design's default instead of raising a banner. It
// stays observable in the console. Every decision about it is in
// `subsetWindow.ts`, where a test can reach it.

import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { cw, fill, type TimelinePhase, type TimelineTag } from "../../services/caseTimeline";
import { getCaseTimeline } from "../../services/caseTimeline";
import { getSubset, type SubsetDetail } from "../../services/caseTimelineSubsets";
import { subsetPopoutPath, timelineEventPath, timelinePath } from "../../utils/routePaths";
import * as d from "./dockStyles";
import * as ws from "./windowStyles";
import {
  type MaybePipWindow,
  popoutSize,
  popupFeatures,
  supportsDocumentPictureInPicture,
} from "./popout";
import {
  type AttachedSubset,
  getScenarioSubsets,
  type ScenarioSubsets,
} from "./scenarioTimeline";
import ScenarioTimelineRow from "./ScenarioTimelineRow";
import SubsetFloatingWindow from "./SubsetFloatingWindow";
import SubsetPopout from "./SubsetPopout";
import SubsetWindowBody from "./SubsetWindowBody";
import {
  clampToViewport,
  decodeWindowState,
  encodeWindowState,
  initialSubsetId,
  openStateFor,
  POPUP_CLOSED_POLL_MS,
  selectorOrder,
  type WindowState,
  windowStorageKey,
} from "./subsetWindow";

type Props = {
  slug: string;
  scenarioId: string;
};

/** Read the remembered window, or null. Never throws — see the module header. */
function readStored(scenarioId: string): WindowState | null {
  try {
    return decodeWindowState(localStorage.getItem(windowStorageKey(scenarioId)));
  } catch (e: unknown) {
    // best-effort: a COSMETIC window position, so a private window, a browser
    // with site data blocked, or any storage failure degrades to the design's
    // open-position rule instead of interrupting a reader mid-question. The
    // Standing Rule 1 carve-out for browser-storage preferences; not a data
    // read — nothing about the subset itself is stored here.
    console.warn("Could not read the timeline window's remembered position.", e);
    return null;
  }
}

/** Remember the window. Never throws — same carve-out as the read. */
function writeStored(scenarioId: string, state: WindowState): void {
  try {
    localStorage.setItem(windowStorageKey(scenarioId), encodeWindowState(state));
  } catch (e: unknown) {
    // best-effort: the window works for this visit and is forgotten by the
    // next, which is a cosmetic loss and does not warrant a banner.
    console.warn("Could not remember the timeline window's position.", e);
  }
}

const ScenarioTimelineDock: React.FC<Props> = ({ slug, scenarioId }) => {
  const [data, setData] = useState<ScenarioSubsets | null>(null);
  const [readError, setReadError] = useState<string | null>(null);
  const [open, setOpen] = useState(false);
  const [win, setWin] = useState<WindowState | null>(null);
  const [subset, setSubset] = useState<SubsetDetail | null>(null);
  const [subsetError, setSubsetError] = useState<string | null>(null);
  const [phases, setPhases] = useState<TimelinePhase[]>([]);
  const [tags, setTags] = useState<TimelineTag[]>([]);
  const [phasesError, setPhasesError] = useState<string | null>(null);
  // The story is in its own desktop window. The in-page one HIDES while it is —
  // two copies of the same story, one of them stale, is the state this avoids.
  // Holding the WINDOW and not a boolean is what makes the dock its owner: it
  // opened it in a click handler, and it is the only thing that closes it.
  const [pipWindow, setPipWindow] = useState<Window | null>(null);
  /** The fallback POPUP, when that is the path this browser took. */
  const [popupWindow, setPopupWindow] = useState<Window | null>(null);
  const host = useRef<HTMLDivElement | null>(null);

  // The button's read. Also the dock's whole vocabulary.
  useEffect(() => {
    let cancelled = false;
    getScenarioSubsets(slug, scenarioId)
      .then((payload) => {
        if (!cancelled) setData(payload);
      })
      .catch((err: unknown) => {
        // Never swallowed. The button is not drawn on a failed read — there is
        // nothing truthful to draw — but the reason reaches the console and the
        // dock's own error line rather than vanishing.
        if (!cancelled) setReadError(err instanceof Error ? err.message : "unknown error");
      });
    return () => {
      cancelled = true;
    };
  }, [slug, scenarioId]);

  const attached: AttachedSubset[] = useMemo(() => data?.subsets ?? [], [data]);

  const openWindow = useCallback(() => {
    // ⚑ THE BUTTON IS ALSO THE WAY BACK, and that is deliberate.
    //
    // "View Timeline" means "show me the story here". If a popped-out window is
    // somehow still recorded as open — the reader closed it in a way this
    // program did not hear about, or a handle went stale — clicking the button
    // must put the story back in the page rather than do nothing visible. A
    // button that produces no effect is the state a reader can only escape by
    // reloading, and they have no reason to guess that.
    setPipWindow((open) => {
      open?.close();
      return null;
    });
    setPopupWindow((open) => {
      open?.close();
      return null;
    });

    const stored = readStored(scenarioId);
    const chosen = initialSubsetId(attached, stored?.subsetId ?? null);
    // A stored position always wins — clamped, because the screen it was stored
    // on may be gone. Otherwise the §11 first-open rule decides, and it needs
    // one measurement: where the app's header strip ends. `data-app-chrome` is
    // the header's own marker, already there for the print sheets; the fallback
    // is 0, which puts the window 20px from the top of the viewport — visibly
    // wrong rather than invisibly absent, if the header is ever renamed.
    const header = document.querySelector("header[data-app-chrome]");
    const headerBottom = header === null ? 0 : header.getBoundingClientRect().bottom;
    const next =
      stored === null
        ? openStateFor(window.innerWidth, window.innerHeight, headerBottom, chosen)
        : clampToViewport({ ...stored, subsetId: chosen }, window.innerWidth, window.innerHeight);
    setWin(next);
    setOpen(true);
  }, [attached, scenarioId]);

  // The window's events, read when it opens and whenever the selector moves.
  useEffect(() => {
    const id = win?.subsetId ?? null;
    if (!open || id === null) return;
    let cancelled = false;
    setSubset(null);
    setSubsetError(null);
    getSubset(id)
      .then((full) => {
        if (!cancelled) setSubset(full);
      })
      .catch((err: unknown) => {
        if (!cancelled) setSubsetError(err instanceof Error ? err.message : "unknown error");
      });
    return () => {
      cancelled = true;
    };
  }, [open, win?.subsetId]);

  // The phases and the TAG VOCABULARY — the dividers, and the colour of the
  // rule down every row's date column. The same payload the timeline page
  // reads: one request, cached by the browser, and no second opinion about what
  // colour `probate` or `financial` is. Reading the tag colours rather than
  // transcribing the mockup's five hexes is also what keeps five domain colour
  // names out of this build (standing rule 2).
  useEffect(() => {
    if (!open || phases.length > 0) return;
    let cancelled = false;
    getCaseTimeline()
      .then((t) => {
        if (!cancelled) {
          setPhases(t.phases);
          setTags(t.tags);
        }
      })
      .catch((e: unknown) => {
        // ⚑ NOT best-effort, and the first draft of this had it wrong.
        //
        // Degrading a fetch to a console.warn is the carve-out for COSMETIC
        // BROWSER-STORAGE preferences only — Standing Rule 1 says in as many
        // words that it "does NOT extend to fetch/authFetch or ANY data read".
        // The proportionality argument for staying quiet (the events still
        // render; only the dividers lose their labels and colours) is real, and
        // it is not the test. The dashboard's own read in this same branch
        // refuses the same carve-out; the dock had no business taking it.
        //
        // So the failure is BOTH rendered and degraded: the line says the
        // dividers are unlabelled, and `phaseOf` still falls back to the slug so
        // the events the reader came for are all there.
        if (!cancelled) setPhasesError(e instanceof Error ? e.message : "unknown error");
      });
    return () => {
      cancelled = true;
    };
  }, [open, phases.length]);

  const persist = useCallback(
    (next: WindowState) => {
      setWin(next);
      writeStored(scenarioId, next);
    },
    [scenarioId],
  );

  /**
   * ⧉ — the story leaves the page.
   *
   * Two paths, and the FEATURE decides which, never the user agent. Chrome and
   * Edge get a real always-on-top window through the Document Picture-in-
   * Picture API, rendered by `SubsetPopout` from this same React tree. Safari
   * and Firefox have no such API and get a plain popup at the subset's own
   * address, which carries identical contents but sits behind the app when she
   * clicks back into her answer — the difference §11 item 5 names.
   *
   * ## ⚑ A BLOCKED POPUP LEAVES THE IN-PAGE WINDOW OPEN, ON PURPOSE
   *
   * `window.open` returns `null` when a popup blocker refuses it. There is no
   * stored sentence in this build for "your browser blocked that" — the T4
   * instruction asked for the existing one and there is none; the row it needs
   * is listed in the T4 report under NEEDS A RULING, and inventing the English
   * here would be the plain rule breach the whole wording store exists to
   * prevent.
   *
   * So the failure is carried by BEHAVIOUR rather than by a word, and it is
   * still observable in the Standing Rule 1 sense: the two outcomes differ on
   * screen. A popup that opened HIDES the in-page window; a popup that was
   * blocked leaves it exactly where it was, so the reader is looking at their
   * story either way and never at nothing. The reason is named in the console.
   */
  const popOut = useCallback(() => {
    if (win === null) return;
    const size = popoutSize(win);
    const api = (window as unknown as MaybePipWindow).documentPictureInPicture;

    if (supportsDocumentPictureInPicture(window as unknown as MaybePipWindow) && api !== undefined) {
      // ⚑ CALLED HERE, SYNCHRONOUSLY, AND THAT IS THE WHOLE POINT.
      //
      // `requestWindow` needs USER ACTIVATION, which the browser grants to the
      // click's own call stack and to nothing later. The first draft of this
      // called it from an effect in `SubsetPopout` after a `setState`, and the
      // browser refused every time:
      //
      //   NotAllowedError: Document PiP requires user activation
      //
      // Found by clicking the button, not by reading the code — no unit test
      // can see it, because it is a property of the browser and of React's
      // scheduler (StrictMode's double mount spent the activation twice over).
      api
        .requestWindow(size)
        .then((pip) => setPipWindow(pip))
        .catch((err: unknown) => {
          // Not swallowed, and the reader is not stranded: the in-page window
          // was never hidden — `pipWindow` is still null — so the story they
          // clicked ⧉ on is still in front of them.
          console.error("Could not open the timeline story in its own window.", err);
        });
      return;
    }

    const id = win.subsetId;
    if (id === null) return;
    const opened = window.open(subsetPopoutPath(id), `subset-${id}`, popupFeatures(size));
    if (opened === null) {
      // ## ⚑ A BLOCKED POPUP LEAVES THE IN-PAGE WINDOW OPEN, ON PURPOSE
      //
      // There is no stored sentence in this build for "your browser blocked
      // that" — T4.3 asked for the existing one and there is none. The row it
      // needs is listed in the T4 report under NEEDS A RULING, and inventing
      // the English here would be the plain rule breach the whole wording store
      // exists to prevent.
      //
      // So the failure is carried by BEHAVIOUR rather than by a word, and it is
      // still observable in the Standing Rule 1 sense — the two outcomes differ
      // on screen. A popup that opened hides the in-page window; a popup that
      // was blocked leaves it exactly where it was, so the reader is looking at
      // their story either way and never at nothing.
      console.error(
        "The browser blocked the popup for the timeline story " +
          `(subset ${id}). The in-page window is left open.`,
      );
      return;
    }
    setPopupWindow(opened);
  }, [win]);

  /** ⇲, and the OS window's own ×: the story comes back to the page, where it was. */
  const popIn = useCallback(() => {
    setPipWindow((open) => {
      open?.close();
      return null;
    });
    setPopupWindow((open) => {
      open?.close();
      return null;
    });
  }, []);

  /**
   * ⚑ THE POPUP CLOSING HAS TO REACH THIS PROGRAM, and a listener will not do.
   *
   * The picture-in-picture path has `pagehide`, which `SubsetPopout` attaches to
   * a document it is already inside. The popup has no such handle: it is a
   * SEPARATE document that navigates after `window.open` returns, and a listener
   * attached to the handle beforehand does not survive that navigation.
   *
   * Without this, closing the popup from its own × left the in-page window
   * hidden with nothing on screen to bring it back — the reader's only escape
   * was reloading the page, which nothing tells them to do. That is the "two
   * operationally distinct states, one observable" failure Standing Rule 1 is
   * about, and it was found in review rather than by clicking.
   *
   * So: a `POPUP_CLOSED_POLL_MS` check of `closed`, which is the documented
   * same-origin way to learn that a popup went away. It is bounded (it runs only
   * while a popup is recorded as open), it is cheap (a boolean read, no
   * request), and it stops the moment it fires.
   */
  useEffect(() => {
    if (popupWindow === null) return;
    const timer = window.setInterval(() => {
      if (popupWindow.closed) setPopupWindow(null);
    }, POPUP_CLOSED_POLL_MS);
    return () => window.clearInterval(timer);
  }, [popupWindow]);

  // Both windows are OS resources this component owns. Leaving the surface — a
  // route change, a scenario change — must not leave one on the desktop with
  // nothing left to close it.
  useEffect(() => {
    return () => {
      popupWindow?.close();
    };
  }, [popupWindow]);

  /** × on the popped-out bar: put the story away entirely. */
  const closeAll = useCallback(() => {
    popIn();
    setOpen(false);
  }, [popIn]);

  // The window is an OS resource this component owns. Leaving the surface —
  // a route change, a scenario change — must not leave it on the desktop with
  // nothing left to close it.
  useEffect(() => {
    return () => {
      pipWindow?.close();
    };
  }, [pipWindow]);

  if (readError !== null) return <div style={ws.errorState}>{readError}</div>;
  if (data === null) return null;

  const wording = data.wording;
  const ordered = selectorOrder(attached, win?.subsetId ?? null);
  const current = attached.find((s) => s.id === win?.subsetId) ?? attached[0];
  const countLine =
    current === undefined
      ? ""
      : fill(cw(wording, "subsets_window_events_count_template"), {
          count: current.event_count,
        });

  /**
   * The window's contents, wherever the window happens to be.
   *
   * ONE tree for both containers — the in-page `Rnd` and the popped-out
   * document render this same value. Screen 5's own words: "one row design, two
   * containers". A second copy for the popped-out case is how the two would
   * come to disagree about what a row looks like.
   */
  const contents = (
    <>
      {phasesError !== null && <div style={ws.errorState}>{phasesError}</div>}
      {subsetError !== null && <div style={ws.errorState}>{subsetError}</div>}
      {subsetError === null && subset === null && (
        <div style={ws.state}>{cw(wording, "subsets_window_loading_label")}</div>
      )}
      {subset !== null && (
        <SubsetWindowBody
          subset={subset}
          phases={phases}
          tags={tags}
          wording={wording}
          onOpenTimeline={() => window.open(timelinePath(), "_blank", "noopener")}
          onEditSubset={() => window.open(timelinePath(), "_blank", "noopener")}
          onOpenEvent={(id) => window.open(timelineEventPath(id), "_blank", "noopener")}
        />
      )}
    </>
  );

  return (
    <div ref={host}>
      {/* ⚑ THE BUTTON hides on `[]`; THE ROW DOES NOT.
          Hiding the whole dock when nothing is attached was the first reading
          of "hidden on []", and it made the feature unreachable: the Attach
          control lives on the row, so a scenario carrying no story could never
          be given its first one. The button is what has nothing to open. */}
      {attached.length > 0 && (
        <button type="button" style={d.button} onClick={openWindow}>
          {cw(wording, "scenario_view_timeline_button")}
        </button>
      )}

      <ScenarioTimelineRow
        slug={slug}
        scenarioId={scenarioId}
        attached={attached}
        wording={wording}
        onChanged={(next) => setData({ subsets: next, wording })}
      />

      {/* The in-page window. A module of its own: a draggable, resizable,
          minimizable shell is a different concern from the button that opens
          it, and this file was over Rule 17's 300 lines with both in it. */}
      {open && pipWindow === null && popupWindow === null && win !== null && current !== undefined && (
        <SubsetFloatingWindow
          win={win}
          current={current}
          ordered={ordered}
          countLine={countLine}
          wording={wording}
          canPopOut={subset !== null}
          onPersist={persist}
          onPopOut={popOut}
          onClose={() => setOpen(false)}
        >
          {contents}
        </SubsetFloatingWindow>
      )}

      {/* The same story, in its own OS window — the window `popOut` opened.
          Rendered only on the API path: the popup fallback is a whole second
          DOCUMENT at `/timeline/subsets/:id/popout` and has nothing to portal. */}
      {open && pipWindow !== null && current !== undefined && (
        <SubsetPopout
          pipWindow={pipWindow}
          title={current.name}
          count={countLine}
          wording={wording}
          onPopIn={popIn}
          onClose={closeAll}
        >
          {contents}
        </SubsetPopout>
      )}
    </div>
  );
};

export default ScenarioTimelineDock;
