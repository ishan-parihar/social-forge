<script lang="ts">
  import CharCountEditor from "./CharCountEditor.svelte";
  let { content = "", onContentChange, integrationId }: {
    content?: string; integrationId?: string;
    onContentChange?: (html: string) => void;
  } = $props();
</script>
<CharCountEditor {content} {onContentChange} {integrationId} platformLabel="LinkedIn" charLimit={3000} placeholder="Share your professional thoughts..." />
