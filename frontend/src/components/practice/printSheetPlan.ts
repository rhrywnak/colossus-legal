// printSheetPlan.ts — what goes on Chuck's paper, decided without a browser.
//
// Named `printSheetPlan` and not `printSheets` because `PrintSheets.tsx` draws
// them: two files differing only in the case of one letter collide on any
// case-insensitive filesystem, and TypeScript refuses the pair outright.
//
// PURE. No React, no DOM, no clock. Every rule Roman ruled on 2026-08-22 is
// decided here, so each is a unit test over a fixture rather than something you
// verify by squinting at a PDF.
//
// ## Why this file exists at all — the shape (A) could not have
//
// The alternative was a print stylesheet on the practice page. It cannot work:
// the deck list is rendered `{open && (…)}` (PracticeDeckList), so a folded deck
// is not in the DOM and no CSS can reveal it; and the list renders
// `editor.editing ? view.all : view.ordered`, so the question set on screen
// follows the *Who's asking?* selector and edit mode. Printing "the whole deck,
// ignoring the selector" from that DOM is not possible. Deciding it from the
// PAYLOAD, here, is.
//
// ## The one assumption this file is forbidden to make
//
// That the three kinds are equal in number. The mockup was drawn against S-5's
// 5 / 5 / 5; S-7 is 6 cross / 8 direct / 2 redirect. Nothing here divides by
// three, indexes one kind by another's position, or writes a count into a
// sentence. `S7_SHAPE` in the tests is the enforcement.

import {
  wordingOf,
  type PracticeQuestion,
  type PracticeWording,
} from "../../services/practice";

/** The three sheets, in the order they print. */
export const SHEET_KINDS = ["cross", "direct", "redirect"] as const;
export type SheetKind = (typeof SHEET_KINDS)[number];

/** One question as the paper needs it, with its antecedent already resolved. */
export type PrintRow = {
  question: PracticeQuestion;
  /**
   * For a redirect: the defense question it repairs.
   *
   * `resolved` carries the antecedent when `follows_key` names a question that
   * is actually in this deck. `missing` says so in words.
   *
   * ## Domain note: why a miss is possible at all
   *
   * `follows_key` holds a deck KEY, not a foreign key — nothing in the database
   * stops the question it names being hidden or removed. A blank quote box would
   * read as a redirect that repairs nothing, which is a different claim.
   */
  after: { kind: "resolved"; antecedent: PracticeQuestion } | { kind: "missing" } | null;
};

/** One sheet, already decided: it prints, and this is what is on it. */
export type PrintSheet = {
  kind: SheetKind;
  title: string;
  subtitle: string;
  howto: string;
  rows: PrintRow[];
};

/** Everything the print view renders, decided from the payload alone. */
export type PrintPlan = {
  sheets: PrintSheet[];
  /** Every question that will print, across all sheets — the `{m}` count. */
  deckTotal: number;
  /** The absent-kinds sentence, or `null` when all three sheets printed. */
  missingLine: string | null;
  /** The hidden-questions sentence, or `null` when none are hidden. */
  hiddenLine: string | null;
};

/**
 * Fill `{name}` placeholders.
 *
 * ## Why a local helper and not the backend's `render`
 *
 * The backend's helper takes UNBRACED keys (`verb`), while every template in the
 * settings store is written with braces (`{when}`). Mixing the two ships the raw
 * token to screen — or, here, to paper. This one takes the braced form, which is
 * what the stored strings actually contain.
 */
export function fill(template: string, values: Record<string, string>): string {
  return Object.entries(values).reduce(
    (out, [key, value]) => out.split(`{${key}}`).join(value),
    template,
  );
}

/**
 * The questions that print: visible ones, in deck order.
 *
 * ## Domain note: hidden questions do not print and are not counted
 *
 * Roman's ruling, 2026-08-22. A hidden question is one the editor has taken out
 * of play, and printing it invites Chuck to spend his morning on a question
 * nobody will ask. Their EXISTENCE is still said — see [`hiddenLine`] — because a
 * Chuck who does not know one is hidden will rewrite a question the deck has.
 */
export function printable(questions: PracticeQuestion[]): PracticeQuestion[] {
  return questions.filter((q) => !q.hidden);
}

/** How many questions are hidden, and therefore absent from every sheet. */
export function hiddenCount(questions: PracticeQuestion[]): number {
  return questions.filter((q) => q.hidden).length;
}

/**
 * The defense question a redirect repairs, looked up BY KEY.
 *
 * ## Why by key and never by position
 *
 * Redirects do not run parallel to the defense list. **[measured 2026-08-22:
 * S-7's `r2` follows `g4`, not `g2`.]** Pairing them by index would silently
 * print the wrong question above a redirect — and the judgement Chuck is making
 * is precisely whether this repairs that.
 */
export function antecedentOf(
  redirect: PracticeQuestion,
  pool: PracticeQuestion[],
): PrintRow["after"] {
  if (redirect.follows_key === null) return null;
  const found = pool.find((q) => q.deck_key === redirect.follows_key);
  return found ? { kind: "resolved", antecedent: found } : { kind: "missing" };
}

/**
 * The absent-kinds sentence, composed from fragments.
 *
 * ## Why fragments and not six whole sentences
 *
 * Any one or any two of the three kinds may be absent — six shapes. (All three
 * absent is the empty deck, where the control is disabled and nothing prints.)
 * Five stored fragments compose all six; six stored sentences would be six rows
 * that drift out of agreement with one another.
 *
 * ## ⚑ The joiner carries no space
 *
 * The settings store trims every value, so `", and "` is stored as `", and"`.
 * This function supplies the space. A renderer that trusted the stored one would
 * print "no redirects, andno questions from Chuck yet".
 */
