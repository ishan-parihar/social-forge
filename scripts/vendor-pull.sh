#!/usr/bin/env bash
#
# vendor-pull.sh — verify vendored / pinned upstream sources.
#
# Phase B0 of docs/planning/PLAN_PARITY_DEPTH_SINGLEUSER_v25.md §9.
#
# The pin contract is docs/planning/VENDOR_PINS.md. This script never edits
# Cargo.toml: a newer upstream release is reported as a FOLLOW-UP and left
# alone (AGENTS.md §9.1 — RC→stable is a deliberate migration, never a side
# effect of a feature phase).
#
#   --check    verify pins, no mutation          → 0 green · 1 drift (+ diff)
#   --update   cargo fetch + rewrite the generated block in VENDOR_PINS.md §6
#   --print    dump the pin table
#
#   VENDOR_PULL_OFFLINE=1   skip every network call
#   VENDOR_PULL_STRICT=1    treat network/FOLLOW-UP findings as fatal
#
# Exit: 0 invariants hold · 1 invariant violation (unified diff printed) · 2 usage.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PINS_DOC="$REPO_ROOT/docs/planning/VENDOR_PINS.md"
CARGO_TOML="$REPO_ROOT/Cargo.toml"
CARGO_LOCK="$REPO_ROOT/Cargo.lock"
VENDOR_DIR="$REPO_ROOT/vendor"
GEN_BEGIN='<!-- vendor-pull:begin -->'
GEN_END='<!-- vendor-pull:end -->'

OFFLINE="${VENDOR_PULL_OFFLINE:-0}"
STRICT="${VENDOR_PULL_STRICT:-0}"
CRATES_IO_UA="social-forge-vendor-pull/1.0 (build-time pin verification)"

FAILURES=0
FINDINGS=0

log()  { printf '[vendor-pull] %s\n' "$*"; }
warn() { printf '[vendor-pull] WARN: %s\n' "$*" >&2; }
note() { printf '[vendor-pull] FOLLOW-UP: %s\n' "$*"; }

# Violation: print a unified diff so the drift is readable, keep going so one
# run reports every problem instead of only the first.
fail_diff() {
    local what="$1" expected="$2" actual="$3" te ta
    printf '\n[vendor-pull] FAIL: %s\n' "$what" >&2
    # two temp files, never `-`: diff reading stdin would swallow the caller's
    # pin-table pipe and hide every remaining pin.
    te="$(mktemp)"; ta="$(mktemp)"
    printf '%s\n' "$expected" >"$te"
    printf '%s\n' "$actual"   >"$ta"
    diff -u --label 'recorded in VENDOR_PINS.md' --label 'actual' "$te" "$ta" >&2 || true
    rm -f "$te" "$ta"
    FAILURES=$((FAILURES + 1))
}

die() { printf '[vendor-pull] ERROR: %s\n' "$*" >&2; exit 2; }

usage() {
    sed -n '2,20p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
    exit "${1:-0}"
}

