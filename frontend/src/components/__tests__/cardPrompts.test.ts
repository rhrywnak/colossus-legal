// =============================================================================
// cardPrompts.test.ts — the two prompts, and the stance that is no longer a const
// =============================================================================
//
// CC_TASK_CARDTRIAGE_SPLIT_v1. These suites MOVED here from `cardTriage.test.ts`
// with the machines they exercise. They are unchanged except for two things,
// both of which are the point of the change:
//
//   * the imports, and
//   * `cards_loaded` now carries the Include picker's opening stance.
//
// Everything else is byte-for-byte what it was, which is how "behaviour
// preserved" is held to something here rather than asserted in a report.
//
// ## What did NOT move, and why
//
// `describe("the defer_required short-circuit")` stayed in `cardTriage.test.ts`.
// It looks like a defer test and is not: it asserts that the queue DOES NOT open
// the prompt when the server has already supplied the reason. That decision is
// `rulingOn`'s, which stayed — and a test that proves a prompt never opened
// belongs with the code that declined to open it.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { DEFER_QUICK_REASONS } from "../cardPrompts";
import { initialQueueState, progress, queueReducer, type QueueState } from "../cardTriage";
import type { CardFactStance, ScenarioCard } from "../../services/scenarioCards";
import { fullCard } from "./cardFixtures";

function stateOf(cards: ScenarioCard[]): QueueState {
  return initialQueueState(cards);
}

/** Press a key in triage mode (not typing into a field). */
function press(state: QueueState, key: string, typing = false) {
  return queueReducer(state, { type: "key", key, typing });
}

/**
 * A queue that has LOADED — which is the only way the stance reaches the state.
 *
 * `initialQueueState` starts on `supports` and nothing may read it (see that
 * function's doc); every test that cares about the stance goes through here.
 */
function loaded(cards: ScenarioCard[], includeDefaultStance: CardFactStance = "supports") {
  return queueReducer(stateOf([]), { type: "cards_loaded", cards, includeDefaultStance }).state;
}

describe("defer", () => {
  it("D opens the prompt on an ordinary card", () => {
    const { state, effect } = press(stateOf([fullCard()]), "d");
    // The prompt remembers the CARD it was opened on (1.7G) as well as the draft.
    expect(state.mode).toEqual({ kind: "deferring", draft: "", graphNodeId: "ev-1" });
    expect(effect).toEqual({ kind: "none" });
  });

  it("a digit picks a quick reason without leaving the keyboard", () => {
    let s = press(stateOf([fullCard()]), "d").state;
    s = press(s, "2").state;
    expect(s.mode).toEqual({
      kind: "deferring",
      draft: DEFER_QUICK_REASONS[1],
      graphNodeId: "ev-1",
    });
  });

  it("Enter commits the drafted reason and closes the prompt", () => {
    let s = press(stateOf([fullCard(), fullCard({ graph_node_id: "ev-2" })]), "d").state;
    s = queueReducer(s, { type: "defer_draft", draft: "waiting on a clean copy" }).state;
    const { state, effect } = press(s, "Enter");

    expect(effect).toEqual({
      kind: "rule",
      graphNodeId: "ev-1",
      action: "defer",
      reason: "waiting on a clean copy",
    });
    expect(state.mode).toEqual({ kind: "triage" });
    expect(state.index).toBe(1);
  });

  it("refuses a blank reason and keeps the prompt open", () => {
    // The backend rejects a reasonless defer; refusing here keeps the human's
    // cursor where it is instead of bouncing an error at them.
    let s = press(stateOf([fullCard()]), "d").state;
    s = queueReducer(s, { type: "defer_draft", draft: "   " }).state;
    const { state, effect } = press(s, "Enter");

    expect(effect).toEqual({ kind: "none" });
    expect(state.mode.kind).toBe("deferring");
  });

  it("Esc cancels without ruling", () => {
    let s = press(stateOf([fullCard()]), "d").state;
    s = queueReducer(s, { type: "defer_draft", draft: "half a thought" }).state;
    const { state, effect } = press(s, "Escape");

    expect(effect).toEqual({ kind: "none" });
    expect(state.mode).toEqual({ kind: "triage" });
    expect(progress(state).ruled).toBe(0);
  });

  it("the prompt owns the keyboard — I does not rule while it is open", () => {
    const s = press(stateOf([fullCard()]), "d").state;
    const { effect } = press(s, "i");
    expect(effect).toEqual({ kind: "none" });
  });
});

