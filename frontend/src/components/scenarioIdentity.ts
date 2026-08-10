// =============================================================================
// scenarioIdentity.ts — the identity modal's pure rules (task 1.7B)
// =============================================================================
//
// The modal edits a scenario's identity: its name, THREE distinct texts, its
// anchor allegations, and its definition body. What goes in the draft and what
// comes back out as a PUT body is decided here, pure and tested — the component
// renders fields and calls these.
//
// ## Domain note: the three texts are never collapsed
//
// They are three sentences with three points of view, and the C1 migration says
// in terms that merging any two destroys what rehearsal mode reads:
//
//   attack_text     — what THEY say         ("Marie is obstructive")
//   theme_statement — how WE answer it       (the one-line tagline)
//   motivation      — what they want the jury to believe by saying it
//
// They also live in three different places — `attack_text` inside the
// `definition` jsonb, the other two as columns — so the mapping below is the one
// place that knows which field goes where.
//
// ## The target joined the draft on 2026-08-07
//
// This modal used to carry `target` THROUGH untouched — it was in the definition
// and the modal refused to look at it. That was tenable only while something
// else supplied one, and nothing did: the create form could not author a
// definition at all, so every UI-created scenario had no target, and the backend
// quietly gathered evidence over the case-default subject instead. The modal is
// now where an existing scenario's target is authored and changed, which is what
// makes completing a legacy scenario a UI action rather than a SQL one.
//
// ## Why `direction` is absent from the patch
//
// A scenario's offense/defense stance is its identity, not an attribute: the
// backend refuses it on the update route (`ScenarioUpdateRequest` has no such
// field and is `deny_unknown_fields`, so sending one is a 400), and a wrongly
// created direction is cured by archive-and-recreate, not by mutation. The modal
// SHOWS it as a chip and never offers to change it.

import { CURRENT_SCHEMA_V, type ScenarioDefinition } from "../pages/trialPrepData";
import type { ScenarioUpdatePayload } from "../services/scenarioCrud";

/** Everything the modal holds while it is open. Strings, never `null`: an input
 *  bound to `null` is React's uncontrolled-component warning waiting to happen,
 *  and "absent" and "cleared" are re-separated on the way out (see `patchFrom`). */
export interface IdentityDraft {
  name: string;
  /** THEIR framing — `definition.attack_text`. */
  attackText: string;
  /** A gloss of their framing — `definition.attack_meaning`. */
  attackMeaning: string;
  /** OUR one-line answer — the `theme_statement` column. */
  themeStatement: string;
  /** What they want the jury to believe — the `motivation` column. */
  motivation: string;
  /** WHO the attack is about — `definition.target`, a party node id from the
   *  live vocabulary. `""` means "no target", which is a scenario that gathers
   *  nothing (and says so). */
  target: string;
  /** Complaint paragraphs this scenario touches — chips in the modal. */
  anchorAllegationIds: string[];
}

/** What the modal needs to seed itself, from the two payloads that carry it. */
export interface IdentitySource {
  name: string;
  themeStatement: string | null;
  motivation: string | null;
  definition?: ScenarioDefinition;
  anchorAllegationIds: string[];
}

/**
 * Seed the draft. Absent text becomes an empty field — never the string "null",
 * and never a placeholder sentence: invented prose sitting in an input is
 * indistinguishable, ten minutes later, from something a human wrote.
 */
export function draftFrom(source: IdentitySource): IdentityDraft {
  return {
    name: source.name,
    attackText: source.definition?.attack_text ?? "",
    attackMeaning: source.definition?.attack_meaning ?? "",
    themeStatement: source.themeStatement ?? "",
    motivation: source.motivation ?? "",
    target: source.definition?.target ?? "",
    anchorAllegationIds: [...source.anchorAllegationIds],
  };
}

/**
 * The PUT body for a draft.
 *
 * ## Why the definition is sent WHOLE
 *
 * The backend replaces the definition blob rather than deep-merging it, so the
 * field the modal does not edit — `wielders` — must be carried through from the
 * existing definition or it would be dropped. That is why this takes the
 * original definition as well as the draft: the modal is not the only author of
 * that body, and it must not act as though it were.
 *
 * ## Why an empty attack text omits the definition entirely
 *
 * `attack_text` is REQUIRED by the backend's parse contract. A draft with none
 * is a scenario that has not been framed yet, and sending `{ attack_text: "" }`
 * would store an empty framing that reads later as "someone wrote nothing here"
 * rather than "nobody has written this yet". Omitting the key leaves the column
 * untouched, which is what "I only changed the name" should mean.
 *
 * That omission is also why {@link definitionWouldBeLost} exists: a target or a
 * gloss inside a definition that is never sent is an edit the human made and the
 * system discarded. The modal refuses those combinations instead of letting this
 * function silently swallow them.
 */
