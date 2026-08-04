// Bias Explorer — filter bar.
//
// Three dropdowns (Speaker, Misconduct Pattern, About) plus a Clear all
// button on the right. Below the row, a counter line that distinguishes
// "Filtered: X of Y instances" (some filter active) from
// "Showing all Y instances" (no filter active).
//
// User-facing labels diverge from the internal field names: SPEAKER drives
// `actor_id`, MISCONDUCT PATTERN drives `pattern_tag`, ABOUT drives
// `subject_id`. The internal names predate the v2 UX work and are not
// renamed here — only the JSX strings change.
//
// Adding a new dropdown stays additive: import the new option list from
// `available`, add a <select>, push the chosen value into the filter
// object passed to onChange. The view doesn't need to change.

import React from "react";

import { filteredCounterLine } from "../../components/shared/filteredCounter";
import type { AvailableFilters, BiasQueryFilters } from "./types";

// ─── Helpers ────────────────────────────────────────────────────────────────

/** Convert a snake_case pattern tag into a Title Case display label. */
function formatTagLabel(raw: string): string {
    return raw
        .split("_")
        .map((part) => (part.length === 0 ? part : part[0].toUpperCase() + part.slice(1)))
        .join(" ");
}

/**
 * Format the result-counter line under the filter row.
 *
 * The wording is driven by `hasAnyFilter` (the user's intent) rather than
 * by numerical equality of `filtered === total`. A graph small enough
 * that every filter combination matches everything would otherwise read
 * "Showing all" while a filter is actively applied — surprising to the
 * user.
 *
 * ## The rule moved OUT of this file (task 1.7E, ruling R1)
 *
 * The candidate list needs the same honesty rule with its own noun, so the rule
 * now lives in `components/shared/filteredCounter` and this function is the Bias
 * Explorer's noun applied to it. Kept as a named function because it is this
 * page's vocabulary — "instances" is a Bias Explorer word, and the shared helper
 * deliberately does not know it.
 */
function formatFilteredCounter(
    filtered: number,
    total: number,
    hasAnyFilter: boolean,
): string {
    return filteredCounterLine(filtered, total, hasAnyFilter, {
        singular: "instance",
        plural: "instances",
    });
}

// ─── Styles ─────────────────────────────────────────────────────────────────

const containerStyle: React.CSSProperties = {
    backgroundColor: "var(--bg-surface)",
    border: "1px solid var(--border-default)",
    borderRadius: "8px",
    padding: "1rem 1.25rem",
    marginBottom: "1rem",
};

const rowStyle: React.CSSProperties = {
    display: "flex",
    flexWrap: "wrap",
    gap: "1rem",
    alignItems: "flex-end",
};

const fieldStyle: React.CSSProperties = {
    display: "flex",
    flexDirection: "column",
    minWidth: "220px",
    flex: "1 1 220px",
};

const labelStyle: React.CSSProperties = {
    fontSize: "0.72rem",
    fontWeight: 700,
    color: "var(--text-disabled)",
    textTransform: "uppercase",
    letterSpacing: "0.05em",
    marginBottom: "0.3rem",
};

const selectStyle: React.CSSProperties = {
    padding: "0.45rem 0.6rem",
    fontSize: "0.88rem",
    border: "1px solid var(--border-default)",
    borderRadius: "6px",
    backgroundColor: "var(--bg-surface)",
    color: "var(--text-primary)",
    fontFamily: "inherit",
};

const clearAllBtnStyle: React.CSSProperties = {
    padding: "0.5rem 0.9rem",
    fontSize: "0.82rem",
    fontWeight: 500,
    border: "1px solid var(--border-default)",
    borderRadius: "6px",
    backgroundColor: "var(--bg-surface)",
    color: "var(--accent-primary)",
    cursor: "pointer",
    fontFamily: "inherit",
    flexShrink: 0,
};

const countStyle: React.CSSProperties = {
    marginTop: "0.85rem",
    fontSize: "0.85rem",
    color: "var(--text-secondary)",
    fontWeight: 500,
};

// ─── Component ──────────────────────────────────────────────────────────────

interface Props {
    available: AvailableFilters;
    filters: BiasQueryFilters;
    onChange: (next: BiasQueryFilters) => void;
    onClearAll: () => void;
    resultCount: number | null;
    totalUnfiltered: number | null;
    hasAnyFilter: boolean;
    loading: boolean;
}

const BiasExplorerFilters: React.FC<Props> = ({
    available,
    filters,
    onChange,
    onClearAll,
    resultCount,
    totalUnfiltered,
    hasAnyFilter,
    loading,
}) => {
    const onActorChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
        const value = e.target.value;
        onChange({
            ...filters,
            actor_id: value === "" ? undefined : value,
        });
    };

    const onPatternChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
        const value = e.target.value;
        onChange({
            ...filters,
            pattern_tag: value === "" ? undefined : value,
        });
    };

    const onSubjectChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
        const value = e.target.value;
        onChange({
            ...filters,
            subject_id: value === "" ? undefined : value,
        });
    };

    return (
        <div style={containerStyle}>
            <div style={rowStyle}>
                <div style={fieldStyle}>
                    <label htmlFor="bias-filter-actor" style={labelStyle}>
                        Speaker
                    </label>
                    <select
                        id="bias-filter-actor"
                        style={selectStyle}
                        value={filters.actor_id ?? ""}
                        onChange={onActorChange}
                    >
                        <option value="">All speakers</option>
                        {available.actors.map((a) => (
                            <option key={a.id} value={a.id}>
                                {a.name} ({a.tagged_statement_count})
                            </option>
                        ))}
                    </select>
                </div>

                <div style={fieldStyle}>
                    <label htmlFor="bias-filter-pattern" style={labelStyle}>
                        Misconduct Pattern
                    </label>
                    <select
                        id="bias-filter-pattern"
                        style={selectStyle}
                        value={filters.pattern_tag ?? ""}
                        onChange={onPatternChange}
                    >
                        <option value="">All patterns</option>
                        {available.pattern_tags.map((tag) => (
                            <option key={tag} value={tag}>
                                {formatTagLabel(tag)}
                            </option>
                        ))}
                    </select>
                </div>

                <div style={fieldStyle}>
                    <label htmlFor="bias-filter-subject" style={labelStyle}>
                        About
                    </label>
                    <select
                        id="bias-filter-subject"
                        style={selectStyle}
                        value={filters.subject_id ?? ""}
                        onChange={onSubjectChange}
                    >
                        <option value="">All subjects</option>
                        {available.subjects.map((s) => (
                            <option key={s.id} value={s.id}>
                                {s.name} ({s.tagged_statement_count})
                            </option>
                        ))}
                    </select>
                </div>

                {/* Clear-all button — always visible per v2 spec §5.3. */}
                <button
                    type="button"
                    style={clearAllBtnStyle}
                    onClick={onClearAll}
                >
                    Clear all
                </button>
            </div>

            <div style={countStyle}>
                {loading
                    ? "Updating..."
                    : resultCount === null || totalUnfiltered === null
                        ? "Showing — instances"
                        : formatFilteredCounter(resultCount, totalUnfiltered, hasAnyFilter)}
            </div>
        </div>
    );
};

export default BiasExplorerFilters;

// Exposed for unit testing — we do not have RTL set up, but pure helper
// tests via vitest are the project's frontend test pattern.
export { formatTagLabel, formatFilteredCounter };
