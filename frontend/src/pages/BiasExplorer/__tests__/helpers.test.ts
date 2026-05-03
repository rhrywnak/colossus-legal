import { describe, expect, it } from "vitest";

import { groupByActor } from "../BiasByActorView";
import { formatTagLabel } from "../BiasExplorerFilters";
import type { BiasInstance } from "../types";

describe("formatTagLabel", () => {
    it("converts snake_case to Title Case with spaces", () => {
        expect(formatTagLabel("selective_enforcement")).toBe("Selective Enforcement");
        expect(formatTagLabel("lies_under_oath")).toBe("Lies Under Oath");
    });

    it("handles single-word tags", () => {
        expect(formatTagLabel("secrecy")).toBe("Secrecy");
    });

    it("preserves empty input", () => {
        expect(formatTagLabel("")).toBe("");
    });
});

describe("groupByActor", () => {
    const phillips: BiasInstance = {
        evidence_id: "e-1",
        title: "T1",
        pattern_tags: ["disparagement"],
        about: [],
        stated_by: {
            id: "person-george",
            name: "George Phillips",
            actor_type: "Person",
            tagged_statement_count: 0,
        },
    };
    const cfs: BiasInstance = {
        evidence_id: "e-2",
        title: "T2",
        pattern_tags: ["secrecy"],
        about: [],
        stated_by: {
            id: "org-cfs",
            name: "Catholic Family Services",
            actor_type: "Organization",
            tagged_statement_count: 0,
        },
    };
    const phillips2: BiasInstance = {
        ...phillips,
        evidence_id: "e-3",
    };

    it("groups items by actor id while preserving server order", () => {
        const groups = groupByActor([phillips, cfs, phillips2]);
        expect(groups).toHaveLength(2);
        expect(groups[0].actor?.id).toBe("person-george");
        expect(groups[0].items.map((i) => i.evidence_id)).toEqual(["e-1", "e-3"]);
        expect(groups[1].actor?.id).toBe("org-cfs");
        expect(groups[1].items.map((i) => i.evidence_id)).toEqual(["e-2"]);
    });

    it("buckets actor-less rows under a sentinel without crashing", () => {
        const orphan: BiasInstance = {
            evidence_id: "e-orphan",
            title: "T",
            pattern_tags: [],
            about: [],
        };
        const groups = groupByActor([orphan]);
        expect(groups).toHaveLength(1);
        expect(groups[0].actor).toBeNull();
        expect(groups[0].items[0].evidence_id).toBe("e-orphan");
    });

    it("returns an empty array for an empty input", () => {
        expect(groupByActor([])).toEqual([]);
    });
});
