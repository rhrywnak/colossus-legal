// =============================================================================
// RehearsalPage — where Marie rehearses, and where she can prepare
// =============================================================================
//
// One READY scenario per screen: what it is, the accusation and every time they
// made it, the timeline, her points, what to watch for — plus the standing strip,
// always visible and never collapsible.
//
// Rebuilt in task 2.11 C to reproduce REHEARSAL_PAGE_MOCKUP_v2_2026-08-06.html,
// the signed visual spec. What changed and what did not:
//
//   changed — the whole render: the bordered section cards, the header controls,
//             the breadcrumb, the drawn timeline, the compact/expanded rows, the
//             slim Always strip, and the ability to EDIT from this page.
//   did not — every law. Every word is still a settings row; every absence is
//             still a named gap; every count still arrives composed; the ready
//             gate is still the server's.
//
// ## The scale, and the three doc comments that used to argue otherwise
//
// B2 argued in three places that this surface departs from §2c with a LARGER
// type scale. The signed mockup is §2c scale, and the mockup supersedes (ruling
// C6). Those comments were corrected with this rebuild rather than left arguing
// the opposite of what ships.
//
// ## Editing here is REUSE, not a second write path
//
// Every act on this page calls the SAME guarded route the working view calls:
// `PUT …/accusation` for the plain-words sentence, `PUT /scenarios/:id` (partial
// body, theme only) for "What this is", and the talking-point and watch-item
// routes for those two lists. The scenario's id travels on the payload for that
// reason and renders nowhere (ruling C1).
//
// ## The per-scenario address
//
// `/cases/:slug/rehearsal/:code` selects within the payload this page already
// loaded. A code that is not in it — because nobody declared that scenario ready
// — gets the stored not-ready sentence. NOT a 404: the address is right and the
// scenario simply is not ready. NOT a leak: the payload never contained it.

