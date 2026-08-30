// =============================================================================
// SubsetModal.tsx — Add / Edit a subset, with the picker (mockup Screen 3)
// =============================================================================
//
// The picker IS the timeline's own phase-sectioned list with a checkbox, an
// order number and a note field per row — design ruling 2026-08-30 (5). It is
// not a second event list: the phases, their colours and their order all come
// from the same `GET /api/timeline` payload the page behind this modal is
// already rendering, and the grouping is `groupByPhase`, the page's own.
//
// ## ⚑ EVERY DECISION HERE IS IN `subsetPicker.ts`, NOT HERE
//
// This component is arrangement. What is picked, what order the story runs in,
// what the counts are and what goes on the wire are pure functions with 25 tests
// behind them, because this project has no component-testing tier and anything
// decided inside a component is decided where no test can reach it.
//
// ## Unchecked rows stay visible
//
// Mockup, and the design's reason: the author must see what is being left out.
// A picker that hid unpicked events would be a list of the answer rather than a
// list of the choice.
//
// ## The gaps a save must not eat
//
// A subset can reference an event since soft-deleted on the chronology. The
// picker cannot list it — the picker lists what exists — so those references are
// rendered in their own block ABOVE the phases, struck through and marked with
// the stored `subsets_removed_event_line`. They stay in `picks`, they keep their
// story numbers, and `toSubsetPayload` carries them through Save untouched. Were
// they merely dropped from the screen, the first Edit would delete them.

import React, { useMemo, useState } from "react";

import type {
  ChronologyWording,
  TimelineEvent,
  TimelinePhase,
} from "../../services/caseTimeline";
import { cw, fill } from "../../services/caseTimeline";
import type { SubsetDetail } from "../../services/caseTimelineSubsets";
import * as m from "./subsetModalStyles";
import {
  gapCount,
  initialPicks,
  isPicked,
  movePick,
  type Pick,
  pickedInPhase,
  pillGapsLine,
  positionOf,
  removedIdsOf,
  setPickNote,
  sizeLine,
  togglePick,
} from "./subsetPicker";
import { groupByPhase } from "./timelineFilters";
import * as w from "./timelineWriteStyles";

type Props = {
  /** The subset being edited, or null when creating one. */
  subset: SubsetDetail | null;
  /** Every LIVE event, in the API's `(event_date, id)` order. */
  events: TimelineEvent[];
  phases: TimelinePhase[];
  wording: ChronologyWording;
  saving: boolean;
  /** A failed write, already a sentence. Rendered, never swallowed. */
  error: string | null;
  onSave: (name: string, description: string, picks: Pick[]) => void;
  onCancel: () => void;
  /** Absent when creating — there is nothing to delete yet. */
  onDelete?: () => void;
};

