// =============================================================================
// SettingsPage — the admin configuration surface (task 1.6, v2 §2b)
// =============================================================================
//
// The law names this page and what it must show: "an admin Settings page listing
// every parameter with its current value, its default, and a one-line
// plain-language meaning; edits take effect on next read."
//
// ## Why this page exists at all
//
// Months of "change a number, rebuild, redeploy". Every threshold, cap and band
// boundary in the scenario function now lives in the database, and this is where
// they are changed. Nothing here is a preference panel — these numbers decide how
// candidate cards are banded and how many talking points a witness carries.
//
// ## What the browser does NOT do
//
// It does not compose the meaning, the hint, the bounds line or the dormancy
// label — those arrive built. It does not validate a value: the rules live beside
// the stored bounds on the backend, and a browser-side copy would be a second
// implementation of the configuration law in the one place that cannot see the
// store. A refusal comes back as a sentence naming the parameter and the limit,
// and is rendered verbatim beside the field that caused it.
//
// ## Dormant parameters are shown, and labelled
//
// Three of the seven have no consumer until Phase 2. They are listed and editable
// — the law says EVERY parameter — with the backend's note saying plainly that
// changing them does nothing today. A page that listed inert knobs silently would
// be lying about its own reach.

import React, { useCallback, useEffect, useState } from "react";

import {
  fetchSettings,
  setSetting,
  type SettingDto,
} from "../services/settings";

const pageStyle: React.CSSProperties = {
  padding: "2rem",
  maxWidth: "60rem",
  margin: "0 auto",
};

const rowStyle: React.CSSProperties = {
  borderTop: "1px solid var(--border-default)",
  padding: "1rem 0",
  display: "grid",
  gridTemplateColumns: "minmax(0, 2fr) minmax(0, 1fr)",
  gap: "1.5rem",
  alignItems: "start",
};

const labelStyle: React.CSSProperties = {
  fontSize: "0.75rem",
  color: "var(--text-muted)",
};

const keyStyle: React.CSSProperties = {
  fontFamily: "ui-monospace, monospace",
  fontSize: "0.72rem",
  color: "var(--text-muted)",
};

const fieldStyle: React.CSSProperties = {
  border: "1px solid var(--border-default)",
  borderRadius: "6px",
  padding: "0.4rem 0.6rem",
  fontWeight: 400,
  width: "100%",
};

/** One parameter's row: what it means on the left, what it is on the right. */
const SettingRow: React.FC<{
  setting: SettingDto;
  onSaved: (message: string) => void;
}> = ({ setting, onSaved }) => {
  const [draft, setDraft] = useState(setting.value);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Re-sync when the page reloads after a save elsewhere: the row's draft must
  // follow the stored value, or a second edit would submit stale text.
  useEffect(() => {
    setDraft(setting.value);
  }, [setting.value]);

  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      const changed = await setSetting(setting.key, draft);
      onSaved(changed.message);
    } catch (e: unknown) {
      // Explicit error UI, never a swallowed rejection. The backend's sentence
      // names the parameter and the bound, so it is shown as it arrived.
      setError(e instanceof Error ? e.message : "That change did not save.");
    } finally {
      setSaving(false);
    }
  };

  const dirty = draft.trim() !== setting.value;

  return (
    <div style={rowStyle}>
      <div>
        <div style={{ fontSize: "0.95rem", lineHeight: 1.6 }}>{setting.meaning}</div>
        <div style={{ ...keyStyle, marginTop: "0.35rem" }}>{setting.key}</div>
        {setting.dormant_note && (
          <div style={{ ...labelStyle, marginTop: "0.35rem", fontStyle: "italic" }}>
            {setting.dormant_note}
          </div>
        )}
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: "0.4rem" }}>
        <input
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          aria-label={setting.key}
          style={fieldStyle}
          disabled={saving}
        />
        <div style={labelStyle}>
          {setting.input_hint}
          {setting.bounds_label && ` · ${setting.bounds_label}`}
        </div>
        <div style={labelStyle}>
          Default {setting.default_value} · {setting.last_changed}
        </div>

        {dirty && (
          <button type="button" onClick={() => void save()} disabled={saving}>
            {saving ? "Saving…" : "Save"}
          </button>
        )}

        {error && (
          <div
            role="alert"
            style={{ fontSize: "0.8rem", color: "var(--state-danger-strong)" }}
          >
            {error}
          </div>
        )}
      </div>
    </div>
  );
};

const SettingsPage: React.FC = () => {
  const [settings, setSettings] = useState<SettingDto[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [confirmation, setConfirmation] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const page = await fetchSettings();
      setSettings(page.settings);
      setError(null);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "The settings did not load.");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  /** After a save: show the backend's confirmation and re-read the stored values. */
  const onSaved = useCallback(
    (message: string) => {
      setConfirmation(message);
      // Re-read rather than patching local state: the stored value is the truth,
      // and the backend trims and normalises what it was sent.
      void load();
    },
    [load],
  );

  if (loading) return <div style={pageStyle}>Loading the settings…</div>;

  if (error) {
    return (
      <div style={pageStyle}>
        <div role="alert" style={{ color: "var(--state-danger-strong)" }}>
          {error}
        </div>
        <button type="button" onClick={() => void load()} style={{ marginTop: "1rem" }}>
          Retry
        </button>
      </div>
    );
  }

  if (!settings) return <div style={pageStyle}>The settings are unavailable.</div>;

  return (
    <div style={pageStyle}>
      <h1 style={{ fontSize: "1.4rem", fontWeight: 500 }}>Settings</h1>
      <p style={{ color: "var(--text-muted)", marginTop: "0.5rem", lineHeight: 1.6 }}>
        Every tunable parameter in the scenario function. Changes take effect on
        the next read — no rebuild, no redeploy.
      </p>

      {confirmation && (
        <div
          role="status"
          style={{
            marginTop: "1rem",
            padding: "0.6rem 0.8rem",
            border: "1px solid var(--border-default)",
            borderRadius: "6px",
            background: "var(--bg-surface)",
          }}
        >
          {confirmation}
        </div>
      )}

      <div style={{ marginTop: "1.5rem" }}>
        {settings.map((setting) => (
          <SettingRow key={setting.key} setting={setting} onSaved={onSaved} />
        ))}
      </div>
    </div>
  );
};

export default SettingsPage;
