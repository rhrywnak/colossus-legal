// discussPanel.test.tsx — the discussion panel's decisions, parser and markup.
//
// Pure-helper and static-markup tests (rule 30: no DOM tier in this project).

import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { SseParser, type ChatMessage, type ChatThreads, type ThreadRow } from "../../../../services/questionChat";
import DiscussMessages from "../DiscussMessages";
import DiscussSwitcher from "../DiscussSwitcher";
import {
  applyEvent,
  canWrite,
  chipText,
  earlierMeta,
  failureSentence,
  joinNames,
  NO_PENDING,
  parseDiscussMode,
  rowTitle,
  switcherLabel,
  unreadBadge,
  visibilityLine,
  waitingLine,
} from "../discussPanelView";

/** The migration's own values for the rows these tests read. */
const WORDS: Record<string, string> = {
  chat_your_thread: "Your thread",
  chat_thread_of_template: "{name}'s thread",
  chat_switch_label: "Switch thread",
  chat_visibility_template: "{others} can read this thread",
  chat_resumed_template: "resumed from {date}",
  chat_readonly_line: "Read-only — you can read this thread, and the AI reads it too, but only its owner writes in it.",
  chat_earlier_readonly_line: "Read-only — the team's discussion of this question from before each person had a thread. The AI reads it too.",
  chat_and: "and",
  chat_grounded_chip_template: "{model} · grounded",
  chat_switcher_own_template: "Your thread · {count} messages",
  chat_switcher_own_one: "Your thread · 1 message",
  chat_switcher_own_empty: "Your thread · no messages yet",
  chat_switcher_other_template: "{name} · {count} messages",
  chat_switcher_other_one: "{name} · 1 message",
  chat_switcher_other_empty: "{name} · no messages yet",
  chat_unread_template: "{count} new",
  chat_readonly_mark: "Read-only",
  chat_switcher_footer: "Everyone on the case can read all threads. The AI reads them too, so insight crosses between you.",
  chat_earlier_label: "Earlier team discussion",
  chat_earlier_meta_template: "{count} messages · last {date}",
  chat_earlier_meta_one: "1 message · {date}",
  chat_empty: "No messages yet — ask the first question.",
  chat_waiting: "Thinking…",
  chat_tool_line: "Checking the record…",
  chat_send_failed: "No reply came back. Your message is saved above — send again to retry.",
  chat_stalled: "The reply stopped arriving. Your message is saved above — send again to retry.",
  chat_refused: "The model declined to answer that. Your message is saved above — try putting it another way.",
  chat_truncated: "The reply ran past its length limit and is not shown. Your message is saved above — send again to retry.",
  chat_cap_reached_template: "This thread has reached its limit of {max} replies.",
};
const w = (key: string) => {
  const v = WORDS[key];
  if (v === undefined) throw new Error(`test wording missing ${key}`);
  return v;
};

function row(over: Partial<ThreadRow>): ThreadRow {
  return {
    username: "docmarie",
    display_name: "Marie",
    initial: "M",
    is_viewer: true,
    read_only: false,
    message_count: 6,
    unread: 0,
    preview: "…I had it in my hands in early December.",
    resumed_from: "Sep 19",
    ...over,
  };
}

function threads(over: Partial<ChatThreads> = {}): ChatThreads {
  return {
    question_id: "q",
    viewer: "docmarie",
    model_short_name: "Opus 5",
    grounded: true,
    others: ["Chuck", "Roman"],
    threads: [
      row({}),
      row({
        username: "cpenzien",
        display_name: "Chuck",
        initial: "C",
        is_viewer: false,
        read_only: true,
        message_count: 2,
        unread: 1,
        preview: "If they lead with the 13-year gap, what's the cleanest exhibit…",
        resumed_from: null,
      }),
      row({ username: "roman", display_name: "Roman", initial: "R", is_viewer: false, read_only: true, message_count: 0, preview: null, resumed_from: null }),
    ],
    earlier: { message_count: 4, last_on: "Sep 17", preview: "old" },
    client_idle_timeout_secs: 150,
    max_turns: 200,
    ...over,
  };
}

const mine = { kind: "thread" as const, username: "docmarie" };
const chucks = { kind: "thread" as const, username: "cpenzien" };

