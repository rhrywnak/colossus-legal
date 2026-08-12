// =============================================================================
// AccusationSection — the working view's accusation block (task 2.11 B1)
// =============================================================================
//
// Roman's model, made editable: *"George portrayed Marie as unreasonable. He
// mentioned it 5 times in 5 different documents. Marie rebutted 5 times."* This
// section is where a human records both halves of that — which statements ARE
// them making the accusation, and what we said back to each one.
//
// B1 only. The rehearsal page that RENDERS this for Marie is B2; nothing here is
// that page, and nothing here is styled as it.
//
// ## Three things on screen, in the order the work is done
//
//   1. The accusation in plain words          (AccusationTextBlock)
//   2. Its instances, each with our answer    (the list below)
//   3. What still needs preparing             (the gap list — the prep list)
//
// ## The section discloses itself
//
// Roman's ruling after 2.13c: a feature discoverable only by hovering is a
// feature nobody finds. The hint line under the heading names the marking, the
// pairing, and what the gap list is for — in the store's words, so he can change
// them without a build.
//
// ## Every write re-reads; nothing is painted optimistically
//
// Server state is the one source of truth here, and it has to be: the count, the
// gaps and the answer-present flag are all DERIVED from what is stored, and a
// browser that painted a mark before the write landed would be showing a count it
// had computed itself — the exact thing §10 exclusion forbids, and the shape of
// the "102 remaining" beside "Not ruled (92)" defect.

import React, { useState } from "react";

import AccusationFactPicker from "./AccusationFactPicker";
import PairCard from "./PairCard";
import SentenceEditor from "./SentenceEditor";
import { includedPickableFacts } from "./accusationFacts";
import { pairCardFromScenarioCard } from "./pairCardModel";
import {
  absentStyle,
  DIVIDER,
  ghostButtonStyle,
  sectionHeaderStyle,
  sectionMetaStyle,
  sectionPaddedPanelStyle,
  sectionTitleStyle,
} from "./scenarioSectionStyles";
import {
  fillDetail,
  GAP_NO_ANSWER,
  markInstance,
  pairAnswer,
  setAccusationText,
  unmarkInstance,
  unpairAnswer,
  type AccusationPanelDto,
} from "../services/scenarioAccusation";
import type { AllegationOptions } from "../services/evidenceLinks";
import type { ScenarioCard } from "../services/scenarioCards";

interface Props {
  slug: string;
  scenarioId: string;
  /**
   * The section's payload, or `null` while it loads or after it failed.
   *
   * `null` renders NOTHING — there are no words to render with (R4), and a
   * section of unlabelled buttons is worse than an absent one. The page shows the
   * failure beside it.
   */
  panel: AccusationPanelDto | null;
  /**
   * This scenario's INCLUDED facts, as whole cards (task R4, P3).
   *
   * Whole cards rather than the three-field picker shape this section used to
   * take: the pair card renders the speaker, the kind, the pinpoint and the
   * context either side of the quote, and every one of those was already on the
   * payload and being thrown away. The picker's narrower shape is derived from
   * these here, so one list feeds both and they cannot disagree about what is
   * included.
   */
  includedCards: ScenarioCard[];
  /**
   * The card grammar — the served words the shared pair card speaks on THIS
   * page (the fold's show/hide pair, and the sentence for a statement whose
   * speaker the record does not name).
   *
   * `null` while the catalogue is unread, which withholds the instance list
   * rather than rendering cards with blank labels — the same honest-gap rule
   * this section already follows for `panel`.
   */
  options: AllegationOptions | null;
  /** Re-read the section after any write. */
  onChanged: () => void;
}

/** What the picker is currently open FOR, and about which instance. */
type PickerState =
  | { mode: "mark" }
  | { mode: "pair"; anchor: string }
  | null;

// REMOVED (task R4, P3): `rowStyle`, `codeStyle` and `quoteStyle`. They drew the
// bare code-and-a-line-of-text row that the shared `PairCard` replaced — the
// presentation Roman ruled against ("the rehearsal rendering wins"). The card
// owns its own visual language now, on both pages, which is the whole point of
// there being one of it.

const gapStyle: React.CSSProperties = {
  margin: 0,
  fontSize: "13px",
  lineHeight: 1.5,
};

/** How this surface renders the stored accusation sentence. */
const sentenceStyle: React.CSSProperties = {
  margin: 0,
  fontSize: "15px",
  lineHeight: 1.55,
  fontWeight: 500,
  color: "var(--text-primary)",
};

/** The box this surface types into. */
const textareaStyle: React.CSSProperties = {
  width: "100%",
  minHeight: "4.5rem",
  fontFamily: "inherit",
  fontSize: "14px",
  lineHeight: 1.55,
  padding: "10px 12px",
  border: DIVIDER,
  borderRadius: "8px",
  color: "var(--text-primary)",
  background: "var(--bg-surface)",
};

