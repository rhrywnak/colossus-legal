#!/usr/bin/env python3
"""Pre-flight extraction gate — REQUIRED before any paid run.

Two modes, one per chunking strategy in use. Both answer the same question:
*will the pipeline do what the profile says it will do?* Neither the structured
splitter nor the full-document path answers that on its own, and both fail in
ways that read as success in the logs.

MODE structured (default) — court_transcript
--------------------------------------------
`court_transcript_v5_3.yaml` uses `strategy: custom` with a speaker-label
`boundary_pattern`, because none of the three shipped strategies matches a
line-numbered transcript. That works — but the failure mode when it does NOT
match is silent and expensive.

`StructureAwareSplitter::split` returns ONE chunk containing the whole document
when the boundary pattern matches nothing (or fails to compile). It stamps
`fallback: true` / `no_boundary_matches` into chunk metadata that the caller
never reads, so `llm_extract.rs` logs `chunk_count = 1` and the run looks
successful. A 36-page transcript then ships to the model as a single prompt.

There is no observable that distinguishes "correctly produced one chunk" from
"pattern matched nothing". This mode is that observable.

MODE full — motion (and any other full-document profile)
--------------------------------------------------------
Full-document mode has no boundary pattern, so it cannot fail that way. Its
failure modes are different and equally quiet:

  * The document OCR'd to nothing, or to almost nothing. Pages below the
    pipeline's own OCR floor produced no usable text, and the model is asked to
    extract from blank pages. Nothing errors.
  * The document is far larger than full mode assumes, so the assembled prompt
    crowds or exceeds the context window. What comes back is a truncated or
    degraded extraction, not a failure.
  * The extracted page count does not match the document, so pages are silently
    missing before the model ever sees them.

This mode checks those three, and nothing about chunk boundaries — there are
none to check.

USAGE
-----
    scripts/transcript-chunk-preflight.py <extracted-text-file> [--profile PATH]
    scripts/transcript-chunk-preflight.py --mode full <text> [--expect-pages N]

In STRUCTURED mode the boundary pattern is read from the profile on disk rather
than assumed, so the check cannot drift from what the pipeline will actually
use. FULL mode reads no profile at all — there is no boundary pattern to compare
against — so `--profile` does not apply there and passing it is reported rather
than silently ignored.

NAMING NOTE: this file is still called transcript-chunk-preflight.py because
the court_transcript profile references it by that path. It now serves both
document types; renaming it would break that reference silently, which is the
class of failure this script exists to prevent.

EXIT CODES (identical in both modes)
------------------------------------
    0  PASS — safe to proceed to a paid run
    1  FAIL — the pipeline will not do what the profile says. Do NOT run.
       structured: unit count outside the band, or zero matches (the
       silent-fallback case). full: text below the OCR floor, oversized prompt,
       or a page-count mismatch.
    2  Cannot check — text file missing/unreadable, profile missing/unreadable,
       a profile key absent-when-required or present-but-malformed, or a
       boundary_pattern that will not compile. This is NEITHER pass nor fail:
       the gate could not determine what the pipeline would do, so it declines
       to vouch for the run. Distinguishing this from 1 matters — 1 means "the
       configuration is wrong", 2 means "the question could not be asked".
"""

import argparse
import re
import sys
from pathlib import Path

# ── structured-mode thresholds ───────────────────────────────────────────────
# Sane band for a hearing transcript of the size in this corpus (36-37 pp,
# ~25 lines/page). Below the floor almost certainly means the pattern is not
# matching speaker labels; above the ceiling means it is matching something it
# should not (every line, or mid-utterance capitals).
MIN_UNITS = 150
MAX_UNITS = 600

# ── full-mode thresholds ─────────────────────────────────────────────────────
# The pipeline's own per-page OCR floor: `OcrConfig.char_threshold` defaults to
# 50 non-whitespace characters, below which extract_text routes a page to Surya.
# A page still under it AFTER extraction produced no usable text at all.
OCR_FLOOR_NONWS_CHARS = 50

# Share of pages allowed to sit under the floor before the document is judged
# unextracted. Not zero: a genuine cover sheet or exhibit divider is legitimately
# near-empty, and failing on one blank page would make the gate noise.
MAX_EMPTY_PAGE_RATIO = 0.25

# Rough chars-per-token for English legal prose. Deliberately conservative —
# over-estimating tokens makes the gate fire early, which is the safe direction.
CHARS_PER_TOKEN = 4

# Ceiling on the DOCUMENT's contribution to the prompt, in estimated tokens.
# Full mode sends the whole document plus the template, schema and global rules
# in one request. The corpus motions sit near 21k; this leaves roughly an order
# of magnitude of headroom for a long filing while still catching a document
# that was never meant for full mode (a 500-page exhibit compilation).
MAX_DOC_TOKENS = 150_000

