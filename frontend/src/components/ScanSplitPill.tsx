// =============================================================================
// ScanSplitPill — "Scan │ {model} ▾", the control that asks instead of running
// =============================================================================
//
// Mockup `FACTS_HEADER_MOCKUP_v1_2026-09-11`: one pill with a hairline divider
// down the middle. The left half is the verb, the right half is the noun it will
// act on, and the caret — drawn over the noun, not beside it — says the noun is
// changeable.
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
  paddingTop: "7px",
  paddingBottom: "7px",
  paddingLeft: "10px",
  // LOAD-BEARING, and a longhand for that reason: this reserves the caret's
  // room INSIDE this control's own box, which is what puts the glyph over the
  // select rather than beside it and makes a click on the arrow land here.
  // 26px is exactly the space the caret used to occupy as a sibling: this
  // control's own 10px right padding plus the caret span's 16px box (a ~5px
  // glyph and its 11px right padding), measured in a browser at both values.
  // So the text stops where it always did, the glyph keeps its 12px inset from
  // the pill's right edge, and the pill is the same width to the pixel.
  //
  // Written as `paddingRight` and not folded into the `padding` shorthand so
  // the guard in `scanPillCaret.test.tsx` can read it: a shorthand does not
  // serialize a `padding-right` for the test to compare against the caret's
  // inset, and that comparison is the whole proof that the arrow is not
  // overhanging the control again.
  paddingRight: "26px",
  cursor: "pointer",
  // `appearance: none` drops the platform arrow; the mockup draws its own caret
  // over it, and two arrows on one control reads as a rendering fault.
  appearance: "none",
  maxWidth: "220px",
  textOverflow: "ellipsis",
};

/**
 * The box the select and its caret share.
 *
 * `position: relative` is the whole mechanism: it makes this the containing
 * block the caret is positioned against, so the glyph lands on top of the
 * select's reserved right padding instead of beside it.
 */
const selectSlotStyle: React.CSSProperties = {
  position: "relative",
  display: "inline-flex",
  alignItems: "center",
};

/**
 * The caret glyph. A glyph, so it stays in code with `⋯` (the standing split).
 *
 * ## Why it is positioned OVER the select and not beside it
 *
 * It was a flex sibling of the `<select>` until 2026-09-11, and that made the
 * most obvious click target on the control dead. `pointer-events: none` let a
 * click fall THROUGH the glyph — but what it fell through to was the pill
 * wrapper, whose `onClick` calls `stopPropagation` to keep the header row from
 * folding. So the click reached the select (which was next to the caret, not
 * under it) never, and reached the row never either: it did nothing at all, on
 * the arrow that exists to say "this opens".
 *
 * Absolutely positioning it inside `selectSlotStyle` puts the select underneath
 * the glyph, so `pointer-events: none` now does what it was always meant to —
 * every click in this region reaches the control it decorates.
 *
 * `right: 11px` reproduces the inset the glyph had as a sibling, so nothing
 * moves on screen; only what is beneath it changed.
 */
const caretStyle: React.CSSProperties = {
  position: "absolute",
  right: "11px",
  // Sized against the mockup's own caret: at 10px muted it read as a speck of
  // dust on the pill rather than as the mark that says the model is changeable.
  fontSize: "11px",
  color: "var(--text-secondary)",
  // Decoration, and now load-bearing: the glyph covers the select's right
  // padding, and this is what lets the click through to it. A screen reader
  // reading "down pointing triangle" after the model name would be noise, which
  // is why the span is `aria-hidden` — the select announces itself.
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

    {/* The caret lives INSIDE this box, over the select's right padding — see
        `caretStyle` for the dead click target that put it there. */}
    <span style={selectSlotStyle}>
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
  </span>
);

export default ScanSplitPill;
