<script lang="ts">
  import RichTextEditor from "../RichTextEditor.svelte";
  let { content = "", onContentChange, integrationId }: {
    content?: string; integrationId?: string;
    onContentChange?: (html: string) => void;
  } = $props();
</script>
<div class="text-xs text-[#6b7280] mb-1">Content</div>
<RichTextEditor {content} onUpdate={onContentChange} placeholder="Write your post..." />
