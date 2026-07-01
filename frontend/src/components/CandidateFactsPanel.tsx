// =============================================================================
// CandidateFactsPanel — find bias-tagged Evidence and add it to a scenario.
// =============================================================================
//
// Phase A curation, candidate side. A collapsible panel on the scenario detail
// page that runs the EXISTING bias query (no new retrieval) so a human can
// browse pre-tagged Evidence and click "Add to scenario" on the ones that
// belong. Reuses the bias EvidenceCard for each row (with an Add action in its
// header slot), so a candidate and a saved fact render identically.
//
// The subject filter is seeded from the server-resolved default subject
// (CASE_DEFAULT_SUBJECT_NAME); no case-specific name is hardcoded here
// (Standing Rule 2). If that default is unset on a deployment, the panel opens
// with no subject pre-selected and the user picks one.

import React, { useEffect, useMemo, useState } from "react";

import {
  getAvailableFilters,
  runBiasQuery,
  type ActorOption,
  type BiasInstance,
  type BiasQueryFilters,
} from "../services/bias";
import { addScenarioFact } from "../services/scenarioFacts";
import EvidenceCard from "../pages/BiasExplorer/EvidenceCard";
import type { ScenarioDefinition } from "../pages/trialPrepData";
import { seedFiltersFromDefinition } from "./candidateSeed";

interface Props {
  slug: string;
  scenarioId: string;
  /** Node ids already saved on this scenario — their Add button reads "Added". */
  savedIds: Set<string>;
  /** Called after a successful add so the parent can refresh the saved list. */
  onAdded: () => void;
  /** This scenario's authored definition (B2a). When it carries an `attack_text`,
   *  the panel auto-seeds its filters to that theme; absent / `{}` → fallback. */
  definition?: ScenarioDefinition;
}

const toggleStyle: React.CSSProperties = {
  background: "var(--bg-surface)",
  border: "1px solid var(--border-default)",
  borderRadius: "6px",
  padding: "0.5rem 0.9rem",
  fontSize: "0.85rem",
  fontWeight: 600,
  color: "var(--text-primary)",
  cursor: "pointer",
};

const panelStyle: React.CSSProperties = {
  marginTop: "0.75rem",
  border: "1px solid var(--border-default)",
  borderRadius: "8px",
  padding: "0.9rem",
  backgroundColor: "var(--bg-page)",
};

const controlRow: React.CSSProperties = {
  display: "flex",
  flexWrap: "wrap",
  alignItems: "center",
  gap: "0.6rem",
  marginBottom: "0.75rem",
};

const selectStyle: React.CSSProperties = {
  padding: "0.35rem 0.5rem",
  fontSize: "0.82rem",
  border: "1px solid var(--border-default)",
  borderRadius: "5px",
  backgroundColor: "var(--bg-surface)",
  color: "var(--text-primary)",
};

const messageStyle: React.CSSProperties = {
  fontSize: "0.84rem",
  color: "var(--text-muted)",
  padding: "0.5rem 0",
};

const errorStyle: React.CSSProperties = {
  margin: "0.5rem 0",
  padding: "0.6rem 0.8rem",
  backgroundColor: "var(--state-danger-bg-soft)",
  border: "1px solid var(--state-danger-border)",
  borderRadius: "6px",
  color: "var(--state-danger-strong)",
  fontSize: "0.82rem",
};

const addBtnStyle: React.CSSProperties = {
  padding: "0.2rem 0.6rem",
  fontSize: "0.74rem",
  fontWeight: 600,
  border: "1px solid var(--accent-primary)",
  borderRadius: "5px",
  backgroundColor: "var(--accent-bg-soft)",
  color: "var(--accent-primary)",
  cursor: "pointer",
};

const addedLabelStyle: React.CSSProperties = {
  fontSize: "0.74rem",
  fontWeight: 600,
  color: "var(--text-disabled)",
};

const ALL_PATTERNS = "";