describe("Include opens the picker instead of ruling", () => {
  /** A card that bears on one accusation, so the picker has a default. */
  const withAccusation = () =>
    fullCard({
      graph_node_id: "ev-1",
      bears_on: [
        { allegation_id: "alleg-7", accusation: "A-7 — they knew", elements: [], count: null },
      ],
    });

  it("THE DEFECT: I rules NOTHING and opens the row (M)", () => {
    // Every Include in the queue returned HTTP 400, because the route has
    // required the accusation and the stance since FACT_CARD_v2 §2 and this
    // reducer sent neither. The button looked like it worked. Nothing was filed.
    const { state, effect } = press(stateOf([withAccusation()]), "i");

    expect(effect).toEqual({ kind: "none" });
    expect(state.mode).toEqual({
      kind: "including",
      graphNodeId: "ev-1",
      allegationId: "alleg-7",
      stance: "supports",
    });
    // The card is untouched until Save.
    expect(state.cards[0].status).not.toBe("included");
    expect(progress(state).ruled).toBe(0);
  });

  it("the BUTTON opens the same row as the keyboard (1.7D)", () => {
    // One state machine, two input devices. Fixing only the button would leave
    // `I` still returning 400, and two controls meaning different things.
    const viaKey = press(stateOf([withAccusation()]), "i").state;
    const viaClick = queueReducer(stateOf([withAccusation()]), {
      type: "rule",
      key: "i",
      graphNodeId: "ev-1",
    }).state;

    expect(viaClick.mode).toEqual(viaKey.mode);
  });

  it("the row carries the card it was opened ON, not the selection", () => {
    // 1.7G, applied to the picker: anything that moves the selection while a
    // human is answering must not redirect the include.
    const pool = [withAccusation(), fullCard({ graph_node_id: "ev-2" })];
    const opened = queueReducer(stateOf(pool), {
      type: "rule",
      key: "i",
      graphNodeId: "ev-2",
    }).state;

    expect(opened.mode).toMatchObject({ kind: "including", graphNodeId: "ev-2" });
  });

  it("opens with NOTHING chosen on a card that bears on nothing", () => {
    // Instruction item 4. `fullCard` carries no bears-on by default.
    const opened = press(stateOf([fullCard({ bears_on: [] })]), "i").state;

    expect(opened.mode).toMatchObject({ kind: "including", allegationId: null });
  });

  it("Save files the include, carrying the pair (M)", () => {
    const opened = press(stateOf([withAccusation()]), "i").state;
    const { state, effect } = queueReducer(opened, { type: "include_save" });

    expect(effect).toMatchObject({
      kind: "rule",
      graphNodeId: "ev-1",
      action: "include",
      allegationId: "alleg-7",
      stance: "supports",
    });
    expect(state.mode).toEqual({ kind: "triage" });
    expect(state.cards[0].status).toBe("included");
  });

  it("choosing an accusation changes the ACCUSATION, not the stance (M)", () => {
    // The two `include_*` choice events are one line each and land on the same
    // machine, so a handler wired to the wrong arm compiles, runs, and passes
    // every test in `includePickerModel.test.ts` — while a curator picking a
    // second accusation silently flips the stance instead.
    const twoAccusations = fullCard({
      graph_node_id: "ev-1",
      bears_on: [
        { allegation_id: "alleg-7", accusation: "A-7 — they knew", elements: [], count: null },
        { allegation_id: "alleg-9", accusation: "A-9 — the funds", elements: [], count: null },
      ],
    });
    const opened = press(stateOf([twoAccusations]), "i").state;

    const after = queueReducer(opened, {
      type: "include_allegation",
      allegationId: "alleg-9",
    }).state;

    expect(after.mode).toEqual({
      kind: "including",
      graphNodeId: "ev-1",
      allegationId: "alleg-9",
      stance: "supports",
    });

    // And it reaches the wire on Save.
    expect(queueReducer(after, { type: "include_save" }).effect).toMatchObject({
      allegationId: "alleg-9",
      stance: "supports",
    });
  });

  it("Save sends the stance the human chose, not the default", () => {
    let s = press(stateOf([withAccusation()]), "i").state;
    s = queueReducer(s, { type: "include_stance", stance: "rebuts" }).state;

    expect(queueReducer(s, { type: "include_save" }).effect).toMatchObject({
      stance: "rebuts",
    });
  });

  it("Cancel files NOTHING and closes the row (M)", () => {
    const opened = press(stateOf([withAccusation()]), "i").state;
    const { state, effect } = queueReducer(opened, { type: "include_cancel" });

    expect(effect).toEqual({ kind: "none" });
    expect(state.mode).toEqual({ kind: "triage" });
    expect(state.cards[0].status).not.toBe("included");
    expect(progress(state).ruled).toBe(0);
  });

  it("Save is refused while no accusation is chosen", () => {
    const opened = press(stateOf([fullCard({ bears_on: [] })]), "i").state;
    const { state, effect } = queueReducer(opened, { type: "include_save" });

    // The row stays OPEN rather than firing the round trip that would 400.
    expect(effect).toEqual({ kind: "none" });
    expect(state.mode).toMatchObject({ kind: "including" });
  });

  it("the row owns the keyboard: Esc cancels, Enter saves (M)", () => {
    const opened = press(stateOf([withAccusation()]), "i").state;

    expect(press(opened, "Escape").state.mode).toEqual({ kind: "triage" });
    expect(press(opened, "Escape").effect).toEqual({ kind: "none" });

    const saved = press(opened, "Enter");
    expect(saved.effect).toMatchObject({ action: "include", allegationId: "alleg-7" });

    // And a letter neither rules nor types while the row is open.
    expect(press(opened, "e").effect).toEqual({ kind: "none" });
    expect(press(opened, "j").state.index).toBe(opened.index);
  });

  it("a click on ANOTHER card's button abandons the open row rather than eating it", () => {
    // The prompt owns the keyboard, not the mouse — `deferring`'s own rule.
    const pool = [withAccusation(), fullCard({ graph_node_id: "ev-2" })];
    const opened = press(stateOf(pool), "i").state;

    const { state, effect } = queueReducer(opened, {
      type: "rule",
      key: "e",
      graphNodeId: "ev-2",
    });

    expect(effect).toMatchObject({ graphNodeId: "ev-2", action: "drop" });
    expect(state.mode).toEqual({ kind: "triage" });
  });

  it("but I on the card the row is ALREADY open for changes nothing", () => {
    // Re-opening would discard a half-answered question to ask it again.
    const opened = press(stateOf([withAccusation()]), "i").state;
    const again = queueReducer(opened, { type: "rule", key: "i", graphNodeId: "ev-1" });

    expect(again.state.mode).toEqual(opened.mode);
    expect(again.effect).toEqual({ kind: "none" });
  });

  it("Exclude, Defer and Undo are UNTOUCHED (M)", () => {
    // The blast radius. Only Include changed; the other three rule exactly as
    // they did, on one press.
    const pool = [withAccusation(), fullCard({ graph_node_id: "ev-2" })];

    expect(press(stateOf(pool), "e").effect).toMatchObject({ action: "drop" });
    expect(press(stateOf(pool), "d").state.mode).toMatchObject({ kind: "deferring" });

    // Undo always emits `reopen` — one word for taking any ruling back (see
    // `undoLast`), whichever ruling it was.
    const ruled = press(stateOf(pool), "e").state;
    expect(press(ruled, "u").effect).toMatchObject({ action: "reopen" });
  });
});