const AccusationSection: React.FC<Props> = ({
  slug,
  scenarioId,
  panel,
  includedCards,
  options,
  onChanged,
}) => {
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [picker, setPicker] = useState<PickerState>(null);

  if (!panel) return null;
  const w = panel.wording;

  /**
   * Run one write, then re-read.
   *
   * Every failure surfaces in the STORED sentence with the failure's own words
   * dropped into the slot it leaves — the only substitution this file performs.
   * A write that failed never closes the picker or clears the error.
   */
  const run = (write: () => Promise<void>) => {
    setBusy(true);
    write()
      .then(() => {
        setError(null);
        setPicker(null);
        onChanged();
      })
      .catch((e: unknown) => {
        const detail = e instanceof Error ? e.message : String(e);
        setError(fillDetail(w.save_failed_template, detail));
      })
      .finally(() => setBusy(false));
  };

  /** The picker's narrower shape, derived from the one included list. */
  const includedFacts = includedPickableFacts(includedCards);

  /** The whole card behind one node id, or `undefined` when it has left. */
  const cardOf = (graphNodeId: string): ScenarioCard | undefined =>
    includedCards.find((c) => c.graph_node_id === graphNodeId);

  /**
   * The picker, wired for whichever gesture opened it.
   *
   * ## Why ONE builder feeds two insertion points (task 394, P1)
   *
   * The two gestures now render the control in two different PLACES — pairing
   * inside the card being answered, marking under the Mark button — but they are
   * the same control with the same words, the same list and the same write path.
   * Writing it twice would be two things to keep in step, and the first thing to
   * drift would be the eligibility predicate, which is the one part of this
   * section the ruling says not to touch.
   *
   * ## Domain note: the predicate is UNCHANGED, deliberately
   *
   * Included-only, with the anchor itself excluded because a statement cannot be
   * its own answer. Nothing else. In particular:
   *
   *   · no already-paired exclusion — S-5 depends on reuse, where C-14 answers
   *     two different instances, and hiding it would break a READY scenario;
   *   · no stance filter — C-74 has to stay offerable.
   */
  const pickerFor = (mode: PickerState) => {
    if (!mode) return null;
    return (
      <AccusationFactPicker
        facts={includedFacts}
        prompt={w.picker_prompt}
        cancelLabel={w.picker_cancel_label}
        noMatchNotice={w.picker_no_match_notice}
        emptyNotice={w.picker_empty_notice}
        // Marking: hide what is already marked. Pairing: hide the instance
        // itself, because a statement cannot be its own answer — the backend
        // refuses it, and offering it would invite a refusal.
        excluded={
          mode.mode === "mark"
            ? panel.instances.map((i) => i.graph_node_id)
            : [mode.anchor]
        }
        onCancel={() => setPicker(null)}
        onChoose={(chosen) =>
          run(() =>
            mode.mode === "mark"
              ? markInstance(slug, scenarioId, chosen)
              : pairAnswer(slug, scenarioId, mode.anchor, chosen),
          )
        }
      />
    );
  };

  return (
    <section>
      <div style={sectionHeaderStyle}>
        <h2 style={sectionTitleStyle}>{w.section_heading}</h2>
        {/* Exactly one of these two is ever present — the backend decides, so
            this cannot render both or neither. */}
        <span style={sectionMetaStyle}>
          {panel.count_line ?? panel.no_instances_notice}
        </span>
      </div>

      <p style={{ margin: "0 0 0.6rem", fontSize: "0.8rem", color: "var(--text-secondary)" }}>
        {w.section_hint}
      </p>

      {error && (
        <div
          role="alert"
          style={{
            color: "var(--state-danger-strong)",
            fontSize: "0.82rem",
            marginBottom: "0.5rem",
          }}
        >
          {error}
        </div>
      )}

      <div style={sectionPaddedPanelStyle}>
        {/* The shared leaf, in this surface's visual language. Task 2.11 C
            generalised it so the rehearsal page's two sentence editors are the
            SAME component rather than a second copy of the same behaviour. */}
        <SentenceEditor
          text={panel.accusation_text}
          missingNotice={w.text_missing_notice}
          label={w.text_label}
          wording={{
            editLabel: w.text_edit_label,
            saveLabel: w.text_save_label,
            cancelLabel: w.text_cancel_label,
            withdrawLabel: w.text_clear_label,
            placeholder: w.text_placeholder,
          }}
          busy={busy}
          onSave={(text) => run(() => setAccusationText(slug, scenarioId, text))}
          sentenceStyle={sentenceStyle}
          buttonStyle={ghostButtonStyle}
          fieldStyle={textareaStyle}
        />

        <div style={{ marginTop: "1rem", display: "flex", flexDirection: "column", gap: "0.75rem" }}>
          {/* THE UNIFIED PAIR CARD (task R4, P3). The identical component the
              rehearsal page renders, with this page's controls at its edge.

              Roman's ruling was "the rehearsal rendering wins": this section used
              to show a code and a bare line of text, so the human doing the
              marking could not see what the human doing the rehearsing would
              read. Everything the card now shows — speaker, kind, pinpoint, the
              context around the quote — was already on the cards payload and was
              being discarded one lookup short of the screen. */}
          {options &&
            panel.instances.map((instance) => {
              const card = cardOf(instance.graph_node_id);
              // A marked instance whose fact has left the scenario. It is NOT
              // rendered as a card — there are no words to put in one — and the
              // gap list below names it, which is where the Remove law already
              // says this belongs.
              if (!card) return null;

              const answerCard =
                instance.answers_graph_node_id && instance.answer_present
                  ? cardOf(instance.answers_graph_node_id)
                  : undefined;

              return (
                <PairCard
                  key={instance.graph_node_id}
                  card={{
                    ...pairCardFromScenarioCard(
                      card,
                      options.card_grammar.speaker_absent_label,
                    ),
                    answer: answerCard
                      ? pairCardFromScenarioCard(
                          answerCard,
                          options.card_grammar.speaker_absent_label,
                        )
                      : null,
                  }}
                  answerLabel={w.answer_label}
                  // This page's own served fold words — the prep page passes its
                  // rehearsal wording instead. Neither speaks a literal.
                  showLabel={options.card_grammar.context_show_label}
                  hideLabel={options.card_grammar.context_hide_label}
                  // The working page names an unanswered instance in its gap list,
                  // which is the surface a human acts on. `null` here keeps that
                  // sentence in one place, as ruling C5 requires on the prep page
                  // for the same reason.
                  gapNotice={null}
                  controls={
                    <>
                      <button
                        type="button"
                        style={ghostButtonStyle}
                        disabled={busy}
                        onClick={() =>
                          setPicker({ mode: "pair", anchor: instance.graph_node_id })
                        }
                      >
                        {instance.answers_graph_node_id ? w.repair_label : w.pair_label}
                      </button>
                      {instance.answers_graph_node_id && (
                        <button
                          type="button"
                          style={ghostButtonStyle}
                          disabled={busy}
                          onClick={() =>
                            run(() => unpairAnswer(slug, scenarioId, instance.graph_node_id))
                          }
                        >
                          {w.unpair_label}
                        </button>
                      )}
                      <button
                        type="button"
                        style={ghostButtonStyle}
                        disabled={busy}
                        onClick={() =>
                          run(() => unmarkInstance(slug, scenarioId, instance.graph_node_id))
                        }
                      >
                        {w.unmark_label}
                      </button>
                    </>
                  }
                  // THE .393 BLOCKER, closed (task 394, P1). The picker opened
                  // by THIS card's Pair button renders inside THIS card, under
                  // the button that asked for it. Every other card renders
                  // nothing here, so there is exactly one picker on screen and
                  // it is never somewhere else.
                  //
                  // What it replaced: one picker at a fixed point at the bottom
                  // of the panel, 777/592/407/92 pixels below the four buttons
                  // that could open it. It worked every time and was invisible
                  // for every card but the last.
                  expansion={
                    picker?.mode === "pair" && picker.anchor === instance.graph_node_id
                      ? pickerFor(picker)
                      : undefined
                  }
                />
              );
            })}
        </div>

        <div style={{ marginTop: "0.75rem" }}>
          <button
            type="button"
            style={ghostButtonStyle}
            disabled={busy}
            onClick={() => setPicker({ mode: "mark" })}
          >
            {w.mark_label}
          </button>

          {/* MARK MODE STAYS WHERE IT IS. Its control is right here, so the
              picker is already adjacent to the button that opens it — the
              defect P1 fixes was never this one, and moving a working control
              to match a broken one's fix would be change for symmetry's sake. */}
          {picker?.mode === "mark" && pickerFor(picker)}
        </div>

        {/* The prep list. Named as work, and the unanswered ones read loudest
            because they are the ones a human can act on today. */}
        <div style={{ marginTop: "1rem" }}>
          <h3 style={{ ...sectionTitleStyle, fontSize: "13.5px" }}>{w.gaps_heading}</h3>
          {panel.gaps.length === 0 ? (
            <p style={{ ...absentStyle, margin: "0.4rem 0 0" }}>{w.no_gaps_notice}</p>
          ) : (
            <ul style={{ margin: "0.4rem 0 0", paddingLeft: "1.1rem" }}>
              {panel.gaps.map((gap) => (
                <li key={`${gap.kind}:${gap.graph_node_id}`} style={{ marginBottom: "0.3rem" }}>
                  {/* Branching on the TOKEN, never on the message: the sentence is
                      Roman's to edit, and a client matching on "NO ANSWER" would
                      silently stop telling the kinds apart the day he did. */}
                  <p
                    style={{
                      ...gapStyle,
                      fontWeight: gap.kind === GAP_NO_ANSWER ? 600 : 400,
                      color:
                        gap.kind === GAP_NO_ANSWER
                          ? "var(--state-danger-strong)"
                          : "var(--text-secondary)",
                    }}
                  >
                    {gap.message}
                  </p>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </section>
  );
};

export default AccusationSection;
