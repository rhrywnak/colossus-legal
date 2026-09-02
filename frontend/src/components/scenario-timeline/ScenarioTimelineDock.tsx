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
// ## ⚑ VIEW TIMELINE OPENS THE FLOATING WINDOW (T7, design §12.1)
//
// It did not always. Until T7 the button opened the in-page window and the
// floating one hid behind a 12-px ⧉ in that window's title bar — "what the
// heck do I need to push a button that is almost impossible to see to get a
// floating popup?" (Roman, on beta.419). The main event does not hide behind a
// glyph, so the click now takes the pop-out path FIRST and falls back down
// `popout.ts`'s chain: picture-in-picture, then a popup, then the page. Both
// directions still work — ⇲ docks it, ⧉ pops it out again.
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
  containerForOutcome,
  firstPopoutRung,
  type MaybePipWindow,
  popoutSize,
  popupFeatures,
  type SubsetContainer,
} from "./popout";
import {
  type AttachedSubset,
  getScenarioSubsets,
  type ScenarioSubsets,
} from "./scenarioTimeline";
import SubsetFloatingWindow from "./SubsetFloatingWindow";
import SubsetPopout from "./SubsetPopout";
import SubsetWindowBody from "./SubsetWindowBody";
import {
  clampToViewport,
  decodeWindowState,
  encodeWindowState,
  initialSubsetId,
  namedSubset,
  openStateFor,
  previewWindowState,
  POPUP_CLOSED_POLL_MS,
  selectorOrder,
  type WindowState,
  windowStorageKey,
} from "./subsetWindow";

