<script lang="ts">
  // Phase 8: /posts/new is now a thin redirect that opens the composer
  // modal and sends the user back to wherever they came from (or the
  // calendar by default). The full composer logic lives in
  // lib/composer/ComposerModal.svelte.
  //
  // This route is kept for direct-link compatibility (e.g., bookmarks,
  // the browser address bar, no-JS fallbacks). The primary create flow
  // is the modal opened via composer.openCreate() from anywhere.

  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { composer } from '$lib/stores/composer.svelte';
  import { browser } from '$app/environment';

  onMount(() => {
    // Read the ?date= param if present (calendar empty-slot click passes it).
    const url = new URL(window.location.href);
    const date = url.searchParams.get('date') || undefined;
    composer.openCreate(date);
    // Send the user back where they came from (via ?from=), or to the
    // Posts list. The route used to default to /calendar, so the Posts
    // page's own "Create your first post" button teleported the user to
    // the calendar with the composer floating over it.
    const from = url.searchParams.get('from');
    goto(from || '/posts', { replaceState: true });
  });
</script>

<div class="page-enter flex items-center justify-center py-20">
  <div class="text-sm text-muted">Opening composer...</div>
</div>
