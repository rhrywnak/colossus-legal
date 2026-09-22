// =============================================================================
// ForYouContext — the menu badge's number, fetched once and refreshed on demand
// =============================================================================
//
// CC_TASK_FOR_YOU_v1 L1. The count rides EVERY page in the app, so it is
// fetched once on mount and shared, exactly as `AuthContext` shares the user.
// Without a context, every screen carrying the header would make its own
// request for the same number.
//
// ## A failed count is VISIBLE, and is not the same thing as zero
//
// The badge is a claim that work is waiting, so when the read fails there is no
// claim to make and no number is drawn. But absent-because-nothing-is-waiting
// and absent-because-we-could-not-tell are two different facts, and Standing
// Rule 1 says every `authFetch` failure gets a user-facing surface — the
// best-effort carve-out covers cosmetic browser storage and explicitly not a
// data read. So the failure is STATE here (`failed`), the header draws a marker
// in the badge's place, and the console keeps the technical cause with the case
// named. A banner across every screen would be disproportionate for a
// background count; a marker where the number would have been is not.

import React, { createContext, useCallback, useContext, useEffect, useState } from "react";

import { fetchForYouSummary } from "../services/forYou";

type ForYouContextValue = {
  /** Unread items for the signed-in person, or `null` when unknown. */
  count: number | null;
  /** True when the last read FAILED — distinct from a count of zero. */
  failed: boolean;
  /** Re-read the count — after opening a question clears one. */
  refresh: () => void;
};

const ForYouContext = createContext<ForYouContextValue | undefined>(undefined);

export const ForYouProvider: React.FC<{ slug: string; children: React.ReactNode }> = ({
  slug,
  children,
}) => {
  const [count, setCount] = useState<number | null>(null);
  const [failed, setFailed] = useState(false);

  const read = useCallback(
    (live: () => boolean) => {
      fetchForYouSummary(slug)
        .then((summary) => {
          if (!live()) return;
          setCount(summary.unread_count);
          setFailed(false);
        })
        .catch((cause: unknown) => {
          // eslint-disable-next-line no-console
          console.error(`for you: the count could not be read (case ${slug})`, cause);
          // Unknown, not zero — see the header. `null` draws no number and
          // claims nothing; `failed` is what puts the marker on screen.
          if (!live()) return;
          setCount(null);
          setFailed(true);
        });
    },
    [slug],
  );

  useEffect(() => {
    let alive = true;
    read(() => alive);
    // Cleanup: no state updates after unmount — the `AuthContext` pattern.
    return () => {
      alive = false;
    };
  }, [read]);

  const refresh = useCallback(() => read(() => true), [read]);

  return (
    <ForYouContext.Provider value={{ count, failed, refresh }}>{children}</ForYouContext.Provider>
  );
};

/**
 * The badge's count and its refresh.
 *
 * Returns a quiet default OUTSIDE the provider rather than throwing, unlike
 * `useAuth`. The difference is deliberate: a screen that needs the user cannot
 * work without it, and a screen that cannot read a badge count can — the print
 * views and the popout render outside this provider on purpose, and a throw
 * there would take down a page over a number it does not show.
 */
export function useForYou(): ForYouContextValue {
  return useContext(ForYouContext) ?? { count: null, failed: false, refresh: () => {} };
}
