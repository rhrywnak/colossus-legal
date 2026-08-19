// =============================================================================
// PracticeAddQuestion.tsx — "+ Add a question" (task B1)
// =============================================================================
//
// Side · tactic · what it attaches to, then the question and an optional
// watch-for.
//
// ## Why "Side" asks ONE question and not two
//
// The form's three choices are cross, direct and redirect — which is the KIND,
// and the side follows from it: a cross is George's and the other two are
// Chuck's. Asking for both would let somebody file a cross question on Chuck's
// side, which is not a thing that exists.
//
// ## What the picker offers, and what it does not
//
// This scenario's ruled instances and talking points, labelled server-side, plus
// the stored "no receipt" — which is the absence of a choice rather than a thing
// to attach to, and is therefore the form's default and not a picker row.

import React from "react";

import type {
  PracticeAttachOption,
  PracticeWording,
} from "../../services/practice";
import type { NewQuestion } from "../../services/practiceEditor";
import { wordingOf } from "../../services/practice";
import * as e from "./practiceEditorStyles";
import * as s from "./practiceStyles";

interface Props {
  wording: PracticeWording;
  attachOptions: PracticeAttachOption[];
  onAdd: (question: NewQuestion) => void;
  onCancel: () => void;
  /** True when a write may be attempted — somebody is signing, none in flight. */
  ready: boolean;
}

/** The three kinds, in the mockup's order, with their stored labels. */
const KINDS: Array<{ value: NewQuestion["kind"]; key: string }> = [
  { value: "cross", key: "editor_side_cross" },
  { value: "direct", key: "editor_side_direct" },
  { value: "redirect", key: "editor_side_redirect" },
];

const PracticeAddQuestion: React.FC<Props> = ({
  wording,
  attachOptions,
  onAdd,
  onCancel,
  ready,
}) => {
  const w = (key: string) => wordingOf(wording, key);
  const [kind, setKind] = React.useState<NewQuestion["kind"]>("cross");
  const [tactic, setTactic] = React.useState("");
  const [attach, setAttach] = React.useState("");
  const [text, setText] = React.useState("");
  const [follows, setFollows] = React.useState("");
  const [watchFor, setWatchFor] = React.useState("");

  const chosen = attachOptions.find(
    (option) => `${option.source_kind}:${option.source_index}` === attach,
  );

  return (
    <div style={e.addBox}>
      <b>{w("editor_add_heading")}</b>

      <div style={{ ...e.addGrid, marginTop: 6 }}>
        <div>
          <label style={e.editLabel}>{w("editor_field_side")}</label>
          <select
            style={e.editInput}
            value={kind}
            onChange={(event) => setKind(event.target.value as NewQuestion["kind"])}
            aria-label={w("editor_field_side")}
          >
            {KINDS.map((option) => (
              <option key={option.value} value={option.value}>
                {w(option.key)}
              </option>
            ))}
          </select>
        </div>

        <div>
          <label style={e.editLabel}>{w("editor_field_tactic")}</label>
          <input
            style={e.editInput}
            value={tactic}
            // A tactic belongs to a cross question and nowhere else; the server
            // refuses one on the other two, so the control withdraws rather than
            // offering something that would be rejected.
            disabled={kind !== "cross"}
            onChange={(event) => setTactic(event.target.value)}
            aria-label={w("editor_field_tactic")}
          />
        </div>

        <div>
          <label style={e.editLabel}>{w("editor_field_attach")}</label>
          <select
            style={e.editInput}
            value={attach}
            onChange={(event) => setAttach(event.target.value)}
            aria-label={w("editor_field_attach")}
          >
            <option value="">{w("editor_attach_none")}</option>
            {attachOptions.map((option) => (
              <option
                key={`${option.source_kind}:${option.source_index}`}
                value={`${option.source_kind}:${option.source_index}`}
              >
                {option.label}
              </option>
            ))}
          </select>
        </div>
      </div>

      {kind === "redirect" && (
        <div style={e.editRow}>
          <label style={e.editLabel}>{w("editor_field_follows")}</label>
          <input
            style={e.editInput}
            value={follows}
            onChange={(event) => setFollows(event.target.value)}
            aria-label={w("editor_field_follows")}
          />
        </div>
      )}

      <div style={e.editRow}>
        <label style={e.editLabel}>{w("editor_field_question")}</label>
        <textarea
          style={e.editTextarea}
          placeholder={w("editor_question_placeholder")}
          value={text}
          onChange={(event) => setText(event.target.value)}
          aria-label={w("editor_field_question")}
        />
      </div>

      <div style={e.editRow}>
        <label style={e.editLabel}>{w("editor_field_watch_for")}</label>
        <input
          style={e.editInput}
          value={watchFor}
          onChange={(event) => setWatchFor(event.target.value)}
          aria-label={w("editor_field_watch_for")}
        />
      </div>

      <div style={{ ...s.row, marginTop: 8 }}>
        <button
          type="button"
          style={s.buttonPrimary}
          disabled={!ready || text.trim() === ""}
          onClick={() =>
            onAdd({
              kind,
              text: text.trim(),
              tactic: kind === "cross" && tactic.trim() !== "" ? Number(tactic) : null,
              follows: kind === "redirect" && follows.trim() !== "" ? follows.trim() : null,
              watch_for: watchFor.trim() === "" ? null : watchFor.trim(),
              source_kind: chosen?.source_kind ?? null,
              source_index: chosen?.source_index ?? null,
            })
          }
        >
          {w("editor_add_button")}
        </button>
        <button type="button" style={s.button} onClick={onCancel}>
          {w("editor_cancel_label")}
        </button>
        <span style={{ ...s.sub, fontSize: 13 }}>{w("editor_add_hint")}</span>
      </div>
    </div>
  );
};

export default PracticeAddQuestion;
