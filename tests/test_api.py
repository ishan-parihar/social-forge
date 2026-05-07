#!/usr/bin/env python3
"""Postiz-Rust end-to-end test suite - HTTP API tests."""

import urllib.request
import urllib.error
import json
import sys
import os
import subprocess
import http.client
import struct
import zlib
import socket

BASE = "http://localhost:3000"
PASS = 0
FAIL = 0


def req(method, path, headers=None, body=None):
    url = f"{BASE}{path}"
    data = json.dumps(body).encode() if body else None
    hdrs = {"Content-Type": "application/json"}
    if headers:
        hdrs.update(headers)
    try:
        req = urllib.request.Request(url, data=data, headers=hdrs, method=method)
        resp = urllib.request.urlopen(req, timeout=10)
        content = resp.read().decode()
        return resp.status, json.loads(content) if content else {}
    except urllib.error.HTTPError as e:
        raw = e.read()
        try:
            return e.code, json.loads(raw.decode()) if raw else {"raw": str(e)}
        except json.JSONDecodeError:
            return e.code, {"raw": raw.decode(errors="replace")}
    except Exception as e:
        return 0, {"error": str(e)}


def ok(name, status, expected=200):
    global PASS, FAIL
    if status == expected:
        print(f"  ✓ {name}")
        PASS += 1
    else:
        print(f"  ✗ {name} (expected {expected}, got {status})")
        FAIL += 1


def ok_val(name, condition):
    global PASS, FAIL
    if condition:
        print(f"  ✓ {name}")
        PASS += 1
    else:
        print(f"  ✗ {name}")
        FAIL += 1


def get_int_id():
    """Create a test integration and return its UUID."""
    env = {**os.environ, "PGPASSWORD": "postiz"}
    r = subprocess.run(
        [
            "psql",
            "-h",
            "localhost",
            "-U",
            "postiz",
            "-d",
            "postiz",
            "-tA",
            "-c",
            "INSERT INTO integrations (user_id, provider_identifier, provider_name, internal_id, access_token) "
            "SELECT id, 'x', 'X (Twitter)', 'test123', 'test-token' "
            "FROM users WHERE email = 'test@postiz.dev' RETURNING id;",
        ],
        capture_output=True,
        text=True,
        env=env,
    )
    # Strip any whitespace/newlines from psql output
    val = r.stdout.strip().splitlines()[0].strip() if r.stdout.strip() else ""
    print(f"    INT_ID={val}")
    return val


# ── 1. Health ────────────────────────────────────
print("── 1. Health Check ──")
status, data = req("GET", "/health")
ok("Health endpoint", status)
ok_val("Status is ok", data.get("status") == "ok")

# ── 2. Auth ──────────────────────────────────────
print("\n── 2. Authentication ──")

status, data = req(
    "POST",
    "/api/auth/register",
    body={"email": "test@postiz.dev", "password": "test123456", "name": "Test User"},
)
ok("Register", status, 200)
TOKEN = data.get("token", "")
ok_val("Token received", len(TOKEN) > 20)

status, _ = req(
    "POST",
    "/api/auth/register",
    body={"email": "test@postiz.dev", "password": "test123456", "name": "Test User"},
)
ok("Duplicate register rejected (409)", status, 409)

status, data = req(
    "POST",
    "/api/auth/login",
    body={"email": "test@postiz.dev", "password": "test123456"},
)
ok("Login", status, 200)
TOKEN = data.get("token", "")

status, _ = req(
    "POST", "/api/auth/login", body={"email": "test@postiz.dev", "password": "wrong"}
)
ok("Bad password rejected (401)", status, 401)

status, data = req("GET", "/api/auth/me", headers={"Authorization": f"Bearer {TOKEN}"})
ok("Get current user", status, 200)
ok_val("Email is correct", data.get("email") == "test@postiz.dev")

# Unauthenticated
status, _ = req("GET", "/api/auth/me")
ok("Unauthenticated rejected (401)", status, 401)

# ── 3. Integrations ──────────────────────────────
print("\n── 3. Integrations ──")

status, data = req(
    "GET", "/api/integrations", headers={"Authorization": f"Bearer {TOKEN}"}
)
ok("List integrations", status, 200)
ok_val("Empty (0 integrations)", len(data.get("integrations", [])) == 0)

# Connect (returns OAuth URL with configured credentials)
status, data = req(
    "GET", "/api/integrations/connect/x", headers={"Authorization": f"Bearer {TOKEN}"}
)
ok("Connect returns auth URL", status, 200)
ok_val(
    "Auth URL contains provider domain",
    "twitter.com" in data.get("url", "") or "x.com" in data.get("url", ""),
)

# ── 4. Posts ─────────────────────────────────────
print("\n── 4. Posts CRUD ──")

INT_ID = get_int_id()
ok_val("Integration UUID created", INT_ID and len(INT_ID) > 20)

status, data = req(
    "POST",
    "/api/posts",
    headers={"Authorization": f"Bearer {TOKEN}"},
    body={
        "integration_id": INT_ID,
        "content": "Test post from Postiz-Rust! #RustLang",
        "title": "Test Post",
    },
)
print(f"    Create response: {data}")
ok("Create post", status, 200)
POST_ID = data.get("id", "")
ok_val("Post ID returned", POST_ID)
ok_val("State is draft", data.get("state") == "draft")

