<script lang="ts">
  // Per-platform character counter for the composer (R-4 / U-3).
  // Shows a row of small badges — one per selected channel — each
  // displaying the current char count vs that platform's limit.
  // Turns yellow at 90% of the limit, red when over.
  //
  // v24-7: X (Twitter) uses weighted length (emoji/CJK = 2 chars, ASCII
  // = 1 char) matching X's API. Previously all platforms used plain
  // .length which undercounted emoji-heavy X posts. The weightedLength
  // function is a lightweight implementation (no twitter-text dep).

  import { providerMeta } from "$lib/providers";

  let { content, selectedIntegrations, integrationProviders, integrationNames }: {
    content: string;
    selectedIntegrations: string[];
    integrationProviders: Map<string, string>;
    integrationNames: Map<string, string>;
  } = $props();

  // Plain-text length of the content (HTML tags stripped).
  let plainText = $derived(content.replace(/<[^>]*>/g, ''));
  let plainTextLength = $derived(plainText.length);

  // v24-7: X weighted length — emoji and CJK characters count as 2,
  // everything else as 1. This matches X's v2 API char counting.
  // We detect "heavy" chars by code point range:
  //   - CJK Unified Ideographs (U+4E00–U+9FFF)
  //   - CJK Extension A (U+3400–U+4DBF)
  //   - CJK Compatibility (U+F900–U+FAFF)
  //   - Hiragana (U+3040–U+309F)
  //   - Katakana (U+30A0–U+30FF)
  //   - Emoji (various ranges — we check the emoji-presentation flag
  //     and common emoji blocks)
  function weightedLength(text: string): number {
    let count = 0;
    for (const char of text) {
      const cp = char.codePointAt(0) ?? 0;
      // CJK + Hiragana + Katakana = 2
      if (
        (cp >= 0x3040 && cp <= 0x30ff) ||  // Japanese kana
        (cp >= 0x3400 && cp <= 0x4dbf) ||  // CJK Extension A
        (cp >= 0x4e00 && cp <= 0x9fff) ||  // CJK Unified
        (cp >= 0xf900 && cp <= 0xfaff) ||  // CJK Compatibility
        (cp >= 0xac00 && cp <= 0xd7af) ||  // Korean Hangul Syllables
        (cp >= 0x1f300 && cp <= 0x1f9ff) || // Emoji (Misc Symbols & Pictographs, Emoticons, etc.)
        (cp >= 0x2600 && cp <= 0x27bf)     // Misc Symbols + Dingbats
      ) {
        count += 2;
      } else {
        count += 1;
      }
    }
    return count;
  }

  // Build the list of {label, limit, count, isOver, isWarning} badges.
  let badges = $derived.by(() => {
    const seen = new Set<string>();
    const out: Array<{ key: string; label: string; limit: number; count: number; isOver: boolean; isWarning: boolean }> = [];
    for (const intId of selectedIntegrations) {
      const provider = integrationProviders.get(intId);
      if (!provider) continue;
      if (seen.has(provider)) continue;
      seen.add(provider);
      const meta = providerMeta(provider);
      // v24-7: X uses weighted length; all other platforms use plain length.
      const count = (provider === 'x' || provider === 'twitter')
        ? weightedLength(plainText)
        : plainTextLength;
      out.push({
        key: provider,
        label: meta.label,
        limit: meta.charLimit,
        count,
        isOver: count > meta.charLimit,
        isWarning: count > meta.charLimit * 0.9 && count <= meta.charLimit,
      });
    }
    return out;
  });
</script>

{#if badges.length > 0}
  <div class="flex flex-wrap gap-2">
    {#each badges as b (b.key)}
      <span
        class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-[10px] font-medium border
          {b.isOver
            ? 'bg-error/10 border-error/30 text-error'
            : b.isWarning
              ? 'bg-warning/10 border-warning/30 text-warning'
              : 'bg-surface-hover border-line text-muted'}"
        title="{b.label} limit: {b.limit} chars"
      >
        <span>{b.label}</span>
        <span class="font-mono">{b.count}/{b.limit}</span>
      </span>
    {/each}
  </div>
{/if}
