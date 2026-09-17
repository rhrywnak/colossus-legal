// =============================================================================
// QuestionDiscussDock.tsx — "Discuss with AI": the button, and the drawer's data
// =============================================================================
//
// CC_TASK_QUESTION_CHAT_v1. Two exports:
//
// - `DiscussButton` — the page's button. Its words are the stored
//   `discuss_button_label` ("Discuss with AI"), never a model name.
// - `QuestionDiscussDock` — mounted by the page ONLY while open. It fetches the
//   thread, owns the message box and the model choice, sends, and renders the
//   presentational `QuestionDiscussDrawer`.
//
// ## The shape it borrows from the Timeline dock (GO ruling 9)
//
// Self-contained: a question id in, its own fetch, its own state, its own words
// read from the wording it is handed. NOT the Timeline dock's floating-window code
// — this is a fixed right drawer over a dimmed page.
//
// ## The page is never disturbed
//
// The page renders this as a conditional SIBLING of its content, and Marie's
// answer box state lives in the page. Opening or closing the drawer therefore
// never remounts the textarea — her unsaved words survive. A source-structure
// test (`questionDiscussMount.test.ts`) holds that shape.

import React from "react";

import { wordingOf, type PracticeWording } from "../../services/practice";
import {
  DiscussionError,
  fetchDiscussion,
  sendDiscussion,
  type DiscussionPayload,
} from "../../services/practiceDiscussion";
import QuestionDiscussDrawer from "./QuestionDiscussDrawer";
import * as d from "./questionDiscussStyles";
import { discussChrome, initialModel, modelById, shouldSendDraft, turnCost } from "./questionDiscussView";

/** The page's "Discuss with AI" button. */
export const DiscussButton: React.FC<{ wording: PracticeWording; onOpen: () => void }> = ({
  wording,
  onOpen,
}) => (
  <button type="button" style={d.openButton} onClick={onOpen} data-discuss-open>
    {wordingOf(wording, "discuss_button_label")}
  </button>
);

interface DockProps {
  questionId: string;
  wording: PracticeWording;
  /** What is in her answer box right now — possibly unsaved. */
  draft: string;
  /** Her saved answer, or null when she has not answered. */
  savedAnswer: string | null;
  onClose: () => void;
}

const QuestionDiscussDock: React.FC<DockProps> = ({ questionId, wording, draft, savedAnswer, onClose }) => {
  const w = (key: string) => wordingOf(wording, key);
  const [payload, setPayload] = React.useState<DiscussionPayload | null>(null);
  const [model, setModel] = React.useState("");
  const [text, setText] = React.useState("");
  const [sending, setSending] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  const load = React.useCallback(() => {
    fetchDiscussion(questionId)
      .then((loaded) => {
        setPayload(loaded);
        setModel((current) => (current === "" ? initialModel(loaded) : current));
      })
      .catch((cause: unknown) => {
        // eslint-disable-next-line no-console
        console.error("discuss: the thread could not be loaded", cause);
        setError(w("discuss_load_failed"));
      });
    // `w` reads the same wording object for the life of the dock.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [questionId]);

  React.useEffect(load, [load]);

  const draftVisible = shouldSendDraft(draft, savedAnswer);

  const onSend = () => {
    const message = text.trim();
    if (payload === null || message === "" || sending) return;
    setSending(true);
    setError(null);
    sendDiscussion(questionId, {
      text: message,
      model,
      ...(draftVisible ? { draft_answer: draft } : {}),
    })
      .then((fresh) => {
        setPayload(fresh);
        setText("");
      })
      .catch((cause: unknown) => {
        // eslint-disable-next-line no-console
        console.error("discuss: the message was not answered", cause);
        // A 409 is the cap — the chrome says so from the fresh payload. Anything
        // else: her message may already be stored, so re-read the thread.
        setError(cause instanceof DiscussionError && cause.status === 409 ? null : w("discuss_send_failed"));
        load();
      })
      .finally(() => setSending(false));
  };

  if (payload === null) {
    return (
      <>
        <div style={d.scrim} onClick={onClose} />
        <aside style={d.drawer} role="dialog" aria-label={w("discuss_button_label")}>
          <p style={error === null ? d.status : d.failure} role={error === null ? undefined : "alert"}>
            {error ?? "…"}
          </p>
        </aside>
      </>
    );
  }

  return (
    <QuestionDiscussDrawer
      chrome={discussChrome(wording, payload, modelById(payload, model), draftVisible)}
      turns={payload.turns}
      models={payload.models}
      model={model}
      onModel={setModel}
      text={text}
      onText={setText}
      onSend={onSend}
      onClose={onClose}
      sending={sending}
      error={error}
      costOf={(turn) => turnCost(turn, wording)}
      labels={{
        placeholder: w("discuss_input_placeholder"),
        send: w("discuss_send_label"),
        close: w("discuss_close_label"),
        model: w("discuss_model_label"),
        empty: w("discuss_empty"),
      }}
    />
  );
};

export default QuestionDiscussDock;
