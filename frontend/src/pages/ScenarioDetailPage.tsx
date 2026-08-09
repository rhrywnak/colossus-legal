// =============================================================================
// ScenarioDetailPage.tsx — /cases/:slug/trial-prep/:scenarioId
// =============================================================================
//
// The FINAL page structure (task 1.7C, SCENARIO_PAGE_DESIGN_v2 §2). Every §2
// component now has exactly one home here, and the future-phase ones have their
// placement reserved and render NOTHING until their task lands — absent, not fake.
//
// SEVEN SECTIONS, TOP TO BOTTOM
//
//   1. Two-tier header + eyebrow      §2.1 (D7) — `ScenarioHeaderTiers`
//   2. Identity block, read-only      §2.2 (D8) — `ScenarioIdentityBlock`
//   3. Scan & candidates              §2.3 (D10) — `ScanSection`
//   4. Scenario facts                 §2.4 — `ScenarioFactsSection`
//   5. Marie's talking points         §2.5 (C5) — `TalkingPointsSection`
//   6. Watch-list                     §2.6 (C6) — `WatchListSection`
//   7. Orphan strip, collapsed        §2.8 (D9) — `ScenarioOrphanStrip`
//
// ## This file is composition and fetching, nothing else
//
// Each section owns its own rendering and its own writes; this page owns the three
// reads they share and the two modals. That is what keeps it inside the 300-line
// limit with seven sections on it — the 1.7B version put the header's markup, its
// styles, and the delete flow inline, and there was no room left.
//
// ## WHAT DIED HERE IN 1.7C
//
//   * `pattern_summary` and `notes`. Both were hardcoded `None` in
//     `scenario_dashboard::build_detail` and neither has a column or a jsonb key —
//     measured on DEV, so they could never render. §2's seven sections are the
//     complete page and neither has a home in it (ruling R5).
//   * The bare `Delete` button in the header row. Destructive actions live in the
//     kebab and nowhere else (D7).
//   * "Marie's responses" / `ResponseCard`. §2.5's talking-points section is what
//     that data is for, and it now has a real one.
//
// ## The augmentation read became unconditional
//
// It used to fire only after a human clicked "Add", because the panel it fed was
// hidden until then. Three sections need it now — the identity block reads the C1
// texts from it, and §2.5/§2.6 are always present — so it loads with the page.

