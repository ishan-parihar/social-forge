# Telegram CLI Integration Plan

## Overview

Integrate `tg/` (vysheng/tg — Telegram MTProto CLI client) as a sidecar daemon
into postiz-rust, following the same IPC pattern as the WhatsApp/wacli integration.

### Current State

- `tg/` contains the **vysheng/tg** source tree (C, autotools build)
- Binary is **NOT built** (`tg/bin/` is empty)
- `tg/telegram-daemon` is a Perl script (Debian daemonization wrapper), not useful
- Current `TelegramProvider` (`src/social/telegram.rs`) uses the **Bot API** only
  (HTTP → `api.telegram.org/bot<TOKEN>/...`)
- WhatsApp daemon pattern exists as reference (`WhatsAppDaemon` + `WhatsAppProvider` + MCP tools)

### Why Add CLI-Based Integration?

The Bot API has limitations:
  - Only works as a bot, not as a user account
  - Cannot list dialogs, contacts, or access full message history
  - Requires a BotFather token and setup

The CLI (MTProto) approach provides:
  - User-account capabilities (phone-based auth)
  - Dialog listing, contact management, message history
  - Can interact with any chat the user has access to

**Recommendation**: Keep Bot API as an alternative; add CLI daemon as a parallel
communication channel. The `SocialProvider` trait is designed for this — both
implementations can coexist under different identifiers
(e.g., `"telegram"` → Bot API, `"telegram-cli"` → daemon).

### Architecture

```
postiz-rust (Rust)
  └── TelegramDaemon (src/services/telegram_daemon.rs)
        └── spawns telegram-cli (tg/bin/telegram-cli)
              └── stdin/stdout: text commands → JSON responses
                    └── MTProto → Telegram servers
```

---

## Phase 1: Build Infrastructure

### 1.1 Create `scripts/build-tg.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../tg"

# Install dependencies if needed
# sudo apt-get install libreadline-dev libconfig-dev libssl-dev \
#   lua5.2 liblua5.2-dev libevent-dev libjansson-dev libpython-dev make

./configure
make -j$(nproc)

# Verify binary exists
ls -la bin/telegram-cli
```

**Output**: `tg/bin/telegram-cli`

### 1.2 Verify CLI Pipe Mode Works

Test the command-line pipe mode that we'll use for IPC:
```bash
echo "dialog_list" | tg/bin/telegram-cli --json -R -D -k tg-server.pub
```

Key flags:
- `--json` — Output responses as JSON
- `-R` — Disable readline (pipe-safe)
- `-D` — Disable extraneous output
- `-k tg-server.pub` — Public server key

---

## Phase 2: Telegram Daemon (Rust)

### 2.1 Create `src/services/telegram_daemon.rs`

**Purpose**: Manage `telegram-cli` child process, send commands via stdin, parse JSON from stdout.

Similar to `WhatsAppDaemon` but adapted for telegram-cli's command protocol
(text commands → JSON responses) rather than JSON-RPC.

```rust
pub struct TelegramDaemon {
    process: Mutex<Child>,
    binary_path: PathBuf,
    pubkey_path: PathBuf,
    store_dir: PathBuf,
}
```

#### IPC Protocol

telegram-cli in `--json -R -D` mode:
  - **stdin**: Write text commands, one per line
  - **stdout**: Read JSON responses (one per command, prefixed by `START`/`END` markers)
  - **stderr**: Log messages

Command → Response flow:
1. Write `"command_name arg1 arg2\n"` to stdin
2. Read JSON response from stdout
3. Handle `START`/`END` markers around multi-line JSON

#### Methods

| Method | telegram-cli command | Response |
|---|---|---|
| `auth_status()` | `get_self` | User info JSON or error |
| `send_msg(peer, text)` | `msg <peer> <text>` | Sent message confirmation |
| `list_dialogs(limit)` | `dialog_list [limit]` | Array of dialog objects |
| `contact_list()` | `contact_list` | Array of contact objects |
| `history(peer, limit)` | `history <peer> [limit]` | Array of message objects |
| `search(peer, pattern)` | `search [peer] pattern` | Array of matching messages |
| `user_info(peer)` | `user_info <peer>` | User info JSON |

#### Auth Flow

Unlike wacli's QR-code auth, telegram-cli uses phone-based auth:
1. `send_request("phone_number <number>", None)` — send phone
2. `send_request("code <code>", None)` — send verification code
3. Optional: `send_request("password <2fa>", None)` — send 2FA

The auth state machine needs to be managed:
  - `NeedsPhone` — waiting for phone number
  - `NeedsCode` — waiting for verification code
  - `NeedsPassword` — waiting for 2FA password
  - `Authenticated` — fully connected

#### Lifecycle

```rust
// Start
pub fn start(store_dir: PathBuf) -> Result<Arc<Self>, String>
pub fn start_with_binary(binary: PathBuf, pubkey: PathBuf, store: PathBuf) -> Result<Arc<Self>, String>

