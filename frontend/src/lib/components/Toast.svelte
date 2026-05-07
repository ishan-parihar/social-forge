<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { onToast, type Toast } from '$lib/stores/toast';

  let toasts = $state<Toast[]>([]);

  onMount(() => onToast(t => {
    if (t.message) toasts = [...toasts, t];
    else toasts = toasts.filter(x => x.id !== t.id);
  }));
</script>

{#each toasts as t (t.id)}
  <div class="fixed bottom-4 right-4 z-50 px-4 py-3 rounded-lg shadow-lg text-sm max-w-sm animate-slide-up
    {t.type === 'success' ? 'bg-green-900 text-green-200' : ''}
    {t.type === 'error' ? 'bg-red-900 text-red-200' : ''}
    {t.type === 'warning' ? 'bg-yellow-900 text-yellow-200' : ''}
    {t.type === 'info' ? 'bg-blue-900 text-blue-200' : ''}"
  >
    {t.type === 'success' && '✅ '}{t.type === 'error' && '❌ '}{t.type === 'warning' && '⚠️ '}{t.type === 'info' && 'ℹ️ '}
    {t.message}
  </div>
{/each}

<style>
  @keyframes slide-up { from { transform: translateY(20px); opacity: 0; } to { transform: translateY(0); opacity: 1; } }
  .animate-slide-up { animation: slide-up 0.3s ease-out; }
</style>
