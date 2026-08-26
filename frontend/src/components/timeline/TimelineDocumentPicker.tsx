// =============================================================================
// TimelineDocumentPicker.tsx — search the store, pick a document, name the page
// =============================================================================
//
// Mockup v2 Screen 3's `.linkadd` row, and Screen 2's "+ Link a document…".
// ONE component for both, because they are the same act: design R9 makes every
// link human-made, so this search is how every link in the chronology is born.
//
// ## ⚑ IT SEARCHES THE SAME TABLE THE RESOLVER READS
//
// `GET /api/timeline/documents` reads `documents` in the pipeline database —
// the table `existing_document_ids` answers "does this link point at something
// real?" from. If the picker searched anywhere else, an author could pick a
// document that the very next render marked "⚠ no document yet". Pick and
// resolve are the same question asked twice, so they ask one table.
//
// ## The cap is visible
//
// The response carries how many documents matched alongside the ones it
// returned. When they differ the picker says so. A short list that looked
// complete is how somebody links the wrong document with no idea a better match
// was cut off.

import React, { useEffect, useState } from "react";

import type { ChronologyWording } from "../../services/caseTimeline";
import { cw, fill } from "../../services/caseTimeline";
import {
  type DocumentChoice,
  type DocumentSearchResult,
  searchTimelineDocuments,
} from "../../services/caseTimelineWrites";
import * as s from "../../components/timeline/timelineStyles";
import * as w from "./timelineWriteStyles";

type Props = {
  wording: ChronologyWording;
  /** What to do with the chosen document and the pinpoint typed beside it. */
  onPick: (choice: DocumentChoice, pinpoint: string) => void;
};

/**
 * How long the picker waits after a keystroke before asking the server.
 *
 * A presentational timing constant, not a §2b tunable: nobody reads it, it does
 * not change what the system decides, and it exists only so typing "complaint"
 * is one request rather than nine. Named so the number is not loose in a hook.
 */
const SEARCH_DEBOUNCE_MS = 250;

const TimelineDocumentPicker: React.FC<Props> = ({ wording, onPick }) => {
  const [query, setQuery] = useState("");
  const [result, setResult] = useState<DocumentSearchResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pinpoint, setPinpoint] = useState("");

  useEffect(() => {
    const needle = query.trim();
    if (needle === "") {
      // Nothing typed is a DIFFERENT state from a fruitless search: the empty
      // state below only speaks when the server was actually asked.
      setResult(null);
      setError(null);
      return;
    }
    let cancelled = false;
    const timer = window.setTimeout(() => {
      searchTimelineDocuments(needle)
        .then((page) => {
          if (!cancelled) {
            setResult(page);
            setError(null);
          }
        })
        .catch((err: unknown) => {
          // Never swallowed. A picker that quietly returned nothing on a failed
          // request tells an author a document does not exist when the truth is
          // that nobody asked.
          if (!cancelled) {
            setResult(null);
            setError(err instanceof Error ? err.message : "unknown error");
          }
        });
    }, SEARCH_DEBOUNCE_MS);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [query]);

  const capped = result !== null && result.total > result.matches.length;

  return (
    <div style={w.picker}>
      <input
        style={w.input}
        placeholder={cw(wording, "document_search_placeholder")}
        aria-label={cw(wording, "document_search_placeholder")}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
      />

      {/* The pinpoint travels WITH the pick, so a document chosen in a hurry
          carries whatever page was typed beside it. Leaving it empty is a real
          choice and the link is marked for it (design R9) — the placeholder
          says so out loud. */}
      <input
        style={{ ...w.input, marginTop: "0.4rem" }}
        placeholder={cw(wording, "pinpoint_placeholder")}
        aria-label={cw(wording, "pinpoint_placeholder")}
        value={pinpoint}
        onChange={(e) => setPinpoint(e.target.value)}
      />

      {error !== null && (
        <div style={w.writeError}>
          {fill(cw(wording, "write_failed_template"), { reason: error })}
        </div>
      )}

      {result !== null && result.matches.length === 0 && error === null && (
        <div style={s.panelEmpty}>{cw(wording, "document_search_empty_label")}</div>
      )}

      {result !== null && result.matches.length > 0 && (
        <div style={w.pickerResults}>
          {result.matches.map((choice) => (
            <button
              key={choice.id}
              type="button"
              style={w.pickerRow}
              onClick={() => {
                onPick(choice, pinpoint);
                // Cleared so the next pick starts fresh: a stale pinpoint
                // silently attached to a second document would be a page
                // number nobody typed for it.
                setQuery("");
                setPinpoint("");
                setResult(null);
              }}
            >
              {choice.title}
            </button>
          ))}
        </div>
      )}

      {capped && result !== null && (
        <div style={w.pickerCapped}>
          {fill(cw(wording, "picker_capped_template"), {
            shown: result.matches.length,
            total: result.total,
          })}
        </div>
      )}
    </div>
  );
};

export default TimelineDocumentPicker;