# ── pin table ────────────────────────────────────────────────────────────────
# Emits one TSV line per pin: id \t kind \t requirement \t resolved \t checksum \t upstream \t paths
parse_pins() {
    [[ -f "$PINS_DOC" ]] || die "pin record not found: $PINS_DOC"
    awk -F'|' '
        /<!-- vendor-pins:table:begin -->/ { intbl = 1; next }
        /<!-- vendor-pins:table:end -->/   { intbl = 0 }
        !intbl || !/^\|/ { next }
        {
            n = 0
            for (i = 2; i <= 8; i++) {
                f = $i
                gsub(/^[ \t]+|[ \t]+$/, "", f)
                gsub(/`/, "", f)
                col[++n] = f
            }
            if (col[1] == "id" || col[1] == "" || col[1] ~ /^-+$/) { n = 0; next }
            print col[1] "\t" col[2] "\t" col[3] "\t" col[4] "\t" col[5] "\t" col[6] "\t" (n >= 7 ? col[7] : "-")
            n = 0
        }
    ' "$PINS_DOC"
}

# ── Cargo.lock / Cargo.toml readers ──────────────────────────────────────────
# Version or checksum of a [[package]] entry ("" when the crate is absent).
lock_field() {
    local crate="$1" field="$2"
    [[ -f "$CARGO_LOCK" ]] || return 0
    awk -v want="$crate" -v field="$2" '
        # `done` guard: awk `exit` re-enters END, so flush() must be idempotent
        function flush() { if (!done && pkg == want) { done = 1; print val } }
        /^$/                               { flush() }
        /^\[\[package\]\]/                 { flush(); inpkg = 1; pkg = ""; val = ""; next }
        inpkg && $1 == "name" && $2 == "=" { pkg = $3; gsub(/"/, "", pkg); next }
        inpkg && $1 == field && $2 == "=" { val = $3; gsub(/"/, "", val) }
        END { flush() }
    ' "$CARGO_LOCK"
}

# Does any package in the resolved graph still pull this crate? (core2 watch)
lock_dependents() {
    local crate="$1"
    [[ -f "$CARGO_LOCK" ]] || return 0
    grep -cE '^[[:space:]]+"'"$crate"'",?$' "$CARGO_LOCK" || true
}

# Version requirement from any manifest in the workspace. "" for a transitive
# dep (no manifest declares it) — the lock check still applies to those.
manifest_req() {
    local crate="$1" m
    for m in "$CARGO_TOML" "$REPO_ROOT"/crates/*/Cargo.toml; do
        [[ -f "$m" ]] || continue
        # slurp the manifest into awk's stdin: a bare getline must not eat the
        # caller's stdin (the pin-table loop feeds us through a pipe)
        local v
        v="$(awk -v want="$crate" '
            $0 ~ "^[[:space:]]*" want "[[:space:]]*=[[:space:]]*\"" {
                line = $0; sub(/^[^=]*=[[:space:]]*/, "", line); gsub(/["[:space:]]/, "", line); print line; exit
            }
            $0 ~ "^[[:space:]]*" want "[[:space:]]*=[[:space:]]*\\{" {
                buf = $0
                while ((getline nl) > 0) { buf = buf " " nl; if (nl ~ /\}/) break }
                if (match(buf, /version[[:space:]]*=[[:space:]]*"[^"]*"/)) {
                    v = substr(buf, RSTART, RLENGTH); sub(/.*=[[:space:]]*"/, "", v); sub(/"$/, "", v); print v; exit
                }
            }
        ' <"$m")"
        [[ -n "$v" ]] && { printf '%s' "$v"; return 0; }
    done
    return 0
}

sha256_stdin() { sha256sum | cut -d' ' -f1; }

# sha256 over vendor/<id> in sorted path order — a machine-independent rev for
# a vendored tree, unlike a whole-file Cargo.lock digest. No `xargs`: GNU xargs
# runs `cat` even with empty input, which would block on (and swallow) the
# caller's stdin.
vendor_digest() {
    local dir="$1" f
    { find "$dir" -type f -print0 | LC_ALL=C sort -z | while IFS= read -r -d '' f; do cat "$f"; done; } \
        | sha256_stdin
}

# ── crates.io (advisory) ─────────────────────────────────────────────────────
# One request per crate. The `versions` array is the only reliable source: the
# crate-level `newest_version` field is stale for wreq (reports 0.16.1 while
# 6.0.0-rc.31 is published), so never read `crate.newest_version` here.
# Emits "<newest-published>\t<pinned-yanked-or-unknown>"; empty output = unreachable.
crates_io() { # $1 crate $2 pinned version
    [[ "$OFFLINE" == "1" ]] && return 0
    command -v curl >/dev/null 2>&1 || return 0
    command -v python3 >/dev/null 2>&1 || return 0
    curl -sS --max-time 10 -H "User-Agent: $CRATES_IO_UA" \
        "https://crates.io/api/v1/crates/$1/versions" 2>/dev/null \
    | python3 -c '
import json, sys
try:
    versions = json.load(sys.stdin)["versions"]
except Exception:
    raise SystemExit(1)
pinned = sys.argv[1]
yanked = ""
for v in versions:
    if v["num"] == pinned:
        yanked = "true" if v["yanked"] else "false"
        break
print(versions[0]["num"], yanked, sep="\t")
' "$2" 2>/dev/null || true
}

finding() {
    FINDINGS=$((FINDINGS + 1))
    if [[ "$STRICT" == "1" ]]; then
        printf '\n[vendor-pull] FAIL (strict): %s\n' "$*" >&2
        FAILURES=$((FAILURES + 1))
    else
        note "$*"
    fi
    return 0
}

# ── modes ────────────────────────────────────────────────────────────────────
MODE="--check"
case "${1:---check}" in
    --check|--update|--print) MODE="$1" ;;
    -h|--help) usage 0 ;;
    *) die "unknown argument: $1 (expected --check | --update | --print)" ;;
esac

[[ -f "$CARGO_TOML" ]] || die "Cargo.toml not found at $CARGO_TOML"

if [[ "$MODE" == "--print" ]]; then
    printf 'id\tkind\trequirement\tresolved\tupstream\n'
    while IFS=$'\t' read -r id kind req res sum up paths; do
        printf '%s\t%s\t%s\t%s\t%s\n' "$id" "$kind" "$req" "$res" "$up"
    done < <(parse_pins)
    exit 0
fi

# ── --update: refetch, then rewrite the generated block ──────────────────────
if [[ "$MODE" == "--update" ]]; then
    if [[ "$OFFLINE" == "1" ]]; then
        warn "VENDOR_PULL_OFFLINE=1 — skipping cargo fetch, only local state is recorded"
    else
        log "cargo fetch (refreshes the registry cache; does not change versions)"
        if ! (cd "$REPO_ROOT" && cargo fetch 2>&1 | sed 's/^/[vendor-pull]   /'); then
            die "cargo fetch failed — pinned sources could not be fetched.
Re-run with network access, or set VENDOR_PULL_OFFLINE=1 to record local state only.
Stale builds must not pass silently; this is a hard failure on purpose."
        fi
    fi

    # Contract digest: sha256 over the canonical pin lines as recorded here.
    contract="$(parse_pins | awk -F'\t' '$2 != "port" { print $1 "|" $2 "|" $3 "|" $4 "|" $5 }')"
    digest="$(printf '%s\n' "$contract" | sha256_stdin)"

    if [[ -f "$CARGO_LOCK" ]] && [[ "$(lock_dependents core2)" != "0" ]]; then
        core2_state="stub.active"
    else
        core2_state="stub.idle"
    fi

    ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    tmp="$(mktemp)"
    awk -v b="$GEN_BEGIN" -v e="$GEN_END" -v d="$digest" -v s="$core2_state" -v t="$ts" '
        $0 == b { print; print "lock_digest: " d; print "core2_state: " s; print "last_checked: " t; skip = 1; next }
        $0 == e { skip = 0; print; next }
        !skip   { print }
    ' "$PINS_DOC" >"$tmp" && mv "$tmp" "$PINS_DOC"
    log "recorded lock_digest=$digest core2_state=$core2_state at $ts"
    exit 0
fi

# ── --check ─────────────────────────────────────────────────────────────────
LOCK_PRESENT=0
[[ -f "$CARGO_LOCK" ]] && LOCK_PRESENT=1

log "verifying pins against $(basename "${PINS_DOC#"$REPO_ROOT"/}")"
if [[ "$LOCK_PRESENT" == "1" ]]; then
    log "Cargo.lock present (gitignored — machine-local, checked against the recorded pins)"
else
    finding "no Cargo.lock — lock-derived checks SKIP. Resolution is unpinned until you run --update."
fi
[[ "$OFFLINE" == "1" ]] && warn "VENDOR_PULL_OFFLINE=1 — upstream checks skipped"

contract="$(parse_pins | awk -F'\t' '$2 != "port" { print $1 "|" $2 "|" $3 "|" $4 "|" $5 }')"
observed_contract="$(printf '%s\n' "$contract" | sha256_stdin)"
recorded_digest="$(awk -v b="$GEN_BEGIN" -v e="$GEN_END" '
    $0 == b { inb = 1; next } $0 == e { inb = 0 }
    inb && /^lock_digest:/ { sub(/^lock_digest:[[:space:]]*/, ""); print; exit }
' "$PINS_DOC")"
recorded_core2="$(awk -v b="$GEN_BEGIN" -v e="$GEN_END" '
    $0 == b { inb = 1; next } $0 == e { inb = 0 }
    inb && /^core2_state:/ { sub(/^core2_state:[[:space:]]*/, ""); print; exit }
' "$PINS_DOC")"

while IFS=$'\t' read -r id kind req res sum up paths; do
    case "$kind" in
        registry)
            # 1. manifest requirement — committed, always enforced
            mreq="$(manifest_req "$id")"
            if [[ -n "$mreq" ]]; then
                [[ "$mreq" == "$req" ]] || fail_diff "$id: Cargo.toml requirement" "$req" "$mreq"
            else
                # transitive dep: no manifest declares it, the lock check carries it
                warn "$id is transitive (no manifest entry) — requirement '$req' is documentation only"
            fi

            # 2. lock resolution + content hash — fatal when a lock exists
            if [[ "$LOCK_PRESENT" == "1" ]]; then
                lres="$(lock_field "$id" version)"
                lsum="$(lock_field "$id" checksum)"
                if [[ -z "$lres" ]]; then
                    fail_diff "$id: missing from Cargo.lock" "$res" "<absent>"
                else
                    if [[ "$lres" != "$res" ]]; then
                        fail_diff "$id: resolved in Cargo.lock" "$res" "$lres"
                    fi
                    if [[ -n "$sum" && "$lsum" != "$sum" ]]; then
                        fail_diff "$id: Cargo.lock checksum" "$sum" "$lsum"
                    fi
                fi
            fi
            ;;

        vendor)
            vdir="$VENDOR_DIR/$id"
            if [[ ! -d "$vdir" ]]; then
                fail_diff "$id: vendored tree missing" "$vdir" "<absent>"
                continue
            fi
            vres="$(awk -F'"' '/^version[[:space:]]*=/ { print $2; exit }' "$vdir/Cargo.toml" 2>/dev/null || true)"
            if [[ "$vres" != "$res" ]]; then
                fail_diff "$id: vendored version" "$res" "${vres:-<absent>}"
            fi
            vdig="$(vendor_digest "$vdir")"
            if [[ "$vdig" != "$sum" ]]; then
                fail_diff "$id: vendored content digest" "$sum" "$vdig"
            fi
            # the patch entry is what makes the stub load-bearing
            if ! grep -qE "^[[:space:]]*${id}[[:space:]]*=" "$CARGO_TOML"; then
                fail_diff "$id: patch entry missing from Cargo.toml" "an entry for ${id} in [patch.crates-io]" "<absent>"
            fi
            ;;

        port)
            IFS=';' read -r -a pf <<<"$paths"
            for f in "${pf[@]}"; do
                if [[ "$f" == "-" || -z "$f" ]]; then continue; fi
                if [[ ! -f "$REPO_ROOT/$f" ]]; then
                    fail_diff "$id: ported file missing" "$f" "<absent>"
                fi
            done
            if [[ "$res" == "unpinned" ]]; then
                warn "$id: source rev is $res — see follow-up F-4 in VENDOR_PINS.md"
            fi
            ;;

        *) fail_diff "$id: unknown pin kind" "registry|vendor|port" "$kind" ;;
    esac
done < <(parse_pins)

# 3. contract digest — guards hand-edits of the pin table
if [[ -n "$recorded_digest" ]]; then
    [[ "$observed_contract" == "$recorded_digest" ]] \
        || fail_diff "VENDOR_PINS.md §6 lock_digest is stale (re-run --update)" "$recorded_digest" "$observed_contract"
else
    fail_diff "VENDOR_PINS.md §6 lock_digest missing" "a recorded digest" "<absent>"
fi

# 4. core2 stub state — removal condition must not be reached silently (§4)
if [[ "$LOCK_PRESENT" == "1" ]]; then
    if [[ "$(lock_dependents core2)" != "0" ]]; then
        core2_state="stub.active"
    else
        core2_state="stub.idle"
    fi
    if [[ -n "$recorded_core2" && "$core2_state" != "$recorded_core2" ]]; then
        finding "core2_state changed ${recorded_core2} → ${core2_state}. Re-read VENDOR_PINS.md §4 (removal condition) before touching vendor/core2."
    fi
    log "core2_state=$core2_state (recorded ${recorded_core2:-<absent>})"
fi

# 5. upstream advisories — never mutate, always report
if [[ "$OFFLINE" != "1" ]]; then
    while IFS=$'\t' read -r id kind req res sum up paths; do
        [[ "$kind" == "registry" ]] || continue
        out="$(crates_io "$id" "$res")"
        if [[ -z "$out" ]]; then
            finding "$id: crates.io unreachable — upstream freshness unverified this run"
            continue
        fi
        newest="${out%%	*}"; yk="${out##*	}"
        if [[ "$newest" != "$res" ]]; then
            if [[ "$res" == *-* && "$newest" != *-* ]]; then
                finding "$id $res is a release candidate; $newest is the first stable release. NOT auto-bumped (VENDOR_PINS.md §5) — migrate via a dedicated phase."
            elif [[ "$res" == *-* ]]; then
                finding "$id: newer pre-release $newest (pinned $res). NOT auto-bumped (VENDOR_PINS.md §5); $id is a pin-sensitive fork."
            else
                finding "$id: newer upstream $newest (pinned $res). NOT auto-bumped (VENDOR_PINS.md §5)."
            fi
        fi
        if [[ "$yk" == "true" ]]; then
            finding "$id $res is YANKED upstream. cargo keeps a locked yanked version but a fresh resolve cannot select it — see VENDOR_PINS.md §7."
        fi
    done < <(parse_pins)
fi

# ── report ───────────────────────────────────────────────────────────────────
if [[ "$FAILURES" -gt 0 ]]; then
    printf '\n[vendor-pull] %d violation(s) — a stale/mis-pinned build must not pass silently.\n' "$FAILURES" >&2
    exit 1
fi
log "OK — $(parse_pins | wc -l | tr -d ' ') pins verified, ${FINDINGS} follow-up(s), 0 violations"
exit 0
