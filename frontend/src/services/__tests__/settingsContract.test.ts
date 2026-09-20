// =============================================================================
// The frontend half of the Settings page wire contract
// =============================================================================
//
// `contracts/settings_page.json` holds the exact bytes the page is served.
// `backend/src/dto/settings_wire_tests.rs` asserts they deserialize as
// `SettingsPageDto` with nothing left over; this file asserts the browser reads
// every field by the name the backend wrote it under, and that the page's own
// helpers group the result the way the page will.
//
// ## What this catches that two green suites do not
//
// The Include button returned 400 for months with both suites passing, because
// each side tested itself against its own idea of the body. This page's version
// of that failure is quieter still: rename `block_id` on one side and nothing
// breaks — every row simply resolves to "Undeclared — read by nothing", which
// looks exactly like a store full of dead rows. A page can be completely wrong
// and completely plausible at the same time.
//
// TypeScript cannot help here: `data as SettingsPageDto` is a promise the
// compiler takes at face value. So each field is READ by name below, and a
// rename on either side reds a test in the other language.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import type { SettingsPageDto } from "../settings";
import {
  changedFromDefault,
  searchSettings,
} from "../../components/settings/settingsSearch";
import { uncoupledIn } from "../../components/settings/coupledList";

const CONTRACT = readFileSync(
  fileURLToPath(new URL("../../../../contracts/settings_page.json", import.meta.url)),
  "utf8",
).trim();

const page = JSON.parse(CONTRACT) as SettingsPageDto;

describe("the settings page contract", () => {
  it("carries every field the page reads, under the names it reads them by (M)", () => {
    // MUTATION: rename any one of these in `dto/settings.rs` and this goes red
    // here, in the other language, which is the only place it could be noticed.
    const [cap] = page.settings;

    expect(cap.key).toBe("talking_points_cap");
    expect(cap.value).toBe("5");
    expect(cap.default_value).toBe("3");
    expect(cap.meaning).toContain("talking points");
    expect(cap.input_hint).toBeTypeOf("string");
    expect(cap.bounds_label).toBe("At least 1");
    expect(cap.dormant_note).toBeNull();
    expect(cap.last_changed).toContain("roman");
    expect(cap.area_id).toBe("core");
    expect(cap.block_id).toBe("core");
    expect(cap.changed_from_default).toBe("Changed — default: 3");
  });

  it("carries a rail whose counts and labels are already taken", () => {
    const [, core, undeclared] = page.areas;

    expect(core.id).toBe("core");
    expect(core.label).toBe("Core");
    expect(core.count).toBe(1);
    expect(core.note).toBeNull();
    expect(core.blocks[0].id).toBe("core");
    expect(core.blocks[0].count).toBe(1);

    // The dead group, last, saying what it is. Ruled 2026-09-19.
    expect(undeclared.id).toBe("undeclared");
    expect(undeclared.label).toBe("Undeclared — read by nothing");
    expect(undeclared.note).toContain("dead");
  });

  it("groups the way the page will group it", () => {
    // The end-to-end claim: these bytes, through the page's own helpers, produce
    // the landing list, the block membership and the search the four states use.
    // Both rows have moved off their defaults, so both are in the landing list
    // — including the coupled one. That is deliberate: "what have I changed
    // here?" must not hide a change because of how it is edited.
    const changed = changedFromDefault(page.settings);
    expect(changed.map((s) => s.key)).toEqual([
      "talking_points_cap",
      "practice_reviewer_usernames",
    ]);
    // And the coupled one carries the group, so the landing list renders it
    // with a pointer to the editor rather than a field that cannot be saved.
    expect(changed[1].group_id).toBe("reviewer_bench");
    expect(
      uncoupledIn(page.settings, "undeclared_dead").map((s: { key: string }) => s.key),
    ).toEqual(["practice_notes_save_label"]);

    const results = searchSettings(page.settings, page.areas, "talking");
    expect(results.total).toBe(1);
    expect(results.groups[0].areaId).toBe("core");
  });

  it("is the shape the page's own validation accepts", () => {
    // `fetchSettings` refuses a response with no `settings` or no `areas`; the
    // contract must be a response it accepts, or the contract describes bytes
    // the page would throw on.
    expect(Array.isArray(page.settings)).toBe(true);
    expect(Array.isArray(page.areas)).toBe(true);
  });
});
