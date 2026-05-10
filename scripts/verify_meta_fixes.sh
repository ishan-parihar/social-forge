#!/bin/bash
# ─── Meta MCP Tools Verification Script ──────────────────────────────
# Tests specific fixes for Facebook and Instagram tools.

set -euo pipefail
BASE="http://localhost:3001"

# Tokens & IDs from DB
FB_PAGE_ID="4372074126446140"
IG_ACCOUNT_ID="17841400680408909"

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

run_tool() {
    local tool="$1"
    local args="$2"
    echo -n "Testing $tool with args $args... "
    
    local response
    response=$(curl -s -X POST "$BASE/mcp/v1/tools/call" \
      -H "Content-Type: application/json" \
      -d "{\"name\": \"$tool\", \"arguments\": $args}")
    
    if echo "$response" | grep -q "error"; then
        echo -e "${RED}FAIL${NC}"
        echo "Response: $response"
        return 1
    else
        echo -e "${GREEN}PASS${NC}"
        return 0
    fi
}

echo "=== Starting Meta MCP Tools Verification ==="

# 1. Test FB Get Page
run_tool "fb_get_page" "{\"page_id\": \"$FB_PAGE_ID\"}"

# 2. Test FB Page Insights (Crucial: Test multi-metric)
# This should now work because the server splits them into individual calls
run_tool "fb_get_page_insights" "{\"page_id\": \"$FB_PAGE_ID\", \"metric\": \"page_fan_count,page_impressions\", \"period\": \"week\"}"

# 3. Test FB Search Pages (Crucial: New endpoint)
run_tool "fb_search_pages" "{\"query\": \"Postiz\"}"

# 4. Test IG Get Media
run_tool "ig_get_media" "{\"ig_id\": \"$IG_ACCOUNT_ID\", \"limit\": 5}"

# 5. Test IG Insights (Crucial: Validation and period forcing)
# Should work for 'reach' and 'follower_count'
run_tool "ig_get_insights" "{\"ig_id\": \"$IG_ACCOUNT_ID\", \"metric\": \"reach,follower_count\", \"period\": \"day\"}"

# 6. Test IG Get Followers (Crucial: New endpoint)
run_tool "ig_get_followers" "{\"ig_id\": \"$IG_ACCOUNT_ID\"}"

echo "=== Verification Complete ==="
