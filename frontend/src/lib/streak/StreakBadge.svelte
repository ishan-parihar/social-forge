<script lang="ts">
  import { onMount } from 'svelte';

  let streak = $state(0);
  let loading = $state(true);

  async function fetchStreak() {
    try {
      const res = await fetch('/api/streak');
      if (res.ok) {
        const data = await res.json();
        streak = data.streak_days || 0;
      }
    } catch {
      // silent fail — streak is non-critical
    }
    loading = false;
  }

  onMount(() => {
    fetchStreak();
    const interval = setInterval(fetchStreak, 5 * 60 * 1000);
    return () => clearInterval(interval);
  });
</script>

{#if !loading && streak > 0}
  <div
    class="flex items-center gap-1 px-2 py-1 rounded-lg bg-warning/10 text-warning"
    title={"You're on a " + streak + " day posting streak! Keep it going!"}
  >
    <svg class="w-4 h-4" viewBox="0 0 24 24" fill="currentColor">
      <path d="M13.5.67s.74 2.65.74 4.8c0 2.06-1.35 3.73-3.41 3.73-2.07 0-3.63-1.67-3.63-3.73l.03-.36C5.21 7.51 4 10.62 4 14c0 4.42 3.58 8 8 8s8-3.58 8-8C20 8.61 17.41 3.8 13.5.67zM11.71 19c-1.78 0-3.22-1.4-3.22-3.14 0-1.62 1.05-2.76 2.81-3.12 1.77-.36 3.6-1.21 4.62-2.58.39 1.29.59 2.65.59 4.04 0 2.65-2.15 4.8-4.8 4.8z"/>
    </svg>
    <span class="text-xs font-bold">{streak}</span>
  </div>
{/if}
