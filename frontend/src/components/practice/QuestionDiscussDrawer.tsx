// =============================================================================
// QuestionDiscussDrawer.tsx — the "Discuss with AI" drawer, presentational
// =============================================================================
//
// CC_TASK_QUESTION_CHAT_v1, drawn on QUESTION_CHAT_DOCK_RULED_2026-09-17. Props in,
// JSX out: no fetch, no state of its own. `QuestionDiscussDock` owns the fetch and
// the writes and hands this what to show — which is what lets a markup test render
// it (no RTL/jsdom here, CLAUDE.md rule 30).
//
// The chip is a real <select> over the payload's models, printing DISPLAY NAMES
// (ruled amendment 2); nothing in this file names a model.

import React from "react";

import type { DiscussModel, DiscussionTurn } from "../../services/practiceDiscussion";
import type { DiscussChrome } from "./questionDiscussView";
import * as d from "./questionDiscussStyles";

export interface DrawerProps {
  chrome: DiscussChrome;
  turns: DiscussionTurn[];
  models: DiscussModel[];
  model: string;
  onModel: (id: string) => void;
  text: string;
  onText: (text: string) => void;
  onSend: () => void;
  onClose: () => void;
  sending: boolean;
  /** A stored failure sentence to show, or null. */
  error: string | null;
  /** The cost line under a model reply, or null — see `turnCost`. */
  costOf: (turn: DiscussionTurn) => string | null;
  /** Words from the store for the controls. */
  labels: { placeholder: string; send: string; close: string; model: string; empty: string };
}

const QuestionDiscussDrawer: React.FC<DrawerProps> = (p) => (
  <>
    <div style={d.scrim} onClick={p.onClose} data-discuss-scrim />
    <aside style={d.drawer} role="dialog" aria-label={p.chrome.title} data-discuss-drawer>
      <div style={d.header}>
        <div>
          <h2 style={d.title}>{p.chrome.title}</h2>
          <div style={d.subtitle}>{p.chrome.subtitle}</div>
        </div>
        <select
          style={d.picker}
          value={p.model}
          aria-label={p.labels.model}
          onChange={(event) => p.onModel(event.target.value)}
          data-discuss-picker
        >
          {p.models.map((m) => (
            <option key={m.model_id} value={m.model_id}>
              {m.display_name}
            </option>
          ))}
        </select>
        <button type="button" style={d.close} aria-label={p.labels.close} onClick={p.onClose}>
          ✕
        </button>
      </div>

      <div style={d.thread} data-discuss-thread>
        <p style={d.contextLine}>{p.chrome.contextLine}</p>
        {p.turns.length === 0 && <p style={d.status}>{p.labels.empty}</p>}
        {p.turns.map((turn) => (
          <div key={turn.id} data-turn={turn.role}>
            <div style={d.authorLine(turn.role)}>
              {turn.role === "model" ? `${turn.author} · ${turn.when}` : turn.author}
            </div>
            <div style={d.bubble(turn.role)}>{turn.text}</div>
            {p.costOf(turn) !== null && (
              <div style={d.cost} data-turn-cost>
                {p.costOf(turn)}
              </div>
            )}
          </div>
        ))}
        {p.sending && <p style={d.status}>{p.chrome.sending}</p>}
        {p.chrome.capReached !== null && <p style={d.failure}>{p.chrome.capReached}</p>}
        {p.error !== null && (
          <p style={d.failure} role="alert">
            {p.error}
          </p>
        )}
      </div>

      <div style={d.composer}>
        <div style={d.inputRow}>
          <input
            style={d.input}
            value={p.text}
            placeholder={p.labels.placeholder}
            aria-label={p.labels.placeholder}
            onChange={(event) => p.onText(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && !event.shiftKey) p.onSend();
            }}
            disabled={p.sending || p.chrome.capReached !== null}
          />
          <button
            type="button"
            style={d.send}
            onClick={p.onSend}
            disabled={p.sending || p.text.trim() === "" || p.chrome.capReached !== null}
          >
            {p.labels.send}
          </button>
        </div>
        <div style={d.footer}>{p.chrome.footer}</div>
      </div>
    </aside>
  </>
);

export default QuestionDiscussDrawer;
