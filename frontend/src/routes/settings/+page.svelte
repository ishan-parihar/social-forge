<script lang="ts">
  // userId comes from +layout.ts - no redundant auth.me() probe here.
  import { timezone } from '$lib/stores/timezone.svelte';
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
      <p class="text-sm mt-0.5">Password gate (set <code class="text-accent">APP_PASSWORD</code> in <code>~/.switchpost/.env</code>)</p>
    </div>
  </div>

  <!-- Phase 5a: the timezone lives here as well as in the sidebar footer.
       The plan's "move the timezone picker to /settings (a setting, not a
       nav)" is what makes the 80px rail safe: the one control that cannot
       shrink to an icon keeps a full-size home.

       Both write the SAME store, so the two controls can never disagree -
       which is exactly the failure a duplicated setting produces. -->
  <div class="bg-surface border border-line rounded-xl p-5 space-y-3">
    <h2 class="text-sm font-medium">Display</h2>
    <div>
      <label for="settings-timezone" class="text-xs text-muted block mb-1">Timezone</label>
      <select
        id="settings-timezone"
        value={timezone.value}
        onchange={(e) => timezone.set(e.currentTarget.value)}
        class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-content focus:outline-none focus:border-accent min-h-11"
      >
        {#each timezone.commonTimezones as tz}
          <option value={tz}>{tz}</option>
        {/each}
      </select>
      <p class="text-xs text-muted mt-1">Used for the calendar and every schedule in the app. Also in the sidebar footer when the rail is expanded.</p>
    </div>
  </div>

  <div class="bg-surface border border-line rounded-xl p-5">
    <h2 class="text-sm font-medium mb-2">API Connection</h2>
    <p class="text-xs text-muted">AI agents can use the MCP interface via <code class="text-accent">switchpost mcp</code></p>
  </div>
</div>
