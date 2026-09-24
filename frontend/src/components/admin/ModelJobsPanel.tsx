// ModelJobsPanel.tsx — Admin → Overview: "Which model and which instructions
// each AI job uses" (CC_TASK_MODEL_JOBS_PANEL_v1, mockup v3 boards 1–4b).
//
// Reads the panel, saves one job at a time, shows board 3's confirmation with
// its Switch back, and draws every refusal the server sends in its own words.
// Every word comes from the server; this file decides nothing (Rule 12).

import React, { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";

import {
  AiJobRefusal,
  getAiJobs,
  saveAiJob,
  switchBackAiJob,
  type AiJobChange,
  type AiJobSaved,
  type AiJobsPanel,
} from "../../services/aiJobs";
import ModelJobRow from "./ModelJobRow";
import { linkTarget } from "./modelJobsView";
import { errorLine, JOBS_CSS } from "./modelJobsStyles";

/** Not in the ruled copy: the panel's one sentence when it cannot be read. */
export const READ_ERROR_LINE = "The AI jobs could not be read. Reload the page to try again.";
/** Not in the ruled copy: a save that failed for a reason no person caused. */
export const SAVE_ERROR_LINE = "That didn't save. Please try again in a minute.";

const ModelJobsPanel: React.FC = () => {
  const [panel, setPanel] = useState<AiJobsPanel | null>(null);
  const [failed, setFailed] = useState(false);
  // Which dropdown is open, as "<job>:<column>"; one at a time, as on board 2.
  const [open, setOpen] = useState<string | null>(null);
  // Board 3's confirmation, after the last Save or Switch back.
  const [saved, setSaved] = useState<AiJobSaved | null>(null);
  // The one line under a row whose save was refused or failed.
  const [rowError, setRowError] = useState<{ job: string; text: string } | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const navigate = useNavigate();

  useEffect(() => {
    getAiJobs()
      .then((p) => {
        setPanel(p);
        setFailed(false);
      })
      .catch((e: unknown) => {
        console.error("AI jobs panel: the panel could not be read", e);
        setFailed(true);
      });
  }, []);

  /** Run a save or a switch back for `job`, and show what came of it. */
  const run = (job: string, call: () => Promise<AiJobSaved>) => {
    setBusy(job);
    setRowError(null);
    call()
      .then((result) => {
        setPanel(result.panel);
        setSaved(result);
      })
      .catch((e: unknown) => {
        console.error(`AI jobs panel: saving ${job} failed`, e);
        const text = e instanceof AiJobRefusal ? e.message : SAVE_ERROR_LINE;
        setRowError({ job, text });
        setSaved(null);
      })
      .finally(() => setBusy(null));
  };
  const onSave = (job: string, changes: AiJobChange[]) => run(job, () => saveAiJob(job, changes));
  const onSwitchBack = (job: string) => run(job, () => switchBackAiJob(job));

  if (failed) {
    return (
      <p style={errorLine} role="alert">
        {READ_ERROR_LINE}
      </p>
    );
  }
  if (!panel) return null;

  return (
    <section data-surface="admin-jobs" data-aj="panel" aria-label={panel.title}>
      <style>{JOBS_CSS}</style>
      <div data-aj="phead">
        <h2 data-aj="phead-title">{panel.title}</h2>
        <div data-aj="phead-intro">{panel.intro}</div>
      </div>
      {saved && (
        <div data-aj="ok" role="status">
          {saved.confirmation}{" "}
          <button
            type="button"
            data-aj="linkbtn"
            disabled={busy !== null}
            onClick={() => onSwitchBack(saved.job)}
          >
            {saved.switch_back}
          </button>
        </div>
      )}
      {panel.jobs.map((job, i) => (
        <ModelJobRow
          // The current values are in the key so a row that was just saved
          // starts again from what the server now holds.
          key={`${job.job}:${job.first.kind === "choose" ? job.first.current : ""}:${
            job.instructions.kind === "choose" ? job.instructions.current : ""
          }`}
          job={job}
          saveLabel={panel.save_label}
          last={i === panel.jobs.length - 1}
          open={open}
          setOpen={setOpen}
          busy={busy === job.job}
          error={rowError?.job === job.job ? rowError.text : null}
          onSave={onSave}
          onSwitchBack={onSwitchBack}
        />
      ))}
      {panel.others.length > 0 && (
        <div data-aj="others">
          <b>{panel.others_title}</b>
          {panel.others.map((line) => (
            <div key={line}>{line}</div>
          ))}
        </div>
      )}
      <div data-aj="links">
        {panel.links.map((link, i) => {
          const to = linkTarget(link.target);
          return (
            <React.Fragment key={link.target}>
              {i > 0 && " · "}
              {link.lead}{" "}
              {to ? (
                <a
                  href={to.path}
                  data-aj="link"
                  onClick={(e) => {
                    e.preventDefault();
                    navigate(to.path, { state: { panel: to.panel } });
                  }}
                >
                  {link.label}
                </a>
              ) : (
                link.label
              )}
            </React.Fragment>
          );
        })}
      </div>
    </section>
  );
};

export default ModelJobsPanel;
