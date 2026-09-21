// DiscussMessages.tsx — one thread's messages, and the reply still arriving.
//
// Presentational: props in, markup out. An assistant message is a run of
// SEGMENTS — prose, then the citation cards that back it (mockup v2, board 1).
// A card's quoted passage is the server's slice of the STORED document; nothing
// here composes or edits it.

import React from "react";
import ReactMarkdown from "react-markdown";

import type { ChatMessage, CitationCard } from "../../../services/questionChat";
import { failureSentence, type Pending, waitingLine, type Words } from "./discussPanelView";
import * as st from "./discussPanelStyles";

type Props = {
  w: Words;
  messages: ChatMessage[];
  pending: Pending | null;
  full: boolean;
  maxTurns: number;
};

/** Paragraphs inside a bubble keep the mockup's line breaks (`<br><br>`). */
const MARKDOWN_COMPONENTS = {
  p: ({ children }: { children?: React.ReactNode }) => <p style={{ margin: "0 0 12px" }}>{children}</p>,
};

const Card: React.FC<{ card: CitationCard }> = ({ card }) => (
  <div style={st.card}>
    <div style={st.cardTitle}>
      {card.document_date === null ? card.document_title : `${card.document_title} — ${card.document_date}`}
    </div>
    <div style={st.cardQuote}>&ldquo;{card.quoted_text}&rdquo;</div>
  </div>
);

const Prose: React.FC<{ text: string; first: boolean }> = ({ text, first }) => (
  <div style={first ? st.replyBubbleFirst : st.replyBubble} data-chat-prose>
    <div style={{ marginBottom: -12 }}>
      <ReactMarkdown components={MARKDOWN_COMPONENTS}>{text}</ReactMarkdown>
    </div>
  </div>
);

const Reply: React.FC<{ message: ChatMessage; full: boolean; w: Words; maxTurns: number }> = ({
  message,
  full,
  w,
  maxTurns,
}) => (
  <div style={full ? st.replyColumnFull : st.replyColumn} data-chat-role="assistant">
    {message.segments.map((segment, i) => (
      <React.Fragment key={i}>
        {segment.text.trim() !== "" && <Prose text={segment.text} first={i === 0} />}
        {segment.cards.map((card, j) => (
          <Card key={j} card={card} />
        ))}
      </React.Fragment>
    ))}
    {message.failure !== null && (
      <div style={st.failureLine} role="status">
        {failureSentence(w, message.failure, maxTurns)}
      </div>
    )}
  </div>
);

const DiscussMessages: React.FC<Props> = ({ w, messages, pending, full, maxTurns }) => {
  const waiting = pending === null ? null : waitingLine(w, pending);
  return (
    <div style={full ? st.messagesFull : st.messages} data-chat-messages>
      {messages.length === 0 && pending === null && <div style={st.quietLine}>{w("chat_empty")}</div>}
      {messages.map((m) =>
        m.role === "user" ? (
          <div key={m.seq} style={full ? st.userBubbleFull : st.userBubble} data-chat-role="user">
            {m.segments.map((s) => s.text).join("")}
          </div>
        ) : (
          <Reply key={m.seq} message={m} full={full} w={w} maxTurns={maxTurns} />
        ),
      )}
      {pending !== null && pending.finished === null && pending.text !== "" && (
        <div style={full ? st.replyColumnFull : st.replyColumn}>
          <Prose text={pending.text} first />
        </div>
      )}
      {waiting !== null && (
        <div style={st.quietLine} role="status" aria-live="polite">
          {waiting}
        </div>
      )}
    </div>
  );
};

export default DiscussMessages;