export function patchFrom(
  draft: IdentityDraft,
  existing?: ScenarioDefinition,
): ScenarioUpdatePayload {
  const patch: ScenarioUpdatePayload = {
    name: draft.name.trim(),
    theme_statement: draft.themeStatement.trim(),
    motivation: draft.motivation.trim(),
    anchor_allegation_ids: draft.anchorAllegationIds,
  };

  const attackText = draft.attackText.trim();
  if (attackText.length > 0) {
    const meaning = draft.attackMeaning.trim();
    const target = draft.target.trim();
    patch.definition = {
      ...(existing ?? { wielders: [], schema_v: CURRENT_SCHEMA_V }),
      attack_text: attackText,
      // An emptied gloss is a REMOVAL, not a blank string: the field is
      // `skip_serializing_if = "Option::is_none"` on the backend, and storing ""
      // would make "cleared" and "written as empty" the same stored state.
      ...(meaning.length > 0 ? { attack_meaning: meaning } : { attack_meaning: undefined }),
      // Same rule for the target, and it matters more: a stored `target: ""`
      // would be trimmed to "no target" by the resolver anyway, but it would
      // read in the column as a choice somebody made. `undefined` removes the
      // key, so "cleared the target" and "never chose one" stay one state with
      // one meaning — the scenario gathers nothing, and says so.
      ...(target.length > 0 ? { target } : { target: undefined }),
      schema_v: CURRENT_SCHEMA_V,
    };
  }
  return patch;
}

/**
 * Which typed definition field saving this draft would silently discard.
 *
 * `null` when nothing would be lost.
 *
 * ## What {@link patchFrom} does, and why anything can be lost at all
 *
 * `attack_text` is required by the backend's parse contract, so a draft with a
 * blank one has no valid definition to send and `patchFrom` omits the definition
 * ENTIRELY. Every other field that lives inside that object goes with it. The
 * name, theme and motivation are columns and survive; `target` and
 * `attack_meaning` are not, and do not.
 *
 * ## Why this replaced `targetWouldBeLost` (task R1 Piece 5a)
 *
 * The old predicate guarded exactly one of the two. A human who typed into
 * "what that is meant to imply" and left "what they say" blank passed
 * {@link canSave}, watched the modal close on a successful write, and lost the
 * sentence they had just written with nothing said — audit defect 16, the same
 * silent-discard shape the target guard was built to close, one field over.
 *
 * ## Why it returns WHICH field rather than a boolean
 *
 * Two operationally distinct states get two different observables (Standing
 * Rule 1). "You picked a person" and "you wrote a sentence" need different
 * instructions, and a refusal that cannot say which of your edits is about to be
 * thrown away is barely better than the silence it replaces. The modal maps each
 * answer to its own stored sentence.
 *
 * The target is reported FIRST when both apply: it is the field with the wider
 * blast radius — a target decides what evidence the scenario can even see — so
 * it is the one to name when a human has somehow done both.
 */
export type LostDefinitionField = "target" | "meaning";

export function definitionWouldBeLost(draft: IdentityDraft): LostDefinitionField | null {
  // Everything below only matters when the definition is about to be omitted.
  if (draft.attackText.trim().length > 0) return null;
  if (draft.target.trim().length > 0) return "target";
  if (draft.attackMeaning.trim().length > 0) return "meaning";
  return null;
}

/**
 * Can this draft be saved?
 *
 * The name is load-bearing — it is `NOT NULL` on the row and is what every other
 * surface calls this scenario. The three texts are all legitimately empty on a
 * scenario nobody has framed yet, and refusing to save until they are written
 * would mean a human could not fix a typo in the name without first inventing a
 * theme statement.
 *
 * The second clause is not a new demand on the human either — it refuses an edit
 * this payload shape cannot carry, which is the only kind of refusal that saves
 * work rather than costing it (see {@link definitionWouldBeLost}).
 */
export function canSave(draft: IdentityDraft): boolean {
  return draft.name.trim().length > 0 && definitionWouldBeLost(draft) === null;
}

/** Add an allegation chip, ignoring a duplicate rather than stacking it. */
export function withAllegation(draft: IdentityDraft, id: string): IdentityDraft {
  if (draft.anchorAllegationIds.includes(id)) return draft;
  return { ...draft, anchorAllegationIds: [...draft.anchorAllegationIds, id] };
}

/** Remove an allegation chip. */
export function withoutAllegation(draft: IdentityDraft, id: string): IdentityDraft {
  return {
    ...draft,
    anchorAllegationIds: draft.anchorAllegationIds.filter((a) => a !== id),
  };
}
