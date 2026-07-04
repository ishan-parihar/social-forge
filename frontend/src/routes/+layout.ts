import { redirect } from '@sveltejs/kit';
import type { LayoutLoad } from './$types';

/**
 * Auth guard for every route except /login and /auth/callback.
 *
 * The backend `auth_middleware` validates the `sf_session` cookie
 * on every `/api/*` request. We probe `GET /api/auth/me` once per
 * navigation; if it returns 200 we know the cookie is valid and we
 * cache `userId` in the layout data so descendants (e.g. /settings)
 * don't need to re-probe.
 *
 * /auth/callback is exempt because it's the OAuth popup target —
 * the popup inherits the parent's cookies anyway, but skipping the
 * probe avoids a race during the OAuth redirect dance.
 */
export const load: LayoutLoad = async ({ url, fetch }) => {
  if (url.pathname === '/login' || url.pathname.startsWith('/auth/callback')) {
    return { authenticated: false, userId: null };
  }

  try {
    const res = await fetch('/api/auth/me', { credentials: 'include' });
    if (res.ok) {
      const data = await res.json();
      return { authenticated: true, userId: data.user_id as string };
    }
  } catch {
    // network error — fall through to redirect
  }
  throw redirect(303, '/login');
};
