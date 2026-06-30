<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";

  interface Message {
    id: string;
    sender: string;
    text: string;
    created_at: string;
    is_mine: boolean;
  }

  interface Conversation {
    id: string;
    platform: string;
    contact: string;
    last_message: string;
    updated_at: string;
    unread_count: number;
    messages: Message[];
  }

  let conversations = $state<Conversation[]>([]);
  let selectedId = $state<string | null>(null);
  let filterPlatform = $state("all");
  let loading = $state(true);
  let error = $state<string | null>(null);
  let newMessage = $state("");
  let sending = $state(false);

  const platforms = ["all", "x", "reddit", "linkedin", "facebook", "instagram", "telegram", "whatsapp", "discord"];

  let selected = $derived(conversations.find(c => c.id === selectedId));

  async function load() {
    loading = true;
    error = null;
    const params = new URLSearchParams();
    if (filterPlatform !== "all") params.set("platform", filterPlatform);
    const r = await api.get<{ conversations: Conversation[] }>(`/api/dms/conversations?${params}`);
    if (r.data) {
      conversations = r.data.conversations;
    } else {
      error = r.error || "Failed to load conversations";
    }
    loading = false;
  }

  async function sendMessage() {
    if (!selectedId || !newMessage.trim()) return;
    sending = true;
    // TODO: Backend expects integration_id, recipient, content, media
    // Frontend currently sends text only - need to track integration_id and recipient
    const r = await api.post(`/api/dms/send`, { 
      integration_id: selectedId,  // FIXME: Should be the actual integration UUID
      recipient: selected?.contact || "",  // FIXME: Should be the recipient identifier
      content: newMessage,
      media: []
    });
    if (r.error) {
      error = r.error;
    } else {
      newMessage = "";
      await load();
    }
    sending = false;
  }

  function selectConversation(id: string) {
    selectedId = id;
    // Mark as read
    const conv = conversations.find(c => c.id === id);
    if (conv) conv.unread_count = 0;
  }

  function platformIcon(p: string): string {
    const icons: Record<string, string> = { x: "𝕏", reddit: "𝗥", linkedin: "in", facebook: "f", instagram: "📷", telegram: "✈", whatsapp: "📱", discord: "🎮" };
    return icons[p] || "•";
  }

  onMount(load);
</script>

<div class="space-y-4">
  <div class="flex items-center justify-between">
    <h2 class="text-xl font-semibold">Direct Messages</h2>
    <button onclick={load} class="px-3 py-1.5 text-sm text-[#6b7280] hover:text-white border border-[#1e2435] rounded-lg transition-colors">↻ Refresh</button>
  </div>

  <!-- Platform filter -->
  <div class="flex gap-1 bg-[#131720] border border-[#1e2435] rounded-lg p-1 overflow-x-auto">
    {#each platforms as p}
      <button
        onclick={() => { filterPlatform = p; load(); }}
        class="px-3 py-1.5 text-xs capitalize rounded-md transition-colors {filterPlatform === p ? 'bg-indigo-600 text-white' : 'text-[#6b7280] hover:bg-[#1a1f2e]'}"
      >{p}</button>
    {/each}
  </div>

  {#if error}
    <div class="text-center py-12 text-sm text-red-400">{error}</div>
  {:else if loading}
    <div class="text-center py-12 text-sm text-[#6b7280]">Loading...</div>
  {:else if conversations.length === 0}
    <div class="text-center py-12 text-sm text-[#6b7280]">No conversations found</div>
  {:else}
    <!-- Two-panel layout -->
    <div class="flex gap-4 h-[calc(100vh-200px)]">
      <!-- Conversation list -->
      <div class="w-80 bg-[#131720] border border-[#1e2435] rounded-xl overflow-hidden flex flex-col shrink-0">
        <div class="p-3 border-b border-[#1e2435]">
          <span class="text-xs text-[#6b7280]">{conversations.length} conversations</span>
        </div>
        <div class="flex-1 overflow-y-auto">
          {#each conversations as conv (conv.id)}
            <button
              onclick={() => selectConversation(conv.id)}
              class="w-full px-3 py-3 border-b border-[#1e2435] hover:bg-[#1a1f2e] transition-colors text-left {selectedId === conv.id ? 'bg-[#1a1f2e]' : ''}"
            >
              <div class="flex items-center gap-2 mb-1">
                <span class="text-xs text-indigo-400">{platformIcon(conv.platform)}</span>
                <span class="text-sm font-medium truncate">{conv.contact}</span>
                {#if conv.unread_count > 0}
                  <span class="ml-auto px-1.5 py-0.5 text-[10px] bg-indigo-600 text-white rounded-full">{conv.unread_count}</span>
                {/if}
              </div>
              <div class="flex items-center gap-2">
                <span class="text-xs text-[#6b7280] truncate flex-1">{conv.last_message}</span>
                <span class="text-[10px] text-[#6b7280] shrink-0">{new Date(conv.updated_at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</span>
              </div>
            </button>
          {/each}
        </div>
      </div>

      <!-- Message thread -->
      <div class="flex-1 bg-[#131720] border border-[#1e2435] rounded-xl flex flex-col">
        {#if selected}
          <!-- Header -->
          <div class="px-4 py-3 border-b border-[#1e2435]">
            <div class="flex items-center gap-2">
              <span class="text-xs text-indigo-400">{platformIcon(selected.platform)}</span>
              <span class="text-sm font-medium">{selected.contact}</span>
            </div>
          </div>

          <!-- Messages -->
          <div class="flex-1 overflow-y-auto p-4 space-y-3">
            {#each selected.messages as msg (msg.id)}
              <div class="flex {msg.is_mine ? 'justify-end' : 'justify-start'}">
                <div class="max-w-[70%] {msg.is_mine ? 'bg-indigo-600/30' : 'bg-[#1e2435]'} rounded-xl px-3 py-2">
                  <p class="text-sm">{msg.text}</p>
                  <span class="text-[10px] text-[#6b7280]">{new Date(msg.created_at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</span>
                </div>
              </div>
            {/each}
          </div>

          <!-- Input -->
          <div class="p-3 border-t border-[#1e2435]">
            <div class="flex gap-2">
              <input
                type="text"
                bind:value={newMessage}
                placeholder="Type a message..."
                class="flex-1 px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm"
                onkeydown={(e) => e.key === "Enter" && sendMessage()}
              />
              <button
                onclick={sendMessage}
                disabled={sending || !newMessage.trim()}
                class="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-sm disabled:opacity-50 transition-colors"
              >
                {sending ? "..." : "Send"}
              </button>
            </div>
          </div>
        {:else}
          <div class="flex-1 flex items-center justify-center text-sm text-[#6b7280]">
            Select a conversation
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>