import React, { useCallback, useEffect, useMemo, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";

import RehearsalPageHeader from "../components/RehearsalPageHeader";
import RehearsalScenarioBlocks, {
  type RehearsalEdits,
} from "../components/RehearsalScenarioBlocks";
import {
  alwaysLabelStyle,
  alwaysRulesStyle,
  alwaysSeparatorStyle,
  alwaysStyle,
  pageStyle,
} from "../components/rehearsalStyles";
import { ghostButtonStyle } from "../components/scenarioSectionStyles";
import { fillDetail } from "../services/scenarioAccusation";
import { fetchRehearsal, type RehearsalPayload } from "../services/rehearsal";
import { rehearsalEdits } from "./rehearsalEdits";
import { fillCode, openSectionsFrom, type OpenSections } from "./rehearsalSections";
import { positionAt, stepForKey, stepTo } from "./rehearsalNav";

// CONST: the three strings that describe the absence of a payload, and therefore
// cannot come from one. Every other word on this page is a settings row.
const LOADING = "Loading rehearsal mode…";
const LOAD_FAILED = "Rehearsal mode did not load.";
const RETRY = "Try again";

const RehearsalPage: React.FC = () => {
  const { slug, code } = useParams<{ slug: string; code?: string }>();
  const navigate = useNavigate();

  const [payload, setPayload] = useState<RehearsalPayload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [index, setIndex] = useState(0);
  const [open, setOpen] = useState<OpenSections | null>(null);
  const [openRows, setOpenRows] = useState<ReadonlySet<number>>(new Set());
  const [busy, setBusy] = useState(false);
  const [writeError, setWriteError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!slug) return;
    setLoading(true);
    try {
      const loaded = await fetchRehearsal(slug);
      setPayload(loaded);
      // The default states are the SERVER's, parsed from the store at boot where a
      // typo is a named refusal. This page receives the decided answer.
      setOpen(openSectionsFrom(loaded.collapse));
      // Clamp rather than reset: a reload after one scenario was demoted should
      // keep Marie roughly where she was, not send her back to the start.
      setIndex((current) => stepTo(current, loaded.scenarios.length, null));
      setError(null);
    } catch (e: unknown) {
      // Explicit error UI, never a swallowed rejection (Standing Rule 1).
      setError(e instanceof Error ? e.message : LOAD_FAILED);
    } finally {
      setLoading(false);
    }
  }, [slug]);

  useEffect(() => {
    void load();
  }, [load]);

  const total = payload?.scenarios.length ?? 0;
  const scenario = payload?.scenarios[index];

  /**
   * Which rows arrive open, whenever the scenario on screen changes.
   *
   * The SERVER decided (`instances_start_expanded`, from the stored cap and the
   * count it just rendered). Recomputing it here from a number would be a second
   * implementation of one rule; this only spreads the decided answer across the
   * rows it applies to.
   */
  useEffect(() => {
    if (!scenario) return;
    setOpenRows(
      scenario.instances_start_expanded
        ? new Set(scenario.accusation.instances.map((i) => i.position))
        : new Set(),
    );
  }, [scenario]);

  /** Where the address points, when it names a scenario. */
  const addressed = useMemo(
    () => payload?.scenarios.findIndex((s) => s.code === code) ?? -1,
    [payload, code],
  );

  // An address that names a READY scenario moves the reader to it. One that names
  // a scenario nobody declared ready falls through to the notice below — the page
  // never navigates somewhere it cannot show.
  useEffect(() => {
    if (addressed >= 0) setIndex(addressed);
  }, [addressed]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      // A keystroke inside a box a human is typing in is text, not navigation.
      const target = event.target as HTMLElement | null;
      const tag = target?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || target?.isContentEditable) return;

      const step = stepForKey(event.key);
      if (step === null) return;
      // Space would otherwise scroll the page out from under the reader.
      event.preventDefault();
      setIndex((current) => {
        const next = stepTo(current, total, step);
        // The address follows the reader, so a scenario can be linked to and
        // returned to. `replace` rather than push: a rehearsal is worked through
        // in one pass, and filling the back button with every step would make the
        // browser's Back mean "one scenario ago" instead of "out of here".
        const moved = payload?.scenarios[next];
        if (moved && slug) {
          navigate(`/cases/${slug}/rehearsal/${moved.code}`, { replace: true });
        }
        return next;
      });
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [total, payload, slug, navigate]);

  const toggle = (section: keyof OpenSections) =>
    setOpen((current) => (current ? { ...current, [section]: !current[section] } : current));

  const setAll = (value: boolean) =>
    setOpen({
      what: value,
      accusation: value,
      timeline: value,
      points: value,
      watchFor: value,
    });

  const toggleRow = (position: number) =>
    setOpenRows((current) => {
      const next = new Set(current);
      if (!next.delete(position)) next.add(position);
      return next;
    });

  /** The prep list's jump: open the row, then bring it into view. */
  const jumpToRow = (position: number) => {
    if (!scenario) return;
    setOpenRows((current) => new Set(current).add(position));
    // After the row has been told to open, so the scroll lands on the expanded
    // height rather than the one-line summary the reader came to leave.
    window.requestAnimationFrame(() => {
      document
        .getElementById(`${scenario.code}-row-${position}`)
        ?.scrollIntoView({ behavior: "smooth", block: "center" });
    });
  };

  /**
   * Run one write, report any failure, re-read, and RETURN what went wrong.
   *
   * Server state is the one source of truth here, and it has to be: the counts,
   * the gaps and the answer-present flags are all DERIVED from what is stored,
   * and a browser that painted a change before the write landed would be showing
   * a count it had computed itself — the exact thing §10 forbids.
   *
   * ## Why the failure is RETURNED and not thrown
   *
   * Two callers need two things from it. The sentence editors have no failure
   * surface of their own and want nothing back — the banner this sets is the
   * whole report. The per-row list editors need a REJECTED promise, because
   * that is what keeps a human's draft on screen instead of closing the box on
   * a save that never happened.
   *
   * Throwing here would have served the second and forced the first to write
   * `.catch(() => undefined)` — a catch that displays nothing, which reads as
   * swallowed however carefully it is commented. Returning the failure means
   * every caller decides deliberately, and there is no discarded rejection
   * anywhere on this page.
   */
  const runWrite = useCallback(
    async (write: () => Promise<void>): Promise<unknown> => {
      setBusy(true);
      try {
        await write();
        setWriteError(null);
        await load();
        return null;
      } catch (e: unknown) {
        // Explicit error UI, never a swallowed rejection — and a WRITE failure
        // says so in the stored save-failure sentence, not in the load failure's
        // words. Two operationally different states, two observables
        // (Standing Rule 1). The template is the human's; `{detail}` is the one
        // value only this side knows.
        const detail = e instanceof Error ? e.message : String(e);
        const template = payload?.wording.editor.save_failed_template;
        setWriteError(template ? fillDetail(template, detail) : detail);
        return e;
      } finally {
        setBusy(false);
      }
    },
    [load, payload],
  );

  if (loading) {
    return <div style={pageStyle} data-surface="v3">{LOADING}</div>;
  }

  if (error || !payload || !open) {
    return (
      <div style={pageStyle} data-surface="v3">
        <div role="alert" style={{ color: "var(--state-danger-strong)" }}>
          {error ?? LOAD_FAILED}
        </div>
        <button
          type="button"
          onClick={() => void load()}
          style={{ ...ghostButtonStyle, marginTop: "1rem" }}
        >
          {RETRY}
        </button>
      </div>
    );
  }

  const w = payload.wording;
  // The address named something this page is not showing. It exists or it does
  // not; either way it is not ready, and that is all this page may say about it.
  const notReady = code !== undefined && addressed < 0;

  const edits: RehearsalEdits | null =
    scenario && slug ? rehearsalEdits(slug, scenario, runWrite, busy) : null;

  return (
    <div style={pageStyle} data-surface="v3">
      <RehearsalPageHeader
        slug={slug ?? ""}
        wording={w}
        scenario={scenario}
        position={positionAt(index, payload.positions)}
        onPrevious={() => setIndex(stepTo(index, total, "previous"))}
        onNext={() => setIndex(stepTo(index, total, "next"))}
        atFirst={index === 0}
        atLast={index >= total - 1}
        onOpenAll={() => setAll(true)}
        onFoldAll={() => setAll(false)}
      />

      {notReady && (
        <p role="status" style={{ marginTop: "18px" }}>
          {fillCode(w.not_ready_notice, code)}
        </p>
      )}

      {/* A write that failed says so at the top of the page as well as at the row
          it came from: the row may have scrolled out of view by the time the
          answer arrives. */}
      {writeError && (
        <div
          role="alert"
          style={{ marginTop: "12px", color: "var(--v3-red-text)", fontSize: "13px" }}
        >
          {writeError}
        </div>
      )}

      {/* An empty rehearsal is a REAL state, not a failure: nobody has declared a
          scenario ready yet. The stored sentence says what to do about it. */}
      {!scenario || !edits ? (
        <p style={{ fontSize: "17px", marginTop: "24px" }}>{w.nothing_ready_notice}</p>
      ) : (
        <RehearsalScenarioBlocks
          scenario={scenario}
          wording={w}
          open={open}
          onToggle={toggle}
          openRows={openRows}
          onToggleRow={toggleRow}
          onJumpToRow={jumpToRow}
          edits={edits}
        />
      )}

      {/* The standing strip. Always visible, on every screen including the empty
          one, and the ONE block with no fold to close — §10 makes it the thing
          that is never scrolled away from. Its lines are backend-composed; this
          renders them verbatim, with only the separator supplied here. */}
      <div style={alwaysStyle}>
        <span style={alwaysLabelStyle}>{payload.always.heading}</span>
        <span style={alwaysRulesStyle}>
          {payload.always.lines.map((line, i) => (
            <React.Fragment key={i}>
              {i > 0 && <span style={alwaysSeparatorStyle}>·</span>}
              {line}
            </React.Fragment>
          ))}
        </span>
      </div>
    </div>
  );
};

export default RehearsalPage;
