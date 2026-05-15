<script lang="ts">
  import { onMount } from "svelte";
  import { postsApi, type PostSummary } from "$lib/api/posts";
  import Badge from "$lib/ui/Badge.svelte";
  import { goto } from "$app/navigation";

  let posts = $state<PostSummary[]>([]);
  let filter = $state("all");
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function load() {
    loading = true;
    error = null;
    const params: any = {};
    if (filter !== "all") params.state = filter;
    const r = await postsApi.list(params);
    if (r.data) posts = r.data.posts;
    else error = r.error || "Failed to load posts";
    loading = false;
  }

  function toggleFilter(f: string) {
    filter = f;
    load();
  }

  onMount(load);

  const filters = ["all", "draft", "queued", "published", "error"];
</script>

<div class="space-y-4">
  <div class="flex items-center justify-between">
    <h2 class="text-xl font-semibold">Posts</h2>
    <button onclick={() => goto("/posts/new")} class="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-sm transition-colors">+ New Post</button>
  </div>

  <!-- Filter tabs -->
  <div class="flex gap-1 bg-[#131720] border border-[#1e2435] rounded-lg p-1 overflow-x-auto">
    {#each filters as f}
      <button
        onclick={() => toggleFilter(f)}
        class="px-3 py-1.5 text-xs capitalize rounded-md transition-colors {filter === f ? 'bg-indigo-600 text-white' : 'text-[#6b7280] hover:bg-[#1a1f2e]'}"
      >{f}</button>
    {/each}
  </div>

  <!-- Post list -->
  {#if error}
    <div class="text-center py-12 text-sm text-red-400">{error}</div>
  {:else if loading}
    <div class="text-center py-12 text-sm text-[#6b7280]">Loading...</div>
  {:else if posts.length === 0}
    <div class="text-center py-12 text-sm text-[#6b7280]">No posts found</div>
  {:else}
    <div class="bg-[#131720] border border-[#1e2435] rounded-xl overflow-hidden">
      {#each posts as post (post.id)}
        <button
          onclick={() => goto(`/posts/${post.id}`)}
          class="w-full flex items-center gap-4 px-4 py-3 border-b border-[#1e2435] last:border-0 hover:bg-[#1a1f2e] transition-colors text-left"
        >
          <div class="flex-1 min-w-0">
            <div class="text-sm truncate">{post.content}</div>
            <div class="text-xs text-[#6b7280] mt-0.5">{post.integration_name}</div>
          </div>
          <div class="text-xs text-[#6b7280] shrink-0">
            {post.scheduled_at ? new Date(post.scheduled_at).toLocaleDateString() : ""}
          </div>
          <Badge state={post.state as "draft" | "queued" | "published" | "error"} />
          {#if post.error_message}
            <span role="img" aria-label={post.error_message} class="text-xs text-red-400" title={post.error_message}>⚠</span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>
