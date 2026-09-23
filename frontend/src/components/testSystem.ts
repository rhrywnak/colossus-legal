// testSystem.ts — which machine is this, and what does the warning say.
//
// PURE, and its own module for the reason CLAUDE.md rule 30 gives: this project
// tests pure helpers, not components, and a decision exported from a `.tsx` is a
// decision that will not be tested. This is the decision a witness's afternoon
// depends on, so it is tested.

/**
 * The one token that means "this is the real system".
 *
 * Ansible writes `dev` or `prod` into `config.js`
 * (`colossus-legal/templates/config.js.j2`) and the backend reads the same word
 * from `COLOSSUS_ENVIRONMENT`. It is not wording and never reaches a screen —
 * it is the vocabulary of the deployment, compared here and shown nowhere.
 */
// STRUCTURAL: wire vocabulary between Ansible and this build. `config.js.j2`
// writes it and this compares it; changing the word means changing both at once,
// which makes it a protocol token and never a per-deployment setting.
const PRODUCTION = "prod";

/**
 * Whether to show the test-system warning.
 *
 * ## ⚑ FAIL LOUD: anything that is not exactly the production token warns
 *
 * Missing config, an empty string, `unknown`, a typo, a hostname, a version —
 * all show the bar. A false "this is the real one" is the dangerous failure and
 * costs a witness her practice days before trial; a false warning costs a
 * moment's doubt. The comparison is exact and trimmed: `"prod "` from a
 * hand-edited file is not a promise this build will act on... it IS trimmed,
 * because a trailing newline in a generated file is not a different machine.
 */
export function isTestSystem(environment: string | undefined | null): boolean {
  return (environment ?? "").trim() !== PRODUCTION;
}

/** What the browser was told it is, if anything. */
export function environmentOf(win: Window | undefined = typeof window === "undefined" ? undefined : window): string | undefined {
  return win?.__COLOSSUS_CONFIG__?.environment;
}

/**
 * The warning, before the stored words arrive — and if they never do.
 *
 * ## ⚑ This duplicates `env_banner_text`, deliberately and provably
 *
 * The bar must paint before any request returns, and it must still be there
 * when the backend is down — a state the test machine reaches far more often
 * than the real one. The stored row remains the source of truth and replaces
 * this the moment it arrives. `envBanner.test.ts` reads the migration off disk
 * and fails if the two sentences ever differ, so a reworded row cannot ship
 * without this line following it.
 *
 * The same carve-out, for the same reason, as `PracticeQuestionPage`'s
 * `LOADING`: the one sentence that cannot wait for the payload it describes.
 */
// CARVE-OUT, ruled by CC_TASK_ENV_BANNER_v1_GO (R2): a configured value compiled
// in for bootstrap resilience, pinned to its source of truth by a test. The
// same shape, and the same reason, as `PracticeQuestionPage`'s `LOADING` — the
// one sentence that cannot wait for the payload that would word it. NOT marked
// STRUCTURAL, because that would claim it cannot vary by deployment, and it can:
// `env_banner_text` is editable on the Settings page and replaces this the
// moment it arrives. `envBanner.test.tsx` reads the migration and fails if the
// two ever differ.
export const FALLBACK_TEXT = "TEST SYSTEM — practice here is not saved for trial.";

/** What the bar shows right now: the stored words if they are here, else the fallback. */
export function bannerText(stored: string | null): string {
  const trimmed = (stored ?? "").trim();
  return trimmed === "" ? FALLBACK_TEXT : trimmed;
}
