// ChatCaseFileBox.tsx — Admin → Overview: keep the chat's case file loaded.
//
// Roman's box (CC_TASK_KEEPWARM_BUTTON_v1): a title, one explanatory line, the
// last chat question, how long the case file stays loaded, whether the automatic
// ping is on (a link to its settings), and one button. Both
// activity lines load when the box opens and are replaced by the server's fresh
// ones after every tap. Every failure is shown in the box and logged.

import React, { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import {
  getChatCaseFile,
  keepLoaded,
  type ChatCaseFileActivity,
} from "../services/chatCaseFile";
import {
  BOX_INTRO,
  BOX_TITLE,
  BUTTON_BUSY_LABEL,
  BUTTON_LABEL,
  ERROR_LINE,
  READ_ERROR_LINE,
  lastQuestionLine,
  loadedLine,
  resultLine,
} from "./chatCaseFileText";
import { adminSettingsPath } from "../utils/routePaths";

const boxStyle: React.CSSProperties = {
  background: "var(--bg-surface)",
  border: "1px solid var(--border-default)",
  borderRadius: "8px",
  padding: "1rem 1.1rem",
  marginBottom: "1.25rem",
  maxWidth: "36rem",
};

const titleStyle: React.CSSProperties = {
  fontSize: "0.95rem",
  fontWeight: 600,
  color: "var(--text-primary)",
  margin: "0 0 0.3rem",
};

const lineStyle: React.CSSProperties = {
  fontSize: "0.84rem",
  color: "var(--text-secondary)",
  margin: "0.2rem 0",
};

/** The automatic line is a link to the settings that govern it. */
const settingsLinkStyle: React.CSSProperties = {
  color: "var(--accent-primary)",
  textDecoration: "underline",
};

const buttonStyle: React.CSSProperties = {
  marginTop: "0.7rem",
  padding: "0.55rem 1rem",
  fontSize: "0.84rem",
  fontWeight: 600,
  fontFamily: "inherit",
  color: "var(--bg-page)",
  background: "var(--accent-primary)",
  border: "none",
  borderRadius: "6px",
  cursor: "pointer",
};

const ChatCaseFileBox: React.FC = () => {
  const [activity, setActivity] = useState<ChatCaseFileActivity | null>(null);
  const [readFailed, setReadFailed] = useState(false);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [resultFailed, setResultFailed] = useState(false);

  const load = useCallback(() => {
    getChatCaseFile()
      .then((a) => {
        setActivity(a);
        setReadFailed(false);
      })
      .catch((e: unknown) => {
        console.error("Chat case file: the activity lines could not be read", e);
        setReadFailed(true);
      });
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const tap = () => {
    setBusy(true);
    setResult(null);
    keepLoaded()
      .then((r) => {
        setResult(resultLine(r));
        setResultFailed(false);
        setActivity(r.activity);
        setReadFailed(false);
      })
      .catch((e: unknown) => {
        console.error("Chat case file: the keep-loaded request failed", e);
        setResult(ERROR_LINE);
        setResultFailed(true);
        // The lines refresh after EVERY tap, a failed one included: a ping that
        // failed on the way back may still have reached the provider.
        load();
      })
      .finally(() => setBusy(false));
  };

  return (
    <section style={boxStyle} aria-label={BOX_TITLE}>
      <h2 style={titleStyle}>{BOX_TITLE}</h2>
      <p style={lineStyle}>{BOX_INTRO}</p>
      {readFailed && (
        <p style={{ ...lineStyle, color: "var(--state-danger-strong)" }} role="alert">
          {READ_ERROR_LINE}
        </p>
      )}
      {activity && (
        <>
          <p style={lineStyle}>{lastQuestionLine(activity)}</p>
          <p style={lineStyle}>{loadedLine(activity)}</p>
          {/* CC_TASK_CACHE_KEEPWARM_v1: whether the automatic ping is on, as
              the server words it; the whole line opens Admin → Settings. */}
          <p style={lineStyle}>
            <Link to={adminSettingsPath()} style={settingsLinkStyle}>
              {activity.automatic_line}
            </Link>
          </p>
        </>
      )}
      <button
        type="button"
        style={{ ...buttonStyle, opacity: busy ? 0.6 : 1, cursor: busy ? "wait" : "pointer" }}
        onClick={tap}
        disabled={busy}
      >
        {busy ? BUTTON_BUSY_LABEL : BUTTON_LABEL}
      </button>
      {result && (
        <p
          style={{
            ...lineStyle,
            marginTop: "0.6rem",
            color: resultFailed ? "var(--state-danger-strong)" : "var(--text-primary)",
          }}
          role={resultFailed ? "alert" : "status"}
        >
          {result}
        </p>
      )}
    </section>
  );
};

export default ChatCaseFileBox;
