// What goes on Chuck's paper, pinned.
//
// ## Why these fixtures are the two REAL deck shapes
//
// The mockup was drawn against S-5, which is 5 cross / 5 direct / 5 redirect —
// and every one of its assumptions holds trivially when the three kinds are equal.
// S-7, the deck Chuck is actually reviewing, is **6 / 8 / 2**. Roman's correction
// M3 of 2026-08-22 says nothing may hardcode a count or assume the kinds are equal
// in number, and `S7_SHAPE` is what enforces it: an implementation that divided by
// three, or paired redirects to defense questions by position, passes every test
// written against S-5 and fails here.

import { describe, expect, it } from "vitest";

import type { PracticeQuestion, PracticeWording } from "../../../services/practice";
import {
  antecedentOf,
  hasPrintableQuestions,
  hiddenLine,
  missingLine,
  planSheets,
  printable,
  printLockReason,
  redirectHowto,
} from "../printSheetPlan";

/** Only the print keys — `wordingOf` throws on a blank, which is the point. */
const WORDING = {
  print_sheet_cross_title: "The defense asks",
  print_sheet_direct_title: "Chuck asks",
  print_sheet_redirect_title: "Chuck, after the defense",
  print_sheet_subtitle_template: "{code} · “{title}”",
  print_sheet_redirect_subtitle: "the redirects — each follows one of the defense's questions",
  print_howto_cross: "In the order the defense would ask them. Trial Prep → {code} → Practice.",
  print_howto_direct: "Your direct — foundation first, then her three points.",
  print_howto_redirect: "Each one repairs one defense question.",
  print_howto_redirect_drafts: "These are drafts, written for you to rewrite.",
  print_missing_prefix: "This deck has",
  print_missing_cross: "no questions from the defense yet",
  print_missing_direct: "no questions from Chuck yet",
  print_missing_redirect: "no redirects",
  print_missing_joiner: ", and",
  print_hidden_template: "{n} questions are hidden and are not shown.",
} as unknown as PracticeWording;

let seq = 0;
function q(over: Partial<PracticeQuestion> & { kind: string }): PracticeQuestion {
  seq += 1;
  return {
    id: `id-${seq}`,
    side: over.kind === "cross" ? "george" : "chuck",
    braid: false,
    text: `question ${seq}`,
    tactic: null,
    receipt: null,
    braid_rows: null,
    watch_for: null,
    pair_said: null,
    pair_admitted: null,
    stronger: null,
    stronger_lean: null,
    flag_note: null,
    deck_key: null,
    follows_key: null,
    hidden: false,
    answered_on: null,
    draft_by: null,
    ...over,
  };
}

/** S-5 as it really is: 5 / 5 / 5, every redirect a draft. **[measured]** */
function s5(): PracticeQuestion[] {
  const cross = [1, 2, 3, 4, 5].map((n) => q({ kind: "cross", deck_key: `g${n}` }));
  const direct = [1, 2, 3, 4, 5].map((n) => q({ kind: "direct", deck_key: `c${n}` }));
  const redirect = [1, 2, 3, 4, 5].map((n) =>
    q({ kind: "redirect", deck_key: `r${n}`, follows_key: `g${n}`, draft_by: "architect" }),
  );
  return [...cross, ...direct, ...redirect];
}

/**
 * S-7 as it really is: **6 cross, 8 direct, 2 redirect, and NO draft_by**.
 *
 * **[measured 2026-08-22]** — and note `r2 → g4`, not `g2`. Redirects do not run
 * parallel to the defense list.
 */
function s7(): PracticeQuestion[] {
  const cross = [1, 2, 3, 4, 5, 6].map((n) => q({ kind: "cross", deck_key: `g${n}` }));
  const direct = [1, 2, 3, 4, 5, 6, 7, 8].map((n) => q({ kind: "direct", deck_key: `c${n}` }));
  const redirect = [
    q({ kind: "redirect", deck_key: "r1", follows_key: "g1" }),
    q({ kind: "redirect", deck_key: "r2", follows_key: "g4" }),
  ];
  return [...cross, ...direct, ...redirect];
}

const plan = (questions: PracticeQuestion[]) => planSheets(questions, "S-7", "Bias against Marie", WORDING);

// ── The three deck states ────────────────────────────────────────────────────

