// ModelJobRow.tsx — one AI job on the Overview panel: name, the two control
// columns, Save, the meta line and any worst-state warning (board 1 / 2 / 4).
//
// The dropdown is drawn by hand, not with a native <select>, because board 2's
// list ends with a line that is not an option ("Not listed: …") and a native
// select cannot show one. It is a listbox button: Escape and a second click
// close it, and it carries the ARIA a screen reader needs.

import React, { useState } from "react";

import type { AiJob, AiJobChange, AiJobControl } from "../../services/aiJobs";
import { optionText, pendingChanges, type JobColumn } from "./modelJobsView";

type Props = {
  job: AiJob;
  saveLabel: string;
  last: boolean;
  /** Which dropdown on the panel is open ("<job>:<column>"), if any. */
  open: string | null;
  setOpen: (id: string | null) => void;
  /** A save or switch back for this row is in flight. */
  busy: boolean;
  /** The refusal or failure line for this row's last save, if any. */
  error: string | null;
  onSave: (job: string, changes: AiJobChange[]) => void;
  onSwitchBack: (job: string) => void;
};

const ModelJobRow: React.FC<Props> = ({
  job,
  saveLabel,
  last,
  open,
  setOpen,
  busy,
  error,
  onSave,
  onSwitchBack,
}) => {
  // What each dropdown shows as chosen, until Save sends it.
  const [picked, setPicked] = useState<Partial<Record<JobColumn, string>>>({});
  const changes = pendingChanges(job, picked);
  return (
    <div data-aj="job" data-last={last ? "true" : "false"} data-job={job.job}>
      <div>
        <div data-aj="jn">{job.title}</div>
        {job.description && <div data-aj="jd">{job.description}</div>}
      </div>
      {(["first", "instructions"] as const).map((column) => (
        <Control
          key={column}
          id={`${job.job}:${column}`}
          control={job[column]}
          open={open}
          setOpen={setOpen}
          picked={picked[column]}
          pick={(value) => setPicked((p) => ({ ...p, [column]: value }))}
        />
      ))}
      <button
        type="button"
        data-aj="btn"
        disabled={changes.length === 0 || busy}
        onClick={() => onSave(job.job, changes)}
      >
        {saveLabel}
      </button>
      <div data-aj="meta">
        {job.meta}
        {job.was && job.switch_back && (
          <>
            {" · "}
            {job.was}
            {" · "}
            <button type="button" data-aj="linkbtn" disabled={busy} onClick={() => onSwitchBack(job.job)}>
              {job.switch_back}
            </button>
          </>
        )}
      </div>
      {job.warnings.map((w) => (
        <div key={w} data-aj="warn" role="alert">
          {w}
        </div>
      ))}
      {error && (
        <div data-aj="warn" role="alert">
          {error}
        </div>
      )}
    </div>
  );
};

type ControlProps = {
  id: string;
  control: AiJobControl;
  open: string | null;
  setOpen: (id: string | null) => void;
  picked: string | undefined;
  pick: (value: string) => void;
};

const Control: React.FC<ControlProps> = ({ id, control, open, setOpen, picked, pick }) => {
  if (control.kind !== "choose") {
    return (
      <div>
        <div data-aj="lbl">{control.label}</div>
        <div data-aj="fixed">{control.text}</div>
      </div>
    );
  }
  const isOpen = open === id;
  const chosen = picked ?? control.current;
  const shown = control.options.find((o) => o.value === chosen)?.label ?? control.current_label;
  const listId = `aj-list-${id}`;
  return (
    <div>
      <div data-aj="lbl" id={`${listId}-label`}>
        {control.label}
      </div>
      <button
        type="button"
        data-aj="sel"
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        aria-controls={listId}
        aria-labelledby={`${listId}-label`}
        onClick={() => setOpen(isOpen ? null : id)}
        onKeyDown={(e) => {
          if (e.key === "Escape") setOpen(null);
        }}
      >
        <span>{shown}</span>
        <span aria-hidden="true">▾</span>
      </button>
      {isOpen && (
        <ul data-aj="menu" role="listbox" id={listId} aria-labelledby={`${listId}-label`}>
          {control.options.map((o) => (
            <li
              key={o.value}
              role="option"
              aria-selected={o.value === chosen}
              onClick={() => {
                pick(o.value);
                setOpen(null);
              }}
            >
              {optionText(o)}
            </li>
          ))}
          {control.foot && (
            <li data-aj="foot" role="presentation">
              {control.foot}
            </li>
          )}
        </ul>
      )}
    </div>
  );
};

export default ModelJobRow;
