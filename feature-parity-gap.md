## Feature Parity Gap Analysis: postiz-app vs. social-forge (postiz-rust)

---

### 1. PROVIDER COVERAGE

**postiz-app:** 34 social network providers
**social-forge:** 16 providers (including 14 shared, 1 unique, 1 split)

```
┌─────────────────────────────────────────────┐
│               VENN DIAGRAM                   │
│                                              │
│  postiz-app (34)                             │
│  ┌──────────────────┐                        │
│  │ TIKTOK, MASTODON  │  social-forge (16)    │
│  │ MEDIUM, DEV.TO    │  ┌───────────────┐    │
│  │ HASNODE, WORDPRESS│  │ WHATSAPP (unq) │    │
│  │ SLACK, TWITCH     │  │               │    │
│  │ VK, DRIBBBLE      │  └───────┬───────┘    │
│  │ KICK, NOSTR       │          │            │
│  │ MOLTBOOK, MEWE    │  ┌───────┴───────┐   │
│  │ LEMMY, LISTMONK   │  │ X │ FB │ IG   │   │
│  │ GMB, WHOP         │  │ IAS│ TH │ LI   │   │
│  │ FARCASTER         │  │ LIP│ YT │ PI   │   │
│  │ (20 providers)    │  │ RD │ BS │ DC   │   │
│  └──────────────────┘  │ TG │ SK    (14)│   │
│                        └───────────────┘    │
└─────────────────────────────────────────────┘
```

#### Shared Providers (14)

| Provider | social-forge | postiz-app | Our Functional Status |
|---|---|---|---|
| X / Twitter | `x` (18 MCP tools) | `x` | ✅ Complete: 18 tools, OAuth1, publishing, comments |
| Facebook | `facebook` (16 MCP tools) | `facebook` | ✅ Complete: 16 tools, Pages, posts, photos, videos |
| Instagram Business | `instagram` (17 MCP tools) | `instagram` | ✅ Complete: 17 tools—Reels, Stories, Insights |
| Instagram Standalone | `instagram-standalone` (7 tools) | `instagram.standalone` | ✅ 7 tools, container publishing |
| Threads | `threads` (9 tools) | `threads` | ✅ 9 tools—Profile, Threads, Replies, Insights |
| LinkedIn Personal | `linkedin` (6 tools) | `linkedin` | ✅ 6 tools—Profile, Posts, Comments |
| LinkedIn Page | `linkedin-page` (4 tools) | `linkedin.page` | ✅ 4 tools—Posts, Pages, Comments |
| YouTube | `youtube` (9 tools) | `youtube` | ⚠️ **Publishing STUB** (returns "coming soon") |
| Pinterest | `pinterest` (7 tools) | `pinterest` | ✅ 7 tools—Pins, Boards, Analytics |
| Reddit | `reddit` (7 tools) | `reddit` | ✅ 7 tools—Posts, Comments, DMs |
| Bluesky | `bluesky` (**0 MCP tools**) | `bluesky` | 🔥 **NO MCP tools** — currently an orphaned provider |
| Discord | `discord` (10 tools) | `discord` | ✅ 10 tools—Messages, Reactions, Channels |
| Telegram | `telegram-bot` + `telegram-user` (7 tools) | `telegram` | ⚠️ Split into Bot/User; no analytics |
| Skool | `skool` (5 tools) | `skool` | ✅ 5 tools—Posting, Community, Comments |

#### Our Unique Provider

| Provider | Our Status | Postiz-app |
|---|---|---|
| **WhatsApp** (4 tools) | ✅ Daemon (wacli)—Messages, Templates, Contacts, Chats | ❌ Not in Postiz-app |

#### 20 Providers Exclusive to Postiz-app (Our Differentiator)

