// AdminLastRun.tsx — Admin → Data → Last document run (CC_TASK_MODEL_JOBS_PANEL_v1,
// board 5): the Overview's old document numbers, fixed, beside the stores and
// the indexing they describe. Every string arrives finished from the server.

import React, { useEffect, useState } from "react";

import { getLastRun, type LastRun } from "../../services/aiJobs";
import { errorLine, JOBS_CSS } from "./modelJobsStyles";

/** Not in the ruled copy: the tab's one sentence when it cannot be read. */
export const READ_ERROR_LINE =
  "The last document run could not be read. Reload the page to try again.";

const AdminLastRun: React.FC = () => {
  const [run, setRun] = useState<LastRun | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    getLastRun()
      .then((r) => {
        setRun(r);
        setFailed(false);
      })
      .catch((e: unknown) => {
        console.error("Last document run: the tab could not be read", e);
        setFailed(true);
      });
  }, []);

  if (failed) {
    return (
      <p style={errorLine} role="alert">
        {READ_ERROR_LINE}
      </p>
    );
  }
  if (!run) return null;

  return (
    <section data-surface="admin-jobs" data-aj="lastrun">
      <style>{JOBS_CSS}</style>
      {run.empty !== null ? (
        <p data-aj="progress">{run.empty}</p>
      ) : (
        <>
          <div data-aj="cards">
            {run.cards.map((c) => (
              <div data-aj="card" key={c.label}>
                <b>{c.value}</b>
                <span>{c.label}</span>
              </div>
            ))}
          </div>
          <p data-aj="progress">{run.progress}</p>
          {run.warnings.map((w) => (
            <p key={w} data-aj="warn" role="alert">
              {w}
            </p>
          ))}
          <table data-aj="table">
            <thead>
              <tr>
                {run.columns.map((c) => (
                  <th key={c}>{c}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {run.steps.map((s) => (
                <tr key={s.label}>
                  <td>{s.label}</td>
                  <td>{s.avg}</td>
                  <td>{s.runs}</td>
                  <td>{s.failed}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p data-aj="summary">{run.summary}</p>
        </>
      )}
    </section>
  );
};

export default AdminLastRun;