describe("the three deck states", () => {
  it("an empty deck has nothing to print, and the control says so", () => {
    expect(hasPrintableQuestions([])).toBe(false);
    // No sheets, and NO missing-line: an empty deck is refused at the control
    // with its own reason, not printed as three headings over white space.
    const p = plan([]);
    expect(p.sheets).toHaveLength(0);
    expect(p.missingLine).toBeNull();
  });

  it("a defense-only deck prints ONE sheet and names both absences", () => {
    const p = plan([q({ kind: "cross", deck_key: "g1" }), q({ kind: "cross", deck_key: "g2" })]);
    expect(p.sheets.map((s) => s.kind)).toEqual(["cross"]);
    expect(p.deckTotal).toBe(2);
    expect(p.missingLine).toBe(
      "This deck has no questions from Chuck yet, and no redirects.",
    );
  });

  it("a two-kind deck names only the one that is absent", () => {
    const p = plan([q({ kind: "cross", deck_key: "g1" }), q({ kind: "direct", deck_key: "c1" })]);
    expect(p.sheets.map((s) => s.kind)).toEqual(["cross", "direct"]);
    expect(p.missingLine).toBe("This deck has no redirects.");
  });

  it("a complete deck prints three sheets and NO missing line", () => {
    const p = plan(s5());
    expect(p.sheets.map((s) => s.kind)).toEqual(["cross", "direct", "redirect"]);
    expect(p.missingLine).toBeNull();
  });
});

// ── M3: nothing assumes the kinds are equal ──────────────────────────────────

describe("S-7's 6 / 8 / 2 — Roman's correction M3", () => {
  it("counts each sheet from its own rows, never from a share of the deck", () => {
    const p = plan(s7());
    expect(p.sheets.map((s) => s.rows.length)).toEqual([6, 8, 2]);
    expect(p.deckTotal).toBe(16);
    // The trap this closes: 16 / 3 is 5, which is a plausible-looking number and
    // wrong for every one of the three sheets.
    expect(p.sheets.every((s) => s.rows.length !== Math.floor(p.deckTotal / 3))).toBe(true);
  });

  it("pairs a redirect to its antecedent BY KEY, not by position", () => {
    const p = plan(s7());
    const redirects = p.sheets.find((s) => s.kind === "redirect");
    const after = redirects?.rows.map((r) =>
      r.after?.kind === "resolved" ? r.after.antecedent.deck_key : null,
    );
    // r2 follows g4. By position it would have been paired with g2.
    expect(after).toEqual(["g1", "g4"]);
  });

  it("no sheet's chrome carries a count of its own", () => {
    // Rendered with a digit-free code and title, so anything numeric left in the
    // output came from a TEMPLATE and not from the scenario. Without that
    // substitution this test fails on "S-7", which is data and perfectly correct.
    const sheets = planSheets(s7(), "SCEN", "Bias against Marie", WORDING).sheets;
    expect(sheets.map((s) => s.rows.length)).toEqual([6, 8, 2]);
    for (const sheet of sheets) {
      for (const line of [sheet.title, sheet.subtitle, sheet.howto]) {
        expect(line, `"${line}" carries a hardcoded number`).not.toMatch(/\d/);
      }
    }
  });
});

// ── M2: the draft claim is withheld when it is not true ──────────────────────

describe("the draft sentence", () => {
  it("is added when at least one redirect is a draft — S-5", () => {
    const sheet = plan(s5()).sheets.find((s) => s.kind === "redirect");
    expect(sheet?.howto).toContain("These are drafts");
  });

  it("is WITHHELD when no redirect is a draft — S-7", () => {
    const sheet = plan(s7()).sheets.find((s) => s.kind === "redirect");
    expect(sheet?.howto).toBe("Each one repairs one defense question.");
    expect(sheet?.howto).not.toContain("drafts");
  });

  it("is decided per sheet, not per deck", () => {
    const mixed = [
      q({ kind: "redirect", deck_key: "r1", follows_key: "g1" }),
      q({ kind: "redirect", deck_key: "r2", follows_key: "g2", draft_by: "architect" }),
    ];
    const rows = mixed.map((question) => ({ question, after: null }));
    expect(redirectHowto(rows, WORDING)).toContain("These are drafts");
  });
});

// ── Ruling 3: hidden questions ───────────────────────────────────────────────

describe("hidden questions", () => {
  it("do not print and are not counted", () => {
    const deck = [
      q({ kind: "cross", deck_key: "g1" }),
      q({ kind: "cross", deck_key: "g2", hidden: true }),
      q({ kind: "direct", deck_key: "c1", hidden: true }),
    ];
    const p = plan(deck);
    expect(printable(deck)).toHaveLength(1);
    expect(p.deckTotal).toBe(1);
    expect(p.sheets.map((s) => s.kind)).toEqual(["cross"]);
    // The hidden direct does NOT make a direct sheet, and the deck reads as
    // having none — which it does, for printing purposes.
    expect(p.missingLine).toContain("no questions from Chuck yet");
  });

  it("are NAMED when any exist, so Chuck does not rewrite one", () => {
    const p = plan([
      q({ kind: "cross", deck_key: "g1" }),
      q({ kind: "cross", deck_key: "g2", hidden: true }),
    ]);
    expect(p.hiddenLine).toBe("1 questions are hidden and are not shown.");
  });

  it("say nothing at all when none are hidden", () => {
    expect(plan(s5()).hiddenLine).toBeNull();
    expect(hiddenLine(0, WORDING)).toBeNull();
  });

  it("a deck that is ENTIRELY hidden has nothing to print", () => {
    const deck = [q({ kind: "cross", deck_key: "g1", hidden: true })];
    expect(hasPrintableQuestions(deck)).toBe(false);
  });
});

