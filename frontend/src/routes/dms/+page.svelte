<script lang="ts">
  import { toast } from "$lib/stores/toast";
  import { onMount, onDestroy } from "svelte";
  import { realtime } from "$lib/stores/realtime";
  import { dmsApi, type Conversation, type DmMessage } from "$lib/api/dms";
  import { integrationsApi, type Integration } from "$lib/api/integrations";

  let conversations = $state<Conversation[]>([]);
  let integrations = $state<Integration[]>([]);
  let selectedIntegrationId = $state<string | null>(null);
  let selectedId = $state<string | null>(null);
  let messages = $state<DmMessage[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let newMessage = $state("");
  let sending = $state(false);

  let selected = $derived(conversations.find(c => c.id === selectedId));

  async function loadIntegrations() {
    const r = await integrationsApi.list();
    if (r.data) {
      integrations = r.data.integrations.filter(i => !i.disabled);
      if (integrations.length > 0 && !selectedIntegrationId) {
        selectedIntegrationId = integrations[0].id;
      }
    }
  }

  async function load() {
    if (!selectedIntegrationId) {
      loading = false;
      return;
    }
    loading = true;
    error = null;
    const r = await dmsApi.listConversations(selectedIntegrationId!);
    if (r.data) {
      conversations = r.data.conversations;
    } else {
      error = r.error || "Failed to load conversations";
    }
    loading = false;
  }

  async function loadMessages(convId: string) {
    if (!selectedIntegrationId) return;
    const r = await dmsApi.getMessages(convId);
    if (r.data) {
      messages = r.data.messages;
    }
  }

  async function sendMessage() {
    if (!selectedId || !newMessage.trim() || !selectedIntegrationId || !selected) return;
    sending = true;
    const recipient = selected.participant_name || selected.participant || "";
    const r = await dmsApi.send(selectedIntegrationId!, recipient, newMessage);
    if (r.error) {
      toast("Error: " + r.error, "error");
    } else {
      newMessage = "";
      await loadMessages(selectedId);
    }
    sending = false;
  }

  function selectConversation(id: string) {
    selectedId = id;
    messages = [];
    loadMessages(id);
  }

  function platformIcon(p: string): string {
    const icons: Record<string, string> = { x: "X", reddit: "R", linkedin: "in", facebook: "f", instagram: "IG", telegram: "TG", whatsapp: "WA", discord: "DC" };
    return icons[p] || "•";
  }

  let dmsUnsubscribers: (() => void)[] = [];

  onMount(async () => {
    await loadIntegrations();
    load();
    dmsUnsubscribers.push(realtime.on('integration_connected', () => load()));
    dmsUnsubscribers.push(realtime.on('integration_disconnected', () => load()));
    dmsUnsubscribers.push(realtime.on('dm_received', () => {
      if (selectedId) loadMessages(selectedId);
      load();
    }));
  });

  onDestroy(() => {
    dmsUnsubscribers.forEach(fn => fn());
  });
</script>

<div class="page-enter space-y-6">
  <div class="flex items-center justify-between">
    <h2 class="text-xl font-semibold">Direct Messages</h2>
    <div class="flex gap-2 items-center">
      {#if integrations.length > 0}
        <select
          bind:value={selectedIntegrationId}
          onchange={load}
          class="px-3 py-1.5 text-sm bg-surface border border-line rounded-lg text-content"
        >
          {#each integrations as int (int.id)}
            <option value={int.id}>{int.provider_identifier} ({int.internal_id.slice(0, 8)})</option>
          {/each}
        </select>
      {/if}
      <button onclick={load} class="px-3 py-1.5 text-sm text-muted hover:text-white border border-line rounded-lg transition-colors">Refresh</button>
    </div>
  </div>

  {#if error}
    <div class="text-center py-12 text-sm text-red-400">{error}</div>
  {:else if loading}
    <div class="text-center py-12 text-sm text-muted">Loading...</div>
  {:else if conversations.length === 0}
    <div class="text-center py-12">
      <p class="text-sm text-muted mb-2">No conversations found</p>
      <p class="text-xs text-muted-dark">DMs are available for X, Instagram, and LinkedIn integrations.</p>
    </div>
  {:else}
    <div class="flex gap-4 h-[calc(100vh-200px)]">
      <!-- Conversation list -->
      <div class="w-80 bg-surface border border-line rounded-xl overflow-hidden flex flex-col shrink-0">
        <div class="p-3 border-b border-line">
          <span class="text-xs text-muted">{conversations.length} conversations</span>
        </div>
        <div class="flex-1 overflow-y-auto">
          {#each conversations as conv (conv.id)}
            <button
              onclick={() => selectConversation(conv.id)}
              class="w-full px-3 py-3 border-b border-line hover:bg-surface-hover transition-colors text-left {selectedId === conv.id ? 'bg-surface-hover' : ''}"
            >
              <div class="flex items-center gap-2 mb-1">
                <span class="text-xs text-indigo-400">{platformIcon(conv.platform)}</span>
                <span class="text-sm font-medium truncate">{conv.participant_name || conv.participant}</span>
                {#if conv.unread_count > 0}
                  <span class="ml-auto px-1.5 py-0.5 text-[10px] bg-indigo-600 text-white rounded-full">{conv.unread_count}</span>
                {/if}
              </div>
              <div class="flex items-center gap-2">
                <span class="text-xs text-muted truncate flex-1">{conv.last_message || '(no messages)'}</span>
                {#if conv.last_message_at}
                  <span class="text-[10px] text-muted shrink-0">{new Date(conv.last_message_at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</span>
                {/if}
              </div>
            </button>
          {/each}
        </div>
      </div>

      <!-- Message thread -->
      <div class="flex-1 bg-surface border border-line rounded-xl flex flex-col">
        {#if selected}
          <div class="px-4 py-3 border-b border-line">
            <div class="flex items-center gap-2">
              <span class="text-xs text-indigo-400">{platformIcon(selected.platform)}</span>
              <span class="text-sm font-medium">{selected.participant_name || selected.participant}</span>
            </div>
          </div>

          <div class="flex-1 overflow-y-auto p-4 space-y-3">
            {#each messages as msg (msg.id)}
              <div class="flex {msg.read ? 'justify-end' : 'justify-start'}">
                <div class="page-enter max-w-[70%] {msg.read ? 'bg-indigo-600/30' : 'bg-line'} rounded-xl px-3 py-2">
                  <p class="text-sm">{msg.content}</p>
                  <span class="text-[10px] text-muted">{new Date(msg.created_at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</span>
                </div>
              </div>
            {/each}
          </div>

          <div class="p-3 border-t border-line">
            <div class="flex gap-2">
              <input
                type="text"
                bind:value={newMessage}
                placeholder="Type a message..."
                class="flex-1 px-3 py-2 bg-background-input border border-line rounded-lg text-sm"
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
          <div class="flex-1 flex items-center justify-center text-sm text-muted">
            Select a conversation
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>
