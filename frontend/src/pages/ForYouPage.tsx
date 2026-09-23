// =============================================================================
// ForYouPage.tsx — "what is waiting for me?", across every deck in the case
// =============================================================================
//
// CC_TASK_FOR_YOU_v1 layer L1, from the ratified mockup
// `FOR_YOU_MOCKUP_v1_2026-09-22.html` (boards 1, 2, 3 and 5).
//
// ## The problem it solves
//
// Marie has answered 185 questions across 11 scenarios. Chuck writes on one of
// them. Until today the only signal was a per-deck count on the war room, and
// nothing inside the deck marked the question — so finding his sentence meant
// opening questions until she found it.
//
// ## One page, two audiences
//
// The same page serves the witness and the reviewers. What differs is which
// side's items the SERVER lists and which sentences it composes for them; this
// file does not know which side it is drawing, beyond rendering the subtitle it
// was handed and showing the "not your list" sentence when the payload says the
// reader is neither. Rule 12: no business logic here.
//
// ## Every string is the server's
//
// A row arrives with its three lines already written, and the tab labels arrive
// with their counts in them. This file fills no template. The only strings it
// reads from the store directly are its own furniture — the title, three day
// headings and the empty state — and each of those is a LITERAL `w("…")` call,
// because the reach scanner that proves every key exists on the wire cannot see
// a key built from a variable.

import React from "react";
import { useParams, useSearchParams } from "react-router-dom";

import ForYouRow from "../components/forYou/ForYouRow";
import { groupByDay } from "../components/forYou/forYouView";
import * as s from "../components/forYou/forYouStyles";
import { useForYou } from "../context/ForYouContext";
import { DEFAULT_CASE_SLUG } from "../services/caseHeader";
import {
  fetchForYou,
  wordingOf,
  type ForYouDay,
  type ForYouPage as Page,
} from "../services/forYou";

/** Which tab is showing. The unread one is where a person starts. */
type Tab = "unread" | "everything";

const ForYouPage: React.FC = () => {
  const { slug = DEFAULT_CASE_SLUG } = useParams();
  // ONE deck, when the war room's count sent the reader here. The rows carry
  // their own deck line either way, so a filtered list still says what it is
  // showing, and the menu entry is the unfiltered address one click away.
  const [search] = useSearchParams();
  // STRUCTURAL: the READ side of the same `deck` parameter `routePaths` and
  // `services/forYou` write. Three sites, one string, and a disagreement shows
  // as an unfiltered page rather than as an error.
  const deck = search.get("deck") ?? undefined;
  const [page, setPage] = React.useState<Page | null>(null);
  const [error, setError] = React.useState<string | null>(null);
  const [tab, setTab] = React.useState<Tab>("unread");
  const { refresh } = useForYou();

  React.useEffect(() => {
    let live = true;
    fetchForYou(slug, deck)
      .then((next) => {
        if (!live) return;
        setPage(next);
        // The page and the badge are two readings of one number; landing here
        // is the moment to make sure the menu agrees with what is on screen.
        refresh();
      })
      .catch((cause: unknown) => {
        // eslint-disable-next-line no-console
        console.error("for you: the list could not be loaded", cause);
        // Standing Rule 1: the message names WHICH case failed. The stored
        // sentence is not available — `wording` arrives inside the payload that
        // did not arrive — so the technical detail is what this page can
        // honestly say, the same carve-out `PracticeReviewPage` records.
        const detail = cause instanceof Error ? cause.message : String(cause);
        if (live) setError(`case ${slug}: ${detail}`);
      });
    return () => {
      live = false;
    };
  }, [slug, deck, refresh]);

  if (page === null) {
    return (
      <div style={s.page} data-for-you-page>
        {error !== null && (
          <div style={s.alert} role="alert">
            {error}
          </div>
        )}
      </div>
    );
  }

  const w = (key: string) => wordingOf(page.wording, key);
  // ⚑ Three LITERAL calls, deliberately. The reach scanner that proves every
  // requested key exists on the wire matches the literal shape `w("key")` only;
  // `w(keyFor(day))` would be invisible to it, and a missing heading would then
  // throw in front of a reader with nothing having warned us. The choice is
  // written AROUND the calls, never inside one.
  const dayHeading = (day: ForYouDay): string => {
    if (day === "today") return w("group_today");
    if (day === "yesterday") return w("group_yesterday");
    return w("group_earlier");
  };
  const rows = tab === "unread" ? page.unread : page.everything;
  const groups = groupByDay(rows);

  return (
    <div style={{ background: "var(--bg-canvas)" }} data-surface="for-you">
      <div style={s.page} data-for-you-page>
        <h1 style={s.title}>{w("title")}</h1>
        <p style={s.subtitle}>{page.subtitle}</p>

        {page.side === "none" ? (
          <div style={s.empty}>
            <div style={s.emptyTitle}>{w("not_your_list")}</div>
          </div>
        ) : (
          <>
            <div style={s.tabs} role="tablist">
              <button
                type="button"
                role="tab"
                aria-selected={tab === "unread"}
                style={tab === "unread" ? s.tabActive : s.tab}
                onClick={() => setTab("unread")}
                data-for-you-tab="unread"
              >
                {page.tab_unread_label}
              </button>
              <button
                type="button"
                role="tab"
                aria-selected={tab === "everything"}
                style={tab === "everything" ? s.tabActive : s.tab}
                onClick={() => setTab("everything")}
                data-for-you-tab="everything"
              >
                {page.tab_everything_label}
              </button>
            </div>

            {groups.length === 0 ? (
              <div style={s.empty} data-for-you-empty>
                <div style={s.emptyTitle}>{w("empty_title")}</div>
                <div style={s.emptyLine}>{page.empty_hint}</div>
                {/* Withheld entirely when this side has never had an item: a
                    line reading "Last one: ." says the page half-failed. */}
                {page.empty_last !== undefined && (
                  <div style={s.emptyLine}>{page.empty_last}</div>
                )}
              </div>
            ) : (
              groups.map((group) => (
                <section key={group.day}>
                  <div style={s.dayHeading}>{dayHeading(group.day)}</div>
                  {group.rows.map((row) => (
                    <ForYouRow key={`${row.kind}:${row.item_id}`} row={row} slug={slug} />
                  ))}
                </section>
              ))
            )}
          </>
        )}
      </div>
    </div>
  );
};

export default ForYouPage;
