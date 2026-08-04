// =============================================================================
// useAllegationOptions — the catalogue two sections share (2.10 / 2.12)
// =============================================================================
//
// The accusations every stuck card's link panel offers, and the wording that the
// facts list's Remove control needs as well.
//
// ## Why the PAGE reads this, and not each section
//
// Two sections need it: the queue's link panels (task 2.10) and the facts list's
// Remove control (task 2.12, item G). Fetching it in both would put two copies
// of one catalogue on one screen, free to disagree the moment somebody edits a
// row on the Settings page — and the queue re-reads its own pool after every
// ruling, so the copies would not even go stale together.
//
// `ScenarioDetailPage` already owns the reads its sections share; this is one of
// them. Extracted into a hook rather than living inline because that page sits
// against the 300-line limit (Rule 17) and this is a self-contained read.
//
// ## Once per scenario, not per ruling
//
// The catalogue is 120 rows on DEV and only changes when a document is processed
// — which the embargo forbids until 2.5. There is deliberately no refresh key.

import { useEffect, useState } from "react";

import { fetchAllegationOptions, type AllegationOptions } from "../services/evidenceLinks";

/**
 * Load the scenario's accusation catalogue once.
 *
 * A failure leaves `linkOptions` null, which withdraws the link panels and the
 * Remove control rather than rendering either with invented words (R4 leaves no
 * fallback to render). It is reported rather than swallowed: a control that is
 * silently absent on ninety-four cards is exactly the kind of absence nobody can
 * diagnose (Standing Rule 1).
 */
export function useAllegationOptions(
  slug: string,
  scenarioId: string | undefined,
): { linkOptions: AllegationOptions | null; linkOptionsError: string | null } {
  const [linkOptions, setLinkOptions] = useState<AllegationOptions | null>(null);
  const [linkOptionsError, setLinkOptionsError] = useState<string | null>(null);

  useEffect(() => {
    if (!scenarioId) return;
    // `cancelled` rather than an AbortController: the request already carries a
    // timeout signal from `authFetch`, and what this guards is a LATE response
    // painting over a newer scenario after navigation — the same discipline the
    // page's other reads use.
    let cancelled = false;

    fetchAllegationOptions(slug, scenarioId)
      .then((options) => {
        if (cancelled) return;
        setLinkOptions(options);
        setLinkOptionsError(null);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        const cause = err instanceof Error ? err.message : String(err);
        setLinkOptionsError(
          `The accusation list could not be loaded, so linking and removing are ` +
            `unavailable on this page: ${cause}`,
        );
      });

    return () => {
      cancelled = true;
    };
  }, [slug, scenarioId]);

  return { linkOptions, linkOptionsError };
}
