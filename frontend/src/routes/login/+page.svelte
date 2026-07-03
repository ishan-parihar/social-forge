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

<div class="min-h-screen flex items-center justify-center bg-[#0b0e14] px-4">
  <div class="w-full max-w-sm">
    <div class="text-center mb-8">
      <div class="inline-flex items-center justify-center w-12 h-12 rounded-xl bg-indigo-500/10 border border-indigo-500/30 mb-4">
        <span class="text-indigo-400 text-xl">⚙</span>
      </div>
      <h1 class="text-2xl font-bold text-[#e8edf5]">Social Forge</h1>
      <p class="text-sm text-[#6b7280] mt-1">Enter your password to access the dashboard</p>
    </div>

    <form onsubmit={handleLogin} class="space-y-4">
      <div>
        <label for="password" class="block text-xs font-medium text-[#6b7280] mb-1.5 uppercase tracking-wider">Password</label>
        <input
          id="password"
          type="password"
          bind:value={password}
          autocomplete="current-password"
          autofocus
          class="w-full px-3.5 py-2.5 bg-[#1a1f2e] border border-[#1e2435] rounded-lg text-[#e8edf5] text-sm
            focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500
            placeholder:text-[#4b5563]"
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
        class="w-full py-2.5 px-4 bg-indigo-500 hover:bg-indigo-400 disabled:bg-[#1a1f2e] disabled:text-[#4b5563]
          text-white font-medium text-sm rounded-lg transition-colors
          focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:ring-offset-[#0b0e14]"
      >
        {loading ? 'Verifying…' : 'Unlock'}
      </button>
    </form>

    <p class="text-center text-xs text-[#4b5563] mt-6">
      Set <code class="text-[#6b7280]">APP_PASSWORD</code> in your <code class="text-[#6b7280]">.env</code> to change this password.
    </p>
  </div>
</div>
