# Extended recipes

Loaded on demand from `SKILL.md`. IDs come from the Finding IDs table there.

### Read a feed

```bash
# X/Twitter timeline
switchpost x timeline --count 5 --pretty

# Reddit subreddit
switchpost reddit browse rust --sort hot --pretty

# Reddit search
switchpost reddit search "async rust" --pretty

# Instagram posts
switchpost instagram posts <ACCOUNT_ID> --pretty

# LinkedIn company posts
switchpost linkedin-page feed <PAGE_ID> --pretty

# Facebook page feed
switchpost facebook posts <PAGE_ID> --pretty

# Import from a provider into local store
switchpost import x --count 20 --pretty

# Browse unified feed (all providers)
switchpost feed --pretty
switchpost feed --provider x --pretty
```

### Check analytics

```bash
# Instagram insights
switchpost instagram insights <ACCOUNT_ID> --metric reach,follower_count --pretty

# LinkedIn page analytics
switchpost linkedin-page analytics <PAGE_ID> --pretty

# LinkedIn page followers
switchpost linkedin-page followers <PAGE_ID> --pretty
```

**Instagram metrics** (comma-separated):
- Day period: `reach`, `follower_count`, `online_followers`
- Lifetime: `total_interactions`, `comments`, `shares`, `saves`, `likes`, `accounts_engaged`, `profile_views`, `website_clicks`

### Manage media

```bash
# Upload a file
switchpost media upload ./photo.jpg --alt "Description" --pretty

# Upload from URL
switchpost media upload https://example.com/image.png --pretty

# List uploaded media
switchpost media list --limit 10 --pretty

# Upload multiple files
switchpost media upload-batch ./img1.jpg ./img2.jpg ./img3.jpg --pretty
```

### Create a carousel

```bash
switchpost carousel "5 Tips for Devs" --integration <UUID> --media https://example.com/tip1.jpg,https://example.com/tip2.jpg --pretty
```

### Comment and reply

```bash
# Get comments on a post
switchpost comment get <INTEGRATION_ID> <POST_ID> --limit 20 --pretty

# Reply to a comment
switchpost comment reply <INTEGRATION_ID> <COMMENT_ID> "Thanks for feedback!" --pretty
```

### Send a DM

```bash
# List conversations
switchpost dm list <INTEGRATION_ID> --limit 20 --pretty

# Get messages in a conversation
switchpost dm messages <INTEGRATION_ID> <CONVERSATION_ID> --limit 20 --pretty

# Send a message
switchpost dm send <INTEGRATION_ID> <RECIPIENT_ID> "Hello!" --pretty
```

### Stage a post (review before publish)

```bash
# Stage across multiple platforms
switchpost stage "Long content that will be split..." --integrations <UUID1>,<UUID2> --pretty

# Preview splits without creating drafts
switchpost stage "Preview mode" --platforms x,linkedin --preview --pretty
```

### Set up automation

```bash
# Auto-reply to comments
switchpost automation create <INTEGRATION_ID> "Auto-thanks" comment "Thanks for commenting!" fixed --pretty

# List rules
switchpost automation list --pretty

# View execution logs
switchpost automation logs <RULE_ID> --limit 50 --pretty
```

### Use MCP tools directly (advanced)

If a CLI command doesn't exist for what you need, use `mcp-call`:

```bash
switchpost mcp-tools --pretty

# Call any tool by name
switchpost mcp-call fb_create_post --args '{"page_id":"123","message":"Hello"}'
switchpost mcp-call x_home_timeline --args '{"count":5}'
switchpost mcp-call posts_create --args '{"content":"test","platforms":["x"]}'
```
