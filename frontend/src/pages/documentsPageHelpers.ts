/**
 * Pure helpers for the Documents page.
 *
 * Extracted for the reason `configurationPanelHelpers.ts` was: this repo has no
 * component-testing setup, so anything left inline in the .tsx cannot be
 * asserted. Subject–verb agreement is exactly the kind of thing that reads fine
 * in review and is wrong on screen.
 */

/**
 * The attention banner's sentence.
 *
 * "1 document needs attention" · "3 documents need attention". The verb changes
 * with the number, which the previous inline expression did not do — it
 * pluralised the noun and left the verb as "need", so a single document read
 * "1 document need attention".
 *
 * Returns `null` for zero, so the caller renders nothing rather than a banner
 * announcing that everything is fine.
 */
export function attentionBannerText(count: number): string | null {
  if (!Number.isFinite(count) || count <= 0) return null;
  return count === 1
    ? "1 document needs attention — click to filter"
    : `${count} documents need attention — click to filter`;
}
