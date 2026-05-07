<script lang="ts">
  import { onMount } from 'svelte';
  import { auth } from '$lib/api';

  let user = $state<{ id: string; email: string; name: string } | null>(null);
  onMount(async () => { const r = await auth.me(); if (r.data) user = r.data; });
</script>

<div class="space-y-4">
  <h2 class="text-xl font-semibold">Settings</h2>
  {#if user}
    <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5 space-y-3">
      <div>
        <label class="text-xs text-[#6b7280]">Name</label>
        <p class="text-sm mt-0.5">{user.name}</p>
      </div>
      <div>
        <label class="text-xs text-[#6b7280]">Email</label>
        <p class="text-sm mt-0.5">{user.email}</p>
      </div>
      <div>
        <label class="text-xs text-[#6b7280]">User ID</label>
        <p class="text-xs text-[#6b7280] mt-0.5 font-mono">{user.id}</p>
      </div>
    </div>
  {/if}
  <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5">
    <h3 class="text-sm font-medium mb-2">API Connection</h3>
    <p class="text-xs text-[#6b7280]">Backend: <span class="text-indigo-400">http://localhost:3000</span></p>
    <p class="text-xs text-[#6b7280] mt-1">AI agents can use the MCP interface via <code class="text-indigo-400">./postiz-rust --mcp</code></p>
  </div>
</div>
