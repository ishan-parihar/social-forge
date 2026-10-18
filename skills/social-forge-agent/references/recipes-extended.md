# Extended recipes

Loaded on demand from `SKILL.md`. IDs come from the Finding IDs table there.

### Read a feed

```bash
# X/Twitter timeline
social-forge x timeline --count 5 --pretty

# Reddit subreddit
social-forge reddit browse rust --sort hot --pretty

# Reddit search
social-forge reddit search "async rust" --pretty

# Instagram posts
social-forge instagram posts <ACCOUNT_ID> --pretty

# LinkedIn company posts
social-forge linkedin-page feed <PAGE_ID> --pretty

# Facebook page feed
social-forge facebook posts <PAGE_ID> --pretty

# Import from a provider into local store
social-forge import x --count 20 --pretty

# Browse unified feed (all providers)
social-forge feed --pretty
social-forge feed --provider x --pretty
```

### Check analytics

```bash
# Instagram insights
social-forge instagram insights <ACCOUNT_ID> --metric reach,follower_count --pretty

# LinkedIn page analytics
social-forge linkedin-page analytics <PAGE_ID> --pretty

# LinkedIn page followers
social-forge linkedin-page followers <PAGE_ID> --pretty
```

**Instagram metrics** (comma-separated):
- Day period: `reach`, `follower_count`, `online_followers`
- Lifetime: `total_interactions`, `comments`, `shares`, `saves`, `likes`, `accounts_engaged`, `profile_views`, `website_clicks`

### Manage media

```bash
# Upload a file
social-forge media upload ./photo.jpg --alt "Description" --pretty

# Upload from URL
social-forge media upload https://example.com/image.png --pretty

# List uploaded media
social-forge media list --limit 10 --pretty

# Upload multiple files
social-forge media upload-batch ./img1.jpg ./img2.jpg ./img3.jpg --pretty
```

### Create a carousel

```bash
social-forge carousel "5 Tips for Devs" --integration <UUID> --media https://example.com/tip1.jpg,https://example.com/tip2.jpg --pretty
```

### Comment and reply

```bash
# Get comments on a post
social-forge comment get <INTEGRATION_ID> <POST_ID> --limit 20 --pretty

# Reply to a comment
social-forge comment reply <INTEGRATION_ID> <COMMENT_ID> "Thanks for feedback!" --pretty
```

### Send a DM

```bash
# List conversations
social-forge dm list <INTEGRATION_ID> --limit 20 --pretty

# Get messages in a conversation
social-forge dm messages <INTEGRATION_ID> <CONVERSATION_ID> --limit 20 --pretty

# Send a message
social-forge dm send <INTEGRATION_ID> <RECIPIENT_ID> "Hello!" --pretty
```

### Stage a post (review before publish)

```bash
# Stage across multiple platforms
social-forge stage "Long content that will be split..." --integrations <UUID1>,<UUID2> --pretty

# Preview splits without creating drafts
social-forge stage "Preview mode" --platforms x,linkedin --preview --pretty
```

### Set up automation

```bash
# Auto-reply to comments
social-forge automation create <INTEGRATION_ID> "Auto-thanks" comment "Thanks for commenting!" fixed --pretty

# List rules
social-forge automation list --pretty

# View execution logs
social-forge automation logs <RULE_ID> --limit 50 --pretty
```

### Use MCP tools directly (advanced)

If a CLI command doesn't exist for what you need, use `mcp-call`:

```bash
social-forge mcp-tools --pretty

# Call any tool by name
social-forge mcp-call fb_create_post --args '{"page_id":"123","message":"Hello"}'
social-forge mcp-call x_home_timeline --args '{"count":5}'
social-forge mcp-call posts_create --args '{"content":"test","platforms":["x"]}'
```