type Props = {
  slug: string;
  scenarioId: string;
  /**
   * Open the window on THIS subset, without it being attached (T5, Screen 4).
   *
   * The Timeline-subsets section's Preview: a reader deciding whether to carry a
   * story should be able to read it first, and the alternative was attach, look,
   * detach — two writes to answer a question. `null` is the ordinary mount.
   */
  previewSubsetId?: string | null;
  /** Preview mode hides the button; the section owns opening and closing. */
  onPreviewClosed?: () => void;
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

const ScenarioTimelineDock: React.FC<Props> = ({
  slug,
  scenarioId,
  previewSubsetId = null,
  onPreviewClosed,
}) => {
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
  /**
   * Which container is showing the story — or `null` while the browser is
   * still deciding.
   *
   * ## ⚑ WHY `null` IS A STATE AND NOT A TIDINESS PROBLEM
   *
   * `requestWindow` returns a PROMISE. Between the click and its answer there
   * is a real moment, and the two things this component could do in it are not
   * equivalent: render the in-page window (which then vanishes a beat later as
   * the OS window arrives — a flash of the very thing T7 exists to stop being
   * the default), or render nothing and let the floating window be the first
   * thing the reader sees. `null` is the second, and it is deliberately NOT
   * the same value as "the story is in the page".
   */
  const [container, setContainer] = useState<SubsetContainer | null>("inpage");
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

  /**
   * The pop-out attempt itself — the chain in `popout.ts`, acted on.
   *
   * ## ⚑ CALLED FROM A CLICK HANDLER, SYNCHRONOUSLY, ALWAYS
   *
   * `requestWindow` needs USER ACTIVATION, which the browser grants to the
   * click's own call stack and to nothing later. T4's first build called it
   * from a `useEffect` after a `setState` and the browser refused every time:
   *
   *   NotAllowedError: Document PiP requires user activation
   *
   * — twice over, because StrictMode's double mount spent the activation on
   * the first invocation. So this takes the window state as an ARGUMENT rather
   * than reading `win` from state: both callers know the state at click time,
   * and neither has to wait for a render to have committed before asking. Two
   * handlers call it, `viewTimeline` and `popOut`, and no effect does. There is
   * a test that reads this file and asserts exactly that.
   *
   * @param state where and how big the window should be — the reader's own
   *              remembered geometry, which is what makes one click enough.
   */
  const floatOut = useCallback((state: WindowState) => {
    const size = popoutSize(state);
    const host = window as unknown as MaybePipWindow;

    if (firstPopoutRung(host) === "pip") {
      const api = host.documentPictureInPicture;
      if (api === undefined) {
        // Unreachable: `firstPopoutRung` answers "pip" only when
        // `requestWindow` is a callable on this very object. It is written out
        // because `tsc` cannot know that, and because the alternative — a `!`
        // — would turn a refuted impossibility into a runtime throw the day
        // the predicate and this branch drift apart.
        setContainer("inpage");
        return;
      }
      api
        .requestWindow(size)
        .then((pip) => {
          setPipWindow(pip);
          setContainer(containerForOutcome({ attempted: "pip", granted: true }));
        })
        .catch((err: unknown) => {
          // Not swallowed, and the reader is not stranded: the resolver's last
          // rung puts the story in the page. Silent in the design's sense —
          // no banner for a browser's private refusal — and observable in
          // Standing Rule 1's sense, in the console and on screen.
          console.error("Could not open the timeline story in its own window.", err);
          setContainer(containerForOutcome({ attempted: "pip", granted: false }));
        });
      return;
    }

    const id = state.subsetId;
    if (id === null) {
      // No subset to address means no popup URL to open. The in-page window
      // needs no id — it renders whatever `namedSubset` resolves — so the
      // reader still gets a window rather than a dead click.
      setContainer("inpage");
      return;
    }
    const opened = window.open(subsetPopoutPath(id), `subset-${id}`, popupFeatures(size));
    if (opened === null) {
      // ⚑ A BLOCKED POPUP IS THE THIRD RUNG, NOT A DEAD END.
      //
      // There is still no stored sentence in this build for "your browser
      // blocked that" — T4.3 asked for the existing one and there is none, and
      // inventing the English here would be the plain rule breach the wording
      // store exists to prevent. So the failure is carried by BEHAVIOUR: the
      // story opens in the page instead, which is a different screen from the
      // one a granted popup produces, and the reason is named in the console.
      console.error(
        "The browser blocked the popup for the timeline story " +
          `(subset ${id}). The story opens in the page instead.`,
      );
      setContainer(containerForOutcome({ attempted: "popup", granted: false }));
      return;
    }
    setPopupWindow(opened);
    setContainer(containerForOutcome({ attempted: "popup", granted: true }));
  }, []);

  /**
   * `View Timeline` — the front door, and after T7 it opens the OS window.
   *
   * ⚑ ALREADY FLOATING? BRING IT FORWARD, DO NOT RE-OPEN IT.
   *
   * A second click while the window is up used to close it and put the story
   * back in the page, which made sense when the in-page window was what the
   * button meant. It no longer is. Closing and re-requesting would also fire
   * the old window's `pagehide` — the same event the reader's own × uses — and
   * the dock cannot tell those apart, so the new window would arrive to a dock
   * that had just been told the reader put the story away. `focus()` is both
   * the honest behaviour and the one with no race in it.
   *
   * A handle whose window is already `closed` is not "floating": that is the
   * stale-handle case the old comment worried about, and it falls through to
   * the chain below so the button always produces something visible.
   */
  const viewTimeline = useCallback(() => {
    const floating = pipWindow ?? popupWindow;
    if (floating !== null && !floating.closed) {
      floating.focus();
      return;
    }
    // Stale handles, dropped rather than closed — see above.
    setPipWindow(null);
    setPopupWindow(null);

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
    // Nothing on screen until the browser answers — see `container`.
    setContainer(null);
    floatOut(next);
  }, [attached, floatOut, pipWindow, popupWindow, scenarioId]);

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

  /**
   * PREVIEW (T5, Screen 4): open straight onto one subset, attached or not.
   *
   * A separate effect and not a branch inside `openWindow`, because the trigger
   * is different in kind: the reader clicked Preview on a row in another
   * component, so there is no click here to hang the open on. Where it opens is
   * `previewWindowState`, which a test can reach.
   */
  useEffect(() => {
    if (previewSubsetId === null) return;
    const header = document.querySelector("header[data-app-chrome]");
    const headerBottom = header === null ? 0 : header.getBoundingClientRect().bottom;
    setWin(previewWindowState(previewSubsetId, window.innerWidth, window.innerHeight, headerBottom));
    setOpen(true);
    // ⚑ PREVIEW STAYS IN THE PAGE, and T7 does not change that. There is no
    // click in THIS component to hang a pop-out on — the reader clicked
    // Preview in another one — and `requestWindow` from an effect is the exact
    // call that fails with `NotAllowedError` (design §12.2). A Preview that
    // floats would need the pop-out attempt moved into that section's own
    // handler, which is a surface T7's diff does not touch.
    setContainer("inpage");
  }, [previewSubsetId]);

  const persist = useCallback(
    (next: WindowState) => {
      setWin(next);
      writeStored(scenarioId, next);
    },
    [scenarioId],
  );

  /**
   * ⧉ in the docked window — the story leaves the page again.
   *
   * T7.2: neither direction is removed. This is now the SECOND way to float a
   * window rather than the only one, and it is the same chain `viewTimeline`
   * takes, so a browser that falls back for one falls back for both.
   *
   * The docked window stays on screen while the request is outstanding — the
   * container is left alone here, unlike in `viewTimeline` — so a rejection
   * leaves the reader looking at the story they already had rather than at a
   * blink of nothing.
   */
  const popOut = useCallback(() => {
    if (win === null) return;
    floatOut(win);
  }, [floatOut, win]);

  /** ⇲ — `Back into the page`: the story comes back to the page, where it was. */
  const popIn = useCallback(() => {
    setPipWindow((prev) => {
      prev?.close();
      return null;
    });
    setPopupWindow((prev) => {
      prev?.close();
      return null;
    });
    setContainer("inpage");
  }, []);

  /**
   * Close the window entirely — the × on either bar.
   *
   * `onPreviewClosed` fires with it so a PREVIEW's owner learns the reader
   * dismissed the window. Without that the section would still be holding the
   * previewed id, and clicking Preview on the same row again would change
   * nothing — a button that works once, which is the failure class T4's
   * follow-up spent a commit on.
   */
  const closeWindow = useCallback(() => {
    setOpen(false);
    onPreviewClosed?.();
  }, [onPreviewClosed]);

  /**
   * The floating window went away by the READER's hand.
   *
   * ## ⚑ AFTER T7 THIS CLOSES THE STORY; BEFORE T7 IT DOCKED IT
   *
   * T7.2, in the instruction's own words: "closing the floating window from
   * the OS leaves the reader on the page with no window, which is correct:
   * they closed it." Until T7 the same event ran `popIn`, and re-showing the
   * in-page window was right then — the floating window was an extra the
   * reader had opted into, so taking it away returned them to the default.
   * The floating window IS the default now, and re-opening a docked copy of a
   * story someone just closed would be the program arguing with them.
   *
   * ⚑ It closes NOTHING. Both callers report a window that is already gone —
   * `pagehide` on the picture-in-picture document, and the popup poll below —
   * so a `close()` here would only be capable of firing `pagehide` again.
   */
  const floatingClosed = useCallback(() => {
    setPipWindow(null);
    setPopupWindow(null);
    setContainer("inpage");
    closeWindow();
  }, [closeWindow]);

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
      // T7.2 read across to the popup rung: on Safari and Firefox THIS is the
      // window `View Timeline` opens, so closing it means the same thing as
      // closing the picture-in-picture one — the reader put the story away.
      if (popupWindow.closed) floatingClosed();
    }, POPUP_CLOSED_POLL_MS);
    return () => window.clearInterval(timer);
  }, [floatingClosed, popupWindow]);

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
    closeWindow();
  }, [popIn, closeWindow]);

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
  // Which subset the bar names — see `namedSubset`, which the preview path broke
  // once already and which is now decided where a test can reach it.
  const current = namedSubset(previewSubsetId, subset, attached, win?.subsetId ?? null);
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
      {/* ⚑ THE BUTTON IS ABSENT ON `[]`, NOT DISABLED — Screen 1's own words:
          "when no subset is attached the button is simply absent — nothing else
          shifts". A disabled button would offer something that does not exist.

          ⚑ AND THE "Timeline: [chips] Attach…" ROW IS GONE.
          It used to render below this button on all five surfaces, and it WAS
          defect D6 — an attach control on every view page, which is editing
          done from a reading surface. T5 moves attaching to its own section
          (`ScenarioSubsetsSection`) and `ScenarioTimelineRow.tsx` is deleted;
          the two wording rows it spoke are retired in T5's migration. The dock
          now keeps only the button and the window. */}
      {attached.length > 0 && previewSubsetId === null && (
        <button type="button" style={d.button} onClick={viewTimeline}>
          {cw(wording, "scenario_view_timeline_button")}
        </button>
      )}

      {/* The in-page window. A module of its own: a draggable, resizable,
          minimizable shell is a different concern from the button that opens
          it, and this file was over Rule 17's 300 lines with both in it. */}
      {open && container === "inpage" && win !== null && current !== undefined && (
        <SubsetFloatingWindow
          win={win}
          current={current}
          ordered={previewSubsetId === null ? ordered : []}
          countLine={countLine}
          wording={wording}
          canPopOut={subset !== null}
          onPersist={persist}
          onPopOut={popOut}
          onClose={closeWindow}
        >
          {contents}
        </SubsetFloatingWindow>
      )}

      {/* The same story, in its own OS window — the window the click opened.
          Rendered only on the API path: the popup fallback is a whole second
          DOCUMENT at `/timeline/subsets/:id/popout` and has nothing to portal,
          which is why `container === "popup"` draws nothing here and is not a
          missing branch. */}
      {open && container === "pip" && pipWindow !== null && current !== undefined && (
        <SubsetPopout
          pipWindow={pipWindow}
          title={current.name}
          count={countLine}
          wording={wording}
          onPopIn={popIn}
          onWindowClosed={floatingClosed}
          onClose={closeAll}
        >
          {contents}
        </SubsetPopout>
      )}
    </div>
  );
};

export default ScenarioTimelineDock;
