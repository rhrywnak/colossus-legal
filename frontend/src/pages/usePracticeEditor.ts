// =============================================================================
// usePracticeEditor.ts — the deck editor's state and its four writes (task B1)
// =============================================================================
//
// Extracted from the start card for the reason `usePracticeDeckControls` was:
// this is a coherent thing on its own — everything Chuck and Roman can do to the
// deck between sittings — and it would otherwise carry `PracticePage` past Rule
// 17 twice over.
//
// ## What signs a change, since 2026-08-19
//
// This hook used to hold an `editingAs` string, filled by a "Who is editing?"
// dropdown, and `run` returned early — silently — while it was `""`. That is the
// defect Roman hit in the first minute of .402: Edit, the arrows and Hide all
// appeared enabled and did nothing, and no sentence anywhere said why.
//
// The premise was wrong. Chuck and Marie have Authentik logins; every write
// already arrives authenticated, and the server now signs each change from the
// session. So the picker is gone, the early return is gone, and `ready` is
// simply "no write in flight". There is no state in which a control on this
// editor is enabled and does nothing.
//
// ## Why edit mode is a MODE
//
// `editing` is not decoration. While it is on, the start card's own controls —
// Start, the count pills, the side cards, the fold, Resume, Start over — are
// disabled, because every one of them navigates away from a half-finished edit.
// The hook owns the flag; `PracticeStart` reads it and disables. See §2 of
// CC_TASK_PRACTICE_V1_HOTFIX_WORKFLOW_v1.
//
// ## Why every write ends in a reload
//
// A move changes two rows' order, an edit changes what the change log says, an
// add changes the deck's length and Marie's badges. Patching the payload in
// place would mean this hook re-deriving five server-composed sentences; asking
// for the deck again is one request and cannot disagree with itself.

import React from "react";

import {
  addQuestion,
  editQuestion,
  hideQuestion,
  moveQuestion,
  reorderQuestion,
  type EditableField,
  type NewQuestion,
} from "../services/practiceEditor";

/** What the start card needs to render and drive the editor. */
export interface PracticeEditor {
  /** True while the list is showing the editor rather than Marie's controls. */
  editing: boolean;
  /** Turn the editor on, or off. The caller confirms an open inline edit first. */
  toggleEditing: () => void;
  /** True when a write may be attempted — i.e. none is in flight. */
  ready: boolean;
  /** True while a write is in flight, so every control can say so. */
  busy: boolean;
  /** The last write's failure sentence, or null. Never swallowed. */
  error: string | null;
  edit: (questionId: string, field: EditableField, value: string | null) => void;
  move: (questionId: string, direction: "up" | "down") => void;
  hide: (questionId: string, hidden: boolean) => void;
  /** Place a question where a drag dropped it. `before === null` = end of side. */
  reorder: (questionId: string, before: string | null) => void;
  add: (question: NewQuestion) => void;
}

/**
 * Hold the editor's state, and make its four writes.
 *
 * `onWritten` is called after every successful write; the page re-reads the deck
 * with it. `failureSentence` is a FUNCTION rather than a string because the
 * stored line arrives on the payload, and this hook is constructed before that
 * payload exists — reading it at failure time is what lets the notice be the
 * store's sentence rather than a literal.
 */
export function usePracticeEditor(
  slug: string,
  scenarioId: string,
  onWritten: () => void,
  failureSentence: () => string,
): PracticeEditor {
  const [editing, setEditing] = React.useState(false);
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  /**
   * Run one write.
   *
   * All four go through here, so the busy flag, the failure sentence and the
   * reload cannot be forgotten on one of four paths — which is exactly how an
   * editor ends up with one control that silently does nothing. There is no
   * guard clause: this function ALWAYS attempts the write, and says so if it
   * fails. The guard that used to sit here (`if (editingAs === "") return;`) is
   * the bug this hotfix exists to remove.
   */
  const run = (what: string, write: () => Promise<unknown>) => {
    setBusy(true);
    setError(null);
    write()
      .then(() => onWritten())
      .catch((caught: unknown) => {
        // eslint-disable-next-line no-console
        console.error(`practice editor: ${what} failed`, caught);
        setError(failureSentence());
      })
      .finally(() => setBusy(false));
  };

  return {
    editing,
    toggleEditing: () => setEditing((was) => !was),
    ready: !busy,
    busy,
    error,
    edit: (questionId, field, value) =>
      run("an edit", () => editQuestion(questionId, field, value)),
    move: (questionId, direction) =>
      run("a move", () => moveQuestion(questionId, direction)),
    reorder: (questionId, before) =>
      run("a drag", () => reorderQuestion(questionId, before)),
    hide: (questionId, hidden) =>
      run("a hide", () => hideQuestion(questionId, hidden)),
    // The only write that needs the case and the scenario: the other three
    // address a question by its own server-minted id, and only a CREATE has to
    // be told where to put the new row.
    add: (question) =>
      run("an add", () => addQuestion(slug, scenarioId, question)),
  };
}
