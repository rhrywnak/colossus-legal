// =============================================================================
// questionDiscussView.ts — the "Discuss with AI" dock, as strings and choices
// =============================================================================
//
// CC_TASK_QUESTION_CHAT_v1. Pure (CLAUDE.md rule 30): the dock renders what this
// returns and decides nothing. Every sentence is a stored `discuss_*` row from the
// deck payload's wording; the model's NAME comes from the payload's model list
// (ruled amendment 2) — nothing here names a model.

import { wordingOf, type PracticeWording } from "../../services/practice";
import type { DiscussModel, DiscussionPayload, DiscussionTurn } from "../../services/practiceDiscussion";
import { pickByCount } from "../../utils/countWording";

/** Replace every `{name}` in a template. */
function fillAll(template: string, values: Record<string, string | number>): string {
  return Object.entries(values).reduce(
    (out, [name, value]) => out.split(`{${name}}`).join(String(value)),
    template,
  );
}

/**
 * Whether her draft rides this message: only when it has words AND differs from
 * the saved answer (GO ruling 11). An unchanged box is her saved answer.
 */
export function shouldSendDraft(draft: string, saved: string | null): boolean {
  const words = draft.trim();
  return words !== "" && words !== (saved ?? "").trim();
}

/** The model a picker starts on: the default row, if the list offers it. */
export function initialModel(payload: DiscussionPayload): string {
  return payload.models.some((m) => m.model_id === payload.default_model)
    ? payload.default_model
    : (payload.models[0]?.model_id ?? payload.default_model);
}

/** The model the picker shows, by id; `null` when the id is not offered. */
export function modelById(payload: DiscussionPayload, id: string): DiscussModel | null {
  return payload.models.find((m) => m.model_id === id) ?? null;
}

/** Everything the dock's chrome says. */
export interface DiscussChrome {
  title: string;
  subtitle: string;
  contextLine: string;
  footer: string;
  sending: string;
  /** `null` while the cap has room. */
  capReached: string | null;
}

/**
 * The dock's words for this payload, this model and this draft state.
 *
 * @param draftVisible whether her unsaved draft goes with the next message
 */
export function discussChrome(
  wording: PracticeWording,
  payload: DiscussionPayload,
  model: DiscussModel | null,
  draftVisible: boolean,
): DiscussChrome {
  const w = (key: string) => wordingOf(wording, key);
  const name = model?.display_name ?? payload.default_model;
  const cost = model?.billing_class === "local" ? w("discuss_cost_local") : w("discuss_cost_billed");
  const full = payload.model_turns >= payload.max_turns;
  return {
    title: fillAll(w("discuss_title_template"), { n: payload.position }),
    subtitle: fillAll(
      draftVisible ? w("discuss_subtitle_draft_template") : w("discuss_subtitle_template"),
      { code: payload.scenario_code },
    ),
    contextLine: draftVisible ? w("discuss_context_line_draft") : w("discuss_context_line"),
    footer: fillAll(w("discuss_footer_template"), { cost, model: name }),
    sending: fillAll(w("discuss_sending_template"), { model: name }),
    capReached: full
      ? fillAll(
          pickByCount(payload.max_turns, w("discuss_cap_reached_one"), w("discuss_cap_reached_template")),
          { max: payload.max_turns },
        )
      : null,
  };
}

/**
 * The small cost line under a model reply, or null when there is nothing to say
 * (a user turn, or a provider that reported no usage). Spend stays observable in
 * the dock rather than only in SQL.
 */
export function turnCost(turn: DiscussionTurn, wording: PracticeWording): string | null {
  if (turn.role !== "model" || turn.input_tokens === null || turn.output_tokens === null || turn.ms === null) {
    return null;
  }
  return fillAll(wordingOf(wording, "discuss_cost_template"), {
    input: turn.input_tokens,
    output: turn.output_tokens,
    seconds: (turn.ms / 1000).toFixed(1),
  });
}
