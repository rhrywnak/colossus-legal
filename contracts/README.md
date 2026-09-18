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

The bytes are what `JSON.stringify` emits — key order included — with one
trailing newline so the file is a well-formed text file. Both readers trim.
