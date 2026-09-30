# Vendor Pins — social-forge

Phase **B0** of [`PLAN_PARITY_DEPTH_SINGLEUSER_v25.md`](./PLAN_PARITY_DEPTH_SINGLEUSER_v25.md) §9.

Every ported / vendored / fork-pinned source is recorded here with its upstream URL and
its pinned rev, and is verified at build time by [`scripts/vendor-pull.sh`](../../scripts/vendor-pull.sh).
The script runs **before `cargo check`** in CI (`.github/workflows/ci.yml`) and as a
prerequisite of the `build` and `check` targets in the `Makefile`.

Nothing in this file is bumped automatically. A newer upstream release is reported as a
`FOLLOW-UP` line and left alone (see §5).

## 1. How it is enforced

```bash
scripts/vendor-pull.sh            # same as --check
scripts/vendor-pull.sh --check    # verify pins, no mutation  → 0 green, 1 drift (+ diff)
scripts/vendor-pull.sh --update   # cargo fetch + rewrite the generated block in §6
scripts/vendor-pull.sh --print    # dump the resolved pin table
```

| Env var | Effect |
|---|---|
| `VENDOR_PULL_OFFLINE=1` | skip every network call; local invariants only |
| `VENDOR_PULL_STRICT=1` | treat upstream `FOLLOW-UP` findings as fatal too |

Two severities, on purpose:

- **Violation (always fatal).** Local drift: a manifest requirement that no longer matches the
  record, a `Cargo.lock` that resolves a pinned crate to a different version or checksum, a
  vendored tree whose contents changed, a missing ported file, or a stale `lock_digest`. A
  unified diff is printed for each. This is the "stale build" case.
- **Follow-up (reported; fatal only under `VENDOR_PULL_STRICT=1`).** Upstream freshness: a newer
  published release, a yanked pin, or an unreachable crates.io. §7 below lists the ones that are
  currently known and accepted, so CI runs without `STRICT`. **When §7 is empty, set
  `VENDOR_PULL_STRICT: "true"` in `.github/workflows/ci.yml` to make the gate permanent.**

`--update` is the one mode that *must* reach the network: if `cargo fetch` fails it exits
non-zero rather than recording a state it could not confirm. Run it with network access, or
with `VENDOR_PULL_OFFLINE=1` to record local state only.

Exit codes: `0` invariants hold · `1` invariant violation (a unified diff is printed) ·
`2` usage error.

**Why the lock is not the record.** `Cargo.lock` is gitignored (`.gitignore:3`), so it is
machine-local and cannot be the committed contract. This file is the contract: the
`requirement` and `resolved` columns below are the assertion, and `--check` fails loud if
the manifest or the local lock disagrees. `--check` therefore degrades to a `SKIP` notice
(not a pass, not a failure) on a fresh clone with no `Cargo.lock`, and prints a
`FOLLOW-UP` telling you to run `--update` to make resolution deterministic.

## 2. Pin table (machine-readable)

`scripts/vendor-pull.sh` parses this table. Edit it by hand when you intentionally move a
pin; the `<!-- vendor-pull:begin/end -->` block in §6 is the only part the script writes.

<!-- vendor-pins:table:begin -->
| id | kind | requirement | resolved | checksum | upstream | paths |
|---|---|---|---|---|---|---|
| `wa-rs` | registry | `0.2.0` | `0.2.0` | `0fecb468bdfe1e7d4c06a1bd12908c66edaca59024862cb64757ad11c3b948b1` | https://github.com/homunbot/wa-rs | - |
| `wa-rs-sqlite-storage` | registry | `0.2.0` | `0.2.0` | `006adc8ec15093946ae4c7b07d3e232c499116d469cef0da46782780df6a132c` | https://github.com/homunbot/wa-rs | - |
| `wreq` | registry | `=6.0.0-rc.23` | `6.0.0-rc.23` | `b7cd527a3265faac3ac44097ccccc76d901230b9bd2866e62d1c18dc348d7aa8` | https://github.com/0x676e67/wreq | - |
| `wreq-util` | registry | `=3.0.0-rc.10` | `3.0.0-rc.10` | `6c6bbe24d28beb9ceb58b514bd6a613c759d3b706f768b9d2950d5d35b543c04` | https://github.com/0x676e67/wreq-util | - |
| `toon-format` | registry | `0.4` | `0.4.6` | `af1dae994fe9adfb44bdc74fc17546651604a59af32136256515bc1218850832` | https://github.com/toon-format/toon-rust | - |
| `glass_pumpkin` | registry | `^1.6` | `1.10.0` | `f09b0eef9941bda7cc263c23c3977d437a9aa19abb6a58ed0234d145d9c024bc` | https://github.com/mikelodder7/glass_pumpkin | - |
| `grammers-crypto` | registry | `0.7` | `0.7.0` | `17c75ce8d715d407a5767a94b5fd7b210106ec5c29c7be8ff4dfdca58de4dcf6` | https://codeberg.org/Lonami/grammers | - |
| `core2` | vendor | `patch.crates-io` | `0.4.0` | `047d324a14bf6e1b84770b8f9d4d69990d75ae7af9b2b28b83f702e5718c6b31` | https://github.com/bbqsrc/core2 | - |
| `gog-google-port` | port | `n/a` | `unpinned` | `n/a` | `UNRESOLVED` | `src/social/google.rs;src/social/calendar.rs;src/social/drive.rs;src/social/google_my_business.rs` |
<!-- vendor-pins:table:end -->

