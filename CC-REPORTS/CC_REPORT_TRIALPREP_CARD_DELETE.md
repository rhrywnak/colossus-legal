# CC_REPORT_TRIALPREP_CARD_DELETE

**Task:** `CC_FIX_TRIALPREP_CARD_DELETE_AND_METRICS_v1` · **Date:** 2026-08-07
**Branch:** `fix/scenario-definition-authoring` (per the instruction's sequencing —
one feature push carries the link fix, the authoring fix, and this)
**Commit:** `afd2f0b` — 13 files (amended from `b85a380` with the §11 findings)
**Rulings applied:** Q1 — remove `speakers` and `response_count` as well · Q2 —
leave the dashboard subtitle alone.

---

## FIX 1 — Delete on the Trial Prep scenario card

Every scenario card now carries the ⋯ kebab with Delete, opening the existing
confirm dialog and calling the existing `DELETE /cases/:slug/scenarios/:id`.
On success the card disappears and the metrics band updates.

Reused, not rebuilt: `ScenarioKebab` (unchanged), `ScenarioDeleteConfirm`
(unchanged), `deleteScenario` (unchanged), the route (unchanged).

### The one real design decision: where the kebab sits [measured]

The whole card is a `<Link>` — it renders an `<a>`, and the entire surface is the
navigation target. So a kebab placed *inside* it would be a `<button>` nested in
an `<a>`: **invalid HTML**, and every click on the menu would also navigate.

The obvious patch is `e.preventDefault(); e.stopPropagation()`. It was rejected.
That leaves the markup invalid, and it only covers the paths you remember to
cover — the keyboard activation path and middle-click still reach the anchor.

Instead the anchor and the kebab are **siblings** inside a `position: relative`
wrapper, with the kebab absolutely positioned over the card's top-right corner
([ScenarioCard.tsx](frontend/src/components/ScenarioCard.tsx)). Nothing needs
suppressing because nothing overlaps: they are two genuinely separate controls.
The card title carries `paddingRight` so it never runs under the menu.

This is a **structural** guarantee, not a behavioural one — worth stating plainly
because it is the instruction's "must not hijack the card link" rule, and this
repo has no component-test infrastructure (Rule 30) to assert it with. The
guarantee is that the button is not a descendant of the anchor, which is visible
in the JSX and cannot be broken without moving the element.

### Failure stays visible

The dialog lives on the **page**, not the card: one dialog serves N cards, and
the page is what owns `refreshKey` — the single re-read that makes the card
vanish *and* the band drop by one, rather than two hand-patched pieces of state.

`pendingDelete` holds the scenario (not a boolean), which is what lets the dialog
name it. On failure the dialog **stays open** with the cause on it and the grid
untouched; it closes only after the DELETE resolves. Closing is never proof a
scenario is gone — the re-read is. Same contract the scenario page's delete
already followed.

### Wording: no new strings [measured]

The three sentences the dialog says were literals inline on the scenario page.
Copying them to a second call site would have created exactly the second voice
the instruction forbids, so they moved to one pure builder —
[scenarioDeleteCopy.ts](frontend/src/components/scenarioDeleteCopy.ts) — which
**both** surfaces now call. Net effect: one fewer copy than before, not one more.

**They are still literals in code, not `app_settings` rows, and that is a
deliberate limit worth naming.** The standing law covers *new* user-facing
strings; these are existing ones relocated. Moving them to rows needs a
migration, a wording block and a delivery channel to two different payloads —
untested string plumbing in the same commit as a new destructive action. Recorded
in the file's own header rather than passed off as done. **Flagged for Roman as
follow-up work, not claimed as complete.**

---

## FIX 2 — The Instances metric is gone

Removed: the band tile, the per-card `N instances · no speakers yet · N responses`
line, `scenarioMetaLine`, the DTO fields, and the plumbing that computed them —
`count_record_rebuts`, `count_rebuts`, and `record_to_card`'s count parameter.

