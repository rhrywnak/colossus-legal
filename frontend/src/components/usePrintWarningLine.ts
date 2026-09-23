// usePrintWarningLine.ts — the stored line the printed warning carries.
//
// A hook, not a prop drilled from the shell: the print pages are routes of their
// own and are reached directly (Chuck opens them in a tab to print), so each one
// asks for the words itself.
//
// On the real system nothing is fetched and nothing is printed — the frame
// renders no wrapper at all there.

import React from "react";

import { environmentOf, isTestSystem } from "./testSystem";
import { fetchEnvBannerWords } from "../services/envBanner";

/**
 * The stored `env_banner_print_line`, or `null` until (or unless) it arrives.
 *
 * `null` is not a failure the caller has to handle: `PrintWarningFrame` falls
 * back to the screen sentence, so paper from the test machine is marked either
 * way.
 */
export function usePrintWarningLine(): string | null {
  const [line, setLine] = React.useState<string | null>(null);
  React.useEffect(() => {
    if (!isTestSystem(environmentOf())) return;
    let live = true;
    void fetchEnvBannerWords().then((words) => {
      if (live && words !== null) setLine(words.print_line);
    });
    return () => {
      live = false;
    };
  }, []);
  return line;
}