// IPC
pub fn send_request(&self, command: &str) -> Result<Value, String>

// Auth
pub fn auth_status(&self) -> Result<Value, String>
pub fn start_auth(&self, phone: &str) -> Result<Value, String>
pub fn submit_code(&self, code: &str) -> Result<Value, String>
pub fn submit_password(&self, password: &str) -> Result<Value, String>

// Messaging
pub fn send_msg(&self, peer: &str, text: &str) -> Result<Value, String>
pub fn list_dialogs(&self, limit: Option<u64>) -> Result<Value, String>
pub fn contact_list(&self) -> Result<Value, String>
pub fn history(&self, peer: &str, limit: Option<u64>) -> Result<Value, String>

// Lifecycle
pub fn is_running(&self) -> bool
pub fn stop(&self) -> Result<(), String>
impl Drop for TelegramDaemon { ... }
```

#### Binary Discovery

```rust
fn which_tg() -> Result<PathBuf, String> {
    let candidates = vec![
        PathBuf::from("./tg/bin/telegram-cli"),
        PathBuf::from("../tg/bin/telegram-cli"),
        // Also check PATH
    ];
    // ... same pattern as which_wacli()
}
```

---

## Phase 3: Telegram CLI SocialProvider

### 3.1 Create `src/social/telegram_cli.rs`

New SocialProvider that uses the daemon instead of Bot API.

```rust
pub struct TelegramCliProvider {
    daemon: Arc<TelegramDaemon>,
    store_dir: PathBuf,
}
```

#### Trait Implementation

| Trait Method | Implementation |
|---|---|
| `identifier()` | `"telegram-cli"` |
| `name()` | `"Telegram (CLI)"` |
| `uses_oauth()` | `false` (phone-based) |
| `one_time_token()` | `true` |
| `max_content_length()` | `4096` |
| `generate_auth_url()` | Returns empty (auth is interactive via phone) |
| `exchange_code()` | Not supported directly (phone auth is multi-step) |
| `publish()` | Calls `daemon.send_msg(access_token, text)` |
| `fetch_page_info()` | Returns error (not applicable) |

#### Auth Token

The access token is a peer identifier (username or phone), stored as the
`access_token` in the AuthToken. The publish method uses this to route messages.

### 3.2 Option: Keep Bot API Provider

Rename the current Bot API provider to `"telegram-bot"` and keep it active
for users who have Bot tokens. The CLI provider becomes `"telegram-cli"`.

---

## Phase 4: MCP Tools

### 4.1 Create `src/mcp/tools_telegram_cli.rs`

| Tool | Description | Handler |
|---|---|---|
| `tg_cli_auth_status` | Check Telegram CLI auth status | `handle_tg_cli_auth_status()` |
| `tg_cli_send_msg` | Send a message via Telegram CLI | `handle_tg_cli_send_msg()` |
| `tg_cli_dialogs` | List Telegram dialogs | `handle_tg_cli_dialogs()` |
| `tg_cli_contacts` | List Telegram contacts | `handle_tg_cli_contacts()` |
| `tg_cli_history` | Get message history for a dialog | `handle_tg_cli_history()` |
| `tg_cli_user_info` | Get info about a user | `handle_tg_cli_user_info()` |

### 4.2 Register in `src/mcp/mod.rs`

```rust
pub mod tools_telegram_cli;

