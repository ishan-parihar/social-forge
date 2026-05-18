#!/bin/bash

# --- CONFIG ---
SERVER_URL="http://localhost:3001"
# Use IDs from previous successful tests
FB_PAGE_ID="100064413228585" # Example ID, will try to fetch from DB if this fails
IG_ID="17841400680408909"    # designaesthetics.co.in

# --- HELPERS ---
call_tool() {
    local tool=$1
    local args=$2
    echo "Testing tool: $tool ..."
    response=$(curl -s -X POST "$SERVER_URL/mcp/v1/tools/call" \
        -H "Content-Type: application/json" \
        -d "{\"name\": \"$tool\", \"arguments\": $args}")
    
    if [[ $response == *"error"* ]]; then
        echo "❌ FAILED: $response"
        return 1
    else
        echo "✅ SUCCESS"
        echo "$response"
        return 0
    fi
}

# --- FACEBOOK CASCADE ---
echo "=== STARTING FACEBOOK AUDIT ==="
# 1. Feed
if call_tool "fb_get_feed" "{\"page_id\": \"$FB_PAGE_ID\"}"; then
    # Try to extract a post_id from the feed
    POST_ID=$(echo "$response" | grep -oP '"id":\s*"\K[^"]+' | head -n 1)
    if [ -n "$POST_ID" ]; then
        echo "Found Post ID: $POST_ID"
        call_tool "fb_get_post" "{\"page_id\": \"$FB_PAGE_ID\", \"post_id\": \"$POST_ID\"}"
        call_tool "fb_get_comments" "{\"page_id\": \"$FB_PAGE_ID\", \"post_id\": \"$POST_ID\"}"
        call_tool "fb_react" "{\"page_id\": \"$FB_PAGE_ID\", \"post_id\": \"$POST_ID\", \"reaction_type\": \"LIKE\"}"
        call_tool "fb_comment" "{\"page_id\": \"$FB_PAGE_ID\", \"post_id\": \"$POST_ID\", \"message\": \"Audit test comment\"}"
        call_tool "fb_delete_post" "{\"page_id\": \"$FB_PAGE_ID\", \"post_id\": \"$POST_ID\"}"
    else
        echo "No posts found in feed to test post-specific tools."
    fi
fi

# Create and Delete (Isolated)
call_tool "fb_create_post" "{\"page_id\": \"$FB_PAGE_ID\", \"message\": \"Full Audit Test Post\"}"
call_tool "fb_create_photo" "{\"page_id\": \"$FB_PAGE_ID\", \"url\": \"https://placehold.co/600x400\", \"caption\": \"Audit photo\"}"
call_tool "fb_create_video" "{\"page_id\": \"$FB_PAGE_ID\", \"file_url\": \"https://www.w3schools.com/html/mov_bbb.mp4\", \"title\": \"Audit Video\"}"
call_tool "fb_page_insights" "{\"page_id\": \"$FB_PAGE_ID\", \"metric\": \"page_post_engagements\"}"
call_tool "fb_albums" "{\"page_id\": \"$FB_PAGE_ID\"}"
call_tool "fb_search_pages" "{\"query\": \"Social Forge\"}"
call_tool "fb_conversations" "{\"page_id\": \"$FB_PAGE_ID\"}"

# --- INSTAGRAM CASCADE ---
echo "=== STARTING INSTAGRAM AUDIT ==="
# 1. Media
if call_tool "ig_get_media" "{\"ig_id\": \"$IG_ID\"}"; then
    MEDIA_ID=$(echo "$response" | grep -oP '"id":\s*"\K[^"]+' | head -n 1)
    if [ -n "$MEDIA_ID" ]; then
        echo "Found Media ID: $MEDIA_ID"
        call_tool "ig_get_media_detail" "{\"ig_id\": \"$IG_ID\", \"media_id\": \"$MEDIA_ID\"}"
        call_tool "ig_get_comments" "{\"ig_id\": \"$IG_ID\", \"media_id\": \"$MEDIA_ID\"}"
        # Try to reply to first comment
        COMMENT_ID=$(echo "$response" | grep -oP '"id":\s*"\K[^"]+' | head -n 1)
        if [ -n "$COMMENT_ID" ]; then
             call_tool "ig_reply_to_comment" "{\"ig_id\": \"$IG_ID\", \"comment_id\": \"$COMMENT_ID\", \"message\": \"Audit reply\"}"
        fi
    else
        echo "No media found in IG feed."
    fi
fi

# Other tools
call_tool "ig_get_reels" "{\"ig_id\": \"$IG_ID\"}"
call_tool "ig_get_stories" "{\"ig_id\": \"$IG_ID\"}"
call_tool "ig_get_followers" "{\"ig_id\": \"$IG_ID\"}"
call_tool "ig_business_discovery" "{\"ig_id\": \"$IG_ID\", \"target_username\": \"instagram\"}"
call_tool "ig_get_insights" "{\"ig_id\": \"$IG_ID\", \"metric\": \"reach\"}"
call_tool "ig_get_insights_audience" "{\"ig_id\": \"$IG_ID\"}"
call_tool "ig_get_tagged" "{\"ig_id\": \"$IG_ID\"}"

# Containers
C_RES=$(call_tool "ig_create_container" "{\"ig_id\": \"$IG_ID\", \"media_type\": \"IMAGE\", \"media_url\": \"https://placehold.co/600x400\", \"caption\": \"Audit post\"}")
CREATION_ID=$(echo "$C_RES" | grep -oP '"id":\s*"\K[^"]+' | head -n 1)
if [ -n "$CREATION_ID" ]; then
    call_tool "ig_publish_container" "{\"ig_id\": \"$IG_ID\", \"creation_id\": \"$CREATION_ID\"}"
fi

echo "=== AUDIT COMPLETE ==="