describe("the header's words", () => {
  it("says who can read her thread and when it began, as the mockup does", () => {
    expect(visibilityLine(w, threads(), mine)).toBe("Chuck and Roman can read this thread · resumed from Sep 19");
    expect(switcherLabel(w, threads(), mine)).toBe("Your thread");
    expect(chipText(w, threads())).toBe("Opus 5 · grounded");
  });

  it("says read-only on someone else's thread and on the earlier discussion", () => {
    expect(switcherLabel(w, threads(), chucks)).toBe("Chuck's thread");
    expect(visibilityLine(w, threads(), chucks)).toMatch(/^Read-only/);
    expect(visibilityLine(w, threads(), { kind: "earlier" })).toMatch(/^Read-only — the team's discussion/);
    expect(switcherLabel(w, threads(), { kind: "earlier" })).toBe("Earlier team discussion");
  });

  it("offers the composer only on the viewer's own thread — the server's read_only decides", () => {
    expect(canWrite(threads(), mine)).toBe(true);
    expect(canWrite(threads(), chucks)).toBe(false);
    expect(canWrite(threads(), { kind: "earlier" })).toBe(false);
  });

  it("joins names in words", () => {
    expect(joinNames(["Chuck"], "and")).toBe("Chuck");
    expect(joinNames(["A", "B", "C"], "and")).toBe("A, B and C");
  });
});

describe("the switcher's rows", () => {
  it("titles each row by owner and count, and badges unread", () => {
    const [m, c, r] = threads().threads;
    expect(rowTitle(w, m)).toBe("Your thread · 6 messages");
    expect(rowTitle(w, c)).toBe("Chuck · 2 messages");
    expect(rowTitle(w, r)).toBe("Roman · no messages yet");
    expect(rowTitle(w, row({ message_count: 1 }))).toBe("Your thread · 1 message");
    expect(unreadBadge(w, c)).toBe("1 new");
    expect(unreadBadge(w, m)).toBeNull();
    expect(earlierMeta(w, 4, "Sep 17")).toBe("4 messages · last Sep 17");
    expect(earlierMeta(w, 1, "Sep 17")).toBe("1 message · Sep 17");
  });

  it("renders the button closed by default, labelled as the mockup", () => {
    const html = renderToStaticMarkup(
      <DiscussSwitcher w={w} threads={threads()} selection={mine} onSelect={() => {}} />,
    );
    expect(html).toContain('aria-label="Switch thread"');
    expect(html).toContain("Your thread");
    expect(html).not.toContain('role="menu"');
  });
});

describe("the stream", () => {
  it("parses events split across chunks, and ignores keep-alive comments", () => {
    const p = new SseParser();
    expect(p.push('event: delta\ndata: {"te')).toEqual([]);
    expect(p.push('xt":"Hel"}\n\n: keep-alive\n\nevent: delta\r\ndata: {"text":"lo"}\r\n\r\n')).toEqual([
      { event: "delta", data: { text: "Hel" } },
      { event: "delta", data: { text: "lo" } },
    ]);
  });

  it("folds events into the pending reply and says what is happening", () => {
    let p = applyEvent(NO_PENDING, { event: "accepted", data: { seq: 3 } });
    expect(waitingLine(w, p)).toBe("Thinking…");
    p = applyEvent(p, { event: "tool", data: { state: "started" } });
    expect(waitingLine(w, p)).toBe("Checking the record…");
    p = applyEvent(p, { event: "delta", data: { text: "Your record " } });
    expect(p.reading).toBe(false);
    expect(waitingLine(w, p)).toBeNull();
    p = applyEvent(p, { event: "failed", data: { failure: "stalled", detail: "idle", messages: [] } });
    expect(p.finished?.failure).toBe("stalled");
  });

  it("names every failure with its own sentence", () => {
    expect(failureSentence(w, "refused", 200)).toBe(WORDS.chat_refused);
    expect(failureSentence(w, "truncated", 200)).toBe(WORDS.chat_truncated);
    expect(failureSentence(w, "stalled", 200)).toBe(WORDS.chat_stalled);
    expect(failureSentence(w, "cap", 200)).toBe("This thread has reached its limit of 200 replies.");
    expect(failureSentence(w, "failed", 200)).toBe(WORDS.chat_send_failed);
  });
});

describe("the messages", () => {
  const reply: ChatMessage = {
    seq: 2,
    role: "assistant",
    author_name: "The AI",
    segments: [
      { text: "Your own record puts the response in **November 2025**:", cards: [
        { document_title: "SSA response letter", document_date: "November 20, 2025", page: 1, quoted_text: "We are writing to tell you…" },
      ] },
      { text: "If you say December on the stand…", cards: [] },
    ],
    at: "Fri 19 Sep · 8:00 am",
    failure: null,
  };

  it("renders each card under the words it backs, with the stored passage", () => {
    const html = renderToStaticMarkup(
      <DiscussMessages w={w} messages={[reply]} pending={null} full={false} maxTurns={200} />,
    );
    const card = html.indexOf("SSA response letter — November 20, 2025");
    expect(card).toBeGreaterThan(html.indexOf("November 2025"));
    expect(card).toBeLessThan(html.indexOf("If you say December"));
    expect(html).toContain("We are writing to tell you…");
    expect(html).toContain("<strong>November 2025</strong>");
  });

  it("says an empty thread in words, and a stored failure by name", () => {
    expect(
      renderToStaticMarkup(<DiscussMessages w={w} messages={[]} pending={null} full={false} maxTurns={200} />),
    ).toContain(WORDS.chat_empty);
    const failed = { ...reply, segments: [], failure: "truncated" };
    expect(
      renderToStaticMarkup(<DiscussMessages w={w} messages={[failed]} pending={null} full={false} maxTurns={200} />),
    ).toContain(WORDS.chat_truncated);
  });
});

describe("the address", () => {
  it("reads the panel's state from ?discuss", () => {
    expect(parseDiscussMode("?discuss=full")).toBe("full");
    expect(parseDiscussMode("?discuss=side")).toBe("side");
    expect(parseDiscussMode("?discuss=wide")).toBe("closed");
    expect(parseDiscussMode("")).toBe("closed");
  });
});

describe("the Earlier team discussion", () => {
  it("prints each message's stored author and time", () => {
    const old: ChatMessage = {
      seq: 1,
      role: "user",
      author_name: "Roman",
      segments: [{ text: "Old-dock note", cards: [] }],
      at: "Wed 17 Sep · 5:36 pm",
      failure: null,
    };
    const html = renderToStaticMarkup(
      <DiscussMessages w={w} messages={[old]} pending={null} full={true} maxTurns={200} showAuthors />,
    );
    expect(html).toContain("Roman · Wed 17 Sep · 5:36 pm");
    const plain = renderToStaticMarkup(
      <DiscussMessages w={w} messages={[old]} pending={null} full={true} maxTurns={200} />,
    );
    expect(plain).not.toContain("Wed 17 Sep");
  });
});