import React, { useCallback, useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";

import AccusationSection from "../components/AccusationSection";
import { includedPickableFacts } from "../components/accusationFacts";
import Breadcrumb from "../components/Breadcrumb";
import ScanSection from "../components/ScanSection";
import ScenarioDeleteConfirm from "../components/ScenarioDeleteConfirm";
import { scenarioDeleteCopy } from "../components/scenarioDeleteCopy";
import ScenarioFactsSection from "../components/ScenarioFactsSection";
import ScenarioHeaderTiers from "../components/ScenarioHeaderTiers";
import { ghostButtonStyle } from "../components/scenarioSectionStyles";
import ScenarioIdentityBlock from "../components/ScenarioIdentityBlock";
import ScenarioIdentityModal from "../components/ScenarioIdentityModal";
import ScenarioOrphanStrip from "../components/ScenarioOrphanStrip";
import TalkingPointsSection from "../components/TalkingPointsSection";
import WatchListSection from "../components/WatchListSection";
import { EmptyState } from "../components/TrialPrepViews";
import { getAllegations, type AllegationDto } from "../services/allegations";
import { DEFAULT_CASE_SLUG } from "../services/caseHeader";
import { useAllegationOptions } from "../components/useAllegationOptions";
import ScenarioNotice from "../components/ScenarioNotice";
import {
  fetchScenarioCards,
  type ProposalSource,
  type ScenarioCard,
} from "../services/scenarioCards";
import {
  fetchAccusationPanel,
  type AccusationPanelDto,
} from "../services/scenarioAccusation";
import {
  fetchAugmentationPanel,
  type AugmentationPanelDto,
} from "../services/scenarioAugmentation";
import { deleteScenario } from "../services/scenarioCrud";
import { getScenarioDetailLive } from "../services/trialPrep";
import { trialPrepPath } from "../utils/routePaths";
import type { ScenarioDetail } from "./trialPrepData";

/**
 * The page container, and the v3 SURFACE SCOPE.
 *
 * `data-surface="v3"` is what makes the mockup's palette apply here and nowhere
 * unreviewed (ruling R1). Every token the v3 `:root` redefines is re-declared under
 * that selector in tokens.css, and custom properties inherit — so the components
 * below keep saying `var(--text-primary)` and get the mockup's #1a202c, while the
 * same components on other screens get the app's #101828. The cascade does the
 * scoping; app-wide adoption is task 3.14.
 *
 * Padding transcribed from the mockup's `body`: 28px top, 40px sides, 80px bottom.
 * Scoped rather than applied to the app shell (ruling R5) — the shell's 32px inset
 * governs every other screen and moving it globally was not this task's to do.
 */
const containerStyle: React.CSSProperties = {
  padding: "28px 40px 80px",
  // The negative side margin ESCAPES the app shell's own 2rem inset before applying
  // the mockup's 40px. Without it the two insets add up: the shell's 1080px box less
  // 64px less 80px left 936px of content where the mockup has exactly 1000px, and
  // the 64px shortfall wrapped the status control onto its own line — which the
  // screenshot comparison caught and a checklist would not have.
  marginLeft: "-2rem",
  marginRight: "-2rem",
};
const messageStyle: React.CSSProperties = { padding: "2rem 0", color: "var(--text-muted)" };
const errorStyle: React.CSSProperties = {
  padding: "0.75rem 1rem",
  border: "1px solid var(--state-danger-border)",
  borderRadius: "6px",
  color: "var(--state-danger-strong)",
  background: "var(--state-danger-bg-soft)",
  fontSize: "0.85rem",
  marginBottom: "1rem",
};

const ScenarioDetailPage: React.FC = () => {
  const { slug: slugParam, scenarioId } = useParams<{ slug: string; scenarioId: string }>();
  const slug = slugParam ?? DEFAULT_CASE_SLUG;
  const navigate = useNavigate();
  const backCrumb = { label: "Trial Prep", to: trialPrepPath(slug) };

  // Gating fetch. `null` after load = a real 404, which renders the "not found"
  // empty state — distinct from a fetch error (banner) and from still-loading.
  const [scenario, setScenario] = useState<ScenarioDetail | null>(null);
  const [augmentation, setAugmentation] = useState<AugmentationPanelDto | null>(null);
  const [allegations, setAllegations] = useState<AllegationDto[]>([]);
  const [cards, setCards] = useState<ScenarioCard[]>([]);
  /**
   * The served "nothing has scanned this yet" sentence, or `null` once a scan has
   * completed (task 2.15, piece 3).
   *
   * Held beside the cards it arrives with and replaced on every read of them, so
   * it can never describe a pool the page is no longer showing — the same rule
   * the queue's own notice follows in `CardQueue`.
   */
  const [neverScannedNotice, setNeverScannedNotice] = useState<string | null>(null);
  /**
   * Which completed run is proposing candidates, or `null` when nothing is
   * (2026-08-08).
   *
   * Read WITH the cards and replaced with them, exactly as `neverScannedNotice`
   * is: the queue's heading and the scan card's summary both name this run, and a
   * value that outlived its payload would have them naming a run whose proposals
   * are no longer on screen.
   */
  const [proposalSource, setProposalSource] = useState<ProposalSource | null>(null);
  /**
   * The accusation section's payload (task 2.11 B1).
   *
   * Its own state and its own read, NOT part of the four-read gate: a scenario
   * whose accusation could not be loaded must still show its facts, its queue and
   * its identity. Folding it into `Promise.all` would replace the whole page with
   * a banner over one section — the same over-reaction the cards re-read notice
   * exists to avoid.
   */
  const [accusation, setAccusation] = useState<AccusationPanelDto | null>(null);
  const [accusationError, setAccusationError] = useState<string | null>(null);
  /**
   * The accusations the link panels offer, and the wording two sections share
   * (tasks 2.10 and 2.12).
   *
   * Read HERE because two sections need it — the queue's link panels and the
   * facts list's Remove control — and this page is where the reads they share
   * live. Fetching it in both would put two copies of one catalogue on one
   * screen, free to disagree after a Settings edit.
   *
   * Once per scenario, not per ruling: the catalogue is 120 rows on DEV and only
   * changes when a document is processed.
   */
  const { linkOptions, linkOptionsError } = useAllegationOptions(slug, scenarioId);
  const linkWording = linkOptions?.wording ?? null;
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // ─── TWO refresh keys, and they must stay two (task 1.7F, ruling R6) ───────
  //
  // Bumped after any write; a re-fetch (never a hand-merged response) is the
  // source of truth, because the backend trims and normalises what it stores.
  //
  // `pageRefreshKey` re-reads the WHOLE payload — all four endpoints — and also
  // feeds `externalRefresh` into ScanSection, which makes the card queue reload
  // its pool and dispatch `cards_loaded`. That is correct after an edit to the
  // scenario's own content, and wrong after a ruling: it would disturb the
  // queue's selection mid-triage, which is precisely the class of defect task
  // 1.7G spent two builds fixing.
  //
  // `cardsRefreshKey` re-reads ONLY the cards, which is what the facts section
  // below is derived from. A ruling bumps this one, so the new fact appears
  // without touching the queue the human is working in.
  //
  // Collapsing them back into one key would be a one-line "simplification" that
  // reintroduces the disturbance. They are named apart so that edit is a visible
  // one.
  const [pageRefreshKey, setPageRefreshKey] = useState(0);
  const refresh = useCallback(() => setPageRefreshKey((k) => k + 1), []);
  const [cardsRefreshKey, setCardsRefreshKey] = useState(0);
  const refreshCards = useCallback(() => setCardsRefreshKey((k) => k + 1), []);

  // `queueRefreshKey` is the THIRD signal, and it exists because the two above
  // do not cover one case: something outside the queue changed a candidate's
  // RULING (task 2.12 item G — removing a fact from its row).
  //
  // The facts list is fed by the page's `cards` state, which `cardsRefreshKey`
  // re-reads. The queue's counts are fed by `CardQueue`'s OWN pool, which it
  // fetches itself and reloads only when `externalRefresh` changes — a
  // deliberate 1.7F design, so a ruling does not reload the list the human is
  // working in. Those are two independently-keyed fetches of the same endpoint,
  // NOT one payload, and a removal that bumped only the first left the facts
  // list correct while the queue still showed the card included. Two surfaces
  // disagreeing about one card, with nothing on screen saying so.
  //
  // Reloading the queue's pool is safe here and does not cost the human their
  // place: `cards_loaded` "keeps the human where they were, clamped to the new
  // length", and a removal changes a card's STATUS without changing the pool's
  // membership or order — so the preserved index still points at the same card.
  //
  // What must NOT happen is the page-level refresh, which re-runs all four reads
  // and collapses the queue's own region. That was the beta.374 defect.
  const [queueRefreshKey, setQueueRefreshKey] = useState(0);
  const refreshAfterRemoval = useCallback(() => {
    setCardsRefreshKey((k) => k + 1);
    setQueueRefreshKey((k) => k + 1);
  }, []);

  // A failed cards RE-READ is not a failed page load, and must not be rendered as
  // one. `error` above replaces the entire page with a banner — right when the
  // scenario could not be loaded at all, and badly wrong here: the ruling SAVED,
  // and taking the queue, the identity block and the scan section off the screen
  // would cost the human their whole working context over one stale list. So the
  // re-read gets its own non-fatal notice, shown beside the facts it is about.
  const [factsNotice, setFactsNotice] = useState<string | null>(null);

  const [showDelete, setShowDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const [editingIdentity, setEditingIdentity] = useState(false);

  const handleDelete = () => {
    if (!scenarioId) return;
    setDeleting(true);
    setDeleteError(null);
    deleteScenario(slug, scenarioId)
      .then(() => navigate(trialPrepPath(slug)))
      .catch((err: unknown) => {
        // Standing Rule 1: a failed DELETE stays visible IN the modal. We do NOT
        // navigate away or close, which would imply a success that did not happen.
        setDeleteError(
          err instanceof Error ? err.message : "Failed to delete the scenario. Try again.",
        );
        setDeleting(false);
      });
  };

  useEffect(() => {
    if (!scenarioId) {
      setLoading(false);
      return;
    }
    let cancelled = false;
    setLoading(true);
    setError(null);

    // Four reads, one gate. `Promise.all` rather than a waterfall: they are
    // independent, and a page that paints its header a beat before its identity
    // block reads as a page that is still broken.
    Promise.all([
      getScenarioDetailLive(slug, scenarioId),
      fetchAugmentationPanel(slug, scenarioId),
      getAllegations(),
      fetchScenarioCards(slug, scenarioId),
    ])
      .then(([detail, panel, allegationList, cardPayload]) => {
        if (cancelled) return;
        setScenario(detail);
        setAugmentation(panel);
        setAllegations(allegationList.allegations);
        setCards([...cardPayload.pool, ...cardPayload.set_aside]);
        setNeverScannedNotice(cardPayload.never_scanned_notice ?? null);
        setProposalSource(cardPayload.proposal_source ?? null);
        setLoading(false);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setError(
          err instanceof Error
            ? err.message
            : "Failed to load the scenario. Try reloading the page.",
        );
        setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [slug, scenarioId, pageRefreshKey]);

  // The cards-only re-read (task 1.7F Part A).
  //
  // A key-bumped effect rather than a bare fetch at the call site, so the
  // cancelled-flag protection above is INHERITED rather than reimplemented: two
  // rulings in quick succession start two reads, and without it the slower
  // response could land last and paint the older pool over the newer one.
  //
  // `cardsRefreshKey === 0` is the initial mount, where the four-read effect
  // above has already loaded the cards. Skipping it there is what stops the page
  // reading the same endpoint twice on arrival.
  useEffect(() => {
    if (!scenarioId || cardsRefreshKey === 0) return;
    let cancelled = false;
    fetchScenarioCards(slug, scenarioId)
      .then((payload) => {
        if (cancelled) return;
        setCards([...payload.pool, ...payload.set_aside]);
        // Re-read with the cards, so a scan that completed while this page was
        // open stops the section claiming nothing has scanned it.
        setNeverScannedNotice(payload.never_scanned_notice ?? null);
        setProposalSource(payload.proposal_source ?? null);
        // A read that worked clears the last one's complaint; leaving it up would
        // put a stale warning over a list that is now current.
        setFactsNotice(null);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        // A failed RE-READ is not a failed save, and saying so would send a human
        // hunting for a ruling that is safely stored (the 1.4 read/write error
        // split). The notice names which half failed, and Try again re-reads
        // rather than making them reload the page and lose their place.
        setFactsNotice(
          err instanceof Error
            ? `Your ruling was saved. The facts list below could not be re-read, so it may be out of date: ${err.message}`
            : "Your ruling was saved. The facts list below could not be re-read, so it may be out of date.",
        );
      });
    return () => {
      cancelled = true;
    };
  }, [slug, scenarioId, cardsRefreshKey]);

  // The accusation read, keyed on the PAGE refresh so a marking or a pairing
  // re-reads it — and so does anything else that changes what is included, which
  // is what the count and the gaps are derived from.
  useEffect(() => {
    if (!scenarioId) return;
    let cancelled = false;
    fetchAccusationPanel(slug, scenarioId)
      .then((payload) => {
        if (cancelled) return;
        setAccusation(payload);
        setAccusationError(null);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        // Standing Rule 1: the section withdraws itself when it has no words to
        // render with, so the absence has to SAY why — otherwise a whole block is
        // simply missing from the page with nothing to diagnose.
        setAccusation(null);
        setAccusationError(
          err instanceof Error
            ? err.message
            : "The accusation section could not be loaded.",
        );
      });
    return () => {
      cancelled = true;
    };
  }, [slug, scenarioId, pageRefreshKey, cardsRefreshKey]);

  const gatingCrumb = (
    <Breadcrumb
      items={[{ label: "Dashboard", to: "/" }, backCrumb, { label: "Scenario" }]}
    />
  );

  if (loading) {
    return (
      <div style={containerStyle} data-surface="v3">
        {gatingCrumb}
        <div style={messageStyle}>Loading scenario…</div>
      </div>
    );
  }
  if (error) {
    return (
      <div style={containerStyle} data-surface="v3">
        {gatingCrumb}
        <div style={errorStyle}>{error}</div>
      </div>
    );
  }
  if (!scenario || !scenarioId) {
    return (
      <div style={containerStyle} data-surface="v3">
        {gatingCrumb}
        <EmptyState message="Scenario not found." />
      </div>
    );
  }

  return (
    <div style={containerStyle} data-surface="v3">
      <Breadcrumb
        items={[{ label: "Dashboard", to: "/" }, backCrumb, { label: scenario.attack }]}
      />

      {/* 1 — §2.1 */}
      <ScenarioHeaderTiers
        slug={slug}
        scenarioId={scenarioId}
        code={scenario.code}
        name={scenario.attack}
        direction={scenario.direction}
        status={scenario.status}
        onEdit={() => setEditingIdentity(true)}
        onDelete={() => {
          setDeleteError(null);
          setShowDelete(true);
        }}
        onReadyChanged={refresh}
      />

      {/* 2 — §2.2. The three texts come from the augmentation identity, which is
          where the backend already serves them composed; `attack_text` also lives
          inside the definition jsonb and the two agree. */}
      <ScenarioIdentityBlock
        attackText={augmentation?.identity.attack_text ?? null}
        themeStatement={augmentation?.identity.theme_statement ?? null}
        motivation={augmentation?.identity.motivation ?? null}
        anchorAllegationIds={scenario.anchor_allegation_ids}
        allegations={allegations}
        onEdit={() => setEditingIdentity(true)}
      />

      {/* 3 — §2.3. A merge inside the scan writes the candidate facts every other
          section below reads, so a merge refreshes the page's whole payload. */}
      <ScanSection
        slug={slug}
        scenarioId={scenarioId}
        linkOptions={linkOptions}
        // `null` while the page is still loading, NOT the empty array it is
        // initialised to. The queue's summary is derived from this, and an empty
        // pool and an unread one must never look the same — collapsing them is
        // what put "No candidates gathered yet" over 148 candidates (task 2.13c).
        cards={loading ? null : cards}
        scenarioTitle={scenario.attack}
        // Either signal reloads the queue's pool: a merge (page-level) or a
        // removal (queue-level). Summed rather than passed as two props because
        // the VALUE is never read — only its change — and both only ever
        // increment, so a bump to either is always a change to the sum.
        externalRefresh={pageRefreshKey + queueRefreshKey}
        // Served, not derived — see the field's note on the payload.
        neverScannedNotice={neverScannedNotice}
        proposalSource={proposalSource}
        onFactsChanged={refresh}
        // A ruling the SERVER confirmed. Re-reads the cards alone, so the fact
        // appears below without the queue above being reloaded under the human.
        onRulingSaved={refreshCards}
      />

      {/* A catalogue that would not load withdraws the link panels and the Remove
          control, so it has to SAY so — otherwise both are simply missing, on
          ninety-four cards, with nothing to diagnose (task 2.12). No Dismiss: it
          is the reason two controls are absent, and hiding it would leave the
          absence unexplained. */}
      {linkOptionsError && <ScenarioNotice message={linkOptionsError} />}

      {/* The re-read notice (task 1.7F Part A). Sits with the list it describes,
          and leaves every other section on screen and working. */}
      {factsNotice && (
        <ScenarioNotice message={factsNotice}>
          <button type="button" onClick={refreshCards} style={ghostButtonStyle}>
            Try again
          </button>
          <button type="button" onClick={() => setFactsNotice(null)} style={ghostButtonStyle}>
            Dismiss
          </button>
        </ScenarioNotice>
      )}

      {/* 4 — §2.4 */}
      <ScenarioFactsSection
        slug={slug}
        scenarioId={scenarioId}
        cards={cards}
        humanFacts={augmentation?.human_facts ?? []}
        wording={linkWording}
        // The whole payload, not just the panel's words: the fact card renders
        // the SHARED body now, which needs the card-grammar block and the two
        // fold thresholds that ride with it (ONE_CARD_GRAMMAR).
        options={linkOptions}
        onChanged={refresh}
        onFactRemoved={refreshAfterRemoval}
      />

      {/* 4b — task 2.11 B1: the accusation, its instances and their answers.
          Placed directly beneath the facts it is built ON, because marking and
          pairing are judgments ABOUT those facts and a human reads the two
          together. A payload that would not load withdraws the whole section, so
          the notice beside it is the only thing saying why. */}
      {accusationError && <ScenarioNotice message={accusationError} />}
      <AccusationSection
        slug={slug}
        scenarioId={scenarioId}
        panel={accusation}
        includedFacts={includedPickableFacts(cards)}
        onChanged={refresh}
      />

      {/* 5 and 6 — §2.5 and §2.6.
          Both withdraw entirely while the panel is unloaded, rather than
          rendering with empty lists as they did before task 2.11 C. Their words
          are stored rows now (ruling C4b), so an unloaded payload leaves no
          headings, no button labels and no empty-state sentence — and a section
          of unlabelled controls is worse than an absent one. Same rule, and same
          reason, as `AccusationSection`'s `if (!panel) return null`. The page's
          own failure notice is what says why. */}
      {augmentation && (
        <>
          <TalkingPointsSection
            slug={slug}
            scenarioId={scenarioId}
            points={augmentation.talking_points}
            cap={augmentation.talking_points_cap}
            wording={augmentation.wording}
            onChanged={refresh}
          />

          {/* The backend splits watch-list notes from human facts; a client that
              re-filtered them would eventually show a note as a fact. */}
          <WatchListSection
            slug={slug}
            scenarioId={scenarioId}
            notes={augmentation.watch_list}
            wording={augmentation.wording}
            onChanged={refresh}
          />
        </>
      )}

      {/* 7 — §2.8 */}
      <ScenarioOrphanStrip
        slug={slug}
        scenarioId={scenarioId}
        externalRefresh={pageRefreshKey}
      />

      {/* Mounted only while open so it re-reads on every open — a dialog holding a
          draft from ten minutes ago would let a human overwrite an edit made since,
          and this is the coldest path here. */}
      {editingIdentity && (
        <ScenarioIdentityModal
          slug={slug}
          scenarioId={scenarioId}
          definition={scenario.definition}
          anchorAllegationIds={scenario.anchor_allegation_ids}
          onSaved={refresh}
          onClose={() => setEditingIdentity(false)}
        />
      )}

      {showDelete && (
        <ScenarioDeleteConfirm
          // One vocabulary, two surfaces: the dashboard's card kebab asks this
          // same question through the same builder (2026-08-07).
          {...scenarioDeleteCopy(scenario.attack)}
          busy={deleting}
          error={deleteError}
          onConfirm={handleDelete}
          onCancel={() => {
            setShowDelete(false);
            setDeleteError(null);
          }}
        />
      )}
    </div>
  );
};

export default ScenarioDetailPage;