DEFAULT_PROFILE = "backend/profiles/court_transcript_v5_3.yaml"

# How many detected speaker labels to show, so a human can eyeball whether the
# pattern found the right thing rather than merely the right NUMBER of things.
SAMPLE_SIZE = 12


class ProfileError(Exception):
    """The profile could not be read well enough to describe what the pipeline
    will do.

    Raised rather than exiting inline so main() owns every exit code in one
    place. Note that `raise SystemExit("some message")` does NOT exit with the
    code this script documents — Python prints the string and exits 1 — which
    would silently collapse a config error into a data failure.
    """


def _scalar_after(line: str, key: str) -> str:
    """Return the unquoted scalar following `key:` on a stripped YAML line."""
    value = line.split(":", 1)[1].strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in ("'", '"'):
        value = value[1:-1]
    return value


def load_boundary_pattern(profile_path: Path) -> str:
    """Read boundary_pattern out of the profile YAML.

    Deliberately a line scan rather than a YAML parse: this script must run with
    no third-party dependencies (it is executed on an operator's machine as part
    of a dry-run gate, not inside the app), and the key is a simple scalar.
    """
    try:
        text = profile_path.read_text(encoding="utf-8")
    except OSError as exc:
        raise ProfileError(f"cannot read profile {profile_path}: {exc}") from exc

    for line in text.splitlines():
        stripped = line.strip()
        if not stripped.startswith("boundary_pattern:"):
            continue
        value = _scalar_after(stripped, "boundary_pattern")
        if not value:
            raise ProfileError(f"boundary_pattern in {profile_path} is empty")
        return value

    raise ProfileError(
        f"no boundary_pattern key found in {profile_path}.\n"
        "       If the profile switched away from strategy: custom, this gate "
        "no longer describes what the pipeline will do — re-check before running."
    )


def load_units_per_chunk(profile_path: Path) -> int | None:
    """Read units_per_chunk so the reported chunk count matches reality.

    Three distinct outcomes, three distinct observables (Standing Rule 1):
      - present and valid  → the integer
      - absent             → None, and main() says so; the unit-count check is
                             still meaningful, only the chunk arithmetic is not
      - present but bad    → ProfileError, which is FATAL. A profile this gate
                             cannot fully read is a profile whose behaviour it
                             cannot vouch for, and returning a PASS on one would
                             defeat the entire point of the gate.
    """
    try:
        text = profile_path.read_text(encoding="utf-8")
    except OSError as exc:
        raise ProfileError(
            f"profile {profile_path} became unreadable mid-run: {exc}"
        ) from exc

    for line in text.splitlines():
        stripped = line.strip()
        if not stripped.startswith("units_per_chunk:"):
            continue
        raw = _scalar_after(stripped, "units_per_chunk")
        try:
            return int(raw)
        except ValueError as exc:
            raise ProfileError(
                f"units_per_chunk in {profile_path} is not an integer: {raw!r}"
            ) from exc

    return None


def split_pages(text: str) -> list[tuple[int, str]]:
    """Split assembled document text on the '--- Page N ---' markers.

    Those markers are injected by the prompt assembler in `llm_extract.rs`, so
    splitting on them measures the same page set the model will be shown.
    Returns [(page_number, page_text)]; empty when no marker is present.
    """
    parts = re.split(r"--- Page (\d+) ---", text)
    if len(parts) < 3:
        return []
    return [(int(parts[i]), parts[i + 1]) for i in range(1, len(parts) - 1, 2)]


def nonws(s: str) -> int:
    """Count non-whitespace characters — the same measure the pipeline's OCR
    threshold uses, so the floor here means what it means there."""
    return sum(1 for c in s if not c.isspace())