// ── The antecedent, and its named absence ────────────────────────────────────

describe("a redirect's antecedent", () => {
  it("resolves to the defense question it follows", () => {
    const pool = s5();
    const r3 = pool.find((x) => x.deck_key === "r3")!;
    const after = antecedentOf(r3, pool);
    expect(after?.kind).toBe("resolved");
  });

  it("says so IN WORDS when follows_key names nothing in the deck", () => {
    // follows_key is a KEY, not a foreign key: nothing stops g9 being removed.
    const orphan = q({ kind: "redirect", deck_key: "r1", follows_key: "g9" });
    expect(antecedentOf(orphan, [orphan])).toEqual({ kind: "missing" });
  });

  it("is absent — not missing — when a redirect follows nothing at all", () => {
    const loose = q({ kind: "redirect", deck_key: "r1" });
    expect(antecedentOf(loose, [loose])).toBeNull();
  });

  it("never resolves to a HIDDEN question", () => {
    const hidden = q({ kind: "cross", deck_key: "g1", hidden: true });
    const r1 = q({ kind: "redirect", deck_key: "r1", follows_key: "g1" });
    const p = planSheets([hidden, r1], "S-7", "t", WORDING);
    const row = p.sheets.find((s) => s.kind === "redirect")?.rows[0];
    // The pool is the PRINTABLE deck, so a hidden antecedent reads as missing
    // rather than quoting a question that is not on any sheet.
    expect(row?.after).toEqual({ kind: "missing" });
  });
});

// ── The absent-kinds sentence ────────────────────────────────────────────────

describe("the missing-kinds line", () => {
  it("composes all six shapes from five stored fragments", () => {
    expect(missingLine(["cross"], WORDING)).toBe(
      "This deck has no questions from the defense yet.",
    );
    expect(missingLine(["direct"], WORDING)).toBe("This deck has no questions from Chuck yet.");
    expect(missingLine(["redirect"], WORDING)).toBe("This deck has no redirects.");
    expect(missingLine(["cross", "direct"], WORDING)).toBe(
      "This deck has no questions from the defense yet, and no questions from Chuck yet.",
    );
    expect(missingLine(["direct", "redirect"], WORDING)).toBe(
      "This deck has no questions from Chuck yet, and no redirects.",
    );
    expect(missingLine([], WORDING)).toBeNull();
  });

  it("supplies the joining space the store trimmed away", () => {
    // The store holds ", and" — a renderer trusting a stored trailing space
    // prints "no redirects, andno questions from Chuck yet" on Chuck's paper.
    const line = missingLine(["direct", "redirect"], WORDING)!;
    expect(line).not.toContain("andno");
    expect(line).toContain(", and no redirects");
  });
});

// ── The control's two refusals — Roman's rulings 3 and 4 ─────────────────────

describe("why the print control refuses", () => {
  const EMPTY = "No questions in this deck yet.";
  const BUSY = "Finish editing first.";

  it("says nothing when there is a deck and no edit in progress", () => {
    expect(printLockReason(s5(), false, EMPTY, BUSY)).toBeNull();
  });

  it("names the EMPTY deck first, even while editing", () => {
    // Order matters: telling someone with no questions to "finish editing first"
    // sends them to fix the wrong thing.
    expect(printLockReason([], true, EMPTY, BUSY)).toBe(EMPTY);
    expect(printLockReason([], false, EMPTY, BUSY)).toBe(EMPTY);
  });

  it("treats an ENTIRELY hidden deck as nothing to print", () => {
    const allHidden = [
      q({ kind: "cross", deck_key: "g1", hidden: true }),
      q({ kind: "direct", deck_key: "c1", hidden: true }),
    ];
    expect(printLockReason(allHidden, false, EMPTY, BUSY)).toBe(EMPTY);
  });

  it("LOCKS in edit mode — ruling 4", () => {
    // Not only for consistency with its neighbours: mid-edit the sheets would
    // print the SAVED deck while the person is looking at unsaved changes.
    expect(printLockReason(s5(), true, EMPTY, BUSY)).toBe(BUSY);
  });

  it("never refuses without saying why", () => {
    // The standing rule of 2026-08-19, as a type-level guarantee: every refusal
    // is a sentence, so a caller cannot disable the control and stay silent.
    for (const [deck, editing] of [
      [[], true],
      [[], false],
      [s5(), true],
    ] as const) {
      const reason = printLockReason(deck as PracticeQuestion[], editing, EMPTY, BUSY);
      expect(reason).not.toBeNull();
      expect(reason!.length).toBeGreaterThan(0);
    }
  });
});