// ─── The stance is a settings row, not a constant ───────────────────────────
//
// `includePickerModel` held `const DEFAULT_STANCE = "supports"` until
// 2026-09-19. These are the tests that would fail if it came back.

describe("the Include picker's opening stance", () => {
  const openOn = (stance: CardFactStance) => {
    const card = fullCard({ graph_node_id: "ev-1" });
    return press(loaded([card], stance), "i").state.mode;
  };

  it("opens on the stance the payload carried", () => {
    expect(openOn("supports")).toMatchObject({ kind: "including", stance: "supports" });
  });

  it("opens on the OTHER stance when the row says so", () => {
    expect(openOn("rebuts")).toMatchObject({ kind: "including", stance: "rebuts" });
  });

  /**
   * (M) THE MUTATION PROOF.
   *
   * The two calls differ in nothing but the stored value, and they must differ
   * in their result. No compiled-in constant can satisfy both, which is exactly
   * what could not be said while `DEFAULT_STANCE` existed: every test then
   * passed against a literal.
   */
  it("cannot be satisfied by any constant (M)", () => {
    const supports = openOn("supports");
    const rebuts = openOn("rebuts");
    expect(supports).not.toEqual(rebuts);
    expect([supports, rebuts].map((m) => (m.kind === "including" ? m.stance : null))).toEqual([
      "supports",
      "rebuts",
    ]);
  });

  it("carries the chosen stance through to the saved ruling", () => {
    // The prefill is only worth having if it is what gets SAVED when the human
    // presses Enter without touching the control.
    const card = fullCard({ graph_node_id: "ev-1" });
    let s = press(loaded([card], "rebuts"), "i").state;
    const { effect } = queueReducer(s, { type: "include_save" });
    expect(effect).toMatchObject({ kind: "rule", action: "include", stance: "rebuts" });
  });

  it("survives a reload that changes it", () => {
    // A Settings edit reaches the picker on the next read of the pool — no
    // rebuild, and no second request.
    const card = fullCard({ graph_node_id: "ev-1" });
    let s = loaded([card], "supports");
    s = queueReducer(s, {
      type: "cards_loaded",
      cards: [card],
      includeDefaultStance: "rebuts",
    }).state;
    expect(press(s, "i").state.mode).toMatchObject({ stance: "rebuts" });
  });
});

