<script lang="ts">
  import { onMount } from "svelte";
  import { postsApi, type PostDetail } from "$lib/api/posts";
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import Badge from "$lib/ui/Badge.svelte";
  import RichTextEditor from "$lib/composer/RichTextEditor.svelte";

  let post = $state<PostDetail | null>(null);
  let editing = $state(false);
  let editContent = $state("");
  let loading = $state(true);

  onMount(async () => {
    const id = $page.params.id;
    if (!id) { loading = false; return; }
    const r = await postsApi.get(id);
    if (r.data) { post = r.data; editContent = r.data.content; }
    loading = false;
  });

  async function save() {
    if (!post) return;
    await postsApi.create({
      integration_ids: [post.integration_id],
      content: editContent,
    });
    post.content = editContent;
    editing = false;
  }

  async function schedulePost() {
    if (!post) return;
    const at = prompt("Schedule for (ISO date):", new Date().toISOString());
    if (at) {
      await postsApi.schedule(post.id, at);
      post.state = "queued";
    }
  }

  async function deletePost() {
    if (!post || !confirm("Delete this post?")) return;
    await postsApi.delete(post.id);
    goto("/posts");
  }
</script>

<div class="max-w-2xl mx-auto space-y-6">
  <button onclick={() => goto("/posts")} class="text-sm text-[#6b7280] hover:text-white">&larr; Back to posts</button>

  {#if loading}
    <div class="text-center py-12 text-sm text-[#6b7280]">Loading...</div>
  {:else if post}
    <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-6 space-y-4">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2">
          <Badge state={post.state as "draft" | "queued" | "published" | "error"} />
          <span class="text-sm text-[#6b7280]">{post.integration_id}</span>
        </div>
        <div class="flex gap-2">
          {#if !editing}
            <button onclick={() => editing = true} class="text-xs text-indigo-400 hover:underline">Edit</button>
            {#if post.state === "draft"}
              <button onclick={schedulePost} class="text-xs text-indigo-400 hover:underline">Schedule</button>
            {/if}
            <button onclick={deletePost} class="text-xs text-red-400 hover:underline">Delete</button>
          {:else}
            <button onclick={save} class="text-xs text-green-400 hover:underline">Save</button>
            <button onclick={() => editing = false} class="text-xs text-[#6b7280] hover:underline">Cancel</button>
          {/if}
        </div>
      </div>

      {#if post.scheduled_at}
        <div class="text-sm text-[#6b7280]">
          Scheduled: {new Date(post.scheduled_at).toLocaleString()}
        </div>
      {/if}

      {#if editing}
        <RichTextEditor content={editContent} onUpdate={(html) => editContent = html} />
      {:else}
        <div class="prose prose-invert max-w-none text-sm">{@html post.content}</div>
      {/if}

      {#if post.platform_post_url}
        <a href={post.platform_post_url} target="_blank" class="text-sm text-indigo-400 hover:underline inline-block">
          View on platform &rarr;
        </a>
      {/if}

      {#if post.error_message}
        <div class="bg-red-500/10 border border-red-500/30 text-red-400 text-sm rounded-lg p-3">
          Error: {post.error_message}
        </div>
      {/if}
    </div>
  {:else}
    <div class="text-center py-12 text-sm text-[#6b7280]">Post not found</div>
  {/if}
</div>
