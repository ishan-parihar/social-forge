# Comment & DM Automation — Full-Scope Implementation Plan

## Executive Summary

Add comprehensive comment and DM management across all 21+ social media platforms, plus an automation engine for auto-replying to comments and DMs with configurable rules and AI-powered responses.

**Timeline**: 3 phases, ~2-3 weeks
**Impact**: Transforms social-forge from a posting tool into a full social media management platform

---

## Current Status (as of 2026-06-28)

### ✅ Implemented

| Feature | Status | Commit |
|---------|--------|--------|
| SocialProvider trait DM methods | ✅ Done | `f7885fa` |
| DmConversation/DmMessage types | ✅ Done | `f7885fa` |
| Automation DB schema (017_automation.sql) | ✅ Done | `f7885fa` |
| Generic comment tools (get/reply/delete) | ✅ Done | `510df98` |
| Generic DM tools (send/list/get) | ✅ Done | `510df98` |
| Automation engine (rules, cooldowns, AI) | ✅ Done | `510df98` |
| Automation MCP tools (CRUD + logs) | ✅ Done | `510df98` |
| MCP media upload tool | ✅ Done | `0e742b5` |
| Multi-platform staging tool | ✅ Done | `0e742b5` |
| Content splitter (character limits) | ✅ Done | `0e742b5` |

### ⏳ Pending (Platform-Specific Implementations)

| Platform | Comment Tools | DM Tools | Priority |
|----------|:---:|:---:|:---:|
| X/Twitter | ❌ Not implemented | ❌ Not implemented | High |
| LinkedIn | ❌ Not implemented | ❌ Not implemented | High |
| Instagram | ❌ Not implemented | ❌ Not implemented | High |
| Bluesky | ❌ Not implemented | ❌ N/A (no API) | Medium |
| Mastodon | ❌ Not implemented | ❌ N/A (no API) | Medium |
| YouTube | ❌ Not implemented | ❌ N/A (no API) | Medium |

### 📊 Coverage Summary

| Category | Before | After | Change |
|----------|:---:|:---:|:---:|
| Platforms with comment support | 9 | 9 | +0 (generic tools added) |
| Platforms with DM support | 8 | 8 | +0 (generic tools added) |
| MCP comment tools | 0 | 3 | +3 |
| MCP DM tools | 0 | 3 | +3 |
| MCP automation tools | 0 | 5 | +5 |
| Automation features | 0 | 6 | +6 |

---

## Phase 1: Trait Extension & Foundation (Days 1-3)

### 1.1 Extend SocialProvider Trait

**File**: `src/social/mod.rs`

Add new trait methods:

```rust
// DM methods (new)
async fn send_dm(
    &self,
    access_token: &str,
    recipient: &str,
    content: &PostContent,
) -> Result<PublishResult, ProviderError> {
    Err(ProviderError::Api("DMs not supported".into()))
}

async fn get_dm_conversations(
    &self,
    access_token: &str,
    limit: u32,
) -> Result<Vec<DmConversation>, ProviderError> {
    Ok(vec![])
}

async fn get_dm_messages(
    &self,
    access_token: &str,
    conversation_id: &str,
    limit: u32,
) -> Result<Vec<DmMessage>, ProviderError> {
    Ok(vec![])
}

// Comment enhancement (existing method stays, add reply method)
async fn reply_to_comment(
    &self,
    access_token: &str,
    comment_id: &str,
    content: &PostContent,
) -> Result<PublishResult, ProviderError> {
    Err(ProviderError::Api("Comment replies not supported".into()))
}

// New: Get comment details
async fn get_comment(
    &self,
    access_token: &str,
    comment_id: &str,
) -> Result<Option<CommentData>, ProviderError> {
    Ok(None)
}

// New: Delete comment
async fn delete_comment(
    &self,
    access_token: &str,
    comment_id: &str,
) -> Result<(), ProviderError> {
    Err(ProviderError::Api("Comment deletion not supported".into()))
}
```

