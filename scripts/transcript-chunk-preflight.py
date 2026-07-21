#!/usr/bin/env python3
"""Pre-flight gate for court_transcript extraction — REQUIRED before any paid run.

WHY THIS EXISTS
---------------
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
"pattern matched nothing". This script is that observable. Run it on the OCR'd
text BEFORE authorising spend.

USAGE
-----
    scripts/transcript-chunk-preflight.py <extracted-text-file> [--profile PATH]

Reads the boundary pattern from the profile YAML (so the check can never drift
from what the pipeline will actually use) and reports the unit count, the chunk
count, and a sample of detected speakers.

EXIT CODES
----------
    0  PASS — unit count within the sane band; safe to proceed to a paid run
    1  FAIL — unit count outside the band, or zero matches (the silent-fallback
       case). Do NOT run. Fix the pattern or check the OCR output first.
    2  Cannot check — text file missing/unreadable, profile missing/unreadable,
       a profile key absent-when-required or present-but-malformed, or a
       boundary_pattern that will not compile. This is NEITHER pass nor fail:
       the gate could not determine what the pipeline would do, so it declines
       to vouch for the run. Distinguishing this from 1 matters — 1 means "the
       pattern is wrong", 2 means "the question could not be asked".
"""

import argparse
import re
import sys
from pathlib import Path

# Sane band for a hearing transcript of the size in this corpus (36-37 pp,
# ~25 lines/page). Below the floor almost certainly means the pattern is not
# matching speaker labels; above the ceiling means it is matching something it
# should not (every line, or mid-utterance capitals).
MIN_UNITS = 150
MAX_UNITS = 600

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


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Pre-flight unit-count gate for court_transcript extraction."
    )
    parser.add_argument("text_file", type=Path, help="OCR'd/extracted transcript text")
    parser.add_argument(
        "--profile",
        type=Path,
        default=Path(DEFAULT_PROFILE),
        help=f"profile YAML to read boundary_pattern from (default: {DEFAULT_PROFILE})",
    )
    args = parser.parse_args()

    if not args.text_file.is_file():
        print(f"ERROR: no such text file: {args.text_file}", file=sys.stderr)
        return 2

    try:
        text = args.text_file.read_text(encoding="utf-8", errors="replace")
    except OSError as exc:
        print(f"ERROR: cannot read {args.text_file}: {exc}", file=sys.stderr)
        return 2

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

    print(f"text file       : {args.text_file}")
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
