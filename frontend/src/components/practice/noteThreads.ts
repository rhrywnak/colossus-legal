// noteThreads.ts — a note and the replies under it (CC_TASK_FOR_YOU_v1 L3).
//
// The panel draws an exchange two lines deep: what Chuck wrote, and what Marie
// wrote back under it. Which notes belong under which is a pure question about
// a list, so it is answered here and tested here — there is no component-test
// rig in this project (CLAUDE.md rule 30), and the property worth holding down
// is a NEGATIVE: that no note can be lost by being threaded.

import type { PracticeNote } from "../../services/practice";

/** One note, with whatever answers it, oldest first. */
export type NoteThread = {
  note: PracticeNote;
  replies: PracticeNote[];
};

/**
 * The notes as exchanges: each note that answers nothing, with its answers.
 *
 * ## Domain note: an ORPHAN reply is still shown
 *
 * A reply whose parent is not in this list — struck and filtered elsewhere, on
 * a superseded attempt, or simply not rendered on this panel — is drawn as a
 * note of its own rather than dropped. Losing it would be the silent failure
 * this whole feature is built against: somebody wrote something and nobody ever
 * saw it. Its own row still says who wrote it and when.
 *
 * ## Why the order is the input's
 *
 * The server sends notes oldest first, and an exchange reads in the order it
 * happened. Nothing here sorts; a panel that re-ordered a conversation would
 * make a reply look like it came before the thing it answers.
 */
export function threadNotes(notes: PracticeNote[]): NoteThread[] {
  const byId = new Set(notes.map((note) => note.id));
  const threads: NoteThread[] = [];
  const index = new Map<string, NoteThread>();

  for (const note of notes) {
    const parent = note.answers_note_id;
    // A root: it answers nothing, or it answers something this panel is not
    // showing (see the doc comment above).
    if (parent === undefined || !byId.has(parent)) {
      const thread: NoteThread = { note, replies: [] };
      threads.push(thread);
      index.set(note.id, thread);
      continue;
    }
    const under = index.get(parent);
    if (under === undefined) {
      // The parent is in the list but has not been seen yet — only possible if
      // a reply precedes its parent, which the server's oldest-first order
      // rules out. Drawn as a root rather than dropped, for the same reason an
      // orphan is.
      const thread: NoteThread = { note, replies: [] };
      threads.push(thread);
      index.set(note.id, thread);
      continue;
    }
    under.replies.push(note);
    // A reply to a reply is filed under the same root by the SERVER, so this
    // never nests further. Indexed anyway, so that if one ever arrived it would
    // land beside its sibling instead of vanishing.
    index.set(note.id, under);
  }
  return threads;
}
