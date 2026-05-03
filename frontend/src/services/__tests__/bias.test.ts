import { afterEach, describe, expect, it, vi } from "vitest";
import { getAvailableFilters, runBiasQuery } from "../bias";

describe("bias service", () => {
    afterEach(() => {
        vi.restoreAllMocks();
    });

    describe("getAvailableFilters", () => {
        it("returns parsed dropdown shape on 200", async () => {
            const payload = {
                actors: [
                    {
                        id: "person-george",
                        name: "George Phillips",
                        actor_type: "Person",
                        tagged_statement_count: 114,
                    },
                ],
                pattern_tags: ["disparagement", "secrecy"],
            };
            // @ts-ignore — overriding global for test
            global.fetch = vi.fn().mockResolvedValue({
                ok: true,
                status: 200,
                statusText: "OK",
                json: async () => payload,
            });

            const result = await getAvailableFilters();
            expect(result).toEqual(payload);
        });

        it("throws with status info when the response is non-OK", async () => {
            // @ts-ignore
            global.fetch = vi.fn().mockResolvedValue({
                ok: false,
                status: 503,
                statusText: "Service Unavailable",
                json: async () => ({}),
            });
            await expect(getAvailableFilters()).rejects.toThrow(/503/);
        });
    });

    describe("runBiasQuery", () => {
        it("posts the filters as JSON and parses the result", async () => {
            const filters = { actor_id: "person-george", pattern_tag: "disparagement" };
            const payload = {
                total_count: 1,
                instances: [],
                applied_filters: filters,
            };
            const fetchMock = vi.fn().mockResolvedValue({
                ok: true,
                status: 200,
                statusText: "OK",
                json: async () => payload,
            });
            // @ts-ignore
            global.fetch = fetchMock;

            const result = await runBiasQuery(filters);

            expect(result).toEqual(payload);
            expect(fetchMock).toHaveBeenCalledTimes(1);
            const callArgs = fetchMock.mock.calls[0];
            const init = callArgs[1] as RequestInit;
            expect(init.method).toBe("POST");
            expect(init.headers).toMatchObject({ "Content-Type": "application/json" });
            expect(init.body).toBe(JSON.stringify(filters));
        });

        it("posts an empty filter object as {} when no filters are set", async () => {
            const fetchMock = vi.fn().mockResolvedValue({
                ok: true,
                status: 200,
                statusText: "OK",
                json: async () => ({ total_count: 0, instances: [], applied_filters: {} }),
            });
            // @ts-ignore
            global.fetch = fetchMock;

            await runBiasQuery({});
            const init = fetchMock.mock.calls[0][1] as RequestInit;
            expect(init.body).toBe("{}");
        });

        it("throws on non-OK response", async () => {
            // @ts-ignore
            global.fetch = vi.fn().mockResolvedValue({
                ok: false,
                status: 500,
                statusText: "Internal Server Error",
                json: async () => ({}),
            });
            await expect(runBiasQuery({})).rejects.toThrow(/500/);
        });
    });
});
