// PrintWarningFrame.tsx — the printed half of the test-system warning (board 6).
//
// Paper leaves the screen behind: a practice deck printed from the test machine
// must say so on every sheet, or it is indistinguishable from the real one on a
// table the morning of trial.
//
// ## ⚑ Why a table-header-group and not `position: fixed`
//
// A fixed element is painted once, on the first printed page, in Chrome. The
// mechanism print engines DO repeat at every page break is a table header, so
// in print media this wrapper becomes `display: table`, the warning row becomes
// `display: table-header-group`, and the content becomes `table-row-group`. The
// rules live in `practice/printStyles.PRINT_CSS` beside the rest of the print
// contract; `printChrome.test.ts` pins the pair, and the build report carries
// the page-by-page proof from a real `printToPDF`.
//
// On screen these are plain `div`s in normal flow, so nothing about the print
// view changes for the person looking at it.

import React from "react";

import { bannerText, environmentOf, isTestSystem } from "./testSystem";

/**
 * Wrap a printable page so its warning repeats at the top of every sheet.
 *
 * On the real system this renders its children and nothing else — no wrapper,
 * no extra element, so a printed deck from the real machine is byte-for-byte
 * the document it is today.
 */
const PrintWarningFrame: React.FC<{ line: string | null; children: React.ReactNode }> = ({
  line,
  children,
}) => {
  if (!isTestSystem(environmentOf())) return <>{children}</>;
  return (
    // ⚑ A REAL <table>, not divs with `display: table`. Chrome repeats a
    // header at a page break only for genuine table markup; the CSS-display
    // version was measured printing the warning on page 1 alone (5 pages, 1
    // carrying it) before this was changed.
    <table data-print-frame style={{ width: "100%", borderCollapse: "collapse" }}>
      <thead data-print-frame-head>
        <tr>
          <td>
            {/* `bannerText`'s sibling: the printed line falls back to the
                screen sentence when the stored words have not arrived, so
                paper is never unmarked. */}
            <div data-print-warning>{line ?? bannerText(null)}</div>
          </td>
        </tr>
      </thead>
      <tbody data-print-frame-body>
        <tr>
          <td>{children}</td>
        </tr>
      </tbody>
    </table>
  );
};

export default PrintWarningFrame;
