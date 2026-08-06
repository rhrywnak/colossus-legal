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

import AccusationFactPicker, { type PickableFact } from "./AccusationFactPicker";
import SentenceEditor from "./SentenceEditor";
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
  /** This scenario's INCLUDED facts — the only things markable or pairable. */
  includedFacts: PickableFact[];
  /** Re-read the section after any write. */
  onChanged: () => void;
}

/** What the picker is currently open FOR, and about which instance. */
type PickerState =
  | { mode: "mark" }
  | { mode: "pair"; anchor: string }
  | null;

const rowStyle: React.CSSProperties = {
  padding: "12px 0",
  borderBottom: DIVIDER,
  display: "flex",
  flexDirection: "column",
  gap: "0.35rem",
};

const codeStyle: React.CSSProperties = {
  fontWeight: 600,
  fontSize: "13px",
  color: "var(--text-secondary)",
};

const quoteStyle: React.CSSProperties = {
  margin: 0,
  fontSize: "13.5px",
  lineHeight: 1.55,
  color: "var(--text-primary)",
};

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
  includedFacts,
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

  /** The words of one fact, for the row that names it. */
  const textOf = (graphNodeId: string): string =>
    includedFacts.find((f) => f.graphNodeId === graphNodeId)?.text ?? graphNodeId;

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

        <div style={{ marginTop: "1rem" }}>
          {panel.instances.map((instance) => (
            <div key={instance.graph_node_id} style={rowStyle}>
              <span style={codeStyle}>{instance.code ?? instance.graph_node_id}</span>
              <p style={quoteStyle}>{textOf(instance.graph_node_id)}</p>

              {/* Our answer, directly beneath the instance it answers — the
                  design's shape. `answer_present` is the SERVER's verdict, so a
                  pairing whose answer has left says so rather than looking whole. */}
              {instance.answers_graph_node_id && (
                <p style={quoteStyle}>
                  <span style={codeStyle}>{w.answer_label}</span>{" "}
                  {instance.answer_present
                    ? textOf(instance.answers_graph_node_id)
                    : (instance.answer_code ?? instance.answers_graph_node_id)}
                </p>
              )}

              <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap" }}>
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
              </div>
            </div>
          ))}
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
        </div>

        {picker && (
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
              picker.mode === "mark"
                ? panel.instances.map((i) => i.graph_node_id)
                : [picker.anchor]
            }
            onCancel={() => setPicker(null)}
            onChoose={(chosen) =>
              run(() =>
                picker.mode === "mark"
                  ? markInstance(slug, scenarioId, chosen)
                  : pairAnswer(slug, scenarioId, picker.anchor, chosen),
              )
            }
          />
        )}

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
