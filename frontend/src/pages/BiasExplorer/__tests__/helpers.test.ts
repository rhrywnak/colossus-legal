import { afterEach, describe, expect, it, vi } from "vitest";

import { groupByActor } from "../BiasByActorView";
import { formatFilteredCounter, formatTagLabel } from "../BiasExplorerFilters";
import { applyDefaultSubject } from "../defaultSubject";
import type { AvailableFilters, BiasInstance } from "../types";

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

describe("formatFilteredCounter", () => {
    it("renders 'Filtered: X of Y' when a filter is active", () => {
        expect(formatFilteredCounter(47, 231, true)).toBe(
            "Filtered: 47 of 231 instances",
        );
    });

    it("renders 'Showing all Y' when no filter is active", () => {
        expect(formatFilteredCounter(231, 231, false)).toBe(
            "Showing all 231 instances",
        );
    });

    it("uses singular noun when total is 1", () => {
        expect(formatFilteredCounter(1, 1, false)).toBe("Showing all 1 instance");
        expect(formatFilteredCounter(0, 1, true)).toBe(
            "Filtered: 0 of 1 instance",
        );
    });

    it("drives wording by intent flag, not numerical equality", () => {
        // X equals Y but a filter is active — must read 'Filtered: ...' so
        // the user sees the filter is in force, not 'Showing all'.
        expect(formatFilteredCounter(231, 231, true)).toBe(
            "Filtered: 231 of 231 instances",
        );
    });
});

describe("applyDefaultSubject", () => {
    afterEach(() => {
        vi.restoreAllMocks();
    });

    function makeAvailable(over: Partial<AvailableFilters>): AvailableFilters {
        return {
            actors: [],
            pattern_tags: [],
            subjects: [],
            ...over,
        };
    }

    it("returns the server-supplied default_subject_id when present", () => {
        const result = applyDefaultSubject(
            makeAvailable({ default_subject_id: "person-marie" }),
        );
        expect(result).toBe("person-marie");
    });

    it("returns null and warns when no default is supplied", () => {
        const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
        const result = applyDefaultSubject(makeAvailable({}));
        expect(result).toBeNull();
        expect(warnSpy).toHaveBeenCalledTimes(1);
        expect(warnSpy.mock.calls[0][0]).toMatch(/no default subject/i);
    });

    it("treats empty-string default_subject_id as no default", () => {
        const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
        const result = applyDefaultSubject(
            makeAvailable({ default_subject_id: "" }),
        );
        expect(result).toBeNull();
        expect(warnSpy).toHaveBeenCalledTimes(1);
    });
});