Per Q1, `response_count` and `speakers` went too. Both were honest stubs
(`0` and `[]`) that existed only to fill out the line displaying the count; with
the line gone they had no reader.

### What that actually deleted [measured]

`assemble` ran **one Neo4j read per anchor allegation, per scenario, on every
dashboard load** to sum REBUTS relationships. That loop is now pure shaping over
the Postgres rows:

```rust
let cards = records.iter().map(record_to_card).collect::<Result<Vec<_>, _>>()?;
```

**Listing scenarios no longer touches Neo4j at all.** The module header said the
dashboard had two data sources; it now has one, and says so. `assemble_detail`
still reads the graph — that is one scenario's timeline, a different question.

### Migration: NONE, and the instruction anticipated one [measured]

The instruction allowed for retiring wording rows "if any exist solely for this
line". **None exist.** A grep of every `pipeline_migrations/*.sql` for
instance/speaker wording returns only task 2.11's *accusation-instances* rows —
which are the unrelated concept whose name collision was half the reason for this
removal. `scenarioMetaLine` composed code literals, and the tile's label was a
code literal. **This change deletes strings rather than retiring rows.**

### Why it was removed, stated once in the code

`instances` is not like the two figures dropped on 2026-07-27. Those were
constants wearing a measurement's clothes. This one was a *real* measurement — of
something nobody could act on, under a name that already meant something else in
this product. That distinction is recorded on `TrialPrepMetrics` so the next
reader does not "restore" it as an oversight.

---

## VERIFICATION [measured]

```
backend:  cargo build --lib --bins                   OK
          cargo test --lib                           1678 passed, 0 failed, 2 ignored
          cargo clippy --lib --bins -D warnings      clean
          cargo fmt --check                          clean
frontend: npm run typecheck                          clean
          npx vitest run                             822 passed, 62 files, 0 failed
          npm run build                              built in 1.61s
          route-link guard                           13 passed (routePaths.test.ts)
```

`cargo test --workspace` not run — `backend/tests/*.rs` has been uncompilable
since ~beta.343 for reasons predating this branch. `--lib` is the honest
baseline. No `npm run lint` script exists in this repo.

### Tests written

| Test | What breaks if it fails |
|---|---|
| `the delete confirmation names the scenario it is about to delete` | On a grid of cards differing only by title, a dialog that does not name its scenario is how the wrong one gets deleted. |
| `says what survives, not only what is destroyed` | Without "the case graph is not affected", a reversible-feeling act reads as unbounded and the human stalls. |
| `the_dashboard_serves_no_instances_figure` (Rust) | The removal being partial: a field still computed and served but unrendered — which would mean the per-scenario graph read is back on every page load. |
| `metrics_band_exposes_no_figure_nobody_can_act_on` (Rust, rewritten) | Now guards against **both** failure modes this band has had: re-deriving a figure from a stub, and re-introducing `instances`. A fourth key fails it by name. |

Not written: that the kebab renders, or that the `<Link>` still has an `href` —
those restate JSX, and Rule 30 means there is no infrastructure to run them.

---

## THINGS FOUND ON THE WAY

**Two Rule 17 splits, both taken.** `TrialPrepViews.tsx` was already over the
limit (310) and my change pushed it to 337. `ScenarioCard` moved to its own
file — and the size was the lesser reason. That file's header opens by declaring
"no fetch, no state, no business logic", and `ScenarioKebab` holds open/closed
state and a document listener. Leaving the card there would have turned a
load-bearing header comment into one nobody could trust. `trialPrepCardStyles.ts`
carries the two style objects both files now need, so a second copy of a shape
token cannot make two pills stop looking alike.

**Final module sizes:** `TrialPrepViews.tsx` 310 → **247**. Still over, both
already over before this change and both now *smaller*:
`scenario_dashboard.rs` 477 → **419**, `ScenarioDetailPage.tsx` 386 → **381**.

**The `dashboard_serializes_to_contract_shape` fixture was extracted** to a
shared `sample_dashboard()` so the exact-JSON test and the absent-keys test read
one shape. Two fixtures would let the second keep passing against a shape the
first no longer describes.