const CandidateFactsPanel: React.FC<Props> = ({
  slug,
  scenarioId,
  savedIds,
  onAdded,
  definition,
}) => {
  const [open, setOpen] = useState(false);

  const [patternTags, setPatternTags] = useState<string[]>([]);
  const [defaultSubjectId, setDefaultSubjectId] = useState<string | undefined>();
  const [patternTag, setPatternTag] = useState<string>(ALL_PATTERNS);
  // Retained from the vocab load so a definition's target/wielder NAMES can be
  // resolved to ids (B2b). Today the fetch discards these; the seed needs them.
  const [subjects, setSubjects] = useState<ActorOption[]>([]);
  const [actors, setActors] = useState<ActorOption[]>([]);

  // Resolve the definition into primitive seed inputs. The loop guard lives in
  // effect (b): it depends on this seed's PRIMITIVE fields (strings/bool), NOT the
  // seed object — so even if this memo recomputed on every render, a seed with
  // identical primitives cannot re-fire the query. (`patternTags` is a memo dep
  // because a seed phrase matches against it.) In practice the memo is also
  // stable: `subjects`/`actors`/`patternTags` are set once by effect (a) and not
  // re-created (its `patternTags.length > 0` guard blocks a refetch on re-open);
  // if that guard ever changes, the memo may recompute but the primitive-dep
  // guard in effect (b) still prevents a query loop.
  const seed = useMemo(
    () => seedFiltersFromDefinition(definition, subjects, actors, patternTags),
    [definition, subjects, actors, patternTags],
  );

  const [instances, setInstances] = useState<BiasInstance[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Per-row add error (keyed by evidence_id) — an add failure is shown on the
  // row that failed, not as a page-level banner.
  const [addError, setAddError] = useState<{ id: string; message: string } | null>(
    null,
  );

  // Load the filter vocabulary once, the first time the panel is opened.
  useEffect(() => {
    if (!open || patternTags.length > 0) return;
    let cancelled = false;
    getAvailableFilters()
      .then((filters) => {
        if (cancelled) return;
        setPatternTags(filters.pattern_tags);
        setDefaultSubjectId(filters.default_subject_id);
        // Retain the actor/subject vocab for name→id seeding (B2b).
        setSubjects(filters.subjects);
        setActors(filters.actors);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        // The service-layer message already names the bias-filters endpoint;
        // append recovery guidance (the dropdowns are empty without the vocab, so
        // the user needs to know it's retryable — Standing Rule 1).
        setError(
          err instanceof Error
            ? `${err.message} — close and reopen the panel to retry.`
            : "Failed to load the candidate filters — close and reopen the panel to retry.",
        );
      });
    return () => {
      cancelled = true;
    };
  }, [open, patternTags.length]);

  // Run the bias query whenever the panel is open and the filter changes.
  //
  // For an AUTHORED scenario (`seed.defined`) the filters are seeded from the
  // definition: the target's subject and the first wielder's actor, plus the
  // seeded tag as a default the user can still override. For an un-authored
  // scenario the behavior is exactly as before — case-default subject, no actor,
  // user-driven tag. Depends on the seed's PRIMITIVE fields (not the object) so a
  // new object identity per render cannot loop the query.
  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    setLoading(true);
    setError(null);
    const userTag = patternTag === ALL_PATTERNS ? undefined : patternTag;
    const filters: BiasQueryFilters = seed.defined
      ? {
          // Seed wins; fall back to the case default only if the target did not
          // resolve. `actor_id` is a new dimension — undefined when the wielder
          // is absent/unresolved. The user's tag choice overrides the seed's.
          subject_id: seed.subjectId ?? defaultSubjectId,
          actor_id: seed.actorId,
          pattern_tag: userTag ?? seed.patternTag,
        }
      : {
          subject_id: defaultSubjectId,
          pattern_tag: userTag,
        };
    runBiasQuery(filters)
      .then((result) => {
        if (cancelled) return;
        setInstances(result.instances);
        setLoading(false);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setError(
          err instanceof Error
            ? err.message
            : "Failed to load candidate facts. Try again.",
        );
        setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [
    open,
    defaultSubjectId,
    patternTag,
    seed.defined,
    seed.subjectId,
    seed.actorId,
    seed.patternTag,
  ]);

  const handleAdd = (evidenceId: string) => {
    setAddError(null);
    addScenarioFact(slug, scenarioId, { graph_node_id: evidenceId })
      .then(() => onAdded())
      .catch((err: unknown) => {
        setAddError({
          id: evidenceId,
          message:
            err instanceof Error ? err.message : "Failed to add this fact.",
        });
      });
  };

  if (!open) {
    return (
      <button type="button" style={toggleStyle} onClick={() => setOpen(true)}>
        + Find candidate facts
      </button>
    );
  }

  return (
    <div>
      <button type="button" style={toggleStyle} onClick={() => setOpen(false)}>
        − Hide candidate facts
      </button>
      <div style={panelStyle}>
        <div style={controlRow}>
          <label style={{ fontSize: "0.82rem", color: "var(--text-secondary)" }}>
            Pattern:{" "}
            <select
              style={selectStyle}
              value={patternTag}
              onChange={(e) => setPatternTag(e.target.value)}
            >
              <option value={ALL_PATTERNS}>All patterns</option>
              {patternTags.map((t) => (
                <option key={t} value={t}>
                  {t}
                </option>
              ))}
            </select>
          </label>
        </div>

        {/* B2b: a definition named a party the graph vocab couldn't match. Muted
            advisory (not an error) — the panel simply didn't filter on it, so the
            miss is visible rather than silently defaulting to the case subject. */}
        {seed.defined &&
          seed.unresolved.map((u) => (
            <div key={`${u.field}:${u.name}`} style={messageStyle}>
              Couldn't match “{u.name}” ({u.field}) to a known party — not
              filtering on it.
            </div>
          ))}

        {error && <div style={errorStyle}>{error}</div>}

        {loading ? (
          <div style={messageStyle}>Loading candidate facts…</div>
        ) : instances.length === 0 ? (
          <div style={messageStyle}>No candidate facts match this filter.</div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "0.6rem" }}>
            {instances.map((inst) => {
              const alreadySaved = savedIds.has(inst.evidence_id);
              return (
                <div key={inst.evidence_id}>
                  <EvidenceCard
                    instance={inst}
                    action={
                      alreadySaved ? (
                        <span style={addedLabelStyle}>Added</span>
                      ) : (
                        <button
                          type="button"
                          style={addBtnStyle}
                          onClick={() => handleAdd(inst.evidence_id)}
                        >
                          Add to scenario
                        </button>
                      )
                    }
                  />
                  {addError?.id === inst.evidence_id && (
                    <div style={errorStyle}>{addError.message}</div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
};

export default CandidateFactsPanel;