Columns: `kind` is `registry` (crates.io — `resolved` + `checksum` are the rev), `vendor`
(a committed path tree — `checksum` is a sha256 over the sorted contents of `vendor/<id>`,
machine-independent) or `port` (source-derived code, nothing fetched at build time; `paths`
lists the files that must still exist). `requirement` is compared against the manifests;
for a transitive pin with no manifest entry it is documentation only.

## 3. Per-source notes

**`wa-rs` / `wa-rs-sqlite-storage` 0.2.0** — the in-tree WhatsApp Web client that replaced
the Go `wacli` sidecar (`Cargo.toml:94-95`). Upstream is a single repo, so the two crates
share one rev stream; `wa-rs 0.2.0` also pulls `wa-rs-core`, `wa-rs-proto`,
`wa-rs-binary`, `wa-rs-tokio-transport` and `wa-rs-ureq-http` at `^0.2.0`. Last crates.io
release 2026-02-17; both crates are at their newest version.

**`wreq` =6.0.0-rc.23 / `wreq-util` =3.0.0-rc.10** — the TLS-fingerprinting `reqwest` fork
used for the X/Twitter GraphQL API (`Cargo.toml:74-80`). The `=` prefix is deliberate:
`reqwest` gets blocked without JA3/JA4 emulation. **AGENTS.md §9.1 forbids upgrading to
`rc.29+`** — breaking API changes landed in `rc.24`…`rc.31`. See §5.

**`toon-format` 0.4.6** — direct dependency for AXI-compliant TOON output
(`Cargo.toml:147`); the `crates/toon-helper` path crate that used to wrap it is gone,
its three functions now live in `src/cli/run.rs`. Pinned at the `0.4` minor; `0.5.0` exists
upstream and is a semver-major bump.

**`glass_pumpkin` 1.10.0 / `grammers-crypto` 0.7.0** — the Telegram MTProto transport
(`Cargo.toml:87-91`). These are transitive, not direct, dependencies; they are recorded here
because they are the *reason* `vendor/core2` exists. See §4.

**`core2` 0.4.0 (`vendor/core2`)** — local stub, see §4.

**`gog-google-port`** — the Google Workspace providers ported from the `gog` CLI's patterns
(`src/social/google.rs`, `calendar.rs`, `drive.rs`, `google_my_business.rs`). This is a
**port, not a vendored tree**: nothing is fetched at build time, so the script verifies only
that the recorded files still exist. The upstream `gog` repo is not recorded because it could
not be resolved to a canonical public URL — see follow-up **F-4**.

## 4. `vendor/core2` — temporary stub, and when to delete it

`core2 0.4.0` is **yanked from crates.io — every published version of `core2` is yanked**
(verified 2026-09-30: `0.0.0`, `0.3.0-alpha.1`, `0.3.0`…`0.3.3`, `0.4.0`). It cannot be
resolved from crates.io at all.

`glass_pumpkin` depended on `core2 ^0.4` through `1.9.0`; the dependency was **dropped in
`1.9.1`**. Current state of this tree:

```
Cargo.lock: glass_pumpkin 1.10.0   → no core2 dependency
Cargo.lock: [[patch.unused]] core2 0.4.0
```

