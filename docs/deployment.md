# Deployment

Moved out of the README to keep it short. Content is unchanged.

### Environment Variables

Every variable has a sensible default, so none is required:

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `sqlite://data/switchpost.db` | SQLite file path. SQLite-only: a `postgres://` (or any other) scheme is rejected at startup. |
| `APP_URL` | `https://localhost:6543` | Public URL of your instance. Used for OAuth redirect URIs. Its port must be the port `serve` listens on (`--port`). |
| `FRONTEND_URL` | Same as `APP_URL` | CORS allowed origin and CSRF origin. Must match the origin the browser sends, port included. A `serve --port` that differs from the `APP_URL` port makes `POST /api/feed/import` return 403 `csrf_origin_mismatch`. Set separately only if frontend is on a different domain. |
| `JWT_SECRET` | Auto-generated | Secret for signing auth tokens. Set a strong value in production. |
| `TOKEN_ENCRYPTION_KEY` | *(optional)* | 64 hex chars. Encrypts OAuth tokens at rest (AES-GCM). |
| `FRONTEND_DIR` | `./frontend/build` | Path to the SvelteKit static build directory. |

All provider-specific variables are documented in `.env.example`.

### Exposing via Tunnel (ngrok, Cloudflare, etc.)

```bash
# 1. Set APP_URL to your public tunnel URL
export APP_URL=https://switchpost.yourdomain.com

# 2. Start the server
switchpost serve --port 6543

# 3. Point your tunnel to localhost:6543
ngrok http 6543
# or: cloudflared tunnel --url https://localhost:6543
```

All OAuth redirect URIs automatically use `{APP_URL}/api/auth/callback`. Register this URL in each platform's developer console.

### Reverse Proxy (Caddy / Nginx)

**Caddy:**
```
switchpost.yourdomain.com {
    reverse_proxy localhost:6543
}
```

**Nginx:**
```nginx
server {
    server_name switchpost.yourdomain.com;
    location / {
        proxy_pass https://localhost:6543;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### Onboarding Flow

1. **Start the server**: `switchpost serve`
2. **Open the dashboard**: `https://localhost:6543` (or your APP_URL)
3. **Connect accounts**: Visit `https://localhost:6543/setup`
   - For OAuth providers (LinkedIn, Facebook, etc.): Click "Connect" → complete OAuth flow
   - For cookie auth (X, Reddit): Click "🍪 Enter Cookies" → paste browser cookies
   - For API key providers (Bluesky, GitHub, etc.): Enter credentials directly
4. **Create content**: Use the dashboard at `/posts/new` or the CLI
5. **Schedule**: Set a date/time or use "Auto-schedule" to find optimal slots

### OAuth Redirect URI Setup

When registering your app with social platforms, use this redirect URI:

```
{APP_URL}/api/auth/callback
```

For example:
- Local dev: `https://localhost:6543/api/auth/callback`
- Production: `https://switchpost.yourdomain.com/api/auth/callback`

> **⚠️ HTTPS is REQUIRED for Instagram and Threads (Meta platforms).**
> Meta does NOT support `http://` redirect URIs, even for localhost.
> Switchpost auto-generates a self-signed TLS certificate on first run.
> Your browser will show a security warning — click "Advanced" → "Proceed" to accept it.
> For a trusted cert, use [mkcert](https://github.com/FiloSottile/mkcert):
> ```bash
> mkcert -install && mkcert -cert-file data/tls/cert.pem -key-file data/tls/key.pem localhost 127.0.0.1
> ```

| Platform | Developer Console |
|----------|-------------------|
| X/Twitter | https://developer.twitter.com/en/portal/projects |
| LinkedIn | https://www.linkedin.com/developers/apps |
| Facebook/Instagram | https://developers.facebook.com/apps |
| Reddit | https://www.reddit.com/prefs/apps |
| YouTube | https://console.cloud.google.com/apis/credentials |
| TikTok | https://developers.tiktok.com/apps |
| Pinterest | https://developers.pinterest.com/apps |
| Discord | https://discord.com/developers/applications |

---