| # | Provider | Platform Type | Operations | Authentication | Priority |
|---|---|---|---|---|---|
| 1 | **TikTok** | Short-form Video | Post, analytics, publication analytics, import, maxVideoLength | OAuth 2.0 + refresh (PKCE) | **High** — major missing platform |
| 2 | **Mastodon** | Fediverse Microblogging | Post, comment, file upload | OAuth 2.0 (configurable instance) | **Medium** — growing |
| 3 | **Mastodon Custom** | Mastodon Instances | Identical to Mastodon + dynamic app registration | Dynamic OAuth Application | **Low** — Mastodon variation |
| 4 | **Medium** | Blogging | Post, publish to a publication | API Key | **High** — popular long-form content |
| 5 | **Dev.to** | Developer Blogging | Post, tags, organizations | API Key | **High** — developer audience |
| 6 | **Hashnode** | Developer Blogging | Post, tags, publications | API Key (PAT) — GraphQL | **High** — developer audience |
| 7 | **WordPress** | CMS | Post, custom post types | Basic Auth (user/pass) | **High** — massive market share |
| 8 | **Slack** | Team Messaging | Post, comment, channels, profile changes | OAuth 2.0 | **High** — enterprise |
| 9 | **Twitch** | Live Streaming | Post (chat/announcements), comment, refresh | OAuth 2.0 + refresh token | **Low** — niche |
| 10 | **VK** | Russian Social Network | Wall post, comment | OAuth 2.0 + PKCE + refresh | **Low** — region-specific |
| 11 | **Dribbble** | Design Portfolio | Post (shots), teams, analytics | OAuth 2.0 | **Low** — niche |
| 12 | **Kick** | Live Streaming | Post (chat), comment, refresh | OAuth 2.0 + PKCE + refresh | **Low** — niche |
| 13 | **Nostr** | Decentralized Protocol | |  |  |  | Post (Kind 1 notes), comment | Private Key (HEX) | **Low** — Emerging |
| 14 | **Moltbook** | Social Web3 | Post, comment, agent | API Key | **Low** —Web3, Early Stage |
| 15 | **MeWe** | Privacy-focused Social | Posts (timeline/groups), Groups, Photos | Custom Auth | **Low** — Niche |
| 16 | **Lemmy** | Fediverse Link Aggregator | Posts (to communities), Comments, Search | User/Pass → JWT | **Low** — Niche |
| 17 | **Listmonk** | Newsletter | Posts (email campaigns), Lists, Templates | Basic Auth (User/Pass) | **Medium** — Useful Audience |
| 18 | **GMB (Google My Business)** | Local Business Listing | Posts (News/Events/Offers), Pages, Analytics, Post Analytics | OAuth 2.0 (Google) | **High** — Local Businesses |
| 19 | **Whop** | Marketplace | Posts (Forum), Comments, Businesses, Experiences | OAuth 2.0 + PKCE | **Low** — Marketplace |
| 20 | **Farcaster** | Decentralized Social | Posts (Casts), Comments, Channels | Neynar API (signer_uuid) | **Low** — Web3, Early Stage |

---

### 2. PROVIDER FEATURE DEPTH (for shared providers)

Operations that postiz-app offers **that we do not**, even for the providers we both support:

| Provider | postiz-app Features WE DO NOT HAVE | Our Status |
|---|---|---|
| **X/Twitter** | Account Analytics (impressions, likes, retweets), Post Analytics, User Mentions/Search, Auto-repost plugs | ❌ No Analytics, No Mentions |
| **Facebook** | Page Analytics (impressions, engagement, followers, video views), Post Analytics (impressions, clicks, reactions), Reconnect, Name/Nickname Change | ❌ No Analytics, No Nickname Change |
| **Instagram** | Analytics (follower count, reach, likes, views, comments, shares, saves, replies), post analytics, reConnect, import missing posts | ❌ No analytics, no import |
| **Personal LinkedIn** | Company mentions/search, commenting, video/PDF carousel/multi-image support | ❌ No mentions or analytics |
| **LinkedIn Page** | Page analytics (views, followers, clicks, shares, engagement, comments), post analytics, mentions/search | ❌ No analytics |
| **YouTube** | Full video publishing (title, description, tags, privacy, thumbnail, MadeForKids), analytics, post analytics | ❌ **Publishing is a stub** |
| **Pinterest** | Video Pins, board() selection, link/dominant color | ⚠️ Images only, no video |
| **Bluesky** | Publishing (text + images/video, rich text facets), threaded comments, mention/actor search | 🔥 **0 MCP tools** |
| **Reddit** | Commenting, flair, subreddit search | ⚠️ No flair or analytics |
| **Discord** | Mentions (users, roles, @here, @everyone), nickname changes, threaded comments | ⚠️ No mentions |
| **Telegram** | Media groups (up to 10 media items per post), commenting (replies) | ⚠️ Daemon stability issues |

---

### 3. INFRASTRUCTURE AND ARCHITECTURE GAP