export function missingLine(
  absent: SheetKind[],
  wording: PracticeWording,
): string | null {
  if (absent.length === 0) return null;
  const w = (key: string) => wordingOf(wording, key);
  const fragmentFor: Record<SheetKind, string> = {
    cross: w("print_missing_cross"),
    direct: w("print_missing_direct"),
    redirect: w("print_missing_redirect"),
  };
  const joiner = `${w("print_missing_joiner")} `;
  const fragments = absent.map((kind) => fragmentFor[kind]);
  return `${w("print_missing_prefix")} ${fragments.join(joiner)}.`;
}

/** The hidden-questions sentence, or `null` when none are hidden. */
export function hiddenLine(hidden: number, wording: PracticeWording): string | null {
  if (hidden === 0) return null;
  return fill(wordingOf(wording, "print_hidden_template"), { n: String(hidden) });
}

/**
 * Sheet 3's instruction — with the draft sentence only when it is TRUE.
 *
 * ## Domain note: the claim is withheld, not softened
 *
 * The mockup read "All five are drafts written for you to rewrite." Wrong twice:
 * the count varies by deck, and draftness is a separate fact. **[measured: all
 * five of S-5's redirects carry `draft_by`; NONE of S-7's do.]** On S-7 the badge
 * does not render, so a sheet claiming draftness would be the paper contradicting
 * its own rows.
 */
export function redirectHowto(
  rows: PrintRow[],
  wording: PracticeWording,
): string {
  const w = (key: string) => wordingOf(wording, key);
  const anyDraft = rows.some((r) => r.question.draft_by !== null);
  return anyDraft
    ? `${w("print_howto_redirect")} ${w("print_howto_redirect_drafts")}`
    : w("print_howto_redirect");
}

/** The title, subtitle and how-to for one sheet. */
function chromeFor(
  kind: SheetKind,
  rows: PrintRow[],
  code: string,
  title: string,
  wording: PracticeWording,
): { title: string; subtitle: string; howto: string } {
  const w = (key: string) => wordingOf(wording, key);
  const scenarioSubtitle = fill(w("print_sheet_subtitle_template"), { code, title });
  switch (kind) {
    case "cross":
      return {
        title: w("print_sheet_cross_title"),
        subtitle: scenarioSubtitle,
        howto: fill(w("print_howto_cross"), { code }),
      };
    case "direct":
      return {
        title: w("print_sheet_direct_title"),
        subtitle: scenarioSubtitle,
        howto: w("print_howto_direct"),
      };
    case "redirect":
      return {
        title: w("print_sheet_redirect_title"),
        // The redirect sheet names what it IS rather than the accusation — and
        // says nothing about counts, because S-7 has six cross and two redirects.
        subtitle: `${code} · ${w("print_sheet_redirect_subtitle")}`,
        howto: redirectHowto(rows, wording),
      };
  }
}

/**
 * Decide the whole printout from the payload.
 *
 * **A sheet with no questions is not printed at all**, and the sheet count
 * adjusts — a defense-only deck prints one sheet reading "sheet 1 of 1", never
 * three sheets two of which are a heading over white space. Most decks are
 * partial; that is the normal case, not the edge case.
 */
export function planSheets(
  questions: PracticeQuestion[],
  code: string,
  title: string,
  wording: PracticeWording,
): PrintPlan {
  const visible = printable(questions);
  const sheets: PrintSheet[] = [];
  const absent: SheetKind[] = [];

  for (const kind of SHEET_KINDS) {
    const of = visible.filter((q) => q.kind === kind);
    if (of.length === 0) {
      absent.push(kind);
      continue;
    }
    const rows: PrintRow[] = of.map((question) => ({
      question,
      after: kind === "redirect" ? antecedentOf(question, visible) : null,
    }));
    sheets.push({ kind, rows, ...chromeFor(kind, rows, code, title, wording) });
  }

  return {
    sheets,
    deckTotal: visible.length,
    // Nothing to say when all three printed — and nothing to say when NONE did,
    // because an empty deck never reaches here: the control is disabled instead.
    missingLine: sheets.length === 0 ? null : missingLine(absent, wording),
    hiddenLine: hiddenLine(hiddenCount(questions), wording),
  };
}

/**
 * Is there anything to print?
 *
 * The control is disabled when this is false, and says why — the standing rule of
 * 2026-08-19: no control on a practice page may be dim and silent. An
 * all-hidden deck counts as nothing to print.
 */
export function hasPrintableQuestions(questions: PracticeQuestion[]): boolean {
  return printable(questions).length > 0;
}

/**
 * Why the print control refuses, or `null` when it does not.
 *
 * ## Why this returns a SENTENCE and not a boolean
 *
 * The same shape `lockReason` uses on the practice card: a caller has nothing to
 * put in `title` unless it also has the reason, so a control cannot be disabled
 * here without saying why. The standing rule of 2026-08-19 — no control on a
 * practice page may be dim and silent — is enforced by the return type rather
 * than by remembering.
 *
 * ## The two refusals, in order
 *
 * NOTHING TO PRINT comes first: an empty deck (or one that is entirely hidden) is
 * a fact about the deck, and saying "finish editing first" to someone whose deck
 * has no questions would send them to fix the wrong thing.
 *
 * EDIT MODE second (Roman's ruling 4, 2026-08-22). Not merely for consistency
 * with its neighbours: mid-edit the sheets would print the SAVED deck while the
 * person is looking at unsaved changes on screen, and the paper would disagree
 * with the monitor beside it.
 */
export function printLockReason(
  questions: PracticeQuestion[],
  editing: boolean,
  emptyHint: string,
  busyHint: string,
): string | null {
  if (!hasPrintableQuestions(questions)) return emptyHint;
  if (editing) return busyHint;
  return null;
}
