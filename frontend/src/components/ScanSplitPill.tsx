// =============================================================================
// ScanSplitPill — "Scan │ {model} ▾", the control that asks instead of running
// =============================================================================
//
// Mockup `FACTS_HEADER_MOCKUP_v1_2026-09-11`: one pill with a hairline divider
// down the middle. The left half is the verb, the right half is the noun it will
// act on, and the caret says the noun is changeable.
//
// ## Why the model picker is back on the page
//
// It has been gone since v2.1.0. `ThemeScanPanel` stopped rendering
// `ScanControlLine` in chromeless mode, and the header that replaced it offered
// `Scan again` with no picker at all — so every scan ran the deployment's
// default and the only way to use another model was to stop using the header.
// The split pill puts the choice back in the one place the verb already is.
//
// ## Why clicking the left half does not scan
//
// It opens the confirmation bar. See `scanConfirmModel` for the machine and for
// why that is asserted as a pure property rather than as a click.

import React from "react";

import type { ScanModel } from "../services/themeScan";

/** The shared pill shell: white, hairline, fully rounded. */
const pillStyle: React.CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  border: "1px solid var(--border-default)",
  borderRadius: "999px",
  background: "var(--bg-surface)",
  overflow: "hidden",
  flexShrink: 0,
};

/** The verb. 13px/600 — it is the half the eye lands on. */
const verbStyle: React.CSSProperties = {
  fontFamily: "inherit",
  fontSize: "13px",
  fontWeight: 600,
  color: "var(--text-primary)",
  background: "none",
  border: "none",
  padding: "7px 12px",
  cursor: "pointer",
};

const verbDisabledStyle: React.CSSProperties = {
  ...verbStyle,
  color: "var(--text-muted)",
  cursor: "not-allowed",
};

/** The hairline between the halves. `--border-default`, never the accent. */
const dividerStyle: React.CSSProperties = {
  width: "1px",
  alignSelf: "stretch",
  background: "var(--border-default)",
  flexShrink: 0,
};

/**
 * The noun. A real `<select>` rather than a bespoke menu: it is a one-of-N
 * choice, and the platform control already knows how to be reachable by
 * keyboard, readable by a screen reader and usable on a phone. The chrome is
 * removed so it reads as the pill's right half.
 */
const modelSelectStyle: React.CSSProperties = {
  fontFamily: "inherit",
  fontSize: "13px",
  color: "var(--text-secondary)",
  background: "none",
  border: "none",
  padding: "7px 10px",
  cursor: "pointer",
  // `appearance: none` drops the platform arrow; the mockup draws its own caret
  // beside it, and two arrows on one control reads as a rendering fault.
  appearance: "none",
  maxWidth: "220px",
  textOverflow: "ellipsis",
};

/** The caret glyph. A glyph, so it stays in code with `⋯` (the standing split). */
const caretStyle: React.CSSProperties = {
  // Sized against the mockup's own caret: at 10px muted it read as a speck of
  // dust on the pill rather than as the mark that says the model is changeable.
  fontSize: "11px",
  color: "var(--text-secondary)",
  paddingRight: "11px",
  // Decoration: the `<select>` beside it already announces itself and carries
  // the keyboard behaviour. A screen reader reading "down pointing triangle"
  // after the model name would be noise.
  pointerEvents: "none",
};

interface Props {
  /** The verb's stored word — "Scan". */
  label: string;
  models: ScanModel[];
  selectedModel: string | null;
  /** False while a run is in flight or the catalogue is empty. */
  canRun: boolean;
  /** Why it is refused, when it is. Shown as the control's tooltip. */
  refusal: string | undefined;
  /** Open the confirmation. NOT a scan — see the module header. */
  onOpenConfirm: () => void;
  onSelect: (modelId: string) => void;
}

/**
 * The split pill.
 *
 * `stopPropagation` on both halves: the whole header row is a collapse target
 * now, and without it choosing a model would also fold the section away under
 * the human's cursor.
 */
const ScanSplitPill: React.FC<Props> = ({
  label,
  models,
  selectedModel,
  canRun,
  refusal,
  onOpenConfirm,
  onSelect,
}) => (
  <span style={pillStyle} onClick={(e) => e.stopPropagation()}>
    <button
      type="button"
      style={canRun ? verbStyle : verbDisabledStyle}
      disabled={!canRun}
      // A control that refuses without saying why reads as a broken one. The
      // reasons it can refuse are genuinely different and say so.
      title={refusal}
      onClick={(e) => {
        e.stopPropagation();
        onOpenConfirm();
      }}
    >
      {label}
    </button>

    <span style={dividerStyle} />

    <select
      aria-label={label}
      style={modelSelectStyle}
      value={selectedModel ?? ""}
      onClick={(e) => e.stopPropagation()}
      onChange={(e) => onSelect(e.target.value)}
    >
      {models.map((m) => (
        <option key={m.model_id} value={m.model_id}>
          {m.display_label}
        </option>
      ))}
    </select>
    <span style={caretStyle} aria-hidden="true">
      ▾
    </span>
  </span>
);

export default ScanSplitPill;
