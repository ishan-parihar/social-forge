<script lang="ts">
  import RichTextEditor from "../RichTextEditor.svelte";
  let { content = "", onContentChange, integrationId }: {
    content?: string; integrationId?: string;
    onContentChange?: (html: string) => void;
  } = $props();
</script>
<div class="space-y-2">
  <div class="text-xs text-[#6b7280] mb-1">Post Content</div>
  <RichTextEditor {content} onUpdate={onContentChange} placeholder="Share an article, idea, or update..." />
</div>
