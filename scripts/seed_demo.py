#!/usr/bin/env python3
"""Seed a social-forge instance with a realistic solo-founder dataset.

An empty-state instance hides every populated-state defect, so this seeds:
  users, integrations (6 providers), posts (draft/queued/published/error/idea),
  campaigns, tags, post_tags, analytics_cache, external_posts + post_engagement,
  events_log, publish_attempts, automation_rules + automation_logs, webhooks,
  signatures, rss_feeds + rss_posts, notifications, post_sets, media, comments.

Storage note: sqlx encodes a `Uuid` column as BLOB(16). A TEXT(36) uuid in
that column fails to decode ("expected 16 bytes, found 36") and every query
against the table errors out. Every id below is written as 16 raw bytes.

Idempotent: deletes every row it owns first, then re-inserts.

Usage: python3 scripts/seed_demo.py <db-path>
"""

import json
import os
import sqlite3
import sys
import time
import uuid

USER = uuid.UUID("22222222-2222-2222-2222-222222222222")
NOW = int(time.time())
DAY = 86_400


def bid(n: int) -> bytes:
    """A deterministic uuid as the 16 bytes sqlx expects."""
    return uuid.UUID(int=n, version=4).bytes


def U() -> bytes:
    return USER.bytes


def iso(ts: int) -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(ts))


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: seed_demo.py <db-path>", file=sys.stderr)
        return 2
    db = sqlite3.connect(sys.argv[1])
    db.execute("PRAGMA foreign_keys=OFF")
    cur = db.cursor()

    for t in [
        "post_engagement", "cached_comments", "resolved_comments",
        "publish_attempts", "automation_logs", "automation_rules",
        "post_plugs", "post_tags", "rss_posts", "rss_feeds",
        "webhook_deliveries", "webhooks", "notifications",
        "post_set_channels", "post_sets", "media",
        "signatures", "brand_profiles", "events_log",
        "analytics_cache", "external_posts", "posts",
        "campaigns", "tags", "api_keys", "integrations",
    ]:
        cur.execute(f"DELETE FROM {t}")
    cur.execute("DELETE FROM users WHERE id <> ?", (U(),))

    # ── user ──
    cur.execute(
        "INSERT OR REPLACE INTO users (id,email,password,name,timezone,"
        "streak_since,streak_days,created_at,updated_at) "
        "VALUES (?,?,?,?,?,?,?,?,?)",
        (U(), "founder@socialforge.local", "!not-a-real-hash",
         "Ishan Parihar", -300, NOW - 120 * DAY, 47, NOW - 120 * DAY, NOW),
    )

    # ── integrations ──
    providers = [
        ("x", "x", "X (Twitter)", "182394471", "@ishanbuilds",
         "https://x.com/ishanbuilds", "cookie",
         '[{"time":480},{"time":570},{"time":810},{"time":1140},{"time":1290}]', 0),
        ("linkedin", "linkedin", "LinkedIn", "urn:li:person:ishanp",
         "Ishan Parihar", "https://linkedin.com/in/ishanp", "oauth",
         '[{"time":540},{"time":750},{"time":1230}]', 0),
        ("bluesky", "bluesky", "Bluesky", "did:plc:ishanforge",
         "ishanforge.bsky.social", "https://bsky.app/profile/ishanforge.bsky.social",
         "app_password", '[{"time":600},{"time":1050},{"time":1320}]', 0),
        ("reddit", "reddit", "Reddit", "ishanforge", "u/ishanforge",
         "https://reddit.com/u/ishanforge", "oauth",
         '[{"time":510},{"time":990}]', 0),
        ("threads", "threads", "Threads", "ishan.builds", "Ishan Builds",
         "https://threads.net/@ishan.builds", "cookie",
         '[{"time":660},{"time":1170}]', 0),
        ("mastodon", "mastodon", "Mastodon", "110471",
         "@ishan@mastodon.social", "https://mastodon.social/@ishan", "oauth",
         '[{"time":570},{"time":1080}]', 1),
    ]
    int_ids = {}
    n = 1
    for key, pid, pname, internal, prof, purl, auth, times, disabled in providers:
        iid = bid(n)
        n += 1
        int_ids[key] = iid
        # X authenticates from a cookie JSON blob in access_token; every other
        # provider takes a bearer token.
        token = '{"auth_token":"seed","ct0":"seed"}' if key == "x" else "seed-token"
        cur.execute(
            "INSERT INTO integrations (id,user_id,provider_identifier,provider_name,"
            "internal_id,root_internal_id,access_token,refresh_token,token_expires_at,"
            "profile_name,profile_picture,profile_url,disabled,refresh_needed,"
            "posting_times,auth_method,created_at,updated_at) "
            "VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            (iid, U(), pid, pname, internal, "", token, "", NOW + 30 * DAY,
             prof, "", purl, disabled, 0, times, auth, NOW - 90 * DAY, NOW),
        )

    # ── campaigns ──
    campaigns = [
        ("launch", "Forge 2.0 launch", "The single-binary social command center.",
         "#6366f1", NOW - 20 * DAY, NOW + 40 * DAY, "Ship Forge 2.0 to 1k waitlist",
         "active", "posts", 24, "engineering", "funding"),
        ("content", "Always-on content", "Weekly build-in-public cadence.",
         "#10b981", NOW - 60 * DAY, NOW + 90 * DAY, "3 posts/week, 5% engagement",
         "active", "engagement", 60, "developers", "education"),
        ("newsletter", "Newsletter growth", "Cross-post the weekly digest.",
         "#f59e0b", NOW - 10 * DAY, NOW + 20 * DAY, "500 subscribers",
         "paused", "followers", 500, "indie hackers", "newsletter"),
    ]
    camp_ids = {}
    for i, (key, name, desc, color, start, end, goal, status, metric, target,
            persona, pillar) in enumerate(campaigns):
        cid = bid(200 + i)
        camp_ids[key] = cid
        cur.execute(
            "INSERT INTO campaigns (id,user_id,name,description,color,start_date,end_date,"
            "goal,status,progress_metric,progress_target,audience_persona,content_pillars,"
            "budget_cents,kpi_targets,deleted_at,sort_order,created_at,updated_at) "
            "VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,NULL,?,?,?)",
            (cid, U(), name, desc, color,
             time.strftime("%Y-%m-%d", time.gmtime(start)),
             time.strftime("%Y-%m-%d", time.gmtime(end)),
             goal, status, metric, target,
             json.dumps({"age_range": "25-44", "location": "Global",
                         "interests": ["rust", "ai", "devtools"],
                         "pain_points": "too many tabs, no scheduling"}),
             json.dumps([{"title": pillar, "description": f"Core {pillar} content",
                          "tags": [pillar, "buildinpublic"]}]),
             0, json.dumps({"min_engagement_rate": 3.5, "min_reach": 1000,
                            "target_clicks": 250}),
             i, NOW - 20 * DAY, NOW),
        )

    # ── tags ──
    tag_defs = [
        ("launch", "launch", "#6366f1"),
        ("buildinpublic", "buildinpublic", "#10b981"),
        ("rust", "rust", "#f97316"),
        ("ai", "ai", "#a855f7"),
        ("tutorial", "tutorial", "#0ea5e9"),
        ("newsletter", "newsletter", "#f59e0b"),
    ]
    tag_ids = {}
    for i, (key, name, color) in enumerate(tag_defs):
        tid = bid(300 + i)
        tag_ids[key] = tid
        cur.execute(
            "INSERT INTO tags (id,user_id,name,color,created_at,updated_at) "
            "VALUES (?,?,?,?,?,?)",
            (tid, U(), name, color, NOW - 60 * DAY, NOW),
        )

    # ── posts ──
    post_defs = [
        ("published", "x",
         "Forge 2.0 ships today. One binary. No Docker. No Postgres. "
         "Everything a solo founder needs to run 30 channels from one VPS.",
         "Forge 2.0 ships", 28, 9, "launch", ["launch", "rust"], [],
         '{"time":480}', "19473829201",
         "https://x.com/ishanbuilds/status/19473829201", None, None, 0),
        ("published", "linkedin",
         "I spent two years building orchestration for social posting. "
         "I no longer believe in it. A single Rust binary with SQLite does more for a "
         "solo founder than a Temporal cluster ever will.",
         "Why I deleted my orchestrator", 24, 10, "content", ["buildinpublic"], [],
         "{}", "", "https://linkedin.com/feed/update/urn:li:activity:729381", None, None, 0),
        ("published", "bluesky",
         "Reddit's OAuth scopes are still a mess in 2026. "
         "Cookieless is a lie; you need the app password flow.",
         "", 21, 11, "content", ["rust"], [],
         "{}", "3kfm2q7x1z9",
         "https://bsky.app/profile/ishanforge.bsky.social/post/3kfm2q7x1z9", None, None, 0),
        ("published", "x",
         "The character count bug that shipped to 1,200 people: "
         "the browser counted UTF-16 code units, the server counted UTF-8 bytes. "
         "One emoji = 2 units = 4 bytes. The composer said 200/280. The server said 800.",
         "The emoji character count bug", 18, 14, "content", ["rust", "buildinpublic"], [],
         '{"time":1290}', "19481005512",
         "https://x.com/ishanbuilds/status/19481005512", None, None, 0),
        ("published", "reddit",
         "I built a social media scheduler that is one Rust binary and "
         "one SQLite file. No Redis, no Postgres, no workers. AMA about the architecture.",
         "Ask me anything about single-binary architecture", 14, 8, "content",
         ["buildinpublic"], [], '{"subreddit":"rust","title":"Ask me anything"}',
         "t3_1k2m3n", "https://reddit.com/r/rust/comments/1k2m3n", None, None, 0),
        ("published", "threads",
         "Day 94 of building in public. Zero infra costs, "
         "one VPS, one binary, 30 channels. This is the whole stack.",
         "", 11, 13, "content", ["buildinpublic"], [],
         "{}", "C3kQm2x9p1", "https://threads.net/@ishan.builds/post/C3kQm2x9p1", None, None, 0),
        ("published", "x",
         "Postiz is a great reference implementation, but three of its "
         "features are actively wrong for a solo founder: teams, billing, and an agent chat "
         "box. You do not need an org chart to post on LinkedIn.",
         "What a solo founder should not copy", 8, 16, "content", ["ai"], [],
         '{"time":480}', "19492003341",
         "https://x.com/ishanbuilds/status/19492003341", None, None, 0),
        ("published", "linkedin",
         "The 3am test: if your tool needs a database server to run, "
         "it is not a tool, it is a platform you have to babysit.",
         "The 3am test for tools", 5, 8, "content", ["buildinpublic"], [],
         "{}", "", "https://linkedin.com/feed/update/urn:li:activity:729500112", None, None, 0),
        ("published", "bluesky",
         "New: the composer now counts characters the way the "
         "server does, so the number you see is the number the server enforces.",
         "", 3, 15, "launch", ["launch", "rust"], [],
         "{}", "3kfm3a11c7",
         "https://bsky.app/profile/ishanforge.bsky.social/post/3kfm3a11c7", None, None, 0),
        ("published", "x",
         "Shipped: drag-and-drop calendar, cross-posting with per-platform "
         "content, and an AI assistant that writes in your voice.",
         "Forge 2.0 features", 2, 10, "launch", ["launch"], [],
         '{"time":810}', "19493110556",
         "https://x.com/ishanbuilds/status/19493110556", None, None, 0),
        ("published", "x",
         "Every post in this thread is a real API call. No mock data, "
         "no retry queue that silently drops errors, no orphan rows in a bucket.",
         "", 1, 11, "launch", ["launch", "buildinpublic"], [],
         '{"time":570}', "19494002288",
         "https://x.com/ishanbuilds/status/19494002288",
         "Every reply to this thread is generated by the same scheduler that published it.",
         None, 0),
        ("queued", "x",
         "Scheduled for the morning slot: the case for boring infrastructure "
         "in a solo founder's stack.",
         "Boring infrastructure", 0, 0, "content", ["rust"], [],
         '{"time":480}', "", "", None, None, 0),
        ("queued", "linkedin",
         "The weekly digest goes out at 09:00 in my timezone, not the "
         "browser's timezone. That bug cost me two posts.",
         "The weekly digest", 0, 0, "newsletter", ["newsletter"], [],
         "{}", "", "", None, None, 0),
        ("draft", "x",
         "Draft: a teardown of five Rust web frameworks and why I picked axum. "
         "Not done yet, the benchmark section is thin.",
         "Why axum", 0, 0, "content", ["rust"], [],
         '{"time":1140}', "", "", None, None, 0),
        ("draft", "bluesky",
         "Idea: a post about how a single-column calendar layout "
         "beats a week grid for one person.",
         "", 0, 0, None, ["tutorial"], [],
         "{}", "", "", None, None, 0),
        ("error", "reddit",
         "Scheduled to r/rust but the subreddit setting was missing, "
         "so the publish failed with \"Missing subreddit in settings\".",
         "The missing subreddit bug", 4, 0, None, ["rust"], [],
         '{"subreddit":""}', "", "", None, None, 0),
        ("idea", "x",
         "Idea: a thread on the economics of running 30 channels from one VPS.",
         "", 0, 0, None, ["buildinpublic"], [],
         "{}", "", "", None, None, 0),
    ]

    post_ids = []
    for i, (state, key, content, title, days_ago, hour, camp, tags, media,
            settings, ppid, purl, fcomment, group, seq) in enumerate(post_defs):
        pid = bid(1000 + i)
        post_ids.append(pid)
        iid = int_ids[key]
        created = NOW - days_ago * DAY - 3 * 3600
        if state == "published":
            sched = NOW - days_ago * DAY
            pub = sched
        elif state in ("queued", "error"):
            sched = NOW - days_ago * DAY
            pub = None
        else:
            sched = None
            pub = None
        cur.execute(
            "INSERT INTO posts (id,user_id,integration_id,state,content,title,media,settings,"
            "scheduled_at,published_at,platform_post_id,platform_post_url,error_message,"
            "first_comment,group_id,sequence,retry_count,campaign_id,kanban_sort_order,"
            "kanban_substate,due_date,priority,idempotency_key,created_at,updated_at) "
            "VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            (pid, U(), iid, state, content, title, json.dumps(media), settings,
             sched, pub, ppid, purl,
             "Missing subreddit in settings" if state == "error" else "",
             fcomment, group, seq, 3 if state == "error" else 0,
             camp_ids.get(camp) if camp else None,
             i * 10,
             "ready_to_publish" if state == "queued" else None,
             (NOW + 2 * DAY) if state == "queued" else None,
             ["low", "medium", "high", "urgent"][i % 4],
             bid(5000 + i), created, created),
        )
        for t in tags:
            cur.execute("INSERT INTO post_tags (post_id,tag_id) VALUES (?,?)",
                        (pid, tag_ids[t]))
        if state == "published":
            cur.execute(
                "INSERT INTO publish_attempts (id,post_id,attempt_number,status,"
                "error_message,started_at,finished_at,created_at) VALUES (?,?,?,?,?,?,?,?)",
                (bid(2000 + i), pid, 1, "success", None,
                 NOW - days_ago * DAY - 5, NOW - days_ago * DAY - 2,
                 NOW - days_ago * DAY - 5),
            )

    # ── events_log ──
    ev_defs = [
        ("post_published", {"title": "Forge 2.0 ships", "provider": "x",
                            "post_id": str(uuid.UUID(bytes=post_ids[0]))}, 28 * DAY),
        ("post_published", {"title": "Why I deleted my orchestrator", "provider": "linkedin",
                            "post_id": str(uuid.UUID(bytes=post_ids[1]))}, 24 * DAY),
        ("post_published", {"title": "The emoji character count bug", "provider": "x",
                            "post_id": str(uuid.UUID(bytes=post_ids[3]))}, 18 * DAY),
        ("post_published", {"title": "What a solo founder should not copy", "provider": "x",
                            "post_id": str(uuid.UUID(bytes=post_ids[6]))}, 8 * DAY),
        ("post_published", {"title": "Forge 2.0 features", "provider": "x",
                            "post_id": str(uuid.UUID(bytes=post_ids[9]))}, 2 * DAY),
        ("post_failed", {"error": "Missing subreddit in settings", "provider": "reddit",
                         "post_id": str(uuid.UUID(bytes=post_ids[15]))}, 4 * DAY),
        ("post_scheduled", {"title": "Boring infrastructure", "provider": "x",
                            "post_id": str(uuid.UUID(bytes=post_ids[11]))}, 2 * 3600),
        ("post_created", {"provider": "x",
                          "post_id": str(uuid.UUID(bytes=post_ids[12]))}, 90 * 60),
        ("post_created", {"provider": "bluesky",
                          "post_id": str(uuid.UUID(bytes=post_ids[13]))}, 60 * 60),
        ("post_deleted", {"title": "A post I deleted", "provider": "x"}, 30 * 3600),
        ("post_stage_changed", {"title": "Forge 2.0 features", "provider": "x",
                                "post_id": str(uuid.UUID(bytes=post_ids[9]))}, 3 * 3600),
        ("integration_connected", {"name": "X (Twitter)", "provider": "x"}, 90 * DAY),
        ("integration_connected", {"name": "Reddit", "provider": "reddit"}, 80 * DAY),
        ("integration_disconnected", {"name": "Mastodon", "provider": "mastodon"}, 12 * DAY),
        ("first_comment_failed", {"provider": "x",
                                  "post_id": str(uuid.UUID(bytes=post_ids[10]))}, 20 * 3600),
    ]
    for i, (etype, payload, ago) in enumerate(ev_defs):
        cur.execute(
            "INSERT INTO events_log (id,user_id,event_type,payload,created_at) "
            "VALUES (?,?,?,?,?)",
            (bid(3000 + i), U(), etype, json.dumps(payload), NOW - ago),
        )

    # ── analytics_cache (provider-shaped series) ──
    def series(label, base, step, n=10):
        return {"label": label, "data": [
            {"total": base + d * step, "date": iso(NOW - (n - d) * DAY)} for d in range(n)]}

    caches = [
        ("x", "19494002288", [series("Impressions", 1200, 90),
                              series("Likes", 34, 4),
                              series("Reposts", 6, 1),
                              series("Replies", 3, 0)]),
        ("linkedin", "urn:li:activity:729500112", [series("Impressions", 800, 60),
                                                  series("Reactions", 20, 3),
                                                  series("Comments", 2, 0)]),
        ("bluesky", "3kfm3a11c7", [series("Impressions", 240, 30),
                                   series("Likes", 11, 2),
                                   series("Reposts", 1, 0)]),
    ]
    for i, (key, ppid, sers) in enumerate(caches):
        cur.execute(
            "INSERT INTO analytics_cache (id,user_id,provider,platform_post_id,data,"
            "cached_at,expires_at) VALUES (?,?,?,?,?,?,?)",
            # The expiry is a YEAR out, not an hour. The cleaner deletes
            # the expired rows every 30 minutes and the refresher can
            # only re-create them with a LIVE provider token - so a
            # one-hour expiry means the seeded instance's own-post
            # engagement silently decays to zero and the analytics page
            # shows "0.0 interactions per post" against real posts.
            (bid(4000 + i), U(), key, ppid, json.dumps(sers), NOW - 3600, NOW + 365 * 86400),
        )

    # ── external_posts + post_engagement ──
    ext_defs = [
        ("x", "1877001112223334445",
         "Rust is the best thing to happen to backend "
         "development in a decade. Async without a runtime tax, fearless concurrency, "
         "and a compiler that catches your bugs before you deploy.",
         "Rustacean", "@rustacean", "https://x.com/rustacean/status/1877001112223334445", 3),
        ("x", "1877001112223334446",
         "Your database is not slow because of the database. "
         "It is slow because you are doing N+1 queries in your ORM and calling it a "
         "feature. Measure first.",
         "Simon", "@simonw", "https://x.com/simonw/status/1877001112223334446", 6),
        ("x", "1877001112223334447",
         "Shipped a rewrite of our scheduler in Rust. "
         "40k lines of TypeScript gone, one binary, 12ms p99. The whole thing is 9MB.",
         "Nadia", "@nadia_dev", "https://x.com/nadia_dev/status/1877001112223334447", 9),
        ("linkedin", "urn:li:ugcPost:729001",
         "Unpopular opinion: the single biggest "
         "productivity gain from AI tooling is not writing faster, it is having fewer "
         "tabs open.",
         "Priya Raman", "@priya_builds",
         "https://linkedin.com/feed/update/urn:li:ugcPost:729001", 12),
        ("linkedin", "urn:li:ugcPost:729002",
         "We cut our social media tool spend from "
         "$400/month to $12/month by replacing a SaaS suite with one script. It took a "
         "weekend. Nobody believes me until they try it.",
         "Marcus Chen", "@mchen",
         "https://linkedin.com/feed/update/urn:li:ugcPost:729002", 15),
        ("bluesky", "3kfm1aaa111",
         "single binary, single file, single owner. "
         "the best infra is the infra you do not have to think about",
         "kiro", "@kiro.bsky.social",
         "https://bsky.app/profile/kiro.bsky.social/post/3kfm1aaa111", 18),
        ("bluesky", "3kfm1aaa222",
         "spent the weekend moving off a queue-based job "
         "runner. the sqlite write-ahead log does the same thing and I can read the "
         "source in one sitting",
         "sol", "@sol.bsky.social",
         "https://bsky.app/profile/sol.bsky.social/post/3kfm1aaa222", 21),
        ("reddit", "t3_1abc2345",
         "After 15 years of backend work I finally understand "
         "why Rust's borrow checker is a feature, not a punishment. It is the only "
         "toolchain that tells you the truth about your program's aliasing.",
         "u/rustdev", "@rustdev", "https://reddit.com/r/rust/comments/1abc2345", 25),
        ("reddit", "t3_1abc2346",
         "PSA: if your deployment needs 3 containers and a "
         "managed Postgres to serve a static page, you are solving a problem you "
         "invented.",
         "u/devops", "@devops", "https://reddit.com/r/devops/comments/1abc2346", 30),
    ]
    for i, (prov, ppid, text, name, handle, url, days) in enumerate(ext_defs):
        eid = bid(5000 + i)
        cur.execute(
            "INSERT INTO external_posts (id,user_id,provider,platform_post_id,text,"
            "author_name,author_handle,author_avatar,created_at,url,media,metadata,"
            "imported_at,hidden_at,saved_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            (eid, U(), prov, ppid, text, name, handle, "",
             NOW - days * DAY, url, "[]", "{}", NOW - days * DAY + 3600, None, None),
        )
        likes = 40 + (len(ext_defs) - i) * 17
        cur.execute(
            "INSERT INTO post_engagement (id,post_id,likes,comments,shares,views,saves,"
            "quotes,reposts,replies,reactions,upvotes,downvotes,upvote_ratio,awards,raw,"
            "fetched_at,created_at,updated_at,impressions) "
            "VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            (bid(6000 + i), eid, likes, 3 + i, 2 + i, likes * 30,
             1 + i, i % 3, 1 + i, 2 + i, "{}", 0, 0, None, 0, "{}",
             NOW - 3600, NOW - 3600, NOW - 3600, likes * 12),
        )

    # ── automation_rules + automation_logs ──
    rules = [
        ("x", "Auto-reply to launch mentions", '{"keyword":"forge"}',
         "Thanks for trying Forge! Here is the quickstart: ...",
         "template", 1, 30, 10),
        ("linkedin", "Comment welcome on new followers", "{}",
         "Thanks for following! Here is what I am building: ...",
         "ai_generated", 1, 60, 5),
        ("bluesky", "Reply to questions about Rust", '{"keyword":"rust"}',
         "Short answer: it depends on your team size.",
         "ai_generated", 0, 120, 3),
    ]
    rule_ids = []
    for i, (key, name, trig, resp, rtype, active, cool, maxr) in enumerate(rules):
        rid = bid(7000 + i)
        rule_ids.append(rid)
        cur.execute(
            "INSERT INTO automation_rules (id,user_id,integration_id,name,trigger_type,"
            "trigger_filter,response_template,response_type,ai_model,is_active,"
            "cooldown_minutes,max_responses_per_hour,created_at,updated_at) "
            "VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            (rid, U(), int_ids[key], name, "comment", trig, resp, rtype,
             "gpt-4o-mini" if rtype == "ai_generated" else None,
             active, cool, maxr, NOW - 30 * DAY, NOW - 30 * DAY),
        )
    log_defs = [
        (0, "sent", None, 3 * 3600),
        (0, "sent", None, 6 * 3600),
        (0, "skipped_cooldown", None, 8 * 3600),
        (1, "failed", "AI provider returned 429 Too Many Requests", 5 * 3600),
        (1, "sent", None, 9 * 3600),
        (1, "skipped_limit", None, 11 * 3600),
    ]
    for i, (ri, status, err, ago) in enumerate(log_defs):
        cur.execute(
            "INSERT INTO automation_logs (id,rule_id,trigger_id,trigger_type,response,"
            "status,error_message,created_at) VALUES (?,?,?,?,?,?,?,?)",
            (bid(8000 + i), rule_ids[ri], f"c-{i}", "comment",
             "Thanks for trying Forge!" if status == "sent" else None,
             status, err, NOW - ago),
        )

    # ── webhooks ──
    hooks = [
        ("Postiz bridge", "https://hooks.example.com/social-forge", "whsec_abc123", 1,
         2 * 3600),
        ("Zapier catch", "https://hooks.zapier.com/hooks/catch/1234/xyz", None, 1, None),
        ("Old webhook", "https://legacy.example.com/hook", None, 0, 40 * DAY),
    ]
    for i, (name, url, secret, active, last) in enumerate(hooks):
        cur.execute(
            "INSERT INTO webhooks (id,user_id,name,url,secret,is_active,last_triggered_at,"
            "created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?)",
            (bid(9000 + i), U(), name, url, secret, active, last, NOW - 30 * DAY, NOW),
        )

    # ── signatures ──
    sigs = [
        ("Default", "— Ish\nBuilding in public. One binary, many channels.", None, 1),
        ("Newsletter", "Get The Weekly Digest: one post, every Friday. No spam.", "x", 0),
    ]
    for i, (name, content, prov, isdef) in enumerate(sigs):
        cur.execute(
            "INSERT INTO signatures (id,user_id,name,content,provider,is_default,created_at,"
            "updated_at) VALUES (?,?,?,?,?,?,?,?)",
            (bid(10000 + i), U(), name, content, prov, isdef, NOW - 60 * DAY, NOW),
        )

    # ── rss_feeds + rss_posts ──
    feeds = [
        ("x", "https://blog.rust-lang.org/feed.xml", "Rust Blog", 1, 0),
        ("linkedin", "https://news.ycombinator.com/rss", "Hacker News Front Page", 1, 1),
        ("bluesky", "https://techcrunch.com/feed/", "TechCrunch", 0, 0),
    ]
    feed_ids = []
    for i, (key, url, title, enabled, use_ai) in enumerate(feeds):
        fid = bid(11000 + i)
        feed_ids.append(fid)
        cur.execute(
            "INSERT INTO rss_feeds (id,user_id,feed_url,integration_id,title,last_polled_at,"
            "poll_interval_min,enabled,use_ai_summary,created_at,updated_at) "
            "VALUES (?,?,?,?,?,?,?,?,?,?,?)",
            (fid, U(), url, int_ids[key], title, NOW - 3600, 30, enabled, use_ai,
             NOW - 30 * DAY, NOW),
        )
    rss_defs = [
        (0, "Announcing Rust 1.90.0",
         "https://blog.rust-lang.org/2026/10/announcing-rust-1-90-0", 2 * DAY),
        (0, "Rust 1.90.0 stabilizes async closures",
         "https://blog.rust-lang.org/2026/10/async-closures", 5 * DAY),
        (1, "Show HN: I built a single-binary social scheduler",
         "https://news.ycombinator.com/item?id=41982233", 1 * DAY),
        (1, "Ask HN: what is your smallest production stack?",
         "https://news.ycombinator.com/item?id=41981122", 3 * DAY),
        (2, "TechCrunch: solo founders are ditching the SaaS stack",
         "https://techcrunch.com/2026/10/solo-saas", 4 * DAY),
    ]
    for i, (fi, title, url, days) in enumerate(rss_defs):
        cur.execute(
            "INSERT INTO rss_posts (id,feed_id,post_id,guid,title,url,published_at,"
            "content_hash,is_imported,created_at) VALUES (?,?,?,?,?,?,?,?,?,?)",
            (bid(12000 + i), feed_ids[fi], None, f"guid-{i}", title, url,
             NOW - days * DAY, f"hash-{i}", 0, NOW - days * DAY),
        )

    # ── notifications ──
    notifs = [
        ("Post published to X", "Forge 2.0 ships was published to X.",
         "post_published", "post", str(uuid.UUID(bytes=post_ids[0])), 0, 28 * DAY),
        ("Publish failed on Reddit", "Missing subreddit in settings", "post_failed",
         "post", str(uuid.UUID(bytes=post_ids[15])), 0, 4 * DAY),
        ("Comment received", "Rustacean replied to your post.", "comment_received",
         "post", str(uuid.UUID(bytes=post_ids[0])), 0, 20 * 3600),
        ("Automation hit its rate limit", "LinkedIn automation skipped 1 reply.",
         "automation", None, None, 1, 11 * 3600),
        ("New feed item", "Rust 1.90.0 is ready to import.", "feed_new", "rss", None, 0, 2 * DAY),
    ]
    for i, (title, body, ntype, rtype, rid, read, ago) in enumerate(notifs):
        cur.execute(
            "INSERT INTO notifications (id,user_id,title,body,notification_type,"
            "reference_type,reference_id,is_read,created_at) VALUES (?,?,?,?,?,?,?,?,?)",
            (bid(13000 + i), U(), title, body, ntype, rtype, rid, read, NOW - ago),
        )

    # ── post_sets ──
    sets = [
        ("Weekly build log", "The standard 3-post weekly cadence.",
         '{"variants": [{"name":"short","content":"Built X today. One binary."}]}',
         ["x", "bluesky"]),
        ("Launch announcements", "Every launch post, reworked per platform.",
         '{"variants": [{"name":"launch","content":"Shipped today: {feature}"}]}',
         ["x", "linkedin", "threads"]),
    ]
    for i, (name, desc, content, chans) in enumerate(sets):
        sid = bid(14000 + i)
        cur.execute(
            "INSERT INTO post_sets (id,user_id,name,description,content,created_at,updated_at) "
            "VALUES (?,?,?,?,?,?,?)",
            (sid, U(), name, desc, content, NOW - 40 * DAY, NOW),
        )
        for ch in chans:
            # `post_set_channels.channel_id` is a TEXT uuid string (the
            # handler binds `channel_id.to_string()` and reads it back as a
            # String), while `post_set_id` is a real `Uuid` column.
            cur.execute("INSERT INTO post_set_channels (post_set_id,channel_id) VALUES (?,?)",
                        (sid, str(uuid.UUID(bytes=int_ids[ch]))))

    # ── media ──
    # The bytes are written to disk so `/api/media/{id}` actually serves
    # them. A media row whose file is missing returns a 404, and a page
    # full of broken images is a defect the probe reports - the seed
    # would be manufacturing a false finding.
    media_dir = os.path.join(os.path.dirname(os.path.abspath(sys.argv[1])), "..", "uploads")
    media_dir = os.path.normpath(media_dir)
    os.makedirs(media_dir, exist_ok=True)

    def png_bytes(w, h, rgb):
        """A minimal valid PNG, so the browser can decode it."""
        import struct
        import zlib

        def chunk(tag, data):
            c = tag + data
            return struct.pack(">I", len(data)) + c + struct.pack(">I", zlib.crc32(c) & 0xFFFFFFFF)

        raw = b"".join(
            b"\x00" + bytes(rgb) * w for _ in range(h)
        )
        return (
            b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw))
            + chunk(b"IEND", b"")
        )

    for i, (name, mime, size, rgb) in enumerate([
        ("launch-banner.png", "image/png", 184320, (99, 102, 241)),
        ("architecture.png", "image/png", 251904, (16, 185, 129)),
    ]):
        mid = bid(15000 + i)
        fname = f"{mid.hex()}.png"
        with open(os.path.join(media_dir, fname), "wb") as fh:
            fh.write(png_bytes(1200, 630, rgb))
        cur.execute(
            "INSERT INTO media (id,user_id,original_name,storage_path,mime_type,file_size,"
            "width,height,created_at,alt_text) VALUES (?,?,?,?,?,?,?,?,?,?)",
            (mid, U(), name, fname, mime, size, 1200, 630, NOW - (i + 1) * DAY,
             "The Forge 2.0 launch banner showing the command center dashboard"),
        )

    # A video row: the file is a placeholder, so it is NOT seeded - a
    # media row that 404s is a false finding. The row is seeded without
    # a file only if the storage backend can serve a placeholder; the
    # local backend cannot, so the video is left out.

    # ── cached_comments (comments on our own published posts) ──
    for i, (own, provider, name, handle, text, days) in enumerate([
        (post_ids[0], "x", "Ada", "@ada_l",
         "This is the first tool I've used that doesn't need a Docker Compose file. Impressive.", 20),
        (post_ids[0], "x", "Linus T", "@linus_t",
         "How is the scheduler handling retries without a queue?", 19),
        (post_ids[1], "linkedin", "Grace", "Grace H",
         "The 3am test is going straight into my notes.", 4),
        (post_ids[9], "bluesky", "Mira", "@mira",
         "Shipped the composer count fix. Thank you.", 2),
    ]):
        cur.execute(
            "INSERT INTO cached_comments (id,user_id,comment_id,post_id,provider,"
            "author_name,author_handle,author_avatar,text,created_at,fetched_at) "
            "VALUES (?,?,?,?,?,?,?,?,?,?,?)",
            (16000 + i, U(), f"cmt-{i}", own, provider, name, handle,
             "", text, NOW - days * DAY, NOW - 3600),
        )

    # ── brand profile ──
    cur.execute(
        "INSERT INTO brand_profiles (user_id,brand_name,description,tone_of_voice,audience,"
        "content_pillars,keywords,hashtag_sets,avoid_topics,posting_frequency,"
        "posts_per_day_goal,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)",
        (U(), "Ishan Builds",
         "One binary, many channels. Build-in-public notes from a "
         "solo founder shipping a social media tool.",
         "direct, technical, no hype", "solo founders and developers",
         json.dumps([{"title": "Engineering", "description": "How the thing is built"},
                     {"title": "Lessons", "description": "What broke and what it cost"}]),
         json.dumps(["rust", "axum", "sqlite", "solo founder", "buildinpublic"]),
         json.dumps([{"name": "core", "tags": ["#buildinpublic", "#rust"]}]),
         json.dumps(["crypto", "web3 airdrops", "engagement bait"]),
         "daily", 1.5, NOW - 60 * DAY, NOW),
    )

    db.commit()

    # ── self-check ──
    # A TEXT uuid in a column sqlx decodes as `Uuid` fails at runtime with
    # "invalid length: expected 16 bytes, found 36" and takes the whole
    # endpoint down, so the seed asserts every uuid column it wrote is
    # BLOB(16).
    uuid_cols = {
        "users": ["id"],
        "integrations": ["id", "user_id"],
        "campaigns": ["id", "user_id"],
        "tags": ["id", "user_id"],
        "posts": ["id", "user_id", "integration_id", "idempotency_key",
                  "campaign_id", "group_id"],
        "post_tags": ["post_id", "tag_id"],
        "events_log": ["id", "user_id"],
        "publish_attempts": ["id", "post_id"],
        "analytics_cache": ["id", "user_id"],
        "external_posts": ["id", "user_id"],
        "post_engagement": ["id", "post_id"],
        "automation_rules": ["id", "user_id", "integration_id"],
        "automation_logs": ["id", "rule_id"],
        "webhooks": ["id", "user_id"],
        "signatures": ["id", "user_id"],
        "rss_feeds": ["id", "user_id", "integration_id"],
        "rss_posts": ["id", "feed_id", "post_id"],
        "notifications": ["id", "user_id"],
        "post_sets": ["id", "user_id"],
        "post_set_channels": ["post_set_id"],
        "media": ["id", "user_id"],
        "cached_comments": ["user_id", "post_id"],
        "brand_profiles": ["user_id"],
    }
    bad = []
    for table, cols in uuid_cols.items():
        for col in cols:
            rows = cur.execute(
                f'SELECT DISTINCT typeof("{col}") FROM "{table}" WHERE "{col}" IS NOT NULL'
            ).fetchall()
            for (ty,) in rows:
                if ty != "blob":
                    bad.append(f"{table}.{col} is {ty}, expected blob")
    if bad:
        db.rollback()
        print("SEED FAILED - uuid columns have the wrong storage type:", file=sys.stderr)
        for b in bad:
            print("  " + b, file=sys.stderr)
        return 1

    counts = {
        "integrations": 6, "posts": len(post_defs), "campaigns": len(campaigns),
        "tags": len(tag_defs), "events": len(ev_defs), "external": len(ext_defs),
        "analytics_cache": len(caches), "rules": len(rules), "webhooks": len(hooks),
        "rss_feeds": len(feeds), "rss_posts": len(rss_defs), "notifications": len(notifs),
        "post_sets": len(sets), "media": 2, "comments": 4,
    }
    print("seeded:", counts)
    return 0


if __name__ == "__main__":
    sys.exit(main())
