// DiscussSwitcher.tsx — "Your thread ▾" and the menu it opens (mockup v2, board 2).
//
// Threads live HERE, never as cards on the page (ruled on mockup v2). Each row:
// avatar, title with count, unread badge, one-line preview, Read-only marking;
// the Earlier team discussion (ADDENDUM_1) below the people; the visibility rule
// in the footer.

import React from "react";

import type { ChatThreads } from "../../../services/questionChat";
import {
  earlierMeta,
  rowTitle,
  type Selection,
  switcherLabel,
  unreadBadge,
  type Words,
} from "./discussPanelView";
import * as st from "./discussPanelStyles";

type Props = {
  w: Words;
  threads: ChatThreads;
  selection: Selection;
  onSelect: (s: Selection) => void;
};

const Chevron: React.FC<{ up: boolean }> = ({ up }) => (
  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" style={{ color: "var(--chat-quote-ink)" }} strokeWidth="2.5" aria-hidden="true">
    <path d={up ? "M18 15l-6-6-6 6" : "M6 9l6 6 6-6"} />
  </svg>
);

const DiscussSwitcher: React.FC<Props> = ({ w, threads, selection, onSelect }) => {
  const [open, setOpen] = React.useState(false);
  const pick = (s: Selection) => {
    setOpen(false);
    onSelect(s);
  };
  const isSelected = (s: Selection) =>
    s.kind === selection.kind && (s.kind === "earlier" || (selection.kind === "thread" && s.username === selection.username));

  return (
    <div style={st.menuAnchor}>
      <button
        type="button"
        aria-label={w("chat_switch_label")}
        aria-expanded={open}
        style={open ? st.switcherButtonOpen : st.switcherButton}
        onClick={() => setOpen((was) => !was)}
      >
        {switcherLabel(w, threads, selection)}
        <Chevron up={open} />
      </button>
      {open && (
        <div style={st.menu} role="menu">
          {threads.threads.map((row, i) => {
            const s: Selection = { kind: "thread", username: row.username };
            const badge = unreadBadge(w, row);
            return (
              <button
                key={row.username}
                type="button"
                role="menuitem"
                style={isSelected(s) ? st.menuRowSelected : st.menuRow}
                onClick={() => pick(s)}
              >
                <div style={{ ...st.avatar, background: st.AVATAR_COLOURS[i % st.AVATAR_COLOURS.length] }}>
                  {row.initial}
                </div>
                {row.message_count === 0 ? (
                  <div style={{ ...st.rowTitleEmpty, alignSelf: "center" }}>{rowTitle(w, row)}</div>
                ) : (
                  <div style={st.rowText}>
                    <div style={st.rowTitle}>
                      {rowTitle(w, row)}
                      {badge !== null && (
                        <>
                          {" · "}
                          <span style={st.rowUnread}>{badge}</span>
                        </>
                      )}
                    </div>
                    {row.preview !== null && <div style={st.rowPreview}>&ldquo;{row.preview}&rdquo;</div>}
                    {row.read_only && <div style={st.rowReadOnly}>{w("chat_readonly_mark")}</div>}
                  </div>
                )}
              </button>
            );
          })}
          {threads.earlier !== null && (
            <button
              type="button"
              role="menuitem"
              style={isSelected({ kind: "earlier" }) ? st.menuRowSelected : st.menuRow}
              onClick={() => pick({ kind: "earlier" })}
            >
              <div style={st.rowText}>
                <div style={st.rowTitle}>{w("chat_earlier_label")}</div>
                <div style={st.rowPreview}>{earlierMeta(w, threads.earlier.message_count, threads.earlier.last_on)}</div>
                <div style={st.rowReadOnly}>{w("chat_readonly_mark")}</div>
              </div>
            </button>
          )}
          <div style={st.menuFooter}>{w("chat_switcher_footer")}</div>
        </div>
      )}
    </div>
  );
};

export default DiscussSwitcher;
