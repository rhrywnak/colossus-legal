# test_fixtures/document_text

Real page text, exported verbatim from the DEV `document_text` table on
2026-08-03 for task 1.7C (defect D6, the context law). Committed as fixtures
rather than hand-written prose because the whole point of D6 is that this
corpus's real formatting — court-reporter gutter numerals, `MR.` speaker tags,
interrogatory sub-part enumerators, affidavit jurats — is what breaks a naive
sentence splitter. A test over invented text would prove nothing about the
documents the product actually reads.

Each file is one page's `text_content`, unmodified.

| File | Source document | Page | Why this page |
|---|---|---|---|
| `transcript_p25.txt` | `doc-hearing-to-approve-plan-for-adminnistration-12-15-2009` (`court_transcript`) | 25 | 26 standalone gutter numerals; `MR.` / `THE COURT:` speaker tags; the mockup's own C-95 example quote lives here |
| `transcript_p14.txt` | same | 14 | gutter numerals PLUS the Surya OCR line-transposition wart (`"I don't know"` and `"if she got hers or not."` arrive out of order) — the honest limit 1.7C must not paper over |
| `discovery_response_p31.txt` | `doc-george-phillips-response-to-discovery` (`discovery_response`) | 31 | 10 line-initial single-letter enumerators (`a.` `b.` `c.` …), the false-boundary class no abbreviation list would catch |
| `court_order_p9.txt` | `doc-judge-tighe-opinion-and-order-041212` (`court_ruling`) | 9 | 6 legal abbreviations (`v.` `Inc.` `No.` `Dec.` `Co.`) plus a one-per-page footer numeral |
| `affidavit_p3.txt` | `doc-sabrina-morris-affidavit` (`affidavit`) | 3 | the `SS.` jurat ("STATE OF MICHIGAN ) SS."), measured 16× across the two affidavits |

Do not edit these files. If a fixture needs to change, re-export the page and
update the row above — an edited fixture is no longer evidence about the corpus.
