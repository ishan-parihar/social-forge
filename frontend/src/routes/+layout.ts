import { redirect } from '@sveltejs/kit';
import type { LayoutLoad } from './$types';

/**
 * Auth guard for every route except /login and /auth/callback.
 *
 * The backend `auth_middleware` validates the `sf_session` cookie
 * on every `/api/*` request. We don't validate the cookie here in
 * JS — instead we make a cheap `GET /api/auth/me` call; if it
 * returns 200 we know the cookie is valid. If it 401s, we redirect
 * to /login.
 *
 * /auth/callback is exempt because it's the OAuth popup target —
 * the popup inherits the parent's cookies anyway, but skipping the
 * probe avoids a race during the OAuth redirect dance.
 */
export const load: LayoutLoad = async ({ url, fetch }) => {
  if (url.pathname === '/login' || url.pathname.startsWith('/auth/callback')) {
    return { authenticated: false };
  }

  try {
    const res = await fetch('/api/auth/me', { credentials: 'include' });
    if (res.ok) {
      return { authenticated: true };
    }
  } catch {
    // network error — fall through to redirect
  }
  throw redirect(303, '/login');
};
