#!/bin/bash
# Postiz-Rust End-to-End Test Suite
set -e

BASE="http://localhost:3000"
PASS=0
FAIL=0

green() {
	echo -e "\033[32m✓ $1\033[0m"
	((PASS++))
}
red() {
	echo -e "\033[31m✗ $1\033[0m"
	((FAIL++))
}

echo "═══════════════════════════════════════════"
echo "  Postiz-Rust End-to-End Test Suite"
echo "═══════════════════════════════════════════"

# ── 1. Health Check ──────────────────────────────
echo -e "\n── 1. Health Check ──"
HEALTH=$(curl -sf $BASE/health)
if [ $? -eq 0 ]; then
	green "Health endpoint"
else
	red "Health check failed"
fi

# ── 2. Auth ──────────────────────────────────────
echo -e "\n── 2. Authentication ──"

# Register
REG=$(curl -sf -X POST $BASE/api/auth/register \
	-H "Content-Type: application/json" \
	-d '{"email":"test@postiz.dev","password":"test123456","name":"Test User"}') || true
if [ -n "$REG" ]; then
	TOKEN=$(echo "$REG" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])" 2>/dev/null)
	green "Register (token: ${TOKEN:0:20}...)"
else
	red "Register failed"
	TOKEN=""
fi

# Duplicate register (should fail)
DUP=$(curl -s -o /dev/null -w "%{http_code}" -X POST $BASE/api/auth/register \
	-H "Content-Type: application/json" \
	-d '{"email":"test@postiz.dev","password":"test123456","name":"Test User"}')
[ "$DUP" = "409" ] && green "Duplicate register rejected (409)" || red "Duplicate register: got $DUP"

# Login
if [ -z "$TOKEN" ]; then
	LOGIN=$(curl -sf -X POST $BASE/api/auth/login \
		-H "Content-Type: application/json" \
		-d '{"email":"test@postiz.dev","password":"test123456"}') || true
	TOKEN=$(echo "$LOGIN" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])" 2>/dev/null)
fi
[ -n "$TOKEN" ] && green "Login (token: ${TOKEN:0:20}...)" || red "Login failed"

# Bad password
BAD=$(curl -s -o /dev/null -w "%{http_code}" -X POST $BASE/api/auth/login \
	-H "Content-Type: application/json" \
	-d '{"email":"test@postiz.dev","password":"wrongpassword"}')
[ "$BAD" = "401" ] && green "Bad password rejected (401)" || red "Bad password: got $BAD"

# GET /me
ME=$(curl -sf $BASE/api/auth/me -H "Authorization: Bearer $TOKEN") || true
if [ -n "$ME" ]; then
	ME_EMAIL=$(echo "$ME" | python3 -c "import sys,json; print(json.load(sys.stdin)['email'])" 2>/dev/null)
	[ "$ME_EMAIL" = "test@postiz.dev" ] && green "Get current user ($ME_EMAIL)" || red "Email mismatch: $ME_EMAIL"
else
	red "Get current user failed"
fi

# ── 3. MCP stdio ─────────────────────────────────
echo -e "\n── 3. MCP Server (stdio) ──"
MCP_OUT=$(printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}\n{"jsonrpc":"2.0","method":"notifications/initialized"}\n{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}\n' | timeout 5 ./target/release/postiz-rust --mcp 2>/dev/null || true)
echo "$MCP_OUT" | grep -q "auth_login" && green "MCP tools/list returns tools" || red "MCP tools/list failed"

# ── 4. Integrations ──────────────────────────────
echo -e "\n── 4. Integrations ──"

INT_LIST=$(curl -sf $BASE/api/integrations -H "Authorization: Bearer $TOKEN") || true
if [ -n "$INT_LIST" ]; then
	INT_COUNT=$(echo "$INT_LIST" | python3 -c "import sys,json; print(len(json.load(sys.stdin)['integrations']))" 2>/dev/null)
	[ "$INT_COUNT" = "0" ] && green "List integrations (empty: $INT_COUNT)" || red "List integrations: $INT_COUNT found"
else
	red "List integrations failed"
fi

