// =============================================================================
// AskModelPicker.tsx — the Chat page's model select, and its three states
// =============================================================================
//
// CC_TASK_CHAT_DEFAULT_MODEL_v1, ruling 4b. Extracted from `AskPage.tsx`, which
// was already over Rule 17's 300 lines before this task added an error state to
// it (Roman's ruling of 2026-09-19: extract the picker).
//
// ## Three states, and why they must look different
//
// LOADING — the catalogue is in flight. Disabled, and says so.
// FAILED  — the catalogue could not be loaded. Disabled, and says THAT, because
//           until 2026-09-19 this state was invisible: the service caught its
//           own failure and returned one hand-written model, so a page that
//           could not reach its server looked like a deployment offering one
//           model. Chat still works here — an absent `model` field means "use
//           the server default" — so the picker refuses without blocking the ask.
// READY   — the models, as served.
//
// An EMPTY list on the READY path is a fourth, legitimate state: a deployment
// with no active Anthropic row. It reads as its own sentence rather than as the
// loading one, because "nothing is offered" and "nothing has arrived yet" are
// different facts and only one of them is worth waiting on.

import React from "react";

import type { ChatModel } from "../../services/ask";

/** What the page knows about the catalogue. */
export type ModelCatalogue =
  | { state: "loading" }
  | { state: "failed"; detail: string }
  | { state: "ready"; models: ChatModel[] };

interface Props {
  catalogue: ModelCatalogue;
  /** The id the next ask will carry. `""` means "let the server decide". */
  selected: string;
  onSelect: (modelId: string) => void;
}

/**
 * The one-line placeholder a disabled select shows, per state.
 *
 * Exported so the wording is testable without rendering — there is no
 * component-testing infrastructure in this project (CLAUDE.md rule 30), and the
 * whole of ruling 4b is which sentence appears when.
 */
export function placeholderFor(catalogue: ModelCatalogue): string | null {
  switch (catalogue.state) {
    case "loading":
      return "Loading models…";
    case "failed":
      return "Models unavailable";
    case "ready":
      return catalogue.models.length === 0 ? "No models configured" : null;
  }
}

/** True when there is nothing to choose between. */
export function isDisabled(catalogue: ModelCatalogue): boolean {
  return placeholderFor(catalogue) !== null;
}

/**
 * Does the catalogue currently OFFER this model?
 *
 * Asked when a history entry names the model it was answered with: the picker
 * may be moved back to it, but only if it is still on offer. A catalogue that
 * is loading or failed offers nothing — moving the picker to a model this build
 * cannot show would leave a `<select>` whose value is not one of its options,
 * which renders as blank and sends that id on the next ask.
 */
export function offers(catalogue: ModelCatalogue, modelId: string): boolean {
  return catalogue.state === "ready" && catalogue.models.some((m) => m.model_id === modelId);
}

const AskModelPicker: React.FC<Props> = ({ catalogue, selected, onSelect }) => {
  const placeholder = placeholderFor(catalogue);
  const disabled = placeholder !== null;
  const models = catalogue.state === "ready" ? catalogue.models : [];

  return (
    <div
      style={{
        position: "absolute",
        bottom: "12px",
        right: "58px",
        display: "flex",
        alignItems: "center",
      }}
    >
      <select
        value={selected}
        onChange={(e) => onSelect(e.target.value)}
        disabled={disabled}
        data-ask-model-picker
        data-catalogue-state={catalogue.state}
        style={{
          appearance: "none",
          WebkitAppearance: "none",
          border: "1px solid var(--border-default)",
          borderRadius: "6px",
          padding: "4px 24px 4px 8px",
          fontSize: "0.78rem",
          color: "var(--text-secondary)",
          backgroundColor: "var(--bg-page)",
          // `wait` only while something IS awaited. A failed catalogue is not
          // coming, and a spinner cursor over it would promise otherwise.
          cursor: catalogue.state === "loading" ? "wait" : disabled ? "not-allowed" : "pointer",
          fontFamily: "inherit",
          backgroundImage: `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%2364748b' stroke-width='2'%3E%3Cpath d='M6 9l6 6 6-6'/%3E%3C/svg%3E")`,
          backgroundRepeat: "no-repeat",
          backgroundPosition: "right 6px center",
          outline: "none",
        }}
        // The failure's cause on hover: the sentence in the select is four
        // words, and the person who can fix it needs the HTTP status.
        title={catalogue.state === "failed" ? catalogue.detail : "Chat model"}
      >
        {placeholder !== null ? (
          <option value="" disabled>
            {placeholder}
          </option>
        ) : (
          models.map((m) => (
            <option key={m.model_id} value={m.model_id}>
              {m.display_name.replace("Claude ", "")}
            </option>
          ))
        )}
      </select>
    </div>
  );
};

export default AskModelPicker;
