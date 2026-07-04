<script lang="ts">
  // userId comes from +layout.ts — no redundant auth.me() probe here.
  let { data } = $props();
  let userId = $derived(data?.userId ?? '');
</script>

<div class="page-enter space-y-6">
  <h2 class="text-xl font-semibold">Settings</h2>
  <div class="bg-surface border border-line rounded-xl p-5 space-y-3">
    <div>
      <label class="text-xs text-muted">Mode</label>
      <p class="text-sm mt-0.5">Single-user (local deployment)</p>
    </div>
    <div>
      <label class="text-xs text-muted">User ID</label>
      <p class="text-xs text-muted mt-0.5 font-mono">{userId}</p>
    </div>
    <div>
      <label class="text-xs text-muted">Authentication</label>
      <p class="text-sm mt-0.5">Password gate (set <code class="text-indigo-400">APP_PASSWORD</code> in <code>~/.social-forge/.env</code>)</p>
    </div>
  </div>
  <div class="bg-surface border border-line rounded-xl p-5">
    <h3 class="text-sm font-medium mb-2">API Connection</h3>
    <p class="text-xs text-muted">AI agents can use the MCP interface via <code class="text-indigo-400">social-forge mcp</code></p>
  </div>
</div>
