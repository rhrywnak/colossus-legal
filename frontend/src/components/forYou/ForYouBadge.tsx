// =============================================================================
// ForYouBadge.tsx — the count beside "For you", in the bar (CC_TASK_FOR_YOU_v1)
// =============================================================================
//
// Its own component rather than forty lines inside `Header.tsx`, which Rule 17
// put at 335 the moment they were added there. The seam is also the right one:
// everything about "what the bar says about waiting work" — the number, its
// absence, and what is shown when the number could not be read — is here, and
// the header only says WHERE it goes.
//
// ## Domain note: three states, and only two of them draw anything
//
//   something waiting → the number
//   nothing waiting   → NOTHING. Not a "0": a zero beside a menu item is a thing
//                       to read and dismiss on every page load (ruling Q1).
//   could not be read → a muted marker, because "nothing waiting" and "we could
//                       not tell" are different facts and Standing Rule 1 asks
//                       for the second to be visible.

import React from "react";

import { useForYou } from "../../context/ForYouContext";
import {
  FOR_YOU_COUNT_UNAVAILABLE_LABEL,
  FOR_YOU_COUNT_UNAVAILABLE_MARK,
} from "../navItems";
import { badgeLabel } from "./forYouView";

const badgeStyle: React.CSSProperties = {
  marginLeft: "0.35rem",
  padding: "0.05rem 0.34rem",
  borderRadius: "999px",
  fontSize: "0.7rem",
  fontWeight: 700,
  lineHeight: 1.6,
  color: "var(--bg-surface)",
  backgroundColor: "var(--accent-primary)",
};

/**
 * Muted rather than alarming: nothing is broken for the person reading, and the
 * list itself is one click away.
 */
const unknownStyle: React.CSSProperties = {
  ...badgeStyle,
  color: "var(--text-muted)",
  backgroundColor: "var(--bg-page)",
};

const ForYouBadge: React.FC = () => {
  const { count, failed } = useForYou();
  const label = badgeLabel(count);

  if (label !== null) {
    return (
      <span style={badgeStyle} data-for-you-badge>
        {label}
      </span>
    );
  }
  if (failed) {
    return (
      <span
        style={unknownStyle}
        data-for-you-badge-failed
        title={FOR_YOU_COUNT_UNAVAILABLE_LABEL}
        aria-label={FOR_YOU_COUNT_UNAVAILABLE_LABEL}
        role="status"
      >
        {FOR_YOU_COUNT_UNAVAILABLE_MARK}
      </span>
    );
  }
  // Nothing waiting: the entry stands on its own.
  return null;
};

export default ForYouBadge;
