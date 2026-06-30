# MCP-CLI Parity Plan

## Executive Summary

Ensure every MCP tool has a corresponding CLI command, and vice versa. This plan identifies gaps and provides implementation guidance for achieving full parity between the two interfaces.

---

## Current State

### MCP Tools (309 total)
- X/Twitter: 18 tools (timeline, search, like, retweet, bookmark, follow, DMs, reply)
- Reddit: 12 tools (browse, search, post, comment, vote, mod)
- LinkedIn: 15 tools (profile, posts, comments, analytics, DMs)
- LinkedIn Page: 8 tools (pages, posts, comments)
- Facebook: 10 tools (feed, posts, comments, insights)
- Instagram: 12 tools (media, comments, reels, stories, insights, DMs)
- Instagram Standalone: 6 tools (media, comments)
- YouTube: 10 tools (search, videos, playlists, comments, analytics, reply)
- Bluesky: 6 tools (profile, timeline, post, search, feed, reply)
- Mastodon: 5 tools (post, timeline, post detail, search, reply)
- TikTok: 4 tools (profile, post, videos)
- Medium: 4 tools (post, list, get)
- WordPress: 3 tools (post, list, get)
- GitHub: 2 tools (repos, issues)
- Telegram: 3 tools (send, groups, info)
- Discord: 2 tools (send, channels)
- Slack: 2 tools (send, channels)
- WhatsApp: 2 tools (send, chats)
- Gmail: 4 tools (profile, list, get, send)
- Calendar: 2 tools (list, create)
- Drive: 2 tools (list, upload)
- Generic: 11 tools (comments, DMs, automation)
- Posts: 8 tools (create, publish, schedule, update, delete, list, get, find_slot)
- Media: 3 tools (upload, list)
- Automation: 5 tools (create, list, update, delete, logs)

### CLI Commands (Current)
- **System**: init, providers, connect, doctor, setup, connect-all, config, serve, mcp, import, feed
- **X/Twitter**: post, timeline, search, like, retweet, delete, bookmark, user
- **Reddit**: browse, search, post, comment, vote, mod (ban, mute, approve, remove)
- **LinkedIn**: profile, posts, post-detail, comments, create-comment, create-post, delete-post, reactions, shares, analytics, post-analytics
- **LinkedIn Page**: list-pages, get-page, get-posts, create-comment, create-post
- **Facebook**: feed, post, comments, create-post, insights
- **Instagram**: media, media-detail, comments, create-container, publish-container, reply-to-comment, reels, stories, followers, business-discovery, insights-audience, mentions, poll-container

---

## Gap Analysis

### MCP → CLI Gaps (MCP tools without CLI equivalents)

| Category | MCP Tool | CLI Command | Priority |
|----------|----------|-------------|----------|
| **X/Twitter** | x_reply_tweet | ❌ Missing | High |
| | x_send_dm | ❌ Missing | High |
| | x_list_dms | ❌ Missing | High |
| | x_get_dm_conversation | ❌ Missing | High |
| **LinkedIn** | li_reply_comment | ❌ Missing | High |
| | li_send_dm | ❌ Missing | High |
| | li_list_conversations | ❌ Missing | High |
| | li_get_messages | ❌ Missing | High |
| **Instagram** | ig_send_dm | ❌ Missing | High |
| | ig_list_conversations | ❌ Missing | High |
| | ig_get_messages | ❌ Missing | High |
| **YouTube** | yt_reply_comment | ❌ Missing | Medium |
| **Bluesky** | bs_reply | ❌ Missing | Medium |
| **Mastodon** | ms_reply | ❌ Missing | Medium |
| **Generic** | get_comments | ❌ Missing | Medium |
| | reply_to_comment | ❌ Missing | Medium |
| | delete_comment | ❌ Missing | Medium |
| | send_dm | ❌ Missing | Medium |
| | list_dm_conversations | ❌ Missing | Medium |
| | get_dm_messages | ❌ Missing | Medium |
| | create_automation_rule | ❌ Missing | Low |
| | list_automation_rules | ❌ Missing | Low |
| | update_automation_rule | ❌ Missing | Low |
| | delete_automation_rule | ❌ Missing | Low |
| | get_automation_logs | ❌ Missing | Low |
| **Posts** | posts_create | ❌ Missing | Medium |
| | posts_publish | ❌ Missing | Medium |
| | posts_schedule | ❌ Missing | Medium |
| | posts_update | ❌ Missing | Medium |
| | posts_delete | ❌ Missing | Medium |
| | posts_list | ❌ Missing | Medium |
| | posts_get | ❌ Missing | Medium |
| | posts_find_slot | ❌ Missing | Low |
| **Media** | posts_media_upload | ❌ Missing | Medium |
| | posts_media_list | ❌ Missing | Low |
| **Staging** | posts_stage | ❌ Missing | Low |

### CLI → MCP Gaps (CLI commands without MCP equivalents)