| Feature | postiz-app | social-forge | Gap |
|---|---|---|---|
| **Workflow Engine** | Temporal (dedicated orchestrator) for publishing, token refreshing, email, streaks | In-process scheduler (Tokio, 30s interval) | **Medium** — Temporal offers better recovery and compensation. Ours is simpler but adequate |
| **Queue** | Redis (BullMQ) | None | **Low** — not required for MVP |
| **CLI** | `apps/commands`: refresh.tokens, config:check, agent.run | No CLI modules | **Medium** — useful for operational tasks |
| **SDK** | `@postiz/node` — Published TypeScript npm SDK | None | **Low** — the MCP interface serves as the SDK |
| **Browser Extension** | Chrome extension for X/Twitter cookie-based auth | None | **Low** — potentially unnecessary for the OAuth API |
| **Media CDN** | Local, Cloudflare R2, Cloudflare Stream | Local file server only | **Medium** — required for production |
| **Rate Limiter** | Redis + NestJS throttler | In-memory sliding window limiter | ✅ Similar |
| **Token Encryption** | None (tokens stored in plaintext/BCrypt) | AES-256-GCM at rest | ✅ **We are better** |
| **OAuth Auth** | PKCE + Standard Flow | PKCE + Standard Flow | ✅ Similar |
| **Multi-Provider Auth** | Local, GitHub, Google, Farcaster, Wallet, Generic | Email/Password only | **Medium** — Social Login |
| **OAuth App Auth (3rd Party)** | `OAuthApp`/`OAuthAuthorization` — "Sign in with Postiz" | None | **Low** — unnecessary for MVP |

---

### 4. API SURFACE GAP (REST)

| Category | postiz-app Endpoints | social-forge Endpoints | Missing Features |
|---|---|---|---|
| **Posts** | CRUD + Status + Stats + Tags + Comments + AI Generation | CRUD + Scheduling + Publishing + Slot Finder | Tags, Team Comments, Stats, AI Generation |
| **Integrations** | CRUD + Connect + Disconnect + Time + Plugins + Mentions | CRUD + Connect |Login + Logout + Available Pages | Plugins (Post Transformation), Mentions, Timeline |
| **Analytics** | By Integration + By Post + Date Range | **None** | 🔥 **MAJOR Gap** — No analytics data |
| **Media** | Upload + Download + List + Delete + AI Generation | Upload + Download + List | AI Image Generation, Cloudflare R2/Stream |
| **Auto-posting** | RSS Rules → Automatic Posting | **None** | 🔥 **MAJOR Gap** — Feature entirely missing |
| **Webhooks** | CRUD + Post Triggering | **None** | 🔥 **MAJOR Gap** — No outgoing events |
| **Sets** | CRUD for Bulk Post Templates | **None** | Template Feature |
| **Settings** | Profile + Notifications + Signatures + Shortlinks | Profile Only | Signatures, Notification Preferences, Shortlinks |
| **Calendar** | GET by Date Range | GET by Date Range | ✅ Match |
| **Admin** | Error Tracking, Monitoring | **None** | Low Priority |
| **Billing** | Stripe, Subscriptions, Lifetime Deals | **None** | Low Priority (MVP) |
| **Notifications** | In-app, Email, Push | **None** | Medium — Required for UX |
| **Signatures** | CRUD for Automatic Signatures | **None** | Low Priority |
| **Public API** | Public V1 with API Keys | **None** | Low — MCP Interface Can Serve |
| **Third-Party** | HeyGen, ReelFarm | **None** | Low — Niche |
| **Monitoring** | Health/Monitor | ✅ `/health` | ✅ Match |

---

### 5. DATABASE SCHEMA GAP

