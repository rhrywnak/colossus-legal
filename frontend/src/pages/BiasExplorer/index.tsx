// Bias Explorer — page shell.
//
// Owns:
//   - the filter state (actor_id, pattern_tag)
//   - one fetch for available dropdown values (on mount)
//   - one fetch per filter change for the result list
//   - error / loading rendering
//   - the slot where view components plug in
//
// Today there is one view (BiasByActorView). When BiasByPatternView and
// BiasByEvidenceChainView arrive, they replace the single <BiasByActorView/>
// render with a tab switcher that selects between sibling components — all
// reading from the same `result` and `filters` props.

import React, { useCallback, useEffect, useState } from "react";

import BiasByActorView from "./BiasByActorView";
import BiasExplorerFilters from "./BiasExplorerFilters";
import { getAvailableFilters, runBiasQuery } from "./api";
import type {
    AvailableFilters,
    BiasQueryFilters,
    BiasQueryResult,
} from "./types";

// ─── Styles ─────────────────────────────────────────────────────────────────

const pageStyle: React.CSSProperties = {
    paddingTop: "2rem",
    paddingBottom: "4rem",
};

const headerStyle: React.CSSProperties = {
    fontSize: "1.55rem",
    fontWeight: 700,
    color: "#0f172a",
    letterSpacing: "-0.02em",
    margin: 0,
    marginBottom: "0.4rem",
};

const subtitleStyle: React.CSSProperties = {
    fontSize: "0.84rem",
    color: "#64748b",
    marginBottom: "1.5rem",
};

const messageBoxStyle: React.CSSProperties = {
    padding: "1rem 1.25rem",
    backgroundColor: "#fef2f2",
    border: "1px solid #fecaca",
    borderRadius: "6px",
    color: "#dc2626",
    marginBottom: "1rem",
    display: "flex",
    alignItems: "center",
    gap: "0.75rem",
};

const retryBtnStyle: React.CSSProperties = {
    padding: "0.3rem 0.7rem",
    fontSize: "0.78rem",
    fontWeight: 500,
    border: "1px solid #fca5a5",
    borderRadius: "5px",
    backgroundColor: "#ffffff",
    color: "#dc2626",
    cursor: "pointer",
    fontFamily: "inherit",
};

const emptyBoxStyle: React.CSSProperties = {
    padding: "2.5rem 1.5rem",
    textAlign: "center",
    color: "#64748b",
    backgroundColor: "#ffffff",
    border: "1px dashed #e2e8f0",
    borderRadius: "8px",
};

const clearBtnStyle: React.CSSProperties = {
    marginTop: "0.75rem",
    padding: "0.4rem 0.9rem",
    fontSize: "0.82rem",
    fontWeight: 500,
    border: "1px solid #cbd5e1",
    borderRadius: "6px",
    backgroundColor: "#ffffff",
    color: "#1d4ed8",
    cursor: "pointer",
    fontFamily: "inherit",
};

// ─── Component ──────────────────────────────────────────────────────────────

const BiasExplorer: React.FC = () => {
    const [available, setAvailable] = useState<AvailableFilters | null>(null);
    const [availableError, setAvailableError] = useState<string | null>(null);

    const [filters, setFilters] = useState<BiasQueryFilters>({});
    const [result, setResult] = useState<BiasQueryResult | null>(null);
    const [queryError, setQueryError] = useState<string | null>(null);
    const [queryLoading, setQueryLoading] = useState(false);

    // Load dropdown contents once on mount.
    const loadAvailable = useCallback(() => {
        setAvailableError(null);
        getAvailableFilters()
            .then((data) => setAvailable(data))
            .catch((err: unknown) => {
                const msg = err instanceof Error ? err.message : String(err);
                setAvailableError(msg);
            });
    }, []);

    useEffect(() => {
        loadAvailable();
    }, [loadAvailable]);

    // Re-run the query whenever filters change. The empty-object request
    // (no filters) is a valid first-load state and returns all instances.
    const runQuery = useCallback((current: BiasQueryFilters) => {
        setQueryError(null);
        setQueryLoading(true);
        runBiasQuery(current)
            .then((data) => setResult(data))
            .catch((err: unknown) => {
                const msg = err instanceof Error ? err.message : String(err);
                setQueryError(msg);
            })
            .finally(() => setQueryLoading(false));
    }, []);

    useEffect(() => {
        runQuery(filters);
    }, [filters, runQuery]);

    const onFiltersChange = (next: BiasQueryFilters) => {
        setFilters(next);
    };

    const clearFilters = () => {
        setFilters({});
    };

    const hasAnyFilter = filters.actor_id != null || filters.pattern_tag != null;

    // ── Render ──

    // Initial-load failure for the dropdown contents. Without this we cannot
    // even render the filter bar.
    if (availableError) {
        return (
            <div style={pageStyle}>
                <h1 style={headerStyle}>Bias Analysis</h1>
                <div style={subtitleStyle}>
                    Track patterns of bias, disparagement, and misconduct across all case documents.
                </div>
                <div style={messageBoxStyle}>
                    <span>Failed to load filter options: {availableError}</span>
                    <button type="button" style={retryBtnStyle} onClick={loadAvailable}>
                        Retry
                    </button>
                </div>
            </div>
        );
    }

    // Dropdowns still loading.
    if (!available) {
        return (
            <div style={pageStyle}>
                <h1 style={headerStyle}>Bias Analysis</h1>
                <div style={subtitleStyle}>
                    Track patterns of bias, disparagement, and misconduct across all case documents.
                </div>
                <div style={emptyBoxStyle}>Loading filters...</div>
            </div>
        );
    }

    // Empty graph (no tagged Evidence at all). Distinct observable per
    // Standing Rule 1 — separate from "no matches for these filters".
    const noTaggedDataAtAll =
        available.actors.length === 0 && available.pattern_tags.length === 0;

    return (
        <div style={pageStyle}>
            <h1 style={headerStyle}>Bias Analysis</h1>
            <div style={subtitleStyle}>
                Track patterns of bias, disparagement, and misconduct across all case documents.
            </div>

            <BiasExplorerFilters
                available={available}
                filters={filters}
                onChange={onFiltersChange}
                resultCount={result?.total_count ?? null}
                loading={queryLoading}
            />

            {queryError && (
                <div style={messageBoxStyle}>
                    <span>Failed to run bias query: {queryError}</span>
                    <button type="button" style={retryBtnStyle} onClick={() => runQuery(filters)}>
                        Retry
                    </button>
                </div>
            )}

            {noTaggedDataAtAll ? (
                <div style={emptyBoxStyle}>
                    No bias evidence has been tagged yet. Pattern tags are populated during
                    document processing.
                </div>
            ) : queryLoading && !result ? (
                <div style={emptyBoxStyle}>Loading instances...</div>
            ) : result && result.instances.length === 0 ? (
                <div style={emptyBoxStyle}>
                    <div>No instances match the current filters.</div>
                    {hasAnyFilter && (
                        <button type="button" style={clearBtnStyle} onClick={clearFilters}>
                            Clear filters
                        </button>
                    )}
                </div>
            ) : result ? (
                <BiasByActorView result={result} />
            ) : null}
        </div>
    );
};

export default BiasExplorer;