# Connect (unconfigured provider → 400)
CONNECT=$(curl -s -o /dev/null -w "%{http_code}" $BASE/api/integrations/connect/x \
	-H "Authorization: Bearer $TOKEN")
[ "$CONNECT" = "400" ] && green "Connect unconfigured returns 400" || red "Connect: got $CONNECT"

# ── 5. Posts CRUD ────────────────────────────────
echo -e "\n── 5. Posts ──"

# Create integration in DB for testing
INT_ID=$(PGPASSWORD=postiz psql -h localhost -U postiz -d postiz -tA \
	-c "INSERT INTO integrations (user_id, provider_identifier, provider_name, internal_id, access_token)
      SELECT id, 'x', 'X (Twitter)', 'test123', 'test-token'
      FROM users WHERE email = 'test@postiz.dev'
      RETURNING id;" 2>/dev/null | head -1)

[ -n "$INT_ID" ] && green "Created test integration ($INT_ID)" || {
	red "Failed to create test integration"
	INT_ID="00000000-0000-0000-0000-000000000000"
}

# Create post
POST=$(curl -sf -X POST $BASE/api/posts \
	-H "Authorization: Bearer $TOKEN" \
	-H "Content-Type: application/json" \
	-d "{\"integration_id\":\"$INT_ID\",\"content\":\"Test post from Postiz-Rust! #RustLang\",\"title\":\"Test Post\"}") || true
if [ -n "$POST" ]; then
	POST_ID=$(echo "$POST" | python3 -c "import sys,json; print(json.load(sys.stdin)['id'])" 2>/dev/null)
	POST_STATE=$(echo "$POST" | python3 -c "import sys,json; print(json.load(sys.stdin)['state'])" 2>/dev/null)
	green "Create post ($POST_ID, state: $POST_STATE)"
else
	red "Create post failed"
	POST_ID=""
fi

# List posts
POSTS=$(curl -sf "$BASE/api/posts?limit=10" -H "Authorization: Bearer $TOKEN") || true
if [ -n "$POSTS" ]; then
	P_COUNT=$(echo "$POSTS" | python3 -c "import sys,json; print(json.load(sys.stdin)['total'])" 2>/dev/null)
	[ "$P_COUNT" -ge 1 ] && green "List posts ($P_COUNT total)" || red "Expected >=1 posts, got $P_COUNT"
else
	red "List posts failed"
fi

# Get single post
if [ -n "$POST_ID" ]; then
	GET_POST=$(curl -sf "$BASE/api/posts/$POST_ID" -H "Authorization: Bearer $TOKEN") || true
	GOT_CONTENT=$(echo "$GET_POST" | python3 -c "import sys,json; print(json.load(sys.stdin)['content'])" 2>/dev/null)
	[ -n "$GOT_CONTENT" ] && green "Get post: ${GOT_CONTENT:0:30}..." || red "Get post failed"
else
	red "Skipping get post (no POST_ID)"
fi

# Schedule post
if [ -n "$POST_ID" ]; then
	SCHED=$(curl -sf -X POST "$BASE/api/posts/$POST_ID/schedule" \
		-H "Authorization: Bearer $TOKEN" \
		-H "Content-Type: application/json" \
		-d '{"scheduled_at":"2026-05-08T10:00:00Z"}') || true
	SCHED_STATE=$(echo "$SCHED" | python3 -c "import sys,json; print(json.load(sys.stdin)['state'])" 2>/dev/null)
	[ "$SCHED_STATE" = "queued" ] && green "Schedule post (state: $SCHED_STATE)" || red "Schedule: got $SCHED_STATE"
else
	red "Skipping schedule (no POST_ID)"
fi

# Find slot
SLOT=$(curl -sf "$BASE/api/posts/find-slot" -H "Authorization: Bearer $TOKEN") || true
if [ -n "$SLOT" ]; then
	SLOT_DATE=$(echo "$SLOT" | python3 -c "import sys,json; print(json.load(sys.stdin)['date'])" 2>/dev/null)
	green "Find slot: $SLOT_DATE"
else
	red "Find slot failed"
fi

# ── 6. Calendar ──────────────────────────────────
echo -e "\n── 6. Calendar ──"
CAL=$(curl -sf "$BASE/api/calendar?start=2026-05-01&end=2026-05-31" \
	-H "Authorization: Bearer $TOKEN") || true
if [ -n "$CAL" ]; then
	CAL_TOTAL=$(echo "$CAL" | python3 -c "import sys,json; print(json.load(sys.stdin)['total'])" 2>/dev/null)
	green "Calendar query (posts: $CAL_TOTAL)"
else
	red "Calendar query failed"
fi

# ── 7. Media Upload ──────────────────────────────
echo -e "\n── 7. Media ──"

# Create a 1x1 PNG
python3 -c "
import struct, zlib
def create_png(path):
    sig = b'\\x89PNG\\r\\n\\x1a\\n'
    ihdr_data = struct.pack('>IIBBBBB', 1, 1, 8, 2, 0, 0, 0)
    ihdr_crc = zlib.crc32(b'IHDR' + ihdr_data) & 0xffffffff
    ihdr = struct.pack('>I', 13) + b'IHDR' + ihdr_data + struct.pack('>I', ihdr_crc)
    raw = b'\\x00\\xff\\x00\\xff\\x00'
    compressed = zlib.compress(raw)
    idat_crc = zlib.crc32(b'IDAT' + compressed) & 0xffffffff
    idat = struct.pack('>I', len(compressed)) + b'IDAT' + compressed + struct.pack('>I', idat_crc)
    iend_crc = zlib.crc32(b'IEND') & 0xffffffff
    iend = struct.pack('>I', 0) + b'IEND' + struct.pack('>I', iend_crc)
    with open(path, 'wb') as f:
        f.write(sig + ihdr + idat + iend)
create_png('/tmp/test_upload.png')
" 2>/dev/null

MEDIA=$(curl -sf -X POST $BASE/api/media \
	-H "Authorization: Bearer $TOKEN" \
	-F "file=@/tmp/test_upload.png;type=image/png") || true
if [ -n "$MEDIA" ]; then
	MEDIA_ID=$(echo "$MEDIA" | python3 -c "import sys,json; print(json.load(sys.stdin)['id'])" 2>/dev/null)
	MEDIA_TYPE=$(echo "$MEDIA" | python3 -c "import sys,json; print(json.load(sys.stdin)['mime_type'])" 2>/dev/null)
	green "Upload media ($MEDIA_ID, $MEDIA_TYPE)"
else
	red "Upload media failed"
	MEDIA_ID=""
fi

# Serve media
if [ -n "$MEDIA_ID" ]; then
	MEDIA_SERVE=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/api/media/$MEDIA_ID")
	[ "$MEDIA_SERVE" = "200" ] && green "Serve media (HTTP $MEDIA_SERVE)" || red "Serve media: got $MEDIA_SERVE"
fi

# ── 8. SSE ───────────────────────────────────────
echo -e "\n── 8. SSE Events ──"
SSE_TEST=$(timeout 3 curl -sN $BASE/api/events 2>/dev/null || true)
echo "$SSE_TEST" | grep -q "keepalive" && green "SSE stream sends keepalive" || red "SSE: no keepalive detected (non-blocking)"

# ── 9. Delete post ───────────────────────────────
echo -e "\n── 9. Cleanup ──"
if [ -n "$POST_ID" ]; then
	DEL=$(curl -sf -X DELETE "$BASE/api/posts/$POST_ID" -H "Authorization: Bearer $TOKEN") || true
	[ -n "$DEL" ] && green "Delete post" || red "Delete post failed"

	DEL_CHECK=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/api/posts/$POST_ID" -H "Authorization: Bearer $TOKEN")
	[ "$DEL_CHECK" = "404" ] && green "Deleted post returns 404" || red "Deleted post: got $DEL_CHECK"
fi

# ── Summary ──────────────────────────────────────
echo -e "\n═══════════════════════════════════════════"
echo "  Results: $PASS passed, $FAIL failed"
echo "═══════════════════════════════════════════"
[ $FAIL -gt 0 ] && exit 1 || exit 0
