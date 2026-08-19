// =============================================================================
// usePracticeEditor.ts — the deck editor's state and its four writes (task B1)
// =============================================================================
//
// Extracted from the start card for the reason `usePracticeDeckControls` was:
// this is a coherent thing on its own — everything Chuck and Roman can do to the
// deck between sittings — and it would otherwise carry `PracticePage` past Rule
// 17 twice over.
//
// ## Why `editingAs` lives here and gates every write
//
// There is one login, and "Editing as Chuck" is the honest substitute for the
// account separation this build does not have. The record is only worth anything
// if it is impossible to change the deck without signing it, so the hook refuses
// to call anything while nobody is chosen — and the SERVER refuses too, which is
// what makes this more than a courtesy.
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
  toggleEditing: () => void;
  /** Who is signing changes. `""` until somebody is chosen. */
  editingAs: string;
  setEditingAs: (who: string) => void;
  /** True when a write may be attempted — signed, and none in flight. */
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
  const [editingAs, setEditingAs] = React.useState("");
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  /**
   * Run one write, or refuse it because nobody has signed.
   *
   * All four go through here, so the busy flag, the failure sentence and the
   * reload cannot be forgotten on one of four paths — which is exactly how an
   * editor ends up with one control that silently does nothing.
   */
  const run = (what: string, write: () => Promise<unknown>) => {
    if (editingAs === "") return;
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
    editingAs,
    setEditingAs,
    ready: editingAs !== "" && !busy,
    busy,
    error,
    edit: (questionId, field, value) =>
      run("an edit", () => editQuestion(questionId, field, value, editingAs)),
    move: (questionId, direction) =>
      run("a move", () => moveQuestion(questionId, direction, editingAs)),
    reorder: (questionId, before) =>
      run("a drag", () => reorderQuestion(questionId, before)),
    hide: (questionId, hidden) =>
      run("a hide", () => hideQuestion(questionId, hidden, editingAs)),
    // The only write that needs the case and the scenario: the other three
    // address a question by its own server-minted id, and only a CREATE has to
    // be told where to put the new row.
    add: (question) =>
      run("an add", () => addQuestion(slug, scenarioId, question, editingAs)),
  };
}
