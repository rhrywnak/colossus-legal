#!/usr/bin/env bash
#
# assert-scratch-db.sh — refuse to run the backend against anything but a
# throwaway database.
#
# ## Why this script exists (2026-09-22, CC_TASK_FOR_YOU_v1 L3)
#
# A local run script derived its two scratch URLs with a `sed` that produced an
# EMPTY string. The shell exported the empty value, the backend's own `.env`
# fallback supplied the real one — and `backend/.env` points BOTH
# `DATABASE_URL` and `PIPELINE_DATABASE_URL` at DEV's `colossus_legal_v2`. The
# boot migrator then applied eleven pending migrations to a live database.
#
# Nothing in that chain was broken: dotenvy is documented not to override a set
# variable, an empty variable is not "set" from the process' point of view, and
# every boot of this binary runs migrations against whatever URL it ends up
# with. The missing piece was a REFUSAL. This is that refusal.
#
# ## Usage
#
#   export DATABASE_URL=… PIPELINE_DATABASE_URL=…
#   ./scripts/assert-scratch-db.sh && ./backend/target/debug/colossus-legal-backend
#
# Exit 0 only when BOTH variables are set, non-empty, and name a database whose
# name begins with `colossus_scratch_` or ends in `_proof`, `_scratch` or
# `_main` — the shapes this project's throwaway databases have used. Anything
# else, including `colossus_legal` and `colossus_legal_v2` themselves, is a
# non-zero exit and a sentence naming the variable and the database.
#
# It deliberately does NOT connect to anything: a guard that needs the network
# is a guard somebody skips when the network is slow.

set -u

# The one place the rule is written. A database this pattern accepts is a
# database a human deliberately named as disposable.
readonly SCRATCH_PATTERN='^(colossus_scratch_[a-z0-9_]+|[a-z0-9_]+_(proof|scratch|main))$'

fail() {
    echo "REFUSING TO START: $1" >&2
    echo "  A local boot runs every pending migration against this database." >&2
    echo "  Point it at a throwaway copy (see docs: live-db scratch recipe)." >&2
    exit 1
}

# The database name is the last path segment, with any ?query stripped.
db_name_of() {
    local url="$1"
    url="${url%%\?*}"
    printf '%s' "${url##*/}"
}

check() {
    local name="$1" url="$2"
    [ -n "$url" ] || fail "$name is empty or unset"
    local db
    db="$(db_name_of "$url")"
    [ -n "$db" ] || fail "$name names no database: $url"
    if ! printf '%s' "$db" | grep -Eq "$SCRATCH_PATTERN"; then
        fail "$name points at '$db', which is not a scratch database"
    fi
}

check DATABASE_URL "${DATABASE_URL-}"
check PIPELINE_DATABASE_URL "${PIPELINE_DATABASE_URL-}"

echo "scratch databases confirmed: $(db_name_of "$DATABASE_URL") + $(db_name_of "$PIPELINE_DATABASE_URL")"