| Category | CLI Command | MCP Tool | Priority |
|----------|-------------|----------|----------|
| **System** | init | ❌ N/A | N/A |
| | providers | ❌ N/A | N/A |
| | connect | ❌ N/A | N/A |
| | doctor | ❌ N/A | N/A |
| | setup | ❌ N/A | N/A |
| | connect-all | ❌ N/A | N/A |
| | config set/get/list | ❌ N/A | N/A |
| | import | ❌ N/A | N/A |
| | feed | ❌ N/A | N/A |
| **X/Twitter** | x user | ❌ Missing | Low |
| **Reddit** | reddit mod (ban, mute, approve, remove) | ❌ Missing | Low |
| **LinkedIn** | linkedin reactions | ❌ Missing | Low |
| | linkedin shares | ❌ Missing | Low |

---

## Implementation Plan

### Phase 1: High Priority CLI Commands (Days 1-2)

#### 1.1 X/Twitter CLI Commands

Add to `XAction` enum in `src/cli/mod.rs`:
```rust
/// Reply to a tweet
Reply {
    /// Tweet ID to reply to
    tweet_id: String,
    /// Reply text
    text: String,
},

/// Send a direct message
Dm {
    /// Recipient user ID
    recipient: String,
    /// Message text
    text: String,
},

/// List DM conversations
DmList {
    /// Number of conversations to fetch
    #[arg(long, default_value_t = 20)]
    count: u32,
},

/// Get messages in a DM conversation
DmMessages {
    /// Conversation ID
    conversation_id: String,
    /// Number of messages to fetch
    #[arg(long, default_value_t = 20)]
    count: u32,
},
```

Add handlers in `src/cli/run.rs`:
- `handle_x_reply()` — Call `provider.reply_to_comment()`
- `handle_x_dm()` — Call `provider.send_dm()`
- `handle_x_dm_list()` — Call `provider.get_dm_conversations()`
- `handle_x_dm_messages()` — Call `provider.get_dm_messages()`

#### 1.2 LinkedIn CLI Commands

Add to `LinkedinAction` enum:
```rust
/// Reply to a comment
Reply {
    /// Comment URN
    comment_id: String,
    /// Reply content
    content: String,
},

/// Send a direct message
Dm {
    /// Recipient URN
    recipient: String,
    /// Message content
    content: String,
},

/// List message conversations
DmList {
    /// Number of conversations to fetch
    #[arg(long, default_value_t = 20)]
    count: u32,
},

/// Get messages in a conversation
DmMessages {
    /// Conversation ID
    conversation_id: String,
    /// Number of messages to fetch
    #[arg(long, default_value_t = 20)]
    count: u32,
},
```

#### 1.3 Instagram CLI Commands

Add to `InstagramAction` enum:
```rust
/// Send a direct message
Dm {
    /// Recipient ID
    recipient: String,
    /// Message content
    content: String,
},

/// List message conversations
DmList {
    /// Number of conversations to fetch
    #[arg(long, default_value_t = 20)]
    count: u32,
},

/// Get messages in a conversation
DmMessages {
    /// Conversation ID
    conversation_id: String,
    /// Number of messages to fetch
    #[arg(long, default_value_t = 20)]
    count: u32,
},
```

### Phase 2: Medium Priority CLI Commands (Days 3-4)

#### 2.1 YouTube CLI Commands

Add to `Command` enum (new YouTube section):
```rust
/// YouTube operations
Youtube {
    #[command(subcommand)]
    action: YoutubeAction,
},

#[derive(Subcommand, Debug)]
pub enum YoutubeAction {
    /// Reply to a comment
    Reply {
        /// Comment ID
        comment_id: String,
        /// Reply content
        content: String,
    },
}
```

#### 2.2 Bluesky CLI Commands

Add to `Command` enum (new Bluesky section):
```rust
/// Bluesky operations
Bluesky {
    #[command(subcommand)]
    action: BlueskyAction,
},

#[derive(Subcommand, Debug)]
pub enum BlueskyAction {
    /// Reply to a post
    Reply {
        /// Post URI
        post_uri: String,
        /// Reply content
        content: String,
    },
}
```

#### 2.3 Mastodon CLI Commands

Add to `Command` enum (new Mastodon section):
```rust
/// Mastodon operations
Mastodon {
    #[command(subcommand)]
    action: MastodonAction,
},

#[derive(Subcommand, Debug)]
pub enum MastodonAction {
    /// Reply to a status
    Reply {
        /// Status ID
        status_id: String,
        /// Reply content
        content: String,
    },
}
```

#### 2.4 Generic Comment/DM CLI Commands