def run_full_mode(text: str, expect_pages: int | None) -> int:
    """Full-document mode: check the text is real, sized, and complete.

    There is no boundary pattern in this mode, so there is nothing to verify
    about chunking. What can go wrong is that the document never extracted, is
    too large for one prompt, or lost pages on the way in.
    """
    pages = split_pages(text)
    total_nonws = nonws(text)
    est_tokens = len(text) // CHARS_PER_TOKEN

    print(f"mode            : full (whole document in one prompt)")
    print(f"characters      : {len(text):,}")
    print(f"non-whitespace  : {total_nonws:,}")
    print(f"est. doc tokens : {est_tokens:,}  (ceiling {MAX_DOC_TOKENS:,})")

    if not pages:
        print("pages           : NO '--- Page N ---' markers found")
        print()
        print("RESULT: FAIL — the text carries no page markers.")
        print()
        print("  The prompt assembler injects these, so text without them is not")
        print("  what the pipeline will send. Either this file was produced some")
        print("  other way, or extract_text stored nothing. Check the source of")
        print("  this file before reading anything else into the result.")
        return 1

    empty = [p for p, body in pages if nonws(body) < OCR_FLOOR_NONWS_CHARS]
    ratio = len(empty) / len(pages)
    print(f"pages           : {len(pages)}")
    print(
        f"pages under OCR floor ({OCR_FLOOR_NONWS_CHARS} non-ws chars): "
        f"{len(empty)} ({ratio:.0%})"
    )
    if empty:
        shown = ", ".join(str(p) for p in empty[:15])
        more = "" if len(empty) <= 15 else f" … +{len(empty) - 15} more"
        print(f"    page numbers: {shown}{more}")
    print()

    failed = False

    if ratio > MAX_EMPTY_PAGE_RATIO:
        print(
            f"RESULT: FAIL — {ratio:.0%} of pages are under the OCR floor "
            f"(limit {MAX_EMPTY_PAGE_RATIO:.0%})."
        )
        print()
        print("  The model would be asked to extract from pages that carry no")
        print("  usable text. Nothing in the pipeline errors on this — it simply")
        print("  extracts less and reports success. Check, in this order:")
        print("    1. Is this a scanned PDF whose OCR step has not run yet?")
        print("    2. Did the OCR service return empty for these pages?")
        print("    3. Are the listed pages genuinely blank (dividers, backs)?")
        failed = True

    if est_tokens > MAX_DOC_TOKENS:
        print(
            f"RESULT: FAIL — estimated {est_tokens:,} document tokens exceeds "
            f"the {MAX_DOC_TOKENS:,} ceiling."
        )
        print()
        print("  Full-document mode sends this in ONE request, on top of the")
        print("  template, schema and global rules. At this size the extraction")
        print("  degrades or truncates rather than failing outright. Either this")
        print("  document needs a chunked profile, or it is a compilation that")
        print("  should be split into its constituent documents first.")
        failed = True

    if expect_pages is not None and len(pages) != expect_pages:
        print(
            f"RESULT: FAIL — extracted {len(pages)} pages, expected {expect_pages}."
        )
        print()
        print("  Pages went missing before the model sees them. Extraction is")
        print("  per-page and a dropped page is silent downstream.")
        failed = True

    if failed:
        return 1

    print(f"RESULT: PASS — {len(pages)} pages, {est_tokens:,} est. tokens, "
          f"{ratio:.0%} under the OCR floor.")
    print("  The whole document will fit one prompt and carries real text.")
    print("  Safe to proceed to the paid run, subject to the rest of the")
    print("  spend-gate discipline.")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Pre-flight extraction gate — structured and full modes."
    )
    parser.add_argument("text_file", type=Path, help="OCR'd/extracted document text")
    parser.add_argument(
        "--mode",
        choices=("structured", "full"),
        default="structured",
        help="which chunking mode the profile uses (default: structured)",
    )
    parser.add_argument(
        "--expect-pages",
        type=int,
        default=None,
        help="full mode only: assert the extracted page count matches this",
    )
    # Default resolved AFTER parsing rather than here, so the code can tell
    # "operator passed --profile" apart from "operator passed nothing" and say
    # so in full mode instead of quietly disregarding the flag.
    parser.add_argument(
        "--profile",
        type=Path,
        default=None,
        help=(
            "structured mode only: profile YAML to read boundary_pattern from "
            f"(default: {DEFAULT_PROFILE}). Not used in full mode."
        ),
    )
    args = parser.parse_args()

    if args.mode == "full" and args.profile is not None:
        print(
            f"NOTE: --profile {args.profile} is not used in full mode — the gate "
            "reads no\n"
            "      boundary pattern in this configuration. Checking the text "
            "itself only.",
            file=sys.stderr,
        )
    if args.profile is None:
        args.profile = Path(DEFAULT_PROFILE)

    if args.mode == "structured" and args.expect_pages is not None:
        print(
            "ERROR: --expect-pages applies to --mode full only.",
            file=sys.stderr,
        )
        return 2

    if not args.text_file.is_file():
        print(f"ERROR: no such text file: {args.text_file}", file=sys.stderr)
        return 2

    try:
        text = args.text_file.read_text(encoding="utf-8", errors="replace")
    except OSError as exc:
        print(f"ERROR: cannot read {args.text_file}: {exc}", file=sys.stderr)
        return 2

    print(f"text file       : {args.text_file}")

    # Full mode reads nothing from the profile — there is no boundary pattern to
    # compare against, and demanding one would make the gate fail on exactly the
    # profiles it is meant to check.
    if args.mode == "full":
        return run_full_mode(text, args.expect_pages)

    try:
        pattern = load_boundary_pattern(args.profile)
        per_chunk = load_units_per_chunk(args.profile)
    except ProfileError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        print(
            "       This gate cannot describe what the pipeline will do with an "
            "unreadable profile,\n"
            "       so it reports neither PASS nor FAIL. Fix the profile and "
            "re-run before authorising spend.",
            file=sys.stderr,
        )
        return 2

    try:
        # The splitter prefixes (?m) before compiling; mirror that exactly.
        regex = re.compile(pattern, re.MULTILINE)
    except re.error as exc:
        print(f"ERROR: boundary_pattern does not compile: {exc}", file=sys.stderr)
        print(
            "       Note the pipeline treats a bad pattern IDENTICALLY to a "
            "pattern that matches nothing —\n"
            "       one whole-document chunk, logged as success.",
            file=sys.stderr,
        )
        return 2

    matches = [m.group(0).strip() for m in regex.finditer(text)]
    count = len(matches)

    chunks = (count + per_chunk - 1) // per_chunk if per_chunk else 0

    print(f"profile         : {args.profile}")
    print(f"boundary_pattern: {pattern}")
    print(f"characters      : {len(text):,}")
    print(f"units detected  : {count}")
    if per_chunk:
        print(f"units_per_chunk : {per_chunk}")
        print(f"chunks (approx) : {chunks}")
    else:
        # Absent, not malformed — a malformed value is fatal above. Say so
        # rather than just omitting the chunk lines, which would look identical
        # to a run where the arithmetic simply wasn't interesting.
        print("units_per_chunk : ABSENT from the profile")
        print("chunks (approx) : not computed")
        print(
            "  ⚠ With strategy: custom the splitter supplies no default for this "
            "key, so chunk\n"
            "    sizing is whatever the splitter falls back to. The unit count "
            "below is still\n"
            "    valid; the chunk arithmetic is not. Consider setting it explicitly."
        )
    print()

    if count == 0:
        print("RESULT: FAIL — ZERO units detected.")
        print()
        print("  This is the silent-fallback case. If run as-is, the splitter will")
        print("  return ONE chunk containing the entire document, the pipeline will")
        print("  log chunk_count = 1, and the run will LOOK successful while sending")
        print("  the whole transcript as a single prompt.")
        print()
        print("  Do NOT authorise a paid run. Check, in this order:")
        print("    1. Did text extraction actually produce speaker labels? (head the file)")
        print("    2. Did the labels survive at line starts, or were lines merged?")
        print("    3. Does the boundary_pattern still match the shape on disk?")
        return 1

    sample = matches[:SAMPLE_SIZE]
    print(f"first {len(sample)} detected units:")
    for label in sample:
        print(f"    {label!r}")

    # Strip the leading gutter line-number before counting distinct speakers.
    # Without this every turn looks unique ("1 MR. SHARP:" vs "4 MR. SHARP:"),
    # the count reports the turn total instead of the speaker total, and the
    # single-label warning below can never fire.
    speakers = {re.sub(r"^\s*\d*\s*", "", m).rstrip(":").strip() for m in matches}
    print()
    print(f"distinct speaker labels: {len(speakers)}")
    print(f"    {sorted(speakers)}")
    distinct = len(speakers)
    if distinct == 1:
        print("  ⚠ WARNING: only ONE distinct label matched. A transcript has several")
        print("    speakers; a single label usually means the pattern is matching a")
        print("    recurring artifact (a page header, a timestamp) rather than speech.")
    print()

    if count < MIN_UNITS:
        print(f"RESULT: FAIL — {count} units is below the floor of {MIN_UNITS}.")
        print("  Too few turns for a full hearing. The pattern is probably matching")
        print("  only some speaker labels — check whether attorney labels (MR./MS.)")
        print("  are being missed while THE COURT matches.")
        return 1

    if count > MAX_UNITS:
        print(f"RESULT: FAIL — {count} units is above the ceiling of {MAX_UNITS}.")
        print("  Too many turns. The pattern is probably matching mid-utterance")
        print("  capitals or per-line artifacts rather than speaker labels, which")
        print("  would fragment single utterances across units.")
        return 1

    print(f"RESULT: PASS — {count} units, within the sane band {MIN_UNITS}-{MAX_UNITS}.")
    print("  Chunking will behave as designed. Safe to proceed to the paid run,")
    print("  subject to the rest of the spend-gate discipline.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
