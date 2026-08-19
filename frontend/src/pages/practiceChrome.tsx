// =============================================================================
// practiceChrome.tsx — the frame both practice pages render inside
// =============================================================================
//
// The breadcrumb, the four states a page can be in before it has a deck
// (loading, load failure, deck failure, empty deck), and the `data-surface`
// attribute that resolves the whole palette.
//
// ## Why this is shared rather than written twice
//
// Section B gave the sitting an address of its own, which made two pages out of
// one — and both of them fetch the same deck, both can fail the same three ways,
// and both must carry `data-surface="practice"` or render with no colour at all.
// Two copies is how one of them eventually loses the attribute, and it would be
// the FAILURE screen, which is the one nobody looks at until it matters.

import React from "react";

import Breadcrumb from "../components/Breadcrumb";
import * as s from "../components/practice/practiceStyles";
import type { PracticeDeck } from "../services/practice";
import { scenarioPagePath, trialPrepPath } from "../utils/routePaths";

/** The trail above every practice screen. */
export const PracticeCrumb: React.FC<{
  slug: string;
  scenarioId: string;
  deck: PracticeDeck | null;
}> = ({ slug, scenarioId, deck }) => (
  <Breadcrumb
    items={[
      { label: "Dashboard", to: "/" },
      { label: "Trial Prep", to: trialPrepPath(slug) },
      ...(deck === null
        ? []
        : [{ label: `${deck.code} · ${deck.title}`, to: scenarioPagePath(slug, scenarioId) }]),
      { label: "Practice" },
    ]}
  />
);

/**
 * One card inside the practice frame, with the palette attribute on it.
 *
 * `alert` is passed for the failure states so a screen reader announces them:
 * the two that can appear are both "the thing you asked for did not happen",
 * which is precisely what the role exists for.
 */
export const PracticeFrame: React.FC<{
  crumb: React.ReactNode;
  alert?: boolean;
  children: React.ReactNode;
}> = ({ crumb, alert = false, children }) => (
  <div style={s.page} data-surface="practice">
    {crumb}
    <section style={s.card} {...(alert ? { role: "alert" } : {})}>
      {children}
    </section>
  </div>
);

/**
 * The load-failure card.
 *
 * It names the UNDERLYING failure rather than replacing it with a friendlier
 * lie, and it cannot use a stored sentence — the payload carrying the wording is
 * the thing that failed.
 */
export const PracticeLoadFailure: React.FC<{ crumb: React.ReactNode; message: string }> = ({
  crumb,
  message,
}) => (
  <PracticeFrame crumb={crumb} alert>
    <div style={s.feedback}>{message}</div>
  </PracticeFrame>
);

/** The card that stands while the deck is in flight. */
export const PracticeLoading: React.FC<{ crumb: React.ReactNode; label: string }> = ({
  crumb,
  label,
}) => (
  <PracticeFrame crumb={crumb}>
    <span style={s.progress}>{label}</span>
  </PracticeFrame>
);
