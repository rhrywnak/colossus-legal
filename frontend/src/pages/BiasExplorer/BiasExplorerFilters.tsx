// Bias Explorer — filter bar.
//
// Two dropdowns (Actor, Pattern), each with an "All" default. Below them,
// a result-count line. Selecting a value emits an updated filter object;
// the parent re-runs the query immediately (no Apply button).
//
// Adding a new dropdown is purely additive: import the new option list
// from `available`, add a <select>, push the chosen value into the filter
// object passed to onChange. The view doesn't need to change.

import React from "react";

import type { AvailableFilters, BiasQueryFilters } from "./types";

// ─── Helpers ────────────────────────────────────────────────────────────────

/** Convert a snake_case pattern tag into a Title Case display label. */
function formatTagLabel(raw: string): string {
    return raw
        .split("_")
        .map((part) => (part.length === 0 ? part : part[0].toUpperCase() + part.slice(1)))
        .join(" ");
}

// ─── Styles ─────────────────────────────────────────────────────────────────

const containerStyle: React.CSSProperties = {
    backgroundColor: "#ffffff",
    border: "1px solid #e2e8f0",
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
    minWidth: "240px",
};

const labelStyle: React.CSSProperties = {
    fontSize: "0.72rem",
    fontWeight: 700,
    color: "#94a3b8",
    textTransform: "uppercase",
    letterSpacing: "0.05em",
    marginBottom: "0.3rem",
};

const selectStyle: React.CSSProperties = {
    padding: "0.45rem 0.6rem",
    fontSize: "0.88rem",
    border: "1px solid #cbd5e1",
    borderRadius: "6px",
    backgroundColor: "#ffffff",
    color: "#0f172a",
    fontFamily: "inherit",
};

const countStyle: React.CSSProperties = {
    marginTop: "0.85rem",
    fontSize: "0.85rem",
    color: "#334155",
    fontWeight: 500,
};

// ─── Component ──────────────────────────────────────────────────────────────

interface Props {
    available: AvailableFilters;
    filters: BiasQueryFilters;
    onChange: (next: BiasQueryFilters) => void;
    resultCount: number | null;
    loading: boolean;
}

const BiasExplorerFilters: React.FC<Props> = ({
    available,
    filters,
    onChange,
    resultCount,
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

    return (
        <div style={containerStyle}>
            <div style={rowStyle}>
                <div style={fieldStyle}>
                    <label htmlFor="bias-filter-actor" style={labelStyle}>
                        Actor
                    </label>
                    <select
                        id="bias-filter-actor"
                        style={selectStyle}
                        value={filters.actor_id ?? ""}
                        onChange={onActorChange}
                    >
                        <option value="">All actors</option>
                        {available.actors.map((a) => (
                            <option key={a.id} value={a.id}>
                                {a.name} ({a.tagged_statement_count})
                            </option>
                        ))}
                    </select>
                </div>

                <div style={fieldStyle}>
                    <label htmlFor="bias-filter-pattern" style={labelStyle}>
                        Pattern
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
            </div>

            <div style={countStyle}>
                {loading
                    ? "Updating..."
                    : resultCount === null
                        ? "Showing — instances"
                        : `Showing ${resultCount} ${resultCount === 1 ? "instance" : "instances"}`}
            </div>
        </div>
    );
};

export default BiasExplorerFilters;

// Exposed for unit testing — we do not have RTL set up, but pure helper
// tests via vitest are the project's frontend test pattern.
export { formatTagLabel };