---

## DEPLOYMENT

- New env vars, migrations, Ansible changes, Traefik/auth changes: **none**.
- **Container rebuild: BOTH.** The DTO field removals mean the two halves must
  ship together — an old frontend against this backend would read `undefined` for
  `instance_count`, and an old backend against this frontend would fail
  `deny_unknown_fields`.
- **Rollback:** revert the commit. No migration, no data change. Nothing is
  written except the DELETE a human explicitly confirms.

### What Roman will see on DEV

- Three metric tiles, not four. No per-card meta line.
- A ⋯ on each card; clicking it opens the menu without navigating. Delete opens
  the same confirm dialog the scenario page uses, naming that scenario.
- On confirm: the card goes and the band drops by one. On failure: the dialog
  stays, with the reason.

**Not deployed. Not pushed. Roman decides both.**

---

## THE §11 GATE

All four agents ran against `b85a380`, committed first on purpose (those agents
`git stash`, and the pop fails on `Cargo.lock` in this tree).

| Agent | Verdict |
|---|---|
| `rules-enforcer` | **PASS** — 13 files, all rules |
| `test-auditor` | **PASS** — 0 new tests required, 1 test asked to be REMOVED |
| `observability-checker` | 1 finding, fixed |
| `architecture-reviewer` | 3 findings — 2 fixed, 1 filed |

### Finding 1 (observability) — a failure message that named nothing

`confirmDelete`'s non-`Error` fallback read "Failed to delete the scenario. Try
again." On the scenario page that is tolerable — the page IS the scenario. On a
grid of cards it identifies none of them, and the operator has to connect the
error line to the dialog title above it.

It now names the scenario and its code. The target is also captured into a local
BEFORE the await, so the failure handler cannot read a `pendingDelete` the human
changed in the meantime.

### Finding 2 (architecture) — two doc lines left claiming the graph read

The module's header BLOCK was updated when the graph reads went. The one-line
file summary and the `ScenarioDashboardAssembler` struct doc were not, and both
still said "live graph-derived counts".

The reviewer named the consequence precisely: someone landing there for latency,
caching or connection-pool reasons would see the held `ScenarioRepository` and
the docs agreeing, and work from "listing scenarios hits Neo4j". It does not —
the repository is there for `assemble_detail` alone. Both lines now say so.

### Finding 3 (test-auditor) — a test that could not fail

`asks the same question however the scenario was reached` called one pure
function twice with identical arguments and compared. It is a complement
assertion: it cannot fail, and it would have passed on the day someone gave the
scenario page its own hardcoded copy — the only drift it claimed to guard.

Removed, with the reason recorded in the file header so it is not helpfully
restored. The consistency property is **structural** (one builder, both surfaces
import it), and no assertion in that file was ever guarding it.

### Finding 4 (architecture) — the wording debt, FILED not fixed

The reviewer held that an inline comment is not a tracked item, and that Rule 8
("no tech debt accumulation … fixed before push") makes this the moment to
either finish the job or record it properly. Correct on both counts.

**Filed as a task**, with the full scope: the wording block, the boot-loader
wiring, the migration via `new-migration.sh`, delivery to both payloads, the
frontend filler — and the `{attack}` placeholder's registration in
`REQUIRED_PLACEHOLDERS`, without which the settings write path would accept a
value with the scenario name stripped out.

**Not done in this commit, and that is a judgement call rather than an
oversight:** it is a migration plus a boot-critical key list, and putting
untested string plumbing in the same commit as a new destructive control is the
trade I declined. Roman's to overrule.

### Honest caveat on the amend

The agents inspected `b85a380`; the three fixes are in `afd2f0b`. Two are comment
text, one is an error-message string and a local binding, and one is a deletion —
none introduces a new pattern. The architecture reviewer assessed the delete
fallback as acceptable *as it stood in `b85a380`*; the observability gate had
judged the same line more strictly, and the stricter reading was taken. The full
mechanical gate was re-run over the amended tree and is green, counts in §2.
