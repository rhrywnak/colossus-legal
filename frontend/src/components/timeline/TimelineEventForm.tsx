// =============================================================================
// TimelineEventForm.tsx — mockup v2 Screen 3, add and edit
// =============================================================================
//
// Date + precision + approximate · title · fact (serif textarea) · tags from the
// stored vocabulary · phase from the stored phases · the document picker. Edit
// is the same form, pre-filled — one layout, two headings, so a reader never has
// to learn a second screen.
//
// ## ⚑ WHERE THIS IS DRAWN, AND WHY IT IS NOT A PAGE
//
// The mockup draws Screen 3 as its own frame. It is built as a panel that opens
// IN PLACE — at the top of the list, or above the event's detail — and the
// reason is R16's rule about the product's shape: expanding a phase deliberately
// did not open a new page, "no new page and no third navigation level — the
// CaseFleet two-levels rule holds". A form at `/timeline/events/new` would be
// that third level, for a control the reader is already looking at. Every field
// and control the mockup draws is reproduced; the placement is the deviation,
// and it is recorded as one.
//
// ## Every decision is next door
//
// What the form starts as, whether it may be submitted, what it submits, how a
// tag toggles — all in `timelineWriteRules.ts`, where `vitest` can reach them.
// This component is arrangement (the Phase B pattern, F10).
//
// ## No wording in here
//
// Every label, placeholder, option and button below is a stored settings row.
// The component contains no user-visible character of its own.

import React, { useState } from "react";

import type {
  ChronologyWording,
  TimelinePhase,
  TimelineTag,
} from "../../services/caseTimeline";
import { cw, fill } from "../../services/caseTimeline";
import type { SubmittedLink } from "../../services/caseTimelineWrites";
import TimelineDocumentPicker from "./TimelineDocumentPicker";
import * as s from "./timelineStyles";
import {
  type EventFormState,
  formIsSubmittable,
  PRECISION_TOKENS,
  toggleTag,
} from "./timelineWriteRules";
import * as w from "./timelineWriteStyles";

/**
 * The words one precision token wears in the select.
 *
 * ⚑ Three LITERAL `cw` calls rather than a computed key, for the reason the
 * heading above gives: the backend's reach guard reads string literals, and a
 * key assembled from a variable is a key no test can see being asked for. The
 * `throw` is not defensive padding — a fourth token would mean the stored
 * vocabulary grew and this build has no word for it, and a silently blank option
 * in a date-precision select is how a fabricated day gets stored.
 */
function precisionLabel(wording: ChronologyWording, token: string): string {
  if (token === "day") return cw(wording, "precision_day_label");
  if (token === "month") return cw(wording, "precision_month_label");
  if (token === "year") return cw(wording, "precision_year_label");
  throw new Error(
    `The case timeline has no word for the date precision "${token}". The stored ` +
      `vocabulary and this build disagree; report it to the site administrator.`,
  );
}

type Props = {
  /** True when this form is creating; false when it is editing. */
  creating: boolean;
  form: EventFormState;
  onChange: (next: EventFormState) => void;
  tags: TimelineTag[];
  phases: TimelinePhase[];
  wording: ChronologyWording;
  /** True while the write is in flight — the control says so and refuses. */
  saving: boolean;
  /** The last failure's sentence, or null. Rendered, never swallowed. */
  error: string | null;
  /** Whether the document picker is offered (adds only — see the rules file). */
  withLinks: boolean;
  onSave: () => void;
  onCancel: () => void;
};