Add to `Command` enum:
```rust
/// Comment operations
Comment {
    #[command(subcommand)]
    action: CommentAction,
},

#[derive(Subcommand, Debug)]
pub enum CommentAction {
    /// Get comments for a post
    Get {
        /// Integration ID
        integration_id: String,
        /// Post ID
        post_id: String,
        /// Number of comments to fetch
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
    /// Reply to a comment
    Reply {
        /// Integration ID
        integration_id: String,
        /// Comment ID
        comment_id: String,
        /// Reply content
        content: String,
    },
    /// Delete a comment
    Delete {
        /// Integration ID
        integration_id: String,
        /// Comment ID
        comment_id: String,
    },
},

/// Direct message operations
Dm {
    #[command(subcommand)]
    action: DmAction,
},

#[derive(Subcommand, Debug)]
pub enum DmAction {
    /// Send a direct message
    Send {
        /// Integration ID
        integration_id: String,
        /// Recipient ID
        recipient: String,
        /// Message content
        content: String,
    },
    /// List DM conversations
    List {
        /// Integration ID
        integration_id: String,
        /// Number of conversations to fetch
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// Get messages in a conversation
    Messages {
        /// Integration ID
        integration_id: String,
        /// Conversation ID
        conversation_id: String,
        /// Number of messages to fetch
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
},
```

### Phase 3: Low Priority CLI Commands (Days 5-6)

#### 3.1 Automation CLI Commands

Add to `Command` enum:
```rust
/// Automation rules management
Automation {
    #[command(subcommand)]
    action: AutomationAction,
},

#[derive(Subcommand, Debug)]
pub enum AutomationAction {
    /// Create an automation rule
    Create {
        /// Integration ID
        integration_id: String,
        /// Rule name
        name: String,
        /// Trigger type (comment, dm, mention, follow)
        trigger_type: String,
        /// Response template
        response_template: String,
        /// Response type (fixed, template, ai_generated)
        response_type: String,
    },
    /// List automation rules
    List {
        /// Integration ID (optional)
        integration_id: Option<String>,
    },
    /// Update an automation rule
    Update {
        /// Rule ID
        rule_id: String,
        /// Rule name (optional)
        name: Option<String>,
        /// Response template (optional)
        response_template: Option<String>,
        /// Is active (optional)
        is_active: Option<bool>,
    },
    /// Delete an automation rule
    Delete {
        /// Rule ID
        rule_id: String,
    },
    /// Get execution logs
    Logs {
        /// Rule ID
        rule_id: String,
        /// Number of logs to fetch
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
},
```

#### 3.2 Posts CLI Commands

Add to `Command` enum:
```rust
/// Post management
Posts {
    #[command(subcommand)]
    action: PostsAction,
},

#[derive(Subcommand, Debug)]
pub enum PostsAction {
    /// Create a post
    Create {
        /// Content
        content: String,
        /// Integration IDs (comma-separated)
        integrations: String,
        /// Schedule time (ISO 8601, optional)
        schedule_at: Option<String>,
    },
    /// Publish a post
    Publish {
        /// Post ID
        post_id: String,
    },
    /// List posts
    List {
        /// Status filter (draft, queued, published, failed)
        status: Option<String>,
        /// Number of posts to fetch
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// Get a post
    Get {
        /// Post ID
        post_id: String,
    },
    /// Delete a post
    Delete {
        /// Post ID
        post_id: String,
    },
    /// Find available time slot
    FindSlot {
        /// Duration in minutes
        duration: u32,
    },
},
```

#### 3.3 Media CLI Commands

Add to `Command` enum:
```rust
/// Media management
Media {
    #[command(subcommand)]
    action: MediaAction,
},

#[derive(Subcommand, Debug)]
pub enum MediaAction {
    /// Upload media file
    Upload {
        /// File path
        file_path: String,
        /// Alt text (optional)
        alt_text: Option<String>,
    },
    /// List uploaded media
    List {
        /// Number of media to fetch
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
},
```

---

## Implementation Checklist

### Phase 1: High Priority
- [ ] X/Twitter: reply, dm, dm-list, dm-messages
- [ ] LinkedIn: reply, dm, dm-list, dm-messages
- [ ] Instagram: dm, dm-list, dm-messages

### Phase 2: Medium Priority
- [ ] YouTube: reply
- [ ] Bluesky: reply
- [ ] Mastodon: reply
- [ ] Generic: comment get/reply/delete, dm send/list/messages

### Phase 3: Low Priority
- [ ] Automation: create/list/update/delete/logs
- [ ] Posts: create/publish/list/get/delete/find-slot
- [ ] Media: upload/list

### Verification
- [ ] Each CLI command has corresponding MCP tool
- [ ] Each MCP tool has corresponding CLI command (where applicable)
- [ ] All CLI commands output JSON
- [ ] All CLI commands handle errors gracefully
- [ ] Documentation updated

---

## Testing Plan

### Unit Tests
1. Test each CLI command with mock provider
2. Test token resolution for each provider
3. Test error handling for missing integrations

### Integration Tests
1. Test end-to-end flow: CLI → Provider → API
2. Test MCP → CLI parity for each tool
3. Test error messages are consistent

### Manual Testing
1. Test each CLI command with real provider
2. Verify JSON output format
3. Verify error messages are helpful

---

## Documentation Updates

### README.md
- Add CLI command reference
- Add MCP tool reference
- Add parity matrix

### CLI Help
- Ensure all commands have helpful descriptions
- Ensure all flags have helpful help text

### MCP Schema
- Ensure all tools have proper descriptions
- Ensure all inputs have proper schemas
