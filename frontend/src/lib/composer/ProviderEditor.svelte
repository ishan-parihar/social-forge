<script lang="ts">
  import { getEditor } from "./providers";
  let { provider = "", content = "", onContentChange, integrationId }: {
    provider?: string; integrationId?: string;
    content?: string; onContentChange?: (html: string) => void;
  } = $props();
  let EditorComponent = $derived(getEditor(provider));
</script>
{#if EditorComponent}
  <EditorComponent {content} {onContentChange} {integrationId} />
{/if}
