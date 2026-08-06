// =============================================================================
// WatchListSection — C6 restored to the page (§2.6)
// =============================================================================
//
// Human watch notes: what the other side will wave around. Own section since §2,
// rather than a block inside the augmentation form.
//
// ## Nothing computed, and nothing fake meanwhile
//
// Phase 2 (task 2.3) computes hazards from the included evidence and merges them
// into this list automatically. Until then this section holds ONLY human notes —
// the mockup's dashed "hazards computed…" row is a placement reservation and
// renders nothing (the Phase-1 law).
//
// ## Why the authoring block is reused rather than rewritten
//
// `WatchListBlock` already owns the write path — a watch-list note is structurally
// a human fact with `kind: "watch_list"`, so one write path enforces §8's
// invariants once for both. This section supplies the §2 chrome around it and the
// shared `run` helper; it does not re-implement the form. A second copy of that
// POST is a second place for the `kind` discriminator to be forgotten.

import React, { useState } from "react";

import WatchListBlock from "./WatchListBlock";
import {
  DIVIDER,
  sectionHeaderStyle,
  sectionMetaStyle,
  sectionPanelStyle,
  sectionTitleStyle,
} from "./scenarioSectionStyles";
import type { AuthoringWordingDto, HumanFactDto } from "../services/scenarioAugmentation";

/** The style bundle `WatchListBlock` takes — it renders, this decides how. */
const blockBoxStyle: React.CSSProperties = {
  background: "var(--bg-surface)",
  display: "flex",
  flexDirection: "column",
  gap: "0.6rem",
  padding: "18px 24px",
  fontWeight: 400,
};

const blockLabelStyle: React.CSSProperties = {
  fontSize: "0.75rem",
  color: "var(--text-muted)",
};

const blockFieldStyle: React.CSSProperties = {
  border: DIVIDER,
  borderRadius: "6px",
  padding: "0.4rem 0.6rem",
  fontWeight: 400,
  width: "100%",
  fontFamily: "inherit",
};

const blockTagStyle: React.CSSProperties = {
  fontSize: "0.72rem",
  color: "var(--text-muted)",
  fontStyle: "italic",
};

interface Props {
  slug: string;
  scenarioId: string;
  notes: HumanFactDto[];
  /** Every word this section speaks, from the store (task 2.11 C, ruling C4b). */
  wording: AuthoringWordingDto;
  /** Re-read the payload after a successful write. */
  onChanged: () => void;
}

const WatchListSection: React.FC<Props> = ({
  slug,
  scenarioId,
  notes,
  wording,
  onChanged,
}) => {
  const [error, setError] = useState<string | null>(null);

  /** Run a write, surface any failure, and re-read so the screen matches the DB. */
  const run = (work: () => Promise<void>) => {
    work()
      .then(() => {
        setError(null);
        onChanged();
      })
      .catch((e: unknown) => {
        // Explicit error UI, never a swallowed rejection: the human just typed
        // this, and they are the only one who can act on the refusal.
        setError(e instanceof Error ? e.message : wording.watch_save_failed_notice);
      });
  };

  return (
    <section>
      <div style={sectionHeaderStyle}>
        <h2 style={sectionTitleStyle}>{wording.watch_section_heading}</h2>
        <span style={sectionMetaStyle}>{wording.watch_section_meta}</span>
      </div>

      <div style={sectionPanelStyle}>
        {error && (
          <div
            role="alert"
            style={{
              color: "var(--state-danger-strong)",
              fontSize: "0.82rem",
              padding: "0.7rem 1.25rem 0",
            }}
          >
            {error}
          </div>
        )}

        <WatchListBlock
          slug={slug}
          scenarioId={scenarioId}
          notes={notes}
          wording={wording}
          run={run}
          boxStyle={blockBoxStyle}
          labelStyle={blockLabelStyle}
          fieldStyle={blockFieldStyle}
          tagStyle={blockTagStyle}
          hairline={DIVIDER}
        />

        {/* COMPUTED hazards (task 2.3) merge into this list. Placement reserved,
            renders nothing — absent, not fake. */}
      </div>
    </section>
  );
};

export default WatchListSection;
