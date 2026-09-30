<script lang="ts">
  // Route error boundary — the last line before a crash becomes a naked page.
  //
  // Before this existed, any render throw (a module that blew up on import, a
  // bad date, a null deref) fell through to SvelteKit's built-in fallback: a
  // bare `<h1>500</h1>` on a white page, unstyled, with no way back into the
  // app. That is a bad failure surface: it looks broken, and it is unreportable
  // because the message is never shown.
  //
  // This boundary does three things the default one does not:
  //   - renders in the app's own tokens, so a failure still looks like the app
  //   - shows the status code AND the message verbatim (diagnostic text, not
  //     decoration — the same reasoning as ErrorState)
  //   - always offers a way forward: Retry, and a link home.
  //
  // It renders inside the root layout when the failure is below the root, and
  // standalone when the root layout itself is what failed — so it deliberately
  // does not import the shell or any store.
  import { page } from '$app/stores';

  const code = $derived($page.status);
  const heading = $derived(code === 404 ? 'Page not found' : 'Something went wrong');
</script>

<svelte:head>
  <title>{code} · Social Forge</title>
</svelte:head>

<div class="min-h-screen bg-background text-content flex items-center justify-center p-6">
  <div class="w-full max-w-md text-center">
    <div
      class="w-12 h-12 rounded-full bg-error/10 border border-error/30 flex items-center justify-center mx-auto mb-4"
      aria-hidden="true"
    >
      <svg class="w-6 h-6 text-error" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
        <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v2m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
    </div>

    <p class="text-4xl font-semibold tabular-nums text-faint">{code}</p>
    <h1 class="text-lg font-medium mt-1">{heading}</h1>

    {#if $page.error?.message}
      <p class="text-sm text-muted mt-2 break-words">{$page.error.message}</p>
    {/if}

    <div class="flex items-center justify-center gap-2 mt-6 flex-wrap">
      <button
        onclick={() => location.reload()}
        class="px-4 py-2 text-sm bg-accent-fill hover:bg-accent-fill-hover text-accent-fg rounded-lg transition-colors"
      >
        Retry
      </button>
      <a
        href="/"
        class="px-4 py-2 text-sm bg-surface border border-line rounded-lg text-content hover:bg-surface-hover transition-colors"
      >
        Go home
      </a>
    </div>
  </div>
</div>
