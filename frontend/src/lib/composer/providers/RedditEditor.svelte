<script lang="ts">
  import CharCountEditor from "./CharCountEditor.svelte";
  let { content = "", onContentChange, integrationId }: {
    content?: string; integrationId?: string;
    onContentChange?: (html: string) => void;
  } = $props();
</script>
<CharCountEditor {content} {onContentChange} {integrationId} platformLabel="Reddit" charLimit={10000} placeholder="Write your post or comment..." />