| Table | postiz-app | social-forge | Missing Fields in social-forge |
|---|---|---|---| | **users** | email, password, name, timezone, provider (enum), inviteId, timezone INT + numerous social columns | email, password, name, timezone INT | No `providerName`, `bio`, `audience`, `pictureId`, `lastOnline`, `inviteId` |
| **integrations** | `internalId`, `providerIdentifier`, `token`, `refreshToken`, `tokenExpiration`, `disabled`, `inBetweenSteps`, `refreshNeeded`, `postingTimes`, `customInstanceDetails`, `rootInternalId`, `additionalSettings` | `internalId`, `providerIdentifier`, `token`, `refreshToken`, `tokenExpiresAt`, `disabled`, `refreshNeeded`, `postingTimes` | ❌ `inBetweenSteps`, `customInstanceDetails`, `rootInternalId`, `additionalSettings` |
| **posts** | content + settings + media + state + publishDate + title + description + releaseURL + error + image + delay + group + intervalInDays + parentPostId | content + settings + media + state + scheduled_at + title + platform_post_url + error_message | ❌ `description`, `delay`, `group` (cross-provider grouping), `intervalInDays` (repetition), `parentPostId` (replies) |
| **media** | name, originalName, path, fileSize, type, thumbnail, alt, thumbnailTimestamp | original_name, storage_path, mime_type, file_size, width, height | ❌ `thumbnail`, `alt`, `type`, `thumbnailTimestamp` |
| **tags** | Name, color, posts relation | **No table** | 🔥 **Feature entirely missing** |
| **comments** | Post comments (team collaboration) | **No table** | 🔥 **Feature entirely missing** |
| **webhooks** | URL, name, integrations relation | **No table** | 🔥 **Feature entirely missing** |
| **notifications** | Content, link, read status | **No table** | 🔥 **Feature entirely missing** |
| **autopost** | Automatic RSS/URL Publishing Configurations | **No table** | 🔥 **Feature entirely missing** |
| **sets** | Grouped Post Templates | **No table** | Feature entirely missing |
| **organization** | Multi-tenancy, API key, Payment info | **No table** | Multi-tenancy missing |
| **oauth_states** | ✅ Present | ✅ Present | ✅ Match |
| **plugs** | Automatic Post Content Transformations | **No table** | Feature entirely missing |

---

### 6. CORE BUSINESS FEATURES — PARITY GAP

#### Features present in postiz-app and missing in social-forge

| Feature | postiz-app | social-forge | Estimated Effort | Priority |
|---|---|---|---|---|
| **📊 Analytics** | Dashboards per platform (FB, IG, LI, YT, TT, PI, TH, X, GMB) with date ranges, trends, per-post analytics | No analytics endpoints or aggregation queries | 2-3 weeks | **High** |
| **👥 Team & Collaboration** | Organizations, roles (USER/ADMIN/SUPERADMIN), email invitations, organization switching | Single-user design | 2-4 weeks | **High** |
| **🔗 Webhooks** | Outbound to any URL, triggered by post/error events | No webhooks | 1 week | **High** |
| **🏷️ Tags** | Colored tags, post filtering by tag, tag management page | No tagging functionality | 3-5 days | **Medium** |
| **🔔 Notifications** | In-app, email (failure/success/streak), dedicated notification channel (SSE) | ✅ SSE present, but no notification storage or email support | 1 week | **Medium** |
| **🤖 AI Agents** | Mastra conversational agent for research and post generation, tone options | No AI agents | 3-4 weeks | **Low** (Premium) |
| **🖼️ AI Image Generation** | 13 AI-generated image styles for posts (via FAL AI) | None | 1-2 weeks | **Low** (Premium) |
| **🔄 Auto-posting** | Automatic posting via RSS feed/URL, hourly repetition | No auto-posting features | 2 weeks | **Medium** |
| **📝 Sets/Templates** | Reusing grouped posts across multiple platforms | No templates | 1 week | **Low** |
| **🔗 Link Shorteners** | Built-in generator + click analytics, preference settings | None | 1 week | **Low** |
| **📋 Signatures** | Customizable signatures, automatically added to posts | None | 2-3 days | **Low** |
| **📄 Public Preview** | Shareable post preview page with multi-platform rendering | None | 1 week | **Medium** |
| **💬 Comments (Team)** | Internal comments on posts (pre-publication) | None | 1 week | **Low** |
| **🗂️ Plugins** | Automatic content transformations (reposting, comment addition, etc.) | None | 2-3 weeks | **Low** |
| **💰 Billing** | Stripe, subscriptions (Standard/Pro/Team/Ultimate), lifetime deals | None | N/A (Out of scope) | **Out of MVP Scope** |
| **🏢 Admin** | Error tracking via Sentry, monitoring | None | 3-5 days | **Low** | ---

### 7. FRONTEND / UI GAP ANALYSIS

**postiz-app:** Full-stack Next.js application featuring 29 component categories, ~35 route files, support for 14 languages, and light/dark themes.
**social-forge:** Separate SvelteKit application (located in `frontend/`) — not evaluated here.

