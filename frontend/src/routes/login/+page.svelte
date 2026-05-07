<script lang="ts">
  import { auth, setToken } from '$lib/api';
    let email = $state(''); let password = $state(''); let name = $state('');
  let isRegister = $state(false); let error = $state(''); let busy = $state(false);

  async function submit(e: Event) {
    e.preventDefault(); busy = true; error = '';
    const r = isRegister ? await auth.register(email, password, name) : await auth.login(email, password);
    busy = false;
    // Full page reload ensures layout re-mounts and auth state is fresh
    if (r.data?.token) { setToken(r.data.token); window.location.href = '/'; }
    else error = r.error || 'Auth failed';
  }
</script>

<div class="min-h-screen flex items-center justify-center bg-[#0b0e14]">
  <div class="w-full max-w-sm">
    <div class="text-center mb-8">
      <div class="text-4xl mb-3">📅</div>
      <h1 class="text-2xl font-bold text-indigo-400">Postiz</h1>
      <p class="text-sm text-[#6b7280] mt-1">Social Media Scheduler</p>
    </div>
    <form onsubmit={submit} class="bg-[#131720] border border-[#1e2435] rounded-xl p-6 space-y-4">
      <h2 class="text-lg font-semibold">{isRegister ? 'Create Account' : 'Sign In'}</h2>
      {#if error}<div class="bg-red-900/40 text-red-300 px-3 py-2 rounded-lg text-sm">{error}</div>{/if}
      <div>
        <label class="block text-xs text-[#6b7280] mb-1.5">Email</label>
        <input type="email" bind:value={email} required class="w-full bg-[#0b0e14] border border-[#1e2435] rounded-lg px-3 py-2.5 text-sm focus:border-indigo-500 outline-none" />
      </div>
      <div>
        <label class="block text-xs text-[#6b7280] mb-1.5">Password</label>
        <input type="password" bind:value={password} required minlength={6} class="w-full bg-[#0b0e14] border border-[#1e2435] rounded-lg px-3 py-2.5 text-sm focus:border-indigo-500 outline-none" />
      </div>
      {#if isRegister}
        <div>
          <label class="block text-xs text-[#6b7280] mb-1.5">Name</label>
          <input type="text" bind:value={name} required class="w-full bg-[#0b0e14] border border-[#1e2435] rounded-lg px-3 py-2.5 text-sm focus:border-indigo-500 outline-none" />
        </div>
      {/if}
      <button type="submit" disabled={busy} class="w-full py-2.5 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 rounded-lg font-medium text-sm transition-colors">
        {isRegister ? 'Create Account' : 'Sign In'}
      </button>
      <button type="button" onclick={() => { isRegister = !isRegister; error = ''; }} class="w-full text-xs text-indigo-400 hover:text-indigo-300">
        {isRegister ? 'Already have an account? Sign in' : "Don't have an account? Create one"}
      </button>
    </form>
  </div>
</div>
