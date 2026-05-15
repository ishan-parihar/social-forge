<script lang="ts">
  import RichTextEditor from "../RichTextEditor.svelte";
  let { content = "", onContentChange, integrationId }: {
    content?: string; integrationId?: string;
    onContentChange?: (html: string) => void;
  } = $props();
</script>
<div class="space-y-2">
  <div class="text-xs text-[#6b7280] mb-1">Facebook Post</div>
  <RichTextEditor {content} onUpdate={onContentChange} placeholder="What's on your mind?" />
</div>
