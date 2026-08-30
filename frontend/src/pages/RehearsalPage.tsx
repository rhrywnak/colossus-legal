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
//
// ## NOTHING IS EVER SHOWN THAT MARIE DID NOT PICK (Roman, 2026-08-10)
//
// Two rules arrived together in .390, and they are the same rule seen from two
// sides. Both replace behaviour that was here and looked reasonable.
//
//   The bare address `/cases/:slug/rehearsal` used to open on the FIRST ready
//   scenario and say nothing about having chosen. It renders a LIST now, and
//   waits. On a case with one ready scenario that is a list of one — still a
//   pick, because "the only one" and "the one you chose" stop being the same
//   sentence the moment a second scenario is declared ready.
//
//   A code the payload does not contain used to render the not-ready sentence
//   BESIDE the first ready scenario's blocks. It renders the sentence INSTEAD
//   of them now. The old arrangement was the worse of the two failures it was
//   made of: a refusal that nonetheless produced content, under another
//   scenario's title.
//
// What is left of the carousel: ‹ Back / Next › between ready scenarios, once
// Marie has picked one. Moving after a choice is not the same as being placed
// somewhere without one.

import React, { useCallback, useEffect, useMemo, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";

import RehearsalPageHeader from "../components/RehearsalPageHeader";
import RehearsalPicker from "../components/RehearsalPicker";
import ScenarioTimelineDock from "../components/scenario-timeline/ScenarioTimelineDock";
import RehearsalScenarioBlocks from "../components/RehearsalScenarioBlocks";
import { fillCode } from "./rehearsalSections";

import {
  alwaysLabelStyle,
  alwaysRulesStyle,
  alwaysSeparatorStyle,
  alwaysStyle,
  pageStyle,
} from "../components/rehearsalStyles";
import { ghostButtonStyle } from "../components/scenarioSectionStyles";

import { fetchRehearsal, type RehearsalPayload } from "../services/rehearsal";


import { positionAt, stepForKey, stepTo, type RehearsalStep } from "./rehearsalNav";
import { rehearsalScenarioPath } from "../utils/routePaths";

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

  const load = useCallback(async () => {
    if (!slug) return;
    setLoading(true);
    try {
      const loaded = await fetchRehearsal(slug);
      setPayload(loaded);
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

  /**
   * Has Marie picked a scenario?
   *
   * The URL is the whole answer, and that is deliberate: a choice that lived in
   * React state would be lost on reload and unsendable to anyone else, which is
   * the property the per-scenario address exists to provide. No code in the
   * address means no pick has been made, whatever the index happens to be.
   */
  const picked = code !== undefined;

  /**
   * Move one scenario, and take the ADDRESS with you.
   *
   * ## Why both the keys and the buttons come through here
   *
   * They did not, until this fix. The keyboard handler updated the URL on every
   * step; the ‹ Back / Next › buttons only moved the index and left the address
   * behind. So arrow-keying to S-3 produced a page you could send someone, and
   * clicking Next to S-3 produced one you could not — the same position, two
   * different degrees of linkable, decided by how the reader happened to get
   * there. One mover, one rule, and the paper cut is gone.
   *
   * `replace` rather than push: a rehearsal is worked through in one pass, and
   * filling history with every step would make the browser's Back button mean
   * "one scenario ago" instead of "out of here" — which is the one thing a
   * witness under stress needs it to mean.
   *
   * ## Why `index` is read directly rather than through a functional update
   *
   * Navigating is a side effect, and a `setIndex(current => …)` updater is not
   * the place for one — React may invoke an updater more than once. Reading
   * `index` from the render it belongs to keeps the effect beside the state
   * change instead of inside it; the cost is that this callback (and the key
   * listener below) is rebuilt when the index moves, which is exactly when the
   * rebuilt version is the correct one.
   */
  const move = useCallback(
    (step: RehearsalStep) => {
      const next = stepTo(index, total, step);
      setIndex(next);
      const moved = payload?.scenarios[next];
      if (moved && slug) {
        navigate(rehearsalScenarioPath(slug, moved.code), { replace: true });
      }
    },
    [index, total, payload, slug, navigate],
  );

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
      move(step);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [move]);

  if (loading) {
    return <div style={pageStyle} data-surface="v3">{LOADING}</div>;
  }

  if (error || !payload) {
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
  const notReady = picked && addressed < 0;

  // What the page is FOR, this render. Exactly one of the three is true, which is
  // what stops the .389 arrangement where a refusal and a scenario rendered
  // together:
  //
  //   refusing — a code was given and no ready scenario answers to it
  //   picking  — no code was given; the list is the page
  //   rehearsing — a code was given and it resolved
  //
  // Written as one expression rather than three scattered conditions so the
  // exclusivity is visible in one place and cannot drift back apart.
  const mode = notReady ? "refusing" : picked ? "rehearsing" : "picking";

  return (
    <div style={pageStyle} data-surface="v3">
      <RehearsalPageHeader
        slug={slug ?? ""}
        wording={w}
        // The header names the scenario ONLY while one is being rehearsed. In the
        // other two modes there is no scenario on screen, and a breadcrumb or a
        // "Scenario page ↗" pointing at one the reader never chose is precisely
        // how the .389 round trip ended up at S-2 twice.
        scenario={mode === "rehearsing" ? scenario : undefined}
        position={mode === "rehearsing" ? positionAt(index, payload.positions) : null}
        onPrevious={() => move("previous")}
        onNext={() => move("next")}
        atFirst={index === 0}
        atLast={index >= total - 1}
      />

      {/* Mockup Screen 1's button, and the window it opens. Gated on
          `rehearsing` for exactly the reason the header above is: in the other
          two modes there is no scenario on screen, and a View Timeline button
          opening one the reader never chose is the .389 round trip again.
          Self-contained otherwise — it fetches its own data and hides itself
          when this scenario carries no subset. */}
      {mode === "rehearsing" && scenario !== undefined && (
        <ScenarioTimelineDock slug={slug ?? ""} scenarioId={scenario.scenario_id} />
      )}

      {mode === "refusing" && (
        <p role="status" style={{ marginTop: "18px" }}>
          {fillCode(w.not_ready_notice, code)}
        </p>
      )}

      {/* THE FRONT DOOR (Roman's ruling, 2026-08-10). No code in the address
          means nobody has picked, so the page offers the ready scenarios and
          waits. An empty rehearsal is a REAL state, not a failure — nobody has
          declared a scenario ready yet — and the stored sentence says what to do
          about it. */}
      {mode === "picking" && (
        <RehearsalPicker
          slug={slug ?? ""}
          heading={w.picker_heading}
          emptyNotice={w.nothing_ready_notice}
          scenarios={payload.scenarios}
        />
      )}

      {/* Rehearsing. `edits` is `null` only when the slug is missing, which the
          router cannot produce — but it gates the blocks rather than being
          asserted away, because a page that edits must never render controls it
          has no write path for. */}
      {mode === "rehearsing" &&
        (!scenario ? (
          <p style={{ fontSize: "17px", marginTop: "24px" }}>{w.nothing_ready_notice}</p>
        ) : (
          <RehearsalScenarioBlocks scenario={scenario} wording={w} />
        ))}

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
