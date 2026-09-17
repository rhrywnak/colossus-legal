// =============================================================================
// questionDiscuss.test.tsx — the dock's words, choices and markup
// =============================================================================

import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { DiscussButton } from "../QuestionDiscussDock";
import QuestionDiscussDrawer from "../QuestionDiscussDrawer";
import { discussChrome, initialModel, modelById, shouldSendDraft, turnCost } from "../questionDiscussView";
import type { DiscussionPayload } from "../../../services/practiceDiscussion";

/** The migration's own values for the dock's rows. */
const WORDING: Record<string, string> = {
  discuss_button_label: "Discuss with AI",
  discuss_title_template: "Discuss · Question {n}",
  discuss_subtitle_template: "{code}",
  discuss_subtitle_draft_template: "{code} · your draft answer is visible to the model",
  discuss_context_line: "The model sees: the question · your current answer",
  discuss_context_line_draft: "The model sees: the question · your unsaved draft",
  discuss_input_placeholder: "Ask about this question or your answer…",
  discuss_send_label: "Send",
  discuss_close_label: "Close the discussion",
  discuss_model_label: "Model",
  discuss_footer_template: "Thread is saved on this question · visible to Marie, Chuck and Roman · {cost} on {model}",
  discuss_cost_billed: "$ per turn",
  discuss_cost_local: "$0 per turn",
  discuss_cost_template: "{input} in · {output} out · {seconds} s",
  discuss_empty: "No discussion yet — ask the first question.",
  discuss_sending_template: "Waiting for {model}…",
  discuss_send_failed: "No reply came back. Your message is saved above — send again to retry.",
  discuss_load_failed: "The discussion could not be loaded.",
  discuss_cap_reached_template: "This question has reached its limit of {max} model replies.",
  discuss_cap_reached_one: "This question has reached its limit of {max} model reply.",
};

function payload(over: Partial<DiscussionPayload> = {}): DiscussionPayload {
  return {
    question_id: "q-4",
    scenario_code: "S-5",
    position: 4,
    turns: [
      { id: "t1", role: "user", author: "Marie", model_id: null, text: "How do I say this?", when: "2:52 pm", input_tokens: null, output_tokens: null, ms: null },
      { id: "t2", role: "model", author: "Claude Opus 5", model_id: "claude-opus-5", text: "Lead with the letter.", when: "2:52 pm", input_tokens: 2377, output_tokens: 327, ms: 6308 },
    ],
    models: [
      { model_id: "claude-opus-5", display_name: "Claude Opus 5", billing_class: "billed" },
      { model_id: "qwen", display_name: "Qwen (local)", billing_class: "local" },
    ],
    default_model: "claude-opus-5",
    max_turns: 40,
    model_turns: 1,
    ...over,
  };
}

describe("shouldSendDraft", () => {
  it("sends a draft only when it has words and differs from the saved answer", () => {
    expect(shouldSendDraft("new words", "old words")).toBe(true);
    expect(shouldSendDraft("new words", null)).toBe(true);
    expect(shouldSendDraft("  old words ", "old words")).toBe(false);
    expect(shouldSendDraft("   ", "old words")).toBe(false);
  });
});

describe("discussChrome", () => {
  it("fills the title, subtitle, footer and sending line from the store and the model list", () => {
    const p = payload();
    const chrome = discussChrome(WORDING, p, modelById(p, "claude-opus-5"), false);
    expect(chrome.title).toBe("Discuss · Question 4");
    expect(chrome.subtitle).toBe("S-5");
    expect(chrome.contextLine).toBe("The model sees: the question · your current answer");
    expect(chrome.footer).toBe(
      "Thread is saved on this question · visible to Marie, Chuck and Roman · $ per turn on Claude Opus 5",
    );
    expect(chrome.sending).toBe("Waiting for Claude Opus 5…");
    expect(chrome.capReached).toBeNull();
  });

  it("flips to the draft variants while her draft goes with the message", () => {
    const chrome = discussChrome(WORDING, payload(), null, true);
    expect(chrome.subtitle).toBe("S-5 · your draft answer is visible to the model");
    expect(chrome.contextLine).toBe("The model sees: the question · your unsaved draft");
  });

  it("prices a local model with the local cost word", () => {
    const p = payload();
    expect(discussChrome(WORDING, p, modelById(p, "qwen"), false).footer).toContain("$0 per turn on Qwen (local)");
  });

  it("says the cap is reached at the limit, singular at a cap of one", () => {
    expect(discussChrome(WORDING, payload({ model_turns: 40 }), null, false).capReached).toBe(
      "This question has reached its limit of 40 model replies.",
    );
    expect(discussChrome(WORDING, payload({ model_turns: 1, max_turns: 1 }), null, false).capReached).toBe(
      "This question has reached its limit of 1 model reply.",
    );
  });

  it("starts on the settings row's default model when the list offers it", () => {
    expect(initialModel(payload())).toBe("claude-opus-5");
    expect(initialModel(payload({ default_model: "gone" }))).toBe("claude-opus-5");
  });
});

describe("turnCost", () => {
  it("prints a model reply's tokens and seconds, and nothing for a user turn or unreported usage", () => {
    const [user, model] = payload().turns;
    expect(turnCost(model, WORDING)).toBe("2377 in · 327 out · 6.3 s");
    expect(turnCost(user, WORDING)).toBeNull();
    expect(turnCost({ ...model, input_tokens: null }, WORDING)).toBeNull();
  });
});

describe("DiscussButton", () => {
  it("prints the stored label — never a model name (M: hardcode a name and this reds)", () => {
    expect(renderToStaticMarkup(<DiscussButton wording={WORDING} onOpen={() => {}} />)).toContain(
      ">Discuss with AI</button>",
    );
    const renamed = { ...WORDING, discuss_button_label: "Talk it through" };
    expect(renderToStaticMarkup(<DiscussButton wording={renamed} onOpen={() => {}} />)).toContain(
      ">Talk it through</button>",
    );
  });
});

describe("QuestionDiscussDrawer markup", () => {
  const p = payload();
  const html = renderToStaticMarkup(
    <QuestionDiscussDrawer
      chrome={discussChrome(WORDING, p, modelById(p, "claude-opus-5"), false)}
      turns={p.turns}
      models={p.models}
      model="claude-opus-5"
      onModel={() => {}}
      text=""
      onText={() => {}}
      onSend={() => {}}
      onClose={() => {}}
      sending={false}
      error={null}
      costOf={(turn) => turnCost(turn, WORDING)}
      labels={{ placeholder: "Ask…", send: "Send", close: "Close the discussion", model: "Model", empty: "Empty" }}
    />,
  );

  it("renders a picker over the models, by display name", () => {
    expect(html).toContain("data-discuss-picker");
    expect(html).toMatch(/<option value="claude-opus-5"[^>]*>Claude Opus 5<\/option>/);
    expect(html).toMatch(/<option value="qwen"[^>]*>Qwen \(local\)<\/option>/);
  });

  it("renders the thread: a person's name alone, a model's name · time", () => {
    expect(html).toContain('data-turn="user"');
    expect(html).toContain(">Marie<");
    expect(html).toContain(">Claude Opus 5 · 2:52 pm<");
    expect(html).toContain(">Lead with the letter.<");
    expect(html).toMatch(/data-turn-cost="true">2377 in · 327 out · 6\.3 s</);
  });

  it("closes with the ✕ button, named for a screen reader", () => {
    expect(html).toMatch(/<button[^>]*aria-label="Close the discussion"[^>]*>✕<\/button>/);
  });
});