// In PostizMcpServer:
#[tool(description = "Check Telegram CLI authentication status")]
async fn tg_cli_auth_status(...)
#[tool(description = "Send a text message via Telegram CLI")]
async fn tg_cli_send_msg(...)
#[tool(description = "List Telegram dialogs")]
async fn tg_cli_dialogs(...)
#[tool(description = "List Telegram contacts")]
async fn tg_cli_contacts(...)
```

---

## Phase 5: Config & Registry Updates

### 5.1 Config (`src/config.rs`)

```rust
pub telegram_store_dir: Option<String>,  // From TELEGRAM_STORE_DIR
pub telegram_cli_path: Option<String>,    // From TELEGRAM_CLI_PATH (optional override)
```

### 5.2 Registry (`src/social/registry.rs`)

```rust
// Always registered (daemon-based, no OAuth credentials)
providers.insert("telegram-cli", Arc::new(telegram_cli::TelegramCliProvider::new(config)));
```

### 5.3 Module Exports

```rust
// src/social/mod.rs
pub mod telegram_cli;

// src/services/mod.rs
pub mod telegram_daemon;

// src/mcp/mod.rs
pub mod tools_telegram_cli;
```

---

## Phase 6: Testing

### 6.1 Integration Test: `tests/telegram_cli_integration_test.rs`

Following the WhatsApp test pattern:

| Test | What It Verifies |
|---|---|
| `test_tg_provider_metadata` | identifier, name, uses_oauth, scopes |
| `test_tg_provider_registration` | ProviderRegistry includes telegram-cli |
| `test_tg_daemon_binary_not_found` | Graceful error handling |
| `test_tg_daemon_ipc_ping` | Send command, verify response |
| `test_tg_daemon_auth_status` | get_self response when unauthenticated |
| `test_tg_mcp_compilation` | PostizMcpServer includes TG CLI tools |

---

## Files to Create

| File | Purpose |
|---|---|
| `scripts/build-tg.sh` | Build telegram-cli from tg/ source |
| `src/services/telegram_daemon.rs` | Telegram daemon IPC lifecycle |
| `src/social/telegram_cli.rs` | Telegram CLI SocialProvider |
| `src/mcp/tools_telegram_cli.rs` | MCP tools for Telegram CLI |
| `tests/telegram_cli_integration_test.rs` | Integration tests |

## Files to Modify

| File | Change |
|---|---|
| `src/services/mod.rs` | Add `pub mod telegram_daemon;` |
| `src/social/mod.rs` | Add `pub mod telegram_cli;` |
| `src/social/registry.rs` | Register `"telegram-cli"` provider |
| `src/mcp/mod.rs` | Add `pub mod tools_telegram_cli;` + tool methods |
| `src/config.rs` | Add `telegram_store_dir`, `telegram_cli_path` |

## Open Questions

1. **Auth UX**: How should the phone→code→password auth flow work?
   - Option A: Multi-step exchange_code with state machine
   - Option B: Pre-auth via external script, daemon just verifies
   - **Recommendation**: Option B for v1 (assume pre-authenticated), Option A for v2

2. **Bot API coexistence**: Should the CLI provider replace or complement the Bot API?
   - **Recommendation**: Complement. Add as `"telegram-cli"`, keep existing as `"telegram"`.

3. **JSON output parsing**: telegram-cli outputs JSON surrounded by `START`/`END` lines.
   Need robust parsing in the daemon to handle multi-line/async responses.
   - **Recommendation**: Strip `START`/`END` markers, buffer until complete JSON object.

4. **Concurrent requests**: telegram-cli processes one command at a time.
   The Mutex-based locking in the daemon handles this, but long-running commands
   (like `history` with large limits) block other requests.
   - **Recommendation**: Keep Mutex; set reasonable default limits.
