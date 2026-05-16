<script lang="ts">
  interface Props {
    onSelect: (emoji: string) => void;
    onClose: () => void;
  }

  let { onSelect, onClose }: Props = $props();

  const categories: { name: string; emojis: string[] }[] = [
    {
      name: 'Smileys',
      emojis: [
        '😀', '😃', '😄', '😁', '😅', '😂', '🤣', '😊', '😇', '🙂',
        '😉', '😌', '😍', '🥰', '😘', '😋', '😛', '😜', '🤪', '😝',
        '🤑', '🤗',
      ],
    },
    {
      name: 'Gestures',
      emojis: [
        '👍', '👎', '👊', '✊', '🤛', '🤜', '👏', '🙌', '👐', '🤲',
        '🤝', '🙏', '✌️', '🤟', '🤘', '👌',
      ],
    },
    {
      name: 'Objects',
      emojis: [
        '❤️', '💔', '💕', '💖', '💗', '💙', '💚', '💛', '🧡', '💜',
        '🖤', '💝', '💘', '💋', '💯', '🔥', '⭐', '✨', '💡',
      ],
    },
    {
      name: 'Symbols',
      emojis: [
        '✅', '❌', '⚠️', '🚫', '❓', '❗', '💢', '♻️', '🏆', '🎯',
        '🎨', '🎭', '📌', '📍', '🎵', '🎶', '🔔', '🎉', '🎊',
      ],
    },
  ];

  function pick(emoji: string) {
    onSelect(emoji);
    onClose();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains('emoji-backdrop')) {
      onClose();
    }
  }

  let emojiContainer = $state<HTMLDivElement | null>(null);
  $effect(() => {
    if (emojiContainer) {
      const first = emojiContainer.querySelector('button');
      first?.focus();
    }
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="emoji-backdrop fixed inset-0 z-40" role="none" onclick={handleBackdropClick}>
  <div
    bind:this={emojiContainer}
    class="emoji-popover"
    role="dialog"
    aria-label="Insert emoji"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    {#each categories as cat (cat.name)}
      <div class="mb-2 last:mb-0">
        <p class="text-xs text-[#6b7280] mb-1 px-0.5">{cat.name}</p>
        <div class="grid grid-cols-8 gap-0.5">
          {#each cat.emojis as emoji (emoji)}
            <button
              onclick={() => pick(emoji)}
              class="emoji-btn"
              aria-label="Insert {emoji}"
            >
              {emoji}
            </button>
          {/each}
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .emoji-popover {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    background: #131720;
    border: 1px solid #1e2435;
    border-radius: 0.5rem;
    padding: 0.75rem;
    width: 20rem;
    max-height: 18rem;
    overflow-y: auto;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    z-index: 50;
  }

  .emoji-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    font-size: 1.15rem;
    border-radius: 0.25rem;
    cursor: pointer;
    background: transparent;
    border: none;
    transition: background 0.15s;
  }

  .emoji-btn:hover {
    background: #1e2435;
  }

  .emoji-backdrop {
    background: rgba(0, 0, 0, 0.3);
  }

  /* scrollbar styling */
  .emoji-popover::-webkit-scrollbar {
    width: 4px;
  }
  .emoji-popover::-webkit-scrollbar-track {
    background: transparent;
  }
  .emoji-popover::-webkit-scrollbar-thumb {
    background: #1e2435;
    border-radius: 2px;
  }
</style>