The major UI categories present in `postiz-app` that would require frontend implementation:

| UI Category | Features (postiz-app) | Priority |
|---|---|---|
| **📅 Calendar View** | Day/Week/Month/List views, drag-and-drop, channel sidebar, date filtering, "Today" button | **Critical** |
| **✍️ Composer** | Rich text editor (TipTap), multiple media support, per-channel content, preview, scheduling, recurrence | **Critical** |
| **📊 Analytics** | Dashboards, sparkline charts, date ranges, trends, per-post analytics | **High** |
| **🔌 Channel Management** | Add/Remove channels, status tracking, per-channel settings, client-based grouping | **High** |
| **📂 Media Library** | Grid view, uploads, selection during composition | **High** |
| **⚙️ Settings** | Profile, notifications, signatures, teams, webhooks, API access, preferences | **High** |
| **👥 Team Management** | Roles, invitations, organization switching | **Medium** |
| **🤖 AI Agent** | Chat interface, post generator with search functionality | **Low** |
| **🔗 Webhooks** | CRUD operations, testing | **Medium** |
| **🔄 Auto-posting** | Rule creation, list view | **Medium** |

---

### 8. PRIORITY RECOMMENDATIONS

#### Phase 1 (Immediate — 1 to 2 weeks)

| Task | Effort | Rationale |
|---|---|---|
| **Add Provider Analytics** to REST endpoints | 2–3 days | The `analytics()` / `post_analytics()` traits already return `Vec<Analytics>` by default — the framework exists, but no provider currently implements it |
| **Implement YouTube Publishing** (remove stub) | 0.5 days | A trivial fix that unblocks YouTube publishing |
| **Bluesky MCP Tools** (5 tools: Profile, Timeline, Post Creation, Search, Feed) | 2–3 days | The last remaining "orphan" provider — Bluesky's audience is growing |

#### Phase 2 (Short Term — 2 to 4 Weeks)

| Task | Effort | Rationale |
|---|---|---|
| **TikTok Provider** | 1–2 weeks | The largest missing social platform. OAuth is well-understood, similar to Meta/X |
| **Outbound Webhooks** | 1 week | Required for integrations (Make.com, N8N, Zapier) |
| **Tags** | 3–5 days | Simple user request; improves organization |
| **Blogging Providers** (Medium, Dev.to, Hashnode) | 1 week (combined) | Same auth model (API key), same endpoints; targets a developer audience |

#### Phase 3 (Medium Term — 1 to 2 Months)

| Task | Effort | Rationale |
|---|---|---|
| **Teams & Organizations** | 2–4 weeks | Unlocks multi-user collaboration |
| **Comprehensive Analytics Dashboard** | 2–3 weeks | Completes analytics endpoints, aggregation queries, and caching |
| **Slack Provider** | 1 week | High enterprise value; simple OAuth implementation |
| **GMB (Google My Business)** | 1 week | High value for local businesses; shares Google OAuth with YouTube |
| **Notifications** | 1 week | Essential for UX — storing and querying notifications | #### Phase 4 (Long-term — 2 to 4 months)

| Task | Rationale |
|---|---|
| **Mastodon** | Fediverse growth, similar to Bluesky |In its philosophy |
| **WordPress** | Massive CMS market share |
| **AI Image Generation** | Premium feature |
| **Auto-posting (RSS → Social)** | Automation for content creators |
| **AI Agent** | Differentiating feature |

---

### EXECUTIVE SUMMARY

**social-forge currently features 16 providers with 126+ MCP tools.** Compared to postiz-app (34 providers, full-stack NestJS platform), our feature gap falls into 4 categories:

1. **Providers** — We are missing 20 providers; the highest-priority ones are TikTok, Medium, Dev.to, Hashnode, Slack, and GMB.
2. **Feature Depth** — Even for the 14 shared providers, postiz-app offers analytics, mentions, and comment management capabilities that we currently lack. The most significant gap: **analytics** (no provider currently implements them).
3. **Infrastructure** — postiz-app features a Temporal workflow engine (a robust publishing orchestrator), a CLI application, an SDK, and a browser extension. We have an adequate in-process scheduler, but we lack command-line maintenance tools.
4. **Business Features** — Teams/organizations, webhooks, tags, notifications, analytics reports, and post templates—these represent the primary gaps in achieving "feature completeness" beyond just the provider integrations.