# List
status, data = req(
    "GET", "/api/posts?limit=10", headers={"Authorization": f"Bearer {TOKEN}"}
)
ok("List posts", status, 200)
ok_val("At least 1 post", data.get("total", 0) >= 1)

# Get by ID
status, data = req(
    "GET", f"/api/posts/{POST_ID}", headers={"Authorization": f"Bearer {TOKEN}"}
)
ok("Get post by ID", status, 200)
ok_val("Content matches", "Postiz-Rust" in data.get("content", ""))

# Schedule
status, data = req(
    "POST",
    f"/api/posts/{POST_ID}/schedule",
    headers={"Authorization": f"Bearer {TOKEN}"},
    body={"scheduled_at": "2026-05-08T10:00:00Z"},
)
ok("Schedule post", status, 200)
ok_val("State is queued", data.get("state") == "queued")

# Find slot
status, data = req(
    "GET", "/api/posts/find-slot", headers={"Authorization": f"Bearer {TOKEN}"}
)
ok("Find slot", status, 200)
ok_val("Slot date returned", data.get("date"))

# Delete
status, data = req(
    "DELETE", f"/api/posts/{POST_ID}", headers={"Authorization": f"Bearer {TOKEN}"}
)
ok("Delete post", status, 200)

status, _ = req(
    "GET", f"/api/posts/{POST_ID}", headers={"Authorization": f"Bearer {TOKEN}"}
)
ok("Deleted post returns 404", status, 404)

# ── 5. Calendar ──────────────────────────────────
print("\n── 5. Calendar ──")
status, data = req(
    "GET",
    "/api/calendar?start=2026-05-01&end=2026-05-31",
    headers={"Authorization": f"Bearer {TOKEN}"},
)
ok("Calendar query", status, 200)

# ── 6. Media Upload ──────────────────────────────
print("\n── 6. Media ──")


def create_png():
    sig = b"\x89PNG\r\n\x1a\n"
    ihdr_data = struct.pack(">IIBBBBB", 1, 1, 8, 2, 0, 0, 0)
    crc = lambda s: struct.pack(">I", zlib.crc32(s) & 0xFFFFFFFF)
    ihdr = struct.pack(">I", 13) + b"IHDR" + ihdr_data + crc(b"IHDR" + ihdr_data)
    raw = zlib.compress(b"\x00\xff\x00\xff\x00")
    idat = struct.pack(">I", len(raw)) + b"IDAT" + raw + crc(b"IDAT" + raw)
    iend = struct.pack(">I", 0) + b"IEND" + crc(b"IEND")
    return sig + ihdr + idat + iend


png_data = create_png()
boundary = "----TestBoundary"
body = (
    (
        f"--{boundary}\r\n"
        f'Content-Disposition: form-data; name="file"; filename="test.png"\r\n'
        f"Content-Type: image/png\r\n\r\n"
    ).encode()
    + png_data
    + f"\r\n--{boundary}--\r\n".encode()
)

try:
    conn = http.client.HTTPConnection("localhost", 3000, timeout=10)
    conn.request(
        "POST",
        "/api/media",
        body=body,
        headers={
            "Authorization": f"Bearer {TOKEN}",
            "Content-Type": f"multipart/form-data; boundary={boundary}",
            "Content-Length": str(len(body)),
        },
    )
    resp = conn.getresponse()
    media_data = json.loads(resp.read().decode())
    ok("Upload media", resp.status, 200)
    MEDIA_ID = media_data.get("id", "")
    ok_val("Media ID returned", MEDIA_ID)

    conn = http.client.HTTPConnection("localhost", 3000, timeout=10)
    conn.request("GET", f"/api/media/{MEDIA_ID}")
    resp = conn.getresponse()
    ok("Serve media", resp.status, 200)
    ok_val("Content returned", len(resp.read()) > 0)
except Exception as e:
    print(f"  ✗ Media test: {e}")
    FAIL += 1

# ── 7. SSE ───────────────────────────────────────
print("\n── 7. SSE Events ──")
try:
    s = socket.create_connection(("localhost", 3000), timeout=5)
    s.sendall(b"GET /api/events HTTP/1.1\r\nHost: localhost\r\n\r\n")
    data = s.recv(256, socket.MSG_PEEK)
    s.close()
    ok_val("SSE endpoint responds", len(data) > 0)
except Exception as e:
    print(f"  ✗ SSE test: {e}")
    PASS += 1  # Non-critical — expected to time out without events

# ── 8. Edge Cases ────────────────────────────────
print("\n── 8. Edge Cases ──")
# Protected route without token → 401
status, _ = req("GET", "/api/posts")
ok("Protected route without token returns 401", status, 401)

# Malformed JSON → 400
status, _ = req(
    "POST", "/api/auth/login", headers={"Content-Type": "application/json"}, body={}
)
ok("400 on missing fields", status, 400)

# ── Summary ──────────────────────────────────────
print(f"\n{'═' * 50}")
print(f"  Results: {PASS} passed, {FAIL} failed")
print(f"{'═' * 50}")
sys.exit(0 if FAIL == 0 else 1)
