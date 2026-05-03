// Bias Explorer — by-actor view.
//
// Groups the result list by `stated_by.id` and renders an EvidenceCard for
// each instance under that actor. When a future BiasByPatternView arrives,
// it lives next to this file and consumes the same `BiasQueryResult` —
// neither view needs to know about the other.

import React from "react";

import EvidenceCard from "./EvidenceCard";
import type { ActorOption, BiasInstance, BiasQueryResult } from "./types";

// ─── Helpers ────────────────────────────────────────────────────────────────

/** Group instances by actor id, preserving the server's row order. */
export function groupByActor(
    instances: BiasInstance[],
): Array<{ actor: ActorOption | null; items: BiasInstance[] }> {
    const order: string[] = [];
    const buckets = new Map<string, { actor: ActorOption | null; items: BiasInstance[] }>();
    // Stable sentinel for rows that somehow lack a stated_by — defensive,
    // since the bias query requires a STATED_BY edge.
    const ORPHAN_KEY = "__orphan__";

    for (const inst of instances) {
        const key = inst.stated_by?.id ?? ORPHAN_KEY;
        if (!buckets.has(key)) {
            order.push(key);
            buckets.set(key, {
                actor: inst.stated_by ?? null,
                items: [],
            });
        }
        buckets.get(key)!.items.push(inst);
    }

    return order.map((k) => buckets.get(k)!);
}

// ─── Styles ─────────────────────────────────────────────────────────────────

const groupContainerStyle: React.CSSProperties = {
    marginBottom: "1.5rem",
};

const groupHeaderStyle: React.CSSProperties = {
    fontSize: "1.05rem",
    fontWeight: 700,
    color: "#0f172a",
    marginBottom: "0.6rem",
    display: "flex",
    alignItems: "center",
    gap: "0.6rem",
};

const actorTypeBadgeStyle: React.CSSProperties = {
    padding: "0.15rem 0.5rem",
    borderRadius: "9999px",
    fontSize: "0.7rem",
    fontWeight: 600,
    backgroundColor: "#eff6ff",
    color: "#1d4ed8",
    textTransform: "uppercase",
    letterSpacing: "0.025em",
};

const groupCountStyle: React.CSSProperties = {
    fontSize: "0.78rem",
    color: "#64748b",
    fontWeight: 500,
};

const cardListStyle: React.CSSProperties = {
    display: "flex",
    flexDirection: "column",
    gap: "0.6rem",
};

// ─── Component ──────────────────────────────────────────────────────────────

interface Props {
    result: BiasQueryResult;
}

const BiasByActorView: React.FC<Props> = ({ result }) => {
    const groups = groupByActor(result.instances);

    return (
        <div>
            {groups.map((g, idx) => {
                const headerName = g.actor?.name ?? "Unknown actor";
                const headerKey = g.actor?.id ?? `orphan-${idx}`;
                const headerType = g.actor?.actor_type;
                return (
                    <div key={headerKey} style={groupContainerStyle}>
                        <div style={groupHeaderStyle}>
                            <span>{headerName}</span>
                            {headerType && <span style={actorTypeBadgeStyle}>{headerType}</span>}
                            <span style={groupCountStyle}>
                                {g.items.length} {g.items.length === 1 ? "instance" : "instances"}
                            </span>
                        </div>
                        <div style={cardListStyle}>
                            {g.items.map((inst) => (
                                <EvidenceCard key={inst.evidence_id} instance={inst} />
                            ))}
                        </div>
                    </div>
                );
            })}
        </div>
    );
};

export default BiasByActorView;