const TimelineEventForm: React.FC<Props> = ({
  creating,
  form,
  onChange,
  tags,
  phases,
  wording,
  saving,
  error,
  withLinks,
  onSave,
  onCancel,
}) => {
  const [pickerOpen, setPickerOpen] = useState(false);
  const submittable = formIsSubmittable(form) && !saving;

  return (
    <div style={w.form}>
      {/* ⚑ Two literal calls and not one with a computed key. The backend's
          reach guard reads the FIRST string literal inside a `cw(` call, so
          `cw(wording, headingKey)` would leave BOTH headings invisible to it —
          declared, seeded, requested at runtime, and guarded by nothing. Every
          `cw` on these surfaces names its key out loud for that reason. */}
      <h2 style={w.formTitle}>
        {creating ? cw(wording, "form_add_title") : cw(wording, "form_edit_title")}
      </h2>

      <div style={w.inline}>
        <label style={w.narrowField("180px")}>
          <span style={w.label}>{cw(wording, "form_date_label")}</span>
          <input
            type="date"
            style={w.input}
            value={form.event_date}
            onChange={(e) => onChange({ ...form, event_date: e.target.value })}
          />
        </label>

        <label style={w.narrowField("160px")}>
          <span style={w.label}>{cw(wording, "form_precision_label")}</span>
          <select
            style={w.input}
            value={form.date_precision}
            onChange={(e) => onChange({ ...form, date_precision: e.target.value })}
          >
            {PRECISION_TOKENS.map((token) => (
              <option key={token} value={token}>
                {precisionLabel(wording, token)}
              </option>
            ))}
          </select>
        </label>

        <label style={w.checkRow}>
          <input
            type="checkbox"
            checked={form.approximate}
            onChange={(e) => onChange({ ...form, approximate: e.target.checked })}
          />
          {cw(wording, "form_approximate_label")}
        </label>
      </div>

      <label style={w.field}>
        <span style={w.label}>{cw(wording, "form_title_label")}</span>
        <input
          type="text"
          style={w.input}
          placeholder={cw(wording, "form_title_placeholder")}
          value={form.title}
          onChange={(e) => onChange({ ...form, title: e.target.value })}
        />
      </label>

      <label style={w.field}>
        <span style={w.label}>{cw(wording, "form_fact_label")}</span>
        <textarea
          rows={3}
          style={w.textarea}
          placeholder={cw(wording, "form_fact_placeholder")}
          value={form.fact}
          onChange={(e) => onChange({ ...form, fact: e.target.value })}
        />
      </label>

      <div style={w.field}>
        <span style={w.label}>{cw(wording, "form_tags_label")}</span>
        {/* THE CHIPS ARE THE STORED VOCABULARY, in each tag's own colour — the
            same rows the filter bar draws. A sixth tag is a row, not a build
            (design R7), and this picker gets it with no code change. */}
        <div style={w.tagPicker}>
          {tags.map((tag) => (
            <button
              key={tag.id}
              type="button"
              style={s.chip(tag.color, form.tags.includes(tag.id))}
              aria-pressed={form.tags.includes(tag.id)}
              onClick={() => onChange({ ...form, tags: toggleTag(form.tags, tag.id) })}
            >
              {tag.label}
            </button>
          ))}
        </div>
      </div>

      <label style={w.narrowField("240px")}>
        <span style={w.label}>{cw(wording, "form_phase_label")}</span>
        {/* The options are the stored phase rows, and the column is a foreign
            key onto them — so what this offers and what the database accepts
            cannot drift. */}
        <select
          style={w.input}
          value={form.phase}
          onChange={(e) => onChange({ ...form, phase: e.target.value })}
        >
          {phases.map((phase) => (
            <option key={phase.id} value={phase.id}>
              {phase.label}
            </option>
          ))}
        </select>
      </label>

      {withLinks && (
        <div style={w.field}>
          <span style={w.label}>{cw(wording, "form_documents_label")}</span>
          {form.links.map((link) => (
            <PickedLink
              key={`${link.target_type}:${link.target_id}`}
              link={link}
              wording={wording}
              onRemove={() =>
                onChange({
                  ...form,
                  links: form.links.filter((held) => held.target_id !== link.target_id),
                })
              }
            />
          ))}
          {pickerOpen ? (
            <TimelineDocumentPicker
              wording={wording}
              onPick={(choice, pinpoint) => {
                onChange({
                  ...form,
                  links: [
                    ...form.links.filter((held) => held.target_id !== choice.id),
                    {
                      target_type: "document",
                      target_id: choice.id,
                      label: choice.title,
                      // Absent stays ABSENT rather than becoming "": the
                      // absence is what marks the link, and an empty string
                      // would store a pinpoint of nothing (design R9).
                      ...(pinpoint.trim() === "" ? {} : { pinpoint: pinpoint.trim() }),
                    },
                  ],
                });
                setPickerOpen(false);
              }}
            />
          ) : (
            <button type="button" style={w.button} onClick={() => setPickerOpen(true)}>
              {cw(wording, "link_document_label")}
            </button>
          )}
        </div>
      )}

      {/* ⚑ A failed write reaches a rendered sentence. Never a dead button. */}
      {error !== null && (
        <div style={w.writeError}>
          {fill(cw(wording, "write_failed_template"), { reason: error })}
        </div>
      )}

      <div style={w.actionBar}>
        <button
          type="button"
          style={w.primaryButton(!submittable)}
          disabled={!submittable}
          onClick={onSave}
        >
          {/* Two calls, not a ternary inside one — see the heading above. */}
          {saving ? cw(wording, "saving_label") : cw(wording, "save_label")}
        </button>
        <button type="button" style={w.button} onClick={onCancel} disabled={saving}>
          {cw(wording, "cancel_label")}
        </button>
      </div>
    </div>
  );
};

/** One document the form has picked but not yet saved. */
const PickedLink: React.FC<{
  link: SubmittedLink;
  wording: ChronologyWording;
  onRemove: () => void;
}> = ({ link, wording, onRemove }) => (
  <div style={w.pickedLink}>
    <span style={s.docLink}>{link.label ?? link.target_id}</span>
    <span style={s.pinpoint}>{link.pinpoint ?? cw(wording, "no_pinpoint_label")}</span>
    <button type="button" style={w.rowAction} onClick={onRemove}>
      {cw(wording, "remove_link_label")}
    </button>
  </div>
);

export default TimelineEventForm;
