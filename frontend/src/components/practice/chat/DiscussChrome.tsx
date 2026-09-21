// DiscussChrome.tsx — the panel's header, the full-screen strip, and the composer.
//
// Presentational. Board 1's header carries the switcher, the visibility line,
// the grounded chip and the expand button; board 2's header carries only the
// switcher and the chip, because the full-screen strip above it holds Back and
// collapse, and the open switcher's footer states the visibility rule.

import React from "react";

import type { ChatThreads } from "../../../services/questionChat";
import DiscussSwitcher from "./DiscussSwitcher";
import { chipText, type Selection, visibilityLine, type Words } from "./discussPanelView";
import * as st from "./discussPanelStyles";

const ExpandIcon = () => (
  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" style={{ color: "var(--chat-quote-ink)" }} strokeWidth="2" aria-hidden="true">
    <path d="M15 3h6v6" />
    <path d="M9 21H3v-6" />
    <path d="M21 3l-7 7" />
    <path d="M3 21l7-7" />
  </svg>
);

const CollapseIcon = () => (
  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" style={{ color: "var(--chat-quote-ink)" }} strokeWidth="2" aria-hidden="true">
    <path d="M4 14h6v6" />
    <path d="M20 10h-6V4" />
    <path d="M14 10l7-7" />
    <path d="M3 21l7-7" />
  </svg>
);

const BackIcon = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" style={{ color: "var(--chat-quote-ink)" }} strokeWidth="2.5" aria-hidden="true">
    <path d="M19 12H5" />
    <path d="M12 19l-7-7 7-7" />
  </svg>
);

export const Header: React.FC<{
  w: Words;
  threads: ChatThreads;
  selection: Selection;
  full: boolean;
  onSelect: (s: Selection) => void;
  onExpand: () => void;
}> = ({ w, threads, selection, full, onSelect, onExpand }) => (
  <div style={full ? st.headerFull : st.header}>
    <DiscussSwitcher w={w} threads={threads} selection={selection} onSelect={onSelect} />
    {full ? (
      <div style={{ flexGrow: 1 }} />
    ) : (
      <div style={st.visibility}>
        <div style={st.visibilityText}>{visibilityLine(w, threads, selection)}</div>
      </div>
    )}
    {threads.grounded && <div style={st.chip}>{chipText(w, threads)}</div>}
    {!full && (
      <button
        type="button"
        aria-label={w("chat_expand_label")}
        title={w("chat_expand_label")}
        style={st.iconButton}
        onClick={onExpand}
      >
        <ExpandIcon />
      </button>
    )}
  </div>
);

export const Strip: React.FC<{
  w: Words;
  codeLine: string;
  questionText: string;
  answerText: string | null;
  onBack: () => void;
  onCollapse: () => void;
}> = ({ w, codeLine, questionText, answerText, onBack, onCollapse }) => (
  <div style={st.strip}>
    <button type="button" aria-label={w("chat_back_aria")} style={st.back} onClick={onBack}>
      <BackIcon />
      {w("chat_back_label")}
    </button>
    <div style={st.stripText}>
      <div style={st.stripCode}>{codeLine}</div>
      <div style={st.stripLine}>
        &ldquo;{questionText}&rdquo;
        {answerText !== null && ` · ${w("chat_strip_answer_template").replace("{answer}", answerText)}`}
      </div>
    </div>
    <button
      type="button"
      aria-label={w("chat_collapse_label")}
      title={w("chat_collapse_label")}
      style={st.iconButton}
      onClick={onCollapse}
    >
      <CollapseIcon />
    </button>
  </div>
);

/** The message box and Send — or, on a thread you cannot write in, the reason. */
export const Composer: React.FC<{
  w: Words;
  full: boolean;
  writable: boolean;
  readOnlyLine: string;
  sending: boolean;
  onSend: (text: string) => void;
}> = ({ w, full, writable, readOnlyLine, sending, onSend }) => {
  const [text, setText] = React.useState("");
  if (!writable) {
    return (
      <div style={full ? st.composerFull : st.composer}>
        <div style={st.quietLine}>{readOnlyLine}</div>
      </div>
    );
  }
  const submit = () => {
    const trimmed = text.trim();
    if (trimmed === "" || sending) return;
    onSend(trimmed);
    setText("");
  };
  return (
    <form
      style={full ? st.composerFull : st.composer}
      onSubmit={(event) => {
        event.preventDefault();
        submit();
      }}
    >
      <input
        aria-label={w("chat_message_label")}
        placeholder={w("chat_input_placeholder")}
        style={st.input}
        value={text}
        onChange={(event) => setText(event.target.value)}
      />
      <button type="submit" style={sending || text.trim() === "" ? st.sendDisabled : st.send} disabled={sending}>
        {w("chat_send_label")}
      </button>
    </form>
  );
};
