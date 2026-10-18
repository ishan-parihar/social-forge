<script lang="ts">
  // userId comes from +layout.ts - no redundant auth.me() probe here.
  let { data } = $props();
  let userId = $derived(data?.userId ?? '');
</script>

<div class="page-enter space-y-6">
  <h1 class="text-xl font-semibold">Settings</h1>
  <div class="bg-surface border border-line rounded-xl p-5 space-y-3">
    <div>
      <span class="text-xs text-muted">Mode</span>
      <p class="text-sm mt-0.5">Single-user (local deployment)</p>
    </div>
    <div>
      <span class="text-xs text-muted">User ID</span>
      <p class="text-xs text-muted mt-0.5 font-mono">{userId}</p>
    </div>
    <div>
      <span class="text-xs text-muted">Authentication</span>
      <p class="text-sm mt-0.5">Password gate (set <code class="text-accent">APP_PASSWORD</code> in <code>~/.social-forge/.env</code>)</p>
    </div>
  </div>
  <div class="bg-surface border border-line rounded-xl p-5">
    <h2 class="text-sm font-medium mb-2">API Connection</h2>
    <p class="text-xs text-muted">AI agents can use the MCP interface via <code class="text-accent">social-forge mcp</code></p>
  </div>
</div>
