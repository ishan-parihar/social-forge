# Targeting, collisions, and the single-writer rule

Covers three failure modes that are invisible from the happy path:

- **G-08 — SQLite is single-writer.** A second long-lived process on the same
  database double-publishes; a short writer can lose a queued post to
  `SQLITE_BUSY`.
- **G-10 — `--platforms` prefix collisions.** `instagram` used to sweep in
  `instagram-standalone`, staging two drafts for one intent.
- **Draft scope — `disabled=1` integrations.** A draft can be staged against an
  integration that can never publish it.

Run `social-forge audit` to see all three against live state.

---

## 1. Single-writer discipline (G-08)

SQLite admits **one writer at a time** per database file. Social Forge layers
two guards on top of that.

### In-process: `max_connections(1)`

The pool in `src/db/mod.rs` is sized to a single connection. A second connection
in the same process would only serialize on the file lock anyway, and WAL
already lets readers run concurrently with the one writer.

### Cross-process: `busy_timeout = 15s`

Every process — `serve`, `mcp`, and each short-lived CLI invocation — opens its
own pool against the same file, so `max_connections(1)` does nothing across
them. The busy timeout is what makes a concurrent writer **wait** instead of
failing immediately with `SQLITE_BUSY`.

It was `5s`, which the publish scheduler could exceed: a slow platform write
holding the write lock long enough would cause the next queued post to be
dropped. `BUSY_TIMEOUT` is now `15s` (`src/db/mod.rs`), sized above the
worst-case single publish transaction (network write + post + analytics).

### One long-lived process per database

`serve` and `mcp` both run the publish scheduler. Two of them on the same
`DATABASE_URL` means two schedulers, and every queued post publishes twice.

Both now claim an exclusive lock file next to the database before opening the
pool:

```
<db-dir>/.social-forge-writer.lock   # contains the owning PID
```

The second process fails at boot with a clear message instead of silently
double-publishing:

```
Another social-forge process already owns the writer lock for this database
(data/.social-forge-writer.lock). SQLite is single-writer: run ONE long-lived
process per DATABASE_URL, or delete the lock if that process is gone.
```

**Short CLI reads do not take the lock** — `providers`, `posts list`, and
`audit` only open the pool. Only `serve` / `mcp` claim it, because only they
run a scheduler.

> ponytail: `O_EXCL` + PID in the file, released on process exit. A
> `kill -9` leaves the lock behind; delete it by hand. Add a PID-liveness probe
> if stale locks actually show up in practice.

### Checking it

```bash
social-forge audit
```

reports `writer_discipline` with `journal_mode` and `busy_timeout_ms` read back
from the open pool, and marks `fail` if the timeout is under 1s.

---

## 2. `--platforms` prefix collisions (G-10)

Two registered providers are a strict prefix of another:

| `--platforms` value | Also used to match | Consequence (old behaviour) |
|---|---|---|
| `instagram` | `instagram-standalone` | two drafts staged |
| `linkedin` | `linkedin-page` | two drafts staged |

The old filter was `identifier == p || identifier.starts_with("{p}-")`, so
`--platforms instagram` selected both providers and staged two drafts. The
stage output looked fine — it just had twice the posts you asked for.

**Exact identifier match now wins.** The prefix fallback applies only to names
with no exact match, which keeps `telegram` → `telegram-bot` /
`telegram-user` working without letting a real provider be shadowed by its own
`-suffix` variant.

```bash
# ONLY the `instagram` integration — one draft
social-forge post "hello" --platforms instagram

# Explicitly BOTH, two drafts — as intended
social-forge post "hello" --platforms instagram,instagram-standalone

# No such connected integration → the base name is a no-op, not a wildcard
social-forge post "hello" --platforms linkedin
```

### Precise targeting: `--integrations`

When a name is ambiguous or you want one specific account, use integration UUIDs.
`--integrations` bypasses platform-name resolution entirely and is mutually
exclusive with `--platforms`:

```bash
social-forge providers          # list connected integrations + internal_ids
social-forge stage "hello" --integrations 6f1c...-...,9a02...-...
```

### Checking it

```bash
social-forge audit
```

`platform_collisions` lists each pair with which side is connected, and
`both_connected: true` when a bare `--platforms X` would have fanned out.
`findings[].id == "platform_collisions"` is `warn` only in that case.

---

## 3. Draft scope: `disabled=1` integrations

An integration row with `disabled = 1` is connected but not usable for
publishing. A draft staged against one is accepted and then never publishes —
the failure is silent from the CLI's point of view, because staging only
validates that the integration *exists*.

```bash
social-forge audit
```

`disabled_integrations` lists each one with its UUID and the impact. When the
count is non-zero, `findings[].id == "disabled_integrations"` is `warn`:

```
disabled_integrations: 26 integration(s) disabled=1 — drafts against them
will not publish
```

Re-enable or remove them before staging, or target enabled accounts explicitly
via `--integrations`.

`social-forge providers` still reports the `active` / `disabled` counts; `audit`
is the command that names the rows and explains the consequence.

---

## 4. Audit output shape

`social-forge audit` is read-only and prints JSON (AXI conventions: pre-computed
aggregates, definitive empty states, no interactive prompts).

```jsonc
{
  "status": "warn",                    // "clean" when every finding is ok
  "findings": [                        // one per check, stable ids
    { "id": "disabled_integrations", "status": "warn", "detail": "..." },
    { "id": "platform_collisions",   "status": "ok",   "detail": "..." },
    { "id": "writer_discipline",     "status": "ok",   "detail": "..." }
  ],
  "counts": {
    "integrations": 26,
    "enabled": 0,
    "disabled": 26,
    "colliding_pairs_both_connected": 0
  },
  "disabled_integrations": [ /* provider, name, integration_id, impact */ ],
  "enabled_providers": [ /* ... */ ],
  "platform_collisions":  [ /* platform, also_matches, connected, both_connected, guidance */ ],
  "writer_discipline": {
    "max_connections": 1,
    "journal_mode": "wal",
    "busy_timeout_ms": 15000,
    "note": "SQLite is single-writer. Run one long-lived process (serve or mcp) per database; ..."
  },
  "help": [ "..." ]
}
```

`status` is `clean` only when all three findings are `ok`, so it is safe to
gate a deploy or a cron entry on it.