// ─── The placeholder in `initialQueueState` is unreachable ──────────────────
//
// `initialQueueState` must put SOMETHING in `includeDefaultStance` — the state
// has no partial form — and it puts `"supports"` there. That is only NOT a
// compiled-in default because nothing can read it before `cards_loaded`
// overwrites it, and the doc comment on that function says so. These two tests
// are what makes the claim checkable: the first holds the behaviour, the second
// holds the call site the behaviour depends on.

describe("the mount-time stance placeholder", () => {
  it("cannot be read: no key opens the picker on an empty queue (mount_state_holds_no_default)", () => {
    // The production mount, exactly: `useQueueReducer` starts the queue with no
    // cards. Every ruling key, every navigation key, and the picker's own
    // events must leave the mode in triage — because there is no card to rule,
    // and the picker is reached only by ruling one.
    const mounted = initialQueueState([]);
    for (const key of ["i", "e", "d", "u", "j", "k", "Enter", "Escape", "1"]) {
      const { state, effect } = press(mounted, key);
      expect(state.mode, `pressing ${key} on an empty queue opened something`).toEqual({
        kind: "triage",
      });
      expect(effect).toEqual({ kind: "none" });
    }
    // And the picker's own events cannot open it either — they act on the mode,
    // which is `triage`, so there is no route in that bypasses a card.
    expect(queueReducer(mounted, { type: "include_save" }).state.mode).toEqual({ kind: "triage" });
    expect(
      queueReducer(mounted, { type: "include_stance", stance: "rebuts" }).state.mode,
    ).toEqual({ kind: "triage" });
  });

  it("is only safe because the production mount is EMPTY (the_production_mount_is_always_empty)", () => {
    // ⚑ The argument above rests on the call site, not on the type: pass a
    // non-empty array to `initialQueueState` and the placeholder becomes
    // reachable — which is exactly what this suite's own `stateOf` helper does.
    // So the call site is read off disk. If `useQueueReducer` ever mounts with
    // real cards, the placeholder becomes a compiled-in default and this reds.
    const source = readFileSync(join(__dirname, "..", "useQueueReducer.tsx"), "utf8")
      .split("\n")
      .map((line) => {
        const at = line.indexOf("//");
        return at === -1 ? line : line.slice(0, at);
      })
      .join("\n");
    const calls = source.match(/initialQueueState\([^)]*\)/g) ?? [];
    expect(calls, "useQueueReducer no longer mounts the queue").toHaveLength(1);
    expect(calls[0]).toBe("initialQueueState([])");
  });
});

// ─── The defer prompt's defensive guard ─────────────────────────────────────

describe("a defer prompt whose card has left the queue", () => {
  it("closes rather than committing against another card", () => {
    // Unreachable in production — a card cannot leave the pool while its defer
    // prompt is open — but the guard exists because the alternative, if it ever
    // WERE reachable, is a reason typed about one card recorded against
    // whatever is selected now. Asserted rather than trusted: the prompt closes
    // and nothing is written.
    const card = fullCard({ graph_node_id: "ev-1" });
    const opened = press(loaded([card]), "d").state;
    expect(opened.mode).toMatchObject({ kind: "deferring", graphNodeId: "ev-1" });

    // The pool is replaced and no longer holds the card the prompt is about.
    const gone: QueueState = { ...opened, cards: [fullCard({ graph_node_id: "ev-9" })] };
    const drafted = queueReducer(gone, { type: "defer_draft", draft: "a reason" }).state;
    const { state, effect } = press(drafted, "Enter");
    expect(state.mode).toEqual({ kind: "triage" });
    expect(effect).toEqual({ kind: "none" });
  });
});