So the stub is currently **inert**: nothing in the resolved graph needs `core2`, and Cargo
reports the `[patch.crates-io]` entry as unused. The stub is retained as the **safety net**
for the range where it *is* required: `grammers-crypto 0.7.0` asks for `glass_pumpkin ^1.6.0`,
so any re-resolve that lands on `glass_pumpkin ≤ 1.9.0` re-introduces `core2 ^0.4` — an
unresolvable yanked crate — and the build breaks without the patch. This is the whole point
of the stub; do not delete it casually (AGENTS.md §9.2).

`vendor/core2/src/lib.rs` re-exports `std::error` for `core2::error`, which is what
`glass_pumpkin` actually uses. In `std` contexts those types are identical.

**Removal condition — remove the stub only when both hold:**

1. The resolved `glass_pumpkin` is `≥ 1.9.1` (first release with no `core2` dependency), **and**
2. `grammers-crypto` is on `≥ 0.10`, which requires `glass_pumpkin ^2.0.0-rc0` (also `core2`-free),
   so re-resolution can never fall back into the `1.6`–`1.9.0` band.

`scripts/vendor-pull.sh --check` reports the live state as `core2_state:` in §6
(`stub.active` when something still depends on `core2`, `stub.idle` when Cargo marks the patch
unused) and warns loudly when it flips, so the removal condition cannot be reached silently.

## 5. RC → stable: flagged, never auto-bumped

`wreq` and `wreq-util` are on release candidates. `--check` reports any newer published
version as a `FOLLOW-UP` and exits `0`; it never edits `Cargo.toml`. A bump is a deliberate,
separately-verified change (it is a real API migration, see AGENTS.md §9.1), not a side effect
of a feature phase (plan §9).

| Pin | Newer upstream at record time | Note |
|---|---|---|
| `wreq` =6.0.0-rc.23 | `6.0.0-rc.24` … `6.0.0-rc.31` | No stable release exists. **rc.29+ is breaking — do not bump.** |
| `wreq-util` =3.0.0-rc.10 | `3.0.0-rc.11` … `3.0.0-rc.14` | No stable release exists. |
| `toon-format` 0.4.6 | `0.5.0` | Semver-major; breaks the `encode_default` call in `src/cli/run.rs`. |
| `grammers-crypto` 0.7.0 | `0.10.0` | Prerequisite for removing `vendor/core2` (§4). |
| `glass_pumpkin` 1.10.0 | `1.9.1` newest non-yanked; `2.0.0-rc0` available | **1.10.0 is itself yanked** — see F-1. |

## 6. Generated block — do not hand-edit

Written by `scripts/vendor-pull.sh --update`.

<!-- vendor-pull:begin -->
lock_digest: 0b8088c4e77f0c253b255ca99ffc969e76c3caa78bbea03baf5431d25d9b2caa
core2_state: stub.idle
last_checked: 2026-09-30T07:19:09Z
<!-- vendor-pull:end -->

## 7. Follow-ups

- **F-1** — `glass_pumpkin 1.10.0` is **yanked** on crates.io. The lock keeps it (cargo
  permits a locked yanked version) but a fresh resolve cannot select it, and it is what
  currently makes the `core2` patch unused. Needs a decision: accept the pin, or move to
  `1.9.1` / `2.0.0-rc0` in a dedicated dependency change. Not done here — no version bumps
  in a feature phase.
- **F-2** — `wreq` / `wreq-util` have 8 / 4 newer RCs with no stable release. Track upstream
  `0x676e67/wreq`; migrate only via a dedicated phase with `cargo check` + `cargo test --lib`
  + live X GraphQL verification.
- **F-3** — `grammers-crypto 0.10.0` is the gate for deleting `vendor/core2` (§4). Moving
  `grammers-* 0.7 → 0.10` is a Telegram transport migration, not a pin bump.
- **F-4** — Record the canonical upstream URL + commit for the `gog` Google-Workspace port
  (`src/social/google*.rs`) the next time those files are touched. `gog` did not resolve to an
  unambiguous public repository, so the pin is recorded as `UNRESOLVED` rather than guessed.
- **F-5** — `Cargo.lock` is gitignored, so `--check` cannot verify resolution on a fresh
  clone. Consider tracking `Cargo.lock` (it is a binary-only application, not a library) so
  the digest becomes a hard gate everywhere. Currently a `SKIP` + `FOLLOW-UP`.
