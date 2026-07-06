<script lang="ts">
  // Settings sub-page breadcrumb (R-16): adds a "← Settings" link at
  // the top of every settings sub-page so the user can navigate back
  // to the settings index without using the sidebar.
  //
  // Usage: <SettingsBreadcrumb title="Brand Profile" />
  // Renders: ← Settings  /  Brand Profile

  let { title }: { title: string } = $props();
</script>

<nav aria-label="Breadcrumb" class="flex items-center gap-2 text-sm">
  <a
    href="/settings"
    class="text-muted hover:text-indigo-400 transition-colors flex items-center gap-1"
  >
    <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <line x1="19" y1="12" x2="5" y2="12"/><polyline points="12 19 5 12 12 5"/>
    </svg>
    Settings
  </a>
  <span class="text-muted-dark">/</span>
  <span class="text-content font-medium">{title}</span>
</nav>