Add new data types:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DmConversation {
    pub id: String,
    pub participant: String,
    pub participant_name: Option<String>,
    pub last_message: Option<String>,
    pub last_message_at: Option<chrono::DateTime<chrono::Utc>>,
    pub unread_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DmMessage {
    pub id: String,
    pub conversation_id: String,
    pub sender: String,
    pub sender_name: Option<String>,
    pub content: String,
    pub media: Vec<MediaAttachment>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub read: bool,
}
```

### 1.2 Database Schema for Automation

**New migration**: `0XX_create_automation.sql`

```sql
-- Automation rules for auto-reply to comments/DMs
CREATE TABLE automation_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    integration_id UUID NOT NULL REFERENCES integrations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    trigger_type TEXT NOT NULL, -- 'comment', 'dm', 'mention', 'follow'
    trigger_filter JSONB DEFAULT '{}', -- { keywords: [...], platforms: [...], min_likes: N }
    response_template TEXT NOT NULL, -- Template with {placeholders}
    response_type TEXT NOT NULL, -- 'ai_generated', 'template', 'fixed'
    ai_model TEXT, -- For AI-generated responses
    is_active BOOLEAN DEFAULT true,
    cooldown_minutes INT DEFAULT 0, -- Min time between triggers
    max_responses_per_hour INT DEFAULT 10,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Automation execution log
CREATE TABLE automation_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rule_id UUID NOT NULL REFERENCES automation_rules(id) ON DELETE CASCADE,
    trigger_id TEXT NOT NULL, -- Comment/DM ID that triggered
    trigger_type TEXT NOT NULL,
    response TEXT,
    status TEXT NOT NULL, -- 'sent', 'failed', 'skipped_cooldown', 'skipped_limit'
    error_message TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

---

## Phase 2: Comment MCP Tools (Days 4-7)

### 2.1 X/Twitter Reply Tool

**File**: `src/mcp/tools_x.rs`

```rust
pub async fn x_reply_tweet(
    state: &AppState,
    input: &XReplyInput,
) -> Result<Json<XReplyOutput>, String> {
    // Uses X GraphQL reply endpoint
    // tweet_id + text -> creates reply tweet
}
```

### 2.2 Bluesky Reply Tool

**File**: `src/mcp/tools_bluesky.rs`

```rust
pub async fn bs_reply(
    state: &AppState,
    input: &BsReplyInput,
) -> Result<Json<BsReplyOutput>, String> {
    // Uses AT Protocol reply endpoint
    // post_uri + text -> creates reply post
}
```

### 2.3 Mastodon Reply Tool

**File**: `src/mcp/tools_mastodon.rs`

```rust
pub async fn ms_reply(
    state: &AppState,
    input: &MsReplyInput,
) -> Result<Json<MsReplyOutput>, String> {
    // Uses Mastodon API in_reply_to_id
    // status_id + text -> creates reply status
}
```

### 2.4 YouTube Comment Reply Tool

**File**: `src/mcp/tools_youtube.rs`

```rust
pub async fn yt_reply_comment(
    state: &AppState,
    input: &YtReplyInput,
) -> Result<Json<YtReplyOutput>, String> {
    // Uses YouTube Data API reply endpoint
    // parent_comment_id + text -> creates reply comment
}
```

### 2.5 Generic Comment Tools (New)

**File**: `src/mcp/tools_comments.rs` (new)

```rust
// Universal comment tools that work across platforms
pub async fn get_comments(
    state: &AppState,
    input: &GetCommentsInput,
) -> Result<Json<GetCommentsOutput>, String> {
    // Resolves provider from integration_id
    // Calls provider.get_post_comments()
}

pub async fn reply_to_comment(
    state: &AppState,
    input: &ReplyToCommentInput,
) -> Result<Json<ReplyToCommentOutput>, String> {
    // Resolves provider from integration_id
    // Calls provider.reply_to_comment()
}

pub async fn delete_comment(
    state: &AppState,
    input: &DeleteCommentInput,
) -> Result<Json<SuccessOutput>, String> {
    // Resolves provider from integration_id
    // Calls provider.delete_comment()
}
```

### 2.6 Platform Coverage Matrix (After Phase 2)

| Platform | Get Comments | Post Comment | Reply to Comment | Delete Comment |
|----------|:---:|:---:|:---:|:---:|
| X/Twitter | ❌ | ❌ | ✅ (new) | ❌ |
| LinkedIn | ✅ | ✅ | ❌ (add) | ❌ |
| LinkedIn Page | ❌ | ✅ | ❌ (add) | ❌ |
| Facebook | ✅ | ✅ | ❌ (add) | ❌ |
| Instagram | ✅ | ✅ (reply) | ✅ | ❌ |
| Threads | ❌ | ✅ (reply) | ❌ (add) | ❌ |
| Bluesky | ❌ | ❌ | ✅ (new) | ❌ |
| Mastodon | ❌ | ❌ | ✅ (new) | ❌ |
| Reddit | ✅ | ✅ | ✅ | ❌ (add) |
| YouTube | ✅ | ❌ | ✅ (new) | ❌ |
| Skool | ❌ | ✅ | ❌ (add) | ❌ |
| TikTok | ❌ | ❌ | ❌ (API limited) | ❌ |
| Pinterest | ❌ | ❌ | ❌ (no API) | ❌ |

---

## Phase 3: DM MCP Tools (Days 8-12)

### 3.1 X/Twitter DM Tools

**File**: `src/mcp/tools_x.rs`

```rust
pub async fn x_send_dm(
    state: &AppState,
    input: &XSendDmInput,
) -> Result<Json<XSendDmOutput>, String> {
    // Uses X API v2 DM endpoint
    // recipient_id + text -> sends DM
}

pub async fn x_list_dms(
    state: &AppState,
    input: &XListDmsInput,
) -> Result<Json<XListDmsOutput>, String> {
    // Uses X API v2 DM events endpoint
    // Returns list of DM conversations
}

pub async fn x_get_dm_conversation(
    state: &AppState,
    input: &XGetDmInput,
) -> Result<Json<XGetDmOutput>, String> {
    // Uses X API v2 DM conversation endpoint
    // Returns messages in a conversation
}
```

### 3.2 LinkedIn Message Tools

**File**: `src/mcp/tools_linkedin.rs`

```rust
pub async fn li_send_message(
    state: &AppState,
    input: &LiSendMsgInput,
) -> Result<Json<LiSendMsgOutput>, String> {
    // Uses LinkedIn Messaging API
    // recipient_urn + message -> sends message
}

pub async fn li_list_conversations(
    state: &AppState,
    input: &LiListConvInput,
) -> Result<Json<LiListConvOutput>, String> {
    // Uses LinkedIn Messaging API
    // Returns list of conversations
}
```

### 3.3 Instagram DM Tools

**File**: `src/mcp/tools_instagram.rs`

```rust
pub async fn ig_send_dm(
    state: &AppState,
    input: &IgSendDmInput,
) -> Result<Json<IgSendDmOutput>, String> {
    // Uses Instagram Messaging API (via Messenger Platform)
    // recipient_id + text -> sends DM
}

pub async fn ig_list_conversations(
    state: &AppState,
    input: &IgListConvInput,
) -> Result<Json<IgListConvOutput>, String> {
    // Uses Instagram Messaging API
    // Returns list of conversations
}
```

### 3.4 Generic DM Tools (New)

**File**: `src/mcp/tools_dm.rs` (new)

```rust
// Universal DM tools that work across platforms
pub async fn send_dm(
    state: &AppState,
    input: &SendDmInput,
) -> Result<Json<SendDmOutput>, String> {
    // Resolves provider from integration_id
    // Calls provider.send_dm()
}

pub async fn list_dm_conversations(
    state: &AppState,
    input: &ListDmInput,
) -> Result<Json<ListDmOutput>, String> {
    // Resolves provider from integration_id
    // Calls provider.get_dm_conversations()
}

pub async fn get_dm_messages(
    state: &AppState,
    input: &GetDmInput,
) -> Result<Json<GetDmOutput>, String> {
    // Resolves provider from integration_id
    // Calls provider.get_dm_messages()
}
```

### 3.5 Platform Coverage Matrix (After Phase 3)

| Platform | List Conversations | Get Messages | Send DM |
|----------|:---:|:---:|:---:|
| X/Twitter | ✅ (new) | ✅ (new) | ✅ (new) |
| LinkedIn | ✅ (new) | ✅ (new) | ✅ (new) |
| Instagram | ✅ (new) | ✅ (new) | ✅ (new) |
| Facebook | ✅ | ✅ | ✅ |
| Slack | ✅ | ✅ | ✅ |
| Discord | ✅ | ✅ | ✅ |
| Telegram Bot | ✅ | ✅ | ✅ |
| Telegram User | ✅ | ✅ | ✅ |
| WhatsApp | ❌ | ❌ | ✅ |
| Gmail | ✅ | ✅ | ✅ |
| Reddit | ✅ | ✅ | ✅ |

---

## Phase 4: Automation Engine (Days 13-17)

### 4.1 Automation Service

**File**: `src/services/automation.rs` (new)

```rust
pub struct AutomationEngine {
    db: PgPool,
    providers: ProviderRegistry,
}

impl AutomationEngine {
    pub async fn check_triggers(
        &self,
        trigger_type: &str,
        integration_id: Uuid,
        trigger_data: &TriggerData,
    ) -> Result<Vec<AutomationAction>, String> {
        // 1. Get active rules for this integration
        // 2. Check trigger filters (keywords, platform, etc.)
        // 3. Check cooldowns and rate limits
        // 4. Return matching actions
    }

    pub async fn execute_action(
        &self,
        action: &AutomationAction,
    ) -> Result<(), String> {
        // 1. Generate response (template or AI)
        // 2. Send via provider
        // 3. Log execution
    }

    pub async fn generate_ai_response(
        &self,
        template: &str,
        context: &TriggerContext,
        model: &str,
    ) -> Result<String, String> {
        // 1. Build prompt with context
        // 2. Call LLM proxy
        // 3. Return generated response
    }
}
```

### 4.2 Comment Monitoring Worker

**File**: `src/worker/comment_monitor.rs` (new)

```rust
pub async fn run_comment_monitor(
    state: AppState,
    interval: Duration,
) {
    loop {
        // 1. For each integration with active comment rules
        // 2. Fetch recent comments via provider.get_post_comments()
        // 3. Check for new comments (not in automation_logs)
        // 4. Run automation engine.check_triggers("comment", ...)
        // 5. Execute matching actions
        // 6. Sleep for interval
    }
}
```

### 4.3 DM Monitoring Worker

**File**: `src/worker/dm_monitor.rs` (new)

```rust
pub async fn run_dm_monitor(
    state: AppState,
    interval: Duration,
) {
    loop {
        // 1. For each integration with active DM rules
        // 2. Fetch recent DMs via provider.get_dm_conversations()
        // 3. Check for new messages (not in automation_logs)
        // 4. Run automation engine.check_triggers("dm", ...)
        // 5. Execute matching actions
        // 6. Sleep for interval
    }
}
```

### 4.4 Automation MCP Tools

**File**: `src/mcp/tools_automation.rs` (new)

```rust
pub async fn create_automation_rule(
    state: &AppState,
    input: &CreateRuleInput,
) -> Result<Json<CreateRuleOutput>, String> {
    // Creates a new automation rule
}

pub async fn list_automation_rules(
    state: &AppState,
    input: &ListRulesInput,
) -> Result<Json<ListRulesOutput>, String> {
    // Lists all rules for user
}

pub async fn update_automation_rule(
    state: &AppState,
    input: &UpdateRuleInput,
) -> Result<Json<UpdateRuleOutput>, String> {
    // Updates an existing rule
}

pub async fn delete_automation_rule(
    state: &AppState,
    input: &DeleteRuleInput,
) -> Result<Json<SuccessOutput>, String> {
    // Deletes a rule
}

pub async fn get_automation_logs(
    state: &AppState,
    input: &GetLogsInput,
) -> Result<Json<GetLogsOutput>, String> {
    // Gets execution logs for a rule
}
```

### 4.5 Automation Rule Schema

```json
{
  "name": "Auto-reply to LinkedIn comments",
  "trigger_type": "comment",
  "integration_id": "uuid",
  "trigger_filter": {
    "keywords": ["interested", "how much", "price"],
    "platforms": ["linkedin"],
    "min_likes": 0,
    "exclude_own": true
  },
  "response_template": "Thanks for your interest! Check out our pricing page at {pricing_url}",
  "response_type": "template",
  "cooldown_minutes": 5,
  "max_responses_per_hour": 10,
  "is_active": true
}
```

---

## Phase 5: Frontend UI (Days 18-21)

### 5.1 Comment Management Page

**File**: `frontend/src/routes/comments/+page.svelte` (new)

- Unified inbox for all platform comments
- Filter by platform, post, date
- Inline reply functionality
- Mark as read/responded

### 5.2 DM Management Page

**File**: `frontend/src/routes/dms/+page.svelte` (new)

- Unified inbox for all platform DMs
- Conversation threading
- Send DM from UI
- Attach media to DMs

### 5.3 Automation Rules UI

**File**: `frontend/src/routes/automation/+page.svelte` (new)

- Create/edit automation rules
- Test rules with sample triggers
- View execution logs
- Toggle rules on/off

---

## Provider Implementation Priority

### High Priority (Most User Demand)
1. **X/Twitter** - Reply to tweets, DMs
2. **LinkedIn** - Reply to comments, messaging
3. **Instagram** - Reply to comments, DMs

### Medium Priority
4. **Bluesky** - Reply to posts
5. **Mastodon** - Reply to statuses
6. **YouTube** - Reply to comments

### Low Priority (API Limited)
7. **TikTok** - Comment replies (API limited)
8. **Pinterest** - No comment API
9. **GitHub** - Issue comments (already have GitHub provider)

---

## Success Metrics

| Metric | Target |
|--------|--------|
| Platforms with comment support | 12/21 (from 9) |
| Platforms with DM support | 11/21 (from 8) |
| Automation rules per user | Unlimited |
| Response latency (comment) | < 30 seconds |
| Response latency (DM) | < 60 seconds |
| AI response quality | Configurable model |

---

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| API rate limits | Cooldown system + max_responses_per_hour |
| Token expiration | Already handled by scheduler |
| Spam/abuse | Keyword filters + cooldowns + manual review |
| Platform policy violations | Template validation + content moderation |
| Cost (AI responses) | Usage tracking + configurable limits |

---

## Implementation Order

1. **Phase 1** (Foundation) - Must complete first
2. **Phase 2** (Comments) - Can parallel with Phase 3
3. **Phase 3** (DMs) - Can parallel with Phase 2
4. **Phase 4** (Automation) - Depends on Phase 2+3
5. **Phase 5** (Frontend) - Can parallel with Phase 4

**Estimated Total**: 21 working days

---

## Full-Scope Upgrade: Next Steps

### Priority 1: X/Twitter Comment & DM Tools

**Files to modify**:
- `src/social/x.rs` — Implement `reply_to_comment()`, `send_dm()`, `get_dm_conversations()`, `get_dm_messages()`
- `src/mcp/tools_x.rs` — Add `x_reply_tweet`, `x_send_dm`, `x_list_dms`, `x_get_dm_conversation`

**API Endpoints**:
- Reply: POST `https://api.x.com/2/tweets` with `reply.in_reply_to_tweet_id`
- DMs: POST `https://api.x.com/2/dm_conversations/with/{participant_id}/messages`
- List DMs: GET `https://api.x.com/2/dm_conversations`
- Get DM messages: GET `https://api.x.com/2/dm_conversations/{id}/messages`

### Priority 2: LinkedIn Comment & DM Tools

**Files to modify**:
- `src/social/linkedin.rs` — Implement `reply_to_comment()`, `send_dm()`, `get_dm_conversations()`, `get_dm_messages()`
- `src/mcp/tools_linkedin.rs` — Add `li_reply_comment`, `li_send_message`, `li_list_conversations`, `li_get_messages`

**API Endpoints**:
- Reply: POST `https://api.linkedin.com/v2/ugcPosts` with `verb=SHARE` and `parentComment=urn:li:comment:xxx`
- Messages: POST `https://api.linkedin.com/v2/messages` with `recipient=urn:li:person:xxx`

### Priority 3: Instagram Comment & DM Tools

**Files to modify**:
- `src/social/instagram.rs` — Implement `reply_to_comment()`, `send_dm()`, `get_dm_conversations()`, `get_dm_messages()`
- `src/mcp/tools_instagram.rs` — Add `ig_reply_comment`, `ig_send_dm`, `ig_list_conversations`, `ig_get_messages`

**API Endpoints**:
- Reply: POST `https://graph.facebook.com/v18.0/{comment_id}/replies` with `message`
- DMs: POST `https://graph.facebook.com/v18.0/me/messages` with `recipient` and `message`

### Priority 4: Bluesky Comment Tools

**Files to modify**:
- `src/social/bluesky.rs` — Implement `reply_to_comment()`
- `src/mcp/tools_bluesky.rs` — Add `bs_reply`

**API Endpoints**:
- Reply: POST `https://bsky.social/xrpc/app.bsky.feed.post.create` with `reply` field

### Priority 5: Mastodon Comment Tools

**Files to modify**:
- `src/social/mastodon.rs` — Implement `reply_to_comment()`
- `src/mcp/tools_mastodon.rs` — Add `ms_reply`

**API Endpoints**:
- Reply: POST `/api/v1/statuses` with `in_reply_to_id`

### Priority 6: YouTube Comment Reply

**Files to modify**:
- `src/social/youtube.rs` — Implement `reply_to_comment()`
- `src/mcp/tools_youtube.rs` — Add `yt_reply_comment`

**API Endpoints**:
- Reply: POST `https://www.googleapis.com/youtube/v3/comments` with `parentId`

---

## Testing Plan

### Unit Tests

1. **Content Splitter Tests**:
   - Test splitting for each platform
   - Test thread numbering
   - Test edge cases (empty content, exact limit)

2. **Automation Engine Tests**:
   - Test keyword matching
   - Test cooldown logic
   - Test rate limiting
   - Test AI response generation

3. **MCP Tool Tests**:
   - Test tool registration
   - Test input validation
   - Test error handling

### Integration Tests

1. **Comment Flow**:
   - Create post → Get comments → Reply to comment → Verify reply

2. **DM Flow**:
   - List conversations → Get messages → Send DM → Verify message

3. **Automation Flow**:
   - Create rule → Trigger comment → Verify auto-reply → Check logs

### Manual Testing

1. **X/Twitter**:
   - Reply to a tweet
   - Send a DM
   - Verify automation triggers

2. **LinkedIn**:
   - Reply to a comment
   - Send a message
   - Verify automation triggers

3. **Instagram**:
   - Reply to a comment
   - Send a DM
   - Verify automation triggers

---

## Deployment Checklist

- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] Manual testing complete for each platform
- [ ] Documentation updated
- [ ] Migration applied to production database
- [ ] Service restarted
- [ ] MCP tools verified in Claude Desktop
