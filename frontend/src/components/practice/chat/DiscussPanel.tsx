// DiscussPanel.tsx — the question chat: side panel or full screen (mockup v2).
//
// This component owns the chat's data and nothing else: which thread is open,
// its messages, the reply in flight. Every decision about what a state MEANS is
// in `discussPanelView` (pure, tested); every value a screen needs to decide
// something — read-only, unread, who is who — comes from the server.
//
// ## Loading states are distinct (Standing Rule 1)
//
// "Loading", "loaded and empty", and "could not load" each render differently:
// the switcher payload is `null` until it arrives, a failed load shows the
// stored failure sentence and logs the cause, and an empty thread says so in
// words (`chat_empty`).

import React from "react";

import {
  ChatCallError,
  type ChatMessage,
  type ChatThreads,
  fetchChatThread,
  fetchChatThreads,
  fetchEarlierDiscussion,
  markChatRead,
  sendChatMessage,
} from "../../../services/questionChat";
import { wordingOf, type PracticeWording } from "../../../services/practice";
import { Composer, Header, Strip } from "./DiscussChrome";
import DiscussMessages from "./DiscussMessages";
import {
  applyEvent,
  canWrite,
  failureSentence,
  NO_PENDING,
  type Pending,
  type Selection,
  visibilityLine,
} from "./discussPanelView";
import * as st from "./discussPanelStyles";

type Props = {
  questionId: string;
  wording: PracticeWording;
  full: boolean;
  codeLine: string;
  questionText: string;
  answerText: string | null;
  onExpand: () => void;
  onCollapse: () => void;
  onBack: () => void;
};

const loadThread = (questionId: string, s: Selection) =>
  s.kind === "earlier" ? fetchEarlierDiscussion(questionId) : fetchChatThread(questionId, s.username);

const DiscussPanel: React.FC<Props> = (props) => {
  const { questionId, wording, full } = props;
  const w = (key: string) => wordingOf(wording, key);
  const [threads, setThreads] = React.useState<ChatThreads | null>(null);
  const [selection, setSelection] = React.useState<Selection | null>(null);
  const [messages, setMessages] = React.useState<ChatMessage[]>([]);
  const [pending, setPending] = React.useState<Pending | null>(null);
  const [problem, setProblem] = React.useState<string | null>(null);

  const refreshThreads = React.useCallback(() => {
    fetchChatThreads(questionId)
      .then((t) => {
        setThreads(t);
        setSelection((was) => was ?? { kind: "thread", username: t.viewer });
      })
      .catch((cause: unknown) => {
        console.error("question chat: the threads could not be loaded", cause);
        setProblem(w("chat_load_failed"));
      });
    // `w` reads the same wording object for the component's life.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [questionId]);

  React.useEffect(refreshThreads, [refreshThreads]);

  React.useEffect(() => {
    if (selection === null) return;
    let live = true;
    setMessages([]);
    loadThread(questionId, selection)
      .then((t) => {
        if (!live) return;
        setMessages(t.messages);
        const last = t.messages[t.messages.length - 1];
        if (selection.kind === "thread" && last !== undefined) {
          markChatRead(questionId, selection.username, last.seq).catch((cause: unknown) => {
            // The thread itself is fine; the badge would stay lit. Said on screen,
            // because a count that silently stops clearing looks like new messages.
            console.error("question chat: the read mark could not be moved", cause);
            if (live) setProblem(w("chat_read_mark_failed"));
          });
        }
      })
      .catch((cause: unknown) => {
        console.error("question chat: the thread could not be loaded", cause);
        if (live) setProblem(w("chat_load_failed"));
      });
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [questionId, selection]);

  const send = (text: string) => {
    if (threads === null || selection === null || selection.kind !== "thread") return;
    setProblem(null);
    setPending(NO_PENDING);
    const optimistic: ChatMessage = {
      seq: Number.MAX_SAFE_INTEGER,
      role: "user",
      author_name: "",
      segments: [{ text, cards: [] }],
      at: "",
      failure: null,
    };
    setMessages((was) => [...was, optimistic]);
    let latest: Pending = NO_PENDING;
    sendChatMessage(questionId, selection.username, text, threads.client_idle_timeout_secs * 1000, (e) => {
      latest = applyEvent(latest, e);
      setPending(latest);
    })
      .then(() => {
        if (latest.finished?.failure) setProblem(failureSentence(w, latest.finished.failure, threads.max_turns));
      })
      .catch((cause: unknown) => {
        console.error("question chat: the message could not be sent", cause);
        const kind = cause instanceof ChatCallError ? cause.kind : "network";
        const status = cause instanceof ChatCallError ? cause.status : null;
        setProblem(failureSentence(w, status === 409 ? "cap" : kind === "stalled" ? "stalled" : "failed", threads.max_turns));
      })
      .finally(() => {
        setPending(null);
        // The server's copy replaces the optimistic one, with the reply after it.
        loadThread(questionId, selection)
          .then((t) => setMessages(t.messages))
          .catch((cause: unknown) => {
            // The reply is stored; what failed is reading it back. Said on screen so
            // a stale thread is never mistaken for a reply that did not come.
            console.error("question chat: reload after send failed", cause);
            setProblem(w("chat_load_failed"));
          });
        refreshThreads();
      });
  };

  const body =
    threads === null || selection === null ? (
      <div style={full ? st.messagesFull : st.messages}>
        <div style={st.quietLine} role={problem === null ? undefined : "alert"}>
          {problem ?? w("chat_waiting")}
        </div>
      </div>
    ) : (
      <>
        <Header
          w={w}
          threads={threads}
          selection={selection}
          full={full}
          onSelect={setSelection}
          onExpand={props.onExpand}
          onClose={props.onBack}
        />
        <DiscussMessages
          w={w}
          messages={messages}
          pending={pending}
          full={full}
          maxTurns={threads.max_turns}
          showAuthors={selection.kind === "earlier"}
        />
        {problem !== null && (
          <div style={{ ...st.failureLine, margin: full ? "0 180px 8px" : "0 24px 8px" }} role="alert">
            {problem}
          </div>
        )}
        <Composer
          w={w}
          full={full}
          writable={canWrite(threads, selection)}
          readOnlyLine={visibilityLine(w, threads, selection)}
          sending={pending !== null}
          onSend={send}
        />
      </>
    );

  if (!full) return <div style={st.panel} data-chat-panel="side">{body}</div>;
  return (
    <div style={st.full} data-chat-panel="full">
      <Strip
        w={w}
        codeLine={props.codeLine}
        questionText={props.questionText}
        answerText={props.answerText}
        onBack={props.onBack}
        onCollapse={props.onCollapse}
      />
      {body}
    </div>
  );
};

export default DiscussPanel;
