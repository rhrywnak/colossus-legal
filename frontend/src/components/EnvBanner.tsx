// EnvBanner.tsx — the bar that says this is not the machine trial runs on.
//
// Mockup of record: ENV_BANNER_MOCKUP_v1_2026-09-22 (ratified). Boards 1, 3, 4
// and 5 are this component in its two states; board 2 is superseded (the
// sign-in page belongs to Authentik, ruled 2026-09-22), and board 6 is the
// printed line below.
//
// ## Why it is here and not in a page
//
// Every page. A warning a person can navigate away from is not a warning, so it
// is in the shell above the header, and there is no close control anywhere in
// this file.
//
// ## ⚑ THE REAL SYSTEM RENDERS NOTHING
//
// Not a hidden element, not an empty bar, not a reserved strip: `null`. Board 4
// is the page exactly as it is today, and a layout that shifted by a bar's
// height on the real machine would be this task making the product worse.
//
// ## Not in the timeline popout (ruled 2026-09-22, R4)
//
// That window is deliberately chrome-free and is opened from a page that is
// already showing this bar. The popout route is matched above `AppShell`, so it
// never reaches this component.

import React from "react";

import { bannerText, environmentOf, isTestSystem } from "./testSystem";
import { fetchEnvBannerWords, type EnvBannerWords } from "../services/envBanner";

const bar: React.CSSProperties = {
  position: "sticky",
  top: 0,
  zIndex: 1000,
  display: "flex",
  flexWrap: "wrap",
  alignItems: "center",
  justifyContent: "center",
  gap: "0.75rem",
  padding: "0.5rem 1rem",
  background: "var(--warn-bg)",
  color: "var(--warn-ink)",
  font: "inherit",
  fontSize: 14,
  fontWeight: 700,
  lineHeight: 1.35,
  textAlign: "center",
};

const link: React.CSSProperties = {
  color: "var(--warn-link)",
  // Underlined as well as coloured: the link's contrast against the warning
  // ground is 6.48:1 — AA, not AAA — and colour alone is never the only signal.
  textDecoration: "underline",
  whiteSpace: "nowrap",
  fontWeight: 700,
};

/**
 * The printed warning: hidden on screen, repeated at the top of every printed
 * page by the frame in `printStyles` (board 6).
 */
export const PRINT_WARNING_ATTR = "data-print-warning";

const EnvBanner: React.FC = () => {
  const show = isTestSystem(environmentOf());
  const [words, setWords] = React.useState<EnvBannerWords | null>(null);

  React.useEffect(() => {
    // Nothing is fetched on the real system: it draws nothing, so there is
    // nothing to word.
    if (!show) return;
    let live = true;
    void fetchEnvBannerWords().then((got) => {
      if (live) setWords(got);
    });
    return () => {
      live = false;
    };
  }, [show]);

  if (!show) return null;

  return (
    <div style={bar} role="status" data-env-banner>
      <span>{bannerText(words?.text ?? null)}</span>
      {/* The link waits for the stored words — the address is a stored row, and
          the warning must never wait for it (ruling R3). */}
      {words !== null && (
        <a style={link} href={words.real_url}>
          {words.link_label} →
        </a>
      )}
    </div>
  );
};

export default EnvBanner;
