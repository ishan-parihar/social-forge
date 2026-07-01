# Social Forge — Upgrade Plan (Post-Audit)

Generated from comprehensive production audit. Covers critical bug fixes, missing workflow coverage, and documentation corrections.

---

## Phase 1: Critical Bug Fixes (P0) — Blocking Production

### 1.1 Fix `mcp-call` Panic
**File**: `src/cli/mod.rs`
**Issue**: `McpCall.json` field conflicts with global `--json` flag, causing clap panic.
**Fix**: Rename `json` field to `args` in `McpCall` variant. Update `run.rs` dispatcher.
**Test**: `social-forge mcp-call integrations_list '{}'` should not panic.

### 1.2 Register All MCP Tools in Bridge
**File**: `src/cli/mcp_bridge.rs`
**Issue**: Only 35 of 273 MCP tools registered. 238 tools unreachable via `mcp-call`.
**Fix**: Auto-generate bridge from MCP tool definitions, or manually register all platform tools.
**Test**: `social-forge mcp-tools` should list 273+ tools. `social-forge mcp-call fb_create_post` should not return "Unknown tool".

### 1.3 Fix `stage --platforms` Flag
**Files**: `src/cli/mod.rs`, `src/cli/run.rs`
**Issue**: SKILL.md documents `--platforms` but Stage command only accepts `--integrations`.
**Fix**: Add `--platforms` flag to Stage that resolves platform names to integration UUIDs automatically.
**Test**: `social-forge stage "test" --platforms x,linkedin --preview` should work.

### 1.4 Fix Error Exit Codes
**File**: `src/cli/run.rs`
**Issue**: Error cases return exit code 0 instead of 1.
**Fix**: Ensure all error paths call `std::process::exit(1)` or return `Err` that propagates to exit 1.
**Test**: `social-forge x post ''` should exit with code 1.

### 1.5 Fix Test Suite Compilation
**File**: `tests/linkedin_e2e_test.rs`
**Issue**: `is_token_expired` method not found on `ProviderError`.
**Fix**: Add the method or use the correct error checking pattern.
**Test**: `cargo test` should compile and run.

---

## Phase 2: Documentation Corrections (P1) — Misleading for AI Agents

### 2.1 Update SKILL.md
- Fix tool count (35 bridge tools, not 311)
- Fix `stage` syntax (remove `--platforms` until implemented, or document it after Phase 1.3)
- Remove `mcp-call` references for tools not in the bridge
- Add accurate `mcp-tools` output example

### 2.2 Update evals.json
- Fix eval #9 carousel syntax (`--media` not `--images`)
- Fix evals #36-38 Dev.to/Hashnode/Medium arg order
- Add eval for `mcp-call` with registered tools
- Add eval for error exit codes

### 2.3 Update providers.md
- Correct MCP tool count
- Mark which tools are bridge-accessible vs MCP-only
- Fix stage syntax

### 2.4 Update quick-reference.md
- Remove mcp-call references for unregistered tools
- Add note about bridge limitations
- Fix stage syntax

---

## Phase 3: Missing Workflow Coverage (P2) — Production Gaps

### 3.1 Instagram Posting Shortcut
**Priority**: HIGH — Most requested social platform
**Current**: Requires 3-step MCP flow (create_container → poll_container → publish_container)
**Plan**: Add `social-forge instagram post <ACCOUNT_ID> "caption" --media <URL>` CLI command that wraps the 3-step flow internally.
**Files**: `src/cli/mod.rs` (add `InstagramAction::Post`), `src/cli/run.rs`, `src/social/instagram.rs`

### 3.2 Recurring Post Scheduling
**Priority**: HIGH — Essential for content calendars
**Current**: One-shot scheduling only
**Plan**: Add `recurring` field to posts table and scheduler. Support daily/weekly/monthly recurrence patterns.
**Files**: `src/db/models.rs`, `src/scheduler/mod.rs`, `src/mcp/tools_posts.rs`, `src/cli/mod.rs`

### 3.3 RSS Auto-Posting
**Priority**: MEDIUM — Content curation automation
**Current**: RSS module exists (`src/rss/`) but no CLI command
**Plan**: Add `social-forge automation create-rss <INTEGRATION_ID> --feed-url <URL> --template "..."` command.
**Files**: `src/rss/mod.rs`, `src/cli/mod.rs`, `src/cli/run.rs`

### 3.4 Cross-Platform Analytics Dashboard
**Priority**: MEDIUM — ROI tracking for corporate clients
**Current**: Per-platform analytics only
**Plan**: Add `social-forge analytics dashboard --days 30` that aggregates metrics across all connected providers.
**Files**: `src/cli/mod.rs`, `src/cli/run.rs`, `src/mcp/tools_analytics.rs`

### 3.5 AI-Generated Response Type
**Priority**: LOW — Automation enhancement
**Current**: `response_type` field supports "ai_generated" but no implementation
**Plan**: Wire up AI response generation using the existing automation framework.
**Files**: `src/services/automation.rs`, `src/mcp/tools_automation.rs`

---

## Phase 4: Missing Engagement Tools (P3) — Feature Parity

### 4.1 Follow/Unfollow CLI Commands
**Files**: `src/cli/mod.rs`, `src/cli/run.rs`
**Plan**: Add `social-forge x follow <USER_ID>` and `social-forge x unfollow <USER_ID>`.

### 4.2 Mute/Block Tools
**Files**: `src/mcp/tools_x.rs`, `src/cli/mod.rs`
**Plan**: Add `x_mute_user`, `x_block_user` MCP tools and CLI commands.

### 4.3 Competitor Analysis
**Files**: `src/mcp/tools_analytics.rs`, `src/cli/mod.rs`
**Plan**: Add `social-forge analytics competitor <USERNAME> --platform x,linkedin` that tracks public metrics.

### 4.4 Hashtag Tracking
**Files**: `src/mcp/tools_instagram.rs`, `src/mcp/tools_x.rs`
**Plan**: Add `social-forge x search-hashtag <TAG>` and `social-forge instagram hashtag <ACCOUNT_ID> <TAG>` with trend data.

### 4.5 Content Performance Scoring
**Files**: `src/mcp/tools_analytics.rs`, `src/services/posts.rs`
**Plan**: Add engagement rate calculation, growth trends, and content performance comparison.

---

## Execution Order

| Phase | Items | Estimated Effort |
|-------|-------|-----------------|
| Phase 1 (P0) | 5 critical bugs | 2-3 hours |
| Phase 2 (P1) | 4 doc updates | 1 hour |
| Phase 3 (P2) | 5 missing workflows | 4-6 hours |
| Phase 4 (P3) | 5 feature additions | 3-4 hours |
| **Total** | **19 items** | **10-14 hours** |

---

## Success Criteria

- [ ] `social-forge mcp-call` does not panic
- [ ] `social-forge mcp-tools` lists 273+ tools
- [ ] `social-forge stage "text" --platforms x,linkedin --preview` works
- [ ] Error cases exit with code 1
- [ ] `cargo test` compiles and passes
- [ ] All skill docs match actual CLI capabilities
- [ ] Instagram posting works via single CLI command
- [ ] Recurring posts can be scheduled
- [ ] RSS auto-posting is functional
- [ ] Cross-platform analytics dashboard works
