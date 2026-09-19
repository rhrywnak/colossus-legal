# contracts/

One file per wire shape that two languages have to agree about, held as the
**exact bytes** rather than as two descriptions of them.

`fact_action_include.json` is the first. It exists because the Include button in
the triage queue returned HTTP 400 for months: the backend has required
`allegation_id` and `stance` on an include since FACT_CARD_v2 §2, the browser
sent `{"action":"include"}`, and nothing in either test suite could see the
mismatch — each side tested itself against its own idea of the body.

A fixture here is read by BOTH sides:

| Side | Test |
|---|---|
| Frontend | `frontend/src/services/__tests__/factActionBody.test.ts` — the builder's output must equal these bytes |
| Backend | `backend/src/dto/scenario_facts_wire_tests.rs` — these bytes must deserialize as `FactActionRequest` and survive `include_link` |

So a change to either side that breaks the agreement fails a test in the OTHER
language, which is the only arrangement that could have caught the original
defect.

`settings_page.json` is the second, and it points the other way: it is a
RESPONSE, not a request body. The admin Settings page groups 860-odd rows by two
fields the server places on each of them — `area_id` and `block_id` — and it
composes nothing itself. Rename either field on one side and no build breaks:
every row resolves to "Undeclared — read by nothing", which looks exactly like a
store full of dead rows rather than like a contract mismatch. A page can be
completely wrong and completely plausible at once, and TypeScript cannot help,
because `data as SettingsPageDto` is a promise the compiler takes at face value.

| Side | Test |
|---|---|
| Frontend | `frontend/src/services/__tests__/settingsContract.test.ts` — every field is read by name, and the page's own helpers group the result |
| Backend | `backend/src/dto/settings_wire_tests.rs` — these bytes deserialize as `SettingsPageDto`, and an undeclared field is refused |

Its two rows are chosen to exercise both ends of the page: one that has moved off
its default (the landing list) and one that no block declares (the dead group).

The bytes are what `JSON.stringify` emits — key order included — with one
trailing newline so the file is a well-formed text file. Both readers trim.