const SubsetModal: React.FC<Props> = ({
  subset,
  events,
  phases,
  wording,
  saving,
  error,
  onSave,
  onCancel,
  onDelete,
}) => {
  const [name, setName] = useState(subset?.name ?? "");
  const [description, setDescription] = useState(subset?.description ?? "");
  const [picks, setPicks] = useState<Pick[]>(() => initialPicks(subset));

  // The subset's own removed references, and the events behind them. Held from
  // the subset the modal opened with, because the live list cannot supply them.
  const removedIds = useMemo(() => removedIdsOf(subset), [subset]);
  const removedEvents = useMemo(
    () => (subset?.events ?? []).filter((e) => e.removed).map((e) => e.event),
    [subset],
  );

  const orderedIds = useMemo(() => events.map((e) => e.id), [events]);
  const groups = useMemo(() => groupByPhase(phases, events), [phases, events]);

  const picked = picks.length;
  const gaps = gapCount(picks, removedIds);
  const tooLong = sizeLine(wording, picked);

  const pickedLine = fill(cw(wording, "subsets_picked_count_template"), { count: picked });
  // Omitted at zero — see `pillGapsLine`, where the ruling and its boundary live
  // so a test can reach them.
  const gapsLine = pillGapsLine(wording, gaps);

  const renderRow = (event: TimelineEvent, removed: boolean) => {
    const on = isPicked(picks, event.id);
    const at = positionOf(picks, event.id);
    return (
      <div key={event.id} style={m.pickRow(on)}>
        <input
          type="checkbox"
          checked={on}
          disabled={removed}
          aria-label={event.title}
          onChange={() => setPicks(togglePick(picks, event.id, orderedIds))}
        />
        <span style={m.orderControls}>
          <span style={m.order}>{at ?? ""}</span>
          {/* The mockup draws a draggable number. Up/down buttons instead —
              CC's call, stated in the report: a drag with no keyboard path is a
              control Marie cannot use on a trackpad mid-question, and the
              design's ruling is "manual reorder allowed", not "by dragging". */}
          {/* ⚑ THE ARIA-LABEL IS A WORDING ROW, and this is where task 2's
              recorded gap closes. These buttons shipped with a hardcoded
              `— earlier` / `— later` in an aria-label; the rules gate caught it,
              the English came out, and the glyph stood alone as the accessible
              name until a row could say it. A screen reader now reads the
              stored sentence instead of the bare glyph.

              The general rule, since this is the second time it has bitten:
              every aria-label, title, alt and placeholder is a user-visible
              string. The reach scanner looks for the wording accessor by name
              and not for string literals, so it cannot catch a hardcoded one —
              only a reader can.

              ⚑ And do not write the accessor's own name followed by an open
              parenthesis in a comment like this one. The scanner reads JSX
              block comments, so it would treat the prose as a request and go
              looking for the next string literal after it — which is an
              attribute on the markup below, not a wording key. That is exactly
              how this comment failed the reach test on its first draft. */}
          {on && (
            <>
              <button
                type="button"
                style={m.orderButton}
                aria-label={cw(wording, "subsets_move_earlier_label")}
                onClick={() => setPicks(movePick(picks, event.id, -1))}
              >
                ▲
              </button>
              <button
                type="button"
                style={m.orderButton}
                aria-label={cw(wording, "subsets_move_later_label")}
                onClick={() => setPicks(movePick(picks, event.id, 1))}
              >
                ▼
              </button>
            </>
          )}
        </span>
        <span style={m.pickDate}>{event.event_date}</span>
        <span style={removed ? m.removedTitle : m.pickTitle}>
          {event.title}
          {removed && (
            <span style={m.removedNote}>{cw(wording, "subsets_removed_event_line")}</span>
          )}
        </span>
        <input
          type="text"
          style={m.noteInput}
          placeholder={cw(wording, "subsets_note_placeholder")}
          value={picks.find((p) => p.event_id === event.id)?.note ?? ""}
          disabled={!on}
          onChange={(e) => setPicks(setPickNote(picks, event.id, e.target.value))}
        />
      </div>
    );
  };

  return (
    <div style={m.scrim} role="dialog" aria-modal="true">
      <div style={m.box}>
        <div style={m.head}>
          {/* Two calls and not one with a ternary inside: the reach guard reads
              the FIRST literal of a call, so a conditional key would leave the
              other invisible to it — declared, requested, and unguarded. */}
          <h3 style={m.headTitle}>
            {subset === null
              ? cw(wording, "subsets_form_add_title")
              : cw(wording, "subsets_window_edit")}
          </h3>
          <span style={m.pill}>{gapsLine === null ? pickedLine : `${pickedLine} · ${gapsLine}`}</span>
        </div>

        <div style={m.form}>
          <div>
            <label style={m.label} htmlFor="subset-name">
              {cw(wording, "subsets_form_name_label")}
            </label>
            <input
              id="subset-name"
              style={m.input}
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </div>
          <div>
            <label style={m.label} htmlFor="subset-description">
              {cw(wording, "subsets_form_description_label")}
            </label>
            <textarea
              id="subset-description"
              rows={2}
              style={m.textarea}
              value={description}
              onChange={(e) => setDescription(e.target.value)}
            />
          </div>
        </div>

        <div style={m.body}>
          <div style={m.hint}>{cw(wording, "subsets_picker_hint")}</div>

          {/* The gaps, above the phases: they belong to no live phase, and a
              reference the picker cannot offer must still be seen and still be
              saved. */}
          {removedEvents.length > 0 && (
            <>
              <div style={m.gapHint}>{cw(wording, "subsets_picker_gap_hint")}</div>
              {removedEvents.map((event) => renderRow(event, true))}
            </>
          )}

          {groups.map((group) => (
            <div key={group.phase.id}>
              <div style={m.pickerPhaseHead(group.phase.color)}>
                <h4 style={m.pickerPhaseLabel}>{group.phase.label}</h4>
                <span style={m.phaseMeta}>
                  {fill(cw(wording, "phase_count_template"), {
                    range: group.phase.date_range,
                    count: group.events.length,
                  })}
                  {" · "}
                  {fill(cw(wording, "subsets_picked_count_template"), {
                    count: pickedInPhase(picks, group.events),
                  })}
                </span>
              </div>
              {group.events.map((event) => renderRow(event, false))}
            </div>
          ))}
        </div>

        {error !== null && (
          <div style={w.writeError}>
            {fill(cw(wording, "write_failed_template"), { reason: error })}
          </div>
        )}

        <div style={m.foot}>
          {tooLong !== null && <span style={m.sizeWarning}>{tooLong}</span>}
          {/* The running count is always shown, as mocked — the size sentence
              only appears past twenty. */}
          <span style={m.footCount}>{pickedLine}</span>
          {onDelete !== undefined && (
            <button type="button" style={w.cardAction} disabled={saving} onClick={onDelete}>
              {cw(wording, "delete_label")}
            </button>
          )}
          <span style={m.footSpacer} />
          <button type="button" style={w.cardAction} disabled={saving} onClick={onCancel}>
            {cw(wording, "cancel_label")}
          </button>
          <button
            type="button"
            style={w.button}
            disabled={saving}
            onClick={() => onSave(name, description, picks)}
          >
            {saving ? cw(wording, "saving_label") : cw(wording, "save_label")}
          </button>
        </div>
      </div>
    </div>
  );
};

export default SubsetModal;
