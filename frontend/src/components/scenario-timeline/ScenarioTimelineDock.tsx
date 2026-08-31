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
import { createPortal } from "react-dom";
import { Rnd } from "react-rnd";

import { cw, fill, type TimelinePhase } from "../../services/caseTimeline";
import { getCaseTimeline } from "../../services/caseTimeline";
import { getSubset, type SubsetDetail } from "../../services/caseTimelineSubsets";
import * as d from "./dockStyles";
import * as ws from "./windowStyles";
import {
  type AttachedSubset,
  getScenarioSubsets,
  type ScenarioSubsets,
} from "./scenarioTimeline";
import ScenarioTimelineRow from "./ScenarioTimelineRow";
import SubsetWindowBody from "./SubsetWindowBody";
import {
  clampToViewport,
  decodeWindowState,
  encodeWindowState,
  initialSubsetId,
  MIN_HEIGHT,
  MIN_WIDTH,
  minimizedPosition,
  openStateFor,
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
  const [phasesError, setPhasesError] = useState<string | null>(null);
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
    const stored = readStored(scenarioId);
    const chosen = initialSubsetId(attached, stored?.subsetId ?? null);
    // A stored position always wins — clamped, because the screen it was stored
    // on may be gone. Otherwise the §10 rule decides, from the room actually
    // beside the content column.
    const box = host.current?.getBoundingClientRect();
    const contentRight = box === undefined ? window.innerWidth : box.right;
    const next =
      stored === null
        ? openStateFor(window.innerWidth, window.innerHeight, contentRight, chosen)
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

  // The phases, for the dividers and the dots. The same payload the timeline
  // page reads — one request, cached by the browser, and no second opinion
  // about what colour `probate` is.
  useEffect(() => {
    if (!open || phases.length > 0) return;
    let cancelled = false;
    getCaseTimeline()
      .then((t) => {
        if (!cancelled) setPhases(t.phases);
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

  if (readError !== null) return <div style={ws.errorState}>{readError}</div>;
  if (data === null) return null;

  const wording = data.wording;
  const ordered = selectorOrder(attached, win?.subsetId ?? null);
  const current = attached.find((s) => s.id === win?.subsetId) ?? attached[0];

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

      {/* ⚑ A PORTAL TO `body`, and it is not decoration.
          `Rnd` places its element relative to the nearest POSITIONED ancestor,
          and every one of the five surfaces wraps this dock in its own laid-out
          page. The open-position rule computes VIEWPORT coordinates — "is there
          460px of free width beside the content column" is a question about the
          screen, not about a card — so the two disagreed and the window landed
          in the middle of the page instead of in the right margin. Portalling to
          `body` makes the coordinate space the one the rule is written in, and
          it also puts the window above every page's own stacking context rather
          than inside one. Found by opening it, not by reading it. */}
      {open &&
        win !== null &&
        createPortal(
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
          // Drag by the title bar only (mockup, and §5C): a window draggable by
          // its body cannot have a scrolling body.
          dragHandleClassName="subset-window-bar"
          enableResizing={!win.minimized}
          style={{ zIndex: 40 }}
          onDragStop={(_e, dd) => persist({ ...win, x: dd.x, y: dd.y })}
          onResizeStop={(_e, _dir, ref, _delta, pos) =>
            persist({
              ...win,
              width: ref.offsetWidth,
              height: ref.offsetHeight,
              x: pos.x,
              y: pos.y,
            })
          }
        >
          <div style={win.minimized ? ws.minimizedBar : ws.shell}>
            <div style={win.minimized ? undefined : ws.bar} className="subset-window-bar">
              <span style={ws.barTitle}>{current.name}</span>
              <span style={ws.barCount}>
                {fill(cw(wording, "subsets_window_events_count_template"), {
                  count: current.event_count,
                })}
              </span>
              {ordered.length > 1 && (
                <select
                  style={ws.barSelect}
                  value={current.id}
                  onChange={(e) => persist({ ...win, subsetId: e.target.value })}
                >
                  {ordered.map((s) => (
                    <option key={s.id} value={s.id}>
                      {s.name}
                    </option>
                  ))}
                </select>
              )}
              <span style={ws.barActions}>
                <button
                  type="button"
                  style={ws.barButton}
                  aria-label={cw(wording, "subsets_window_minimize_label")}
                  title={cw(wording, "subsets_window_minimize_label")}
                  onClick={() => persist({ ...win, minimized: !win.minimized })}
                >
                  {win.minimized ? "▢" : "–"}
                </button>
                <button
                  type="button"
                  style={ws.barButton}
                  aria-label={cw(wording, "subsets_window_close_label")}
                  title={cw(wording, "subsets_window_close_label")}
                  onClick={() => setOpen(false)}
                >
                  ×
                </button>
              </span>
            </div>

            {/* A failed phases read is shown, not swallowed: the dividers lose
                their labels and colours, and the reader is told why rather than
                left to wonder at a list of bare slugs. */}
            {!win.minimized && phasesError !== null && (
              <div style={ws.errorState}>{phasesError}</div>
            )}
            {!win.minimized && subsetError !== null && (
              <div style={ws.errorState}>{subsetError}</div>
            )}
            {/* ⚑ The loading moment now has a word of its own.
                It shipped blank for a day rather than borrowing a wrong one:
                `saving_label` ("Saving…") is a WRITE and would have told a
                reader their work was being written, and `BOOTSTRAP_TEXT.loading`
                does not apply here because that exception exists for the request
                that DELIVERS the wording — by this point the whole block is
                already in hand, so English in code would have been a plain rule
                breach rather than a bootstrap. Ruled and seeded 2026-08-30. */}
            {!win.minimized && subsetError === null && subset === null && (
              <div style={ws.state}>{cw(wording, "subsets_window_loading_label")}</div>
            )}
            {!win.minimized && subset !== null && (
              <SubsetWindowBody
                subset={subset}
                phases={phases}
                wording={wording}
                onOpenTimeline={() => window.open("/timeline", "_blank", "noopener")}
                onEditSubset={() => window.open("/timeline", "_blank", "noopener")}
                onOpenEvent={(id) =>
                  window.open(`/timeline/events/${encodeURIComponent(id)}`, "_blank", "noopener")
                }
              />
            )}
            </div>
          </Rnd>,
          document.body,
        )}
    </div>
  );
};

export default ScenarioTimelineDock;
