// Bias Explorer — default-subject application helper.
//
// The backend resolves CASE_DEFAULT_SUBJECT_NAME (an Ansible-managed env
// var) against the subjects list and surfaces the matching id as
// `available.default_subject_id`. The frontend only has to apply it, not
// match names — case-specific data stays out of the JS bundle.
//
// This module is split into a tiny pure helper so it can be unit-tested
// without React or jsdom (the project's frontend test pattern is
// pure-helper tests via vitest).
//
// The helper logs to `console.warn` when the server provides no default,
// so the absence is observable in dev tools (Standing Rule 1: distinct
// states must be observable). The user-facing observable is the About
// dropdown reading "All subjects" rather than a selected name.

import type { AvailableFilters } from "./types";

/**
 * Pick the initial value for the About filter from the server response.
 *
 * @returns the subject id to apply, or null if no default should be applied
 *          (server unset CASE_DEFAULT_SUBJECT_NAME or did not find a match).
 */
export function applyDefaultSubject(available: AvailableFilters): string | null {
    if (available.default_subject_id) {
        return available.default_subject_id;
    }
    // Distinct from a server error: the request succeeded, but the server
    // told us "no default". Surface to dev tools without disrupting the UI.
    console.warn(
        "Bias Explorer: server provided no default subject; About filter defaults to All subjects",
    );
    return null;
}
