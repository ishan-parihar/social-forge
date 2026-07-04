<script lang="ts">
  import { auth } from '$lib/api/auth';
  import { goto } from '$app/navigation';

  let password = $state('');
  let error = $state('');
  let loading = $state(false);

  async function handleLogin(e: SubmitEvent) {
    e.preventDefault();
    if (!password) return;
    error = '';
    loading = true;
    const r = await auth.login(password);
    loading = false;
    if (r.error) {
      error = r.error;
      return;
    }
    // Cookie is now set by the server. Redirect to dashboard.
    goto('/');
  }
</script>

<svelte:head>
  <title>Social Forge — Login</title>
</svelte:head>

<div class="page-enter min-h-screen flex items-center justify-center bg-background px-4">
  <div class="w-full max-w-sm">
    <div class="text-center mb-8">
      <div class="inline-flex items-center justify-center w-12 h-12 rounded-xl bg-indigo-500/10 border border-indigo-500/30 mb-4">
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="w-6 h-6 text-indigo-400"><path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/></svg>
      </div>
      <h1 class="text-2xl font-bold text-content">Social Forge</h1>
      <p class="text-sm text-muted mt-1">Enter your password to access the dashboard</p>
    </div>

    <form onsubmit={handleLogin} class="space-y-4">
      <div>
        <label for="password" class="block text-xs font-medium text-muted mb-1.5 uppercase tracking-wider">Password</label>
        <input
          id="password"
          type="password"
          bind:value={password}
          autocomplete="current-password"
          autofocus
          class="w-full px-3.5 py-2.5 bg-surface-hover border border-line rounded-lg text-content text-sm
            focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500
            placeholder:text-muted-dark"
          placeholder="••••••••••••"
        />
      </div>

      {#if error}
        <div class="px-3 py-2 bg-red-500/10 border border-red-500/30 rounded-lg text-red-400 text-xs">
          {error}
        </div>
      {/if}

      <button
        type="submit"
        disabled={loading || !password}
        class="w-full py-2.5 px-4 bg-indigo-500 hover:bg-indigo-400 disabled:bg-surface-hover disabled:text-muted-dark
          text-white font-medium text-sm rounded-lg transition-colors
          focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:ring-offset-background"
      >
        {loading ? 'Verifying…' : 'Unlock'}
      </button>
    </form>

    <p class="text-center text-xs text-muted-dark mt-6">
      Set <code class="text-muted">APP_PASSWORD</code> in your <code class="text-muted">.env</code> to change this password.
    </p>
  </div>
</div>
