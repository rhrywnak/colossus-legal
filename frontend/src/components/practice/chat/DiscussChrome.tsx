// DiscussChrome.tsx — the panel's header, the full-screen strip, and the composer.
//
// Presentational. Board 1's header carries the switcher, the visibility line,
// the grounded chip, the expand button and — since v2.2.1 — a close (×) button
// after it (CC_TASK_PRACTICE_FIXES_v2.2.1, Fix 1: before it, the side panel
// could be shut only by expanding it and pressing Back); board 2's header carries only the
// switcher and the chip, because the full-screen strip above it holds Back and
// collapse, and the open switcher's footer states the visibility rule.

import React from "react";

import type { ChatThreads } from "../../../services/questionChat";
import DiscussSwitcher from "./DiscussSwitcher";
import { chipText, rowFor, type Selection, visibilityLine, type Words } from "./discussPanelView";
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

/** The ×: two strokes, drawn in the same 15px ink as the expand icon beside it. */
const CloseIcon = () => (
  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" style={{ color: "var(--chat-quote-ink)" }} strokeWidth="2" aria-hidden="true">
    <path d="M18 6L6 18" />
    <path d="M6 6l12 12" />
  </svg>
);

const BackIcon = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" style={{ color: "var(--chat-quote-ink)" }} strokeWidth="2.5" aria-hidden="true">
    <path d="M19 12H5" />
    <path d="M12 19l-7-7 7-7" />
  </svg>
);

/**
 * The side header's line under the top row, or `null` for none (v2.2.2).
 *
 * Only on the VIEWER's OWN thread, where it says who can read what she writes.
 * On a read-only selection — the earlier discussion, or someone else's thread —
 * the composer footer already shows the read-only sentence, and the header
 * repeating it put the same words on screen twice.
 */
export function headerLine(w: Words, threads: ChatThreads, selection: Selection): string | null {
  if (selection.kind === "earlier") return null;
  const row = rowFor(threads, selection);
  if (row !== null && !row.is_viewer) return null;
  return visibilityLine(w, threads, selection);
}

export const Header: React.FC<{
  w: Words;
  threads: ChatThreads;
  selection: Selection;
  full: boolean;
  onSelect: (s: Selection) => void;
  onExpand: () => void;
  /** Shuts the panel and drops `?discuss` — side mode only. */
  onClose: () => void;
}> = ({ w, threads, selection, full, onSelect, onExpand, onClose }) => {
  // One row of controls, identical in both modes: switcher · spacer · chip ·
  // (side only) expand · close. The spacer takes the slack, so no text is ever
  // squeezed into what is left between the controls.
  const controls = (
    <>
      <DiscussSwitcher w={w} threads={threads} selection={selection} onSelect={onSelect} />
      <div style={{ flexGrow: 1 }} />
      {threads.grounded && (
        <div style={st.chip} title={chipText(w, threads)}>
          {chipText(w, threads)}
        </div>
      )}
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
      {/* Side mode only: full screen already has Back and Collapse on its strip. */}
      {!full && (
        <button
          type="button"
          aria-label={w("chat_close_label")}
          title={w("chat_close_label")}
          style={st.iconButton}
          onClick={onClose}
        >
          <CloseIcon />
        </button>
      )}
    </>
  );
  // Full screen: unchanged — one row, no visibility line.
  if (full) return <div style={st.headerFull}>{controls}</div>;
  const line = headerLine(w, threads, selection);
  return (
    <div style={st.headerSide}>
      <div style={st.headerRow}>{controls}</div>
      {line !== null && <div style={st.visibilityText}>{line}</div>}
    </div>
  );
};

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
