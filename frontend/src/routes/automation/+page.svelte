<script lang="ts">
  import { onMount } from "svelte";
  import { automationApi, type AutomationRule, type ExecutionLog } from "$lib/api/automation";
  import { toast } from "$lib/stores/toast";

  let rules = $state<AutomationRule[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let showModal = $state(false);
  let editingRule = $state<AutomationRule | null>(null);
  let showLogs = $state<string | null>(null);
  let logs = $state<ExecutionLog[]>([]);
  let loadingLogs = $state(false);

  // Form state
  let formName = $state("");
  let formPlatform = $state("x");
  let formTrigger = $state("comment");
  let formResponseType = $state("fixed");
  let formTemplate = $state("");
  let saving = $state(false);

  const platforms = ["x", "reddit", "linkedin", "facebook", "instagram", "telegram", "discord"];
  const triggers = ["comment", "dm", "mention", "follow"];
  const responseTypes = ["fixed", "template", "ai_generated"];

  async function load() {
    loading = true;
    error = null;
    const r = await automationApi.listRules();
    if (r.data) {
      rules = r.data.rules;
    } else {
      error = r.error || "Failed to load rules";
    }
    loading = false;
  }

  function openCreate() {
    editingRule = null;
    formName = "";
    formPlatform = "x";
    formTrigger = "comment";
    formResponseType = "fixed";
    formTemplate = "";
    showModal = true;
  }

  function openEdit(rule: AutomationRule) {
    editingRule = rule;
    formName = rule.name;
    formPlatform = rule.platform;
    formTrigger = rule.trigger_type;
    formResponseType = rule.response_type;
    formTemplate = rule.response_template;
    showModal = true;
  }

  async function saveRule() {
    saving = true;
    error = null;
    const body = {
      name: formName,
      platform: formPlatform,
      trigger_type: formTrigger,
      response_type: formResponseType,
      response_template: formTemplate,
    };
    const r = editingRule
      ? await automationApi.updateRule(editingRule.id, body as Partial<AutomationRule>)
      : await automationApi.createRule(body as Omit<AutomationRule, 'id' | 'is_active'>);
    if (r.error) {
      error = r.error;
    } else {
      showModal = false;
      await load();
    }
    saving = false;
  }

  async function deleteRule(id: string) {
    if (!confirm("Delete this automation rule?")) return;
    const r = await automationApi.deleteRule(id);
    if (r.error) {
      error = r.error;
    } else {
      await load();
    }
  }

  async function toggleActive(rule: AutomationRule) {
    const r = await automationApi.updateRule(rule.id, { is_active: !rule.is_active });
    if (r.error) {
      error = r.error;
    } else {
      rules = rules.map(r2 => r2.id === rule.id ? { ...r2, is_active: !r2.is_active } : r2);
    }
  }

  async function viewLogs(ruleId: string) {
    showLogs = ruleId;
    loadingLogs = true;
    const r = await automationApi.getLogs(ruleId);
    if (r.data) {
      logs = r.data.logs;
    } else {
      error = r.error || "Failed to load logs";
    }
    loadingLogs = false;
  }

  onMount(load);
</script>

<div class="page-enter space-y-6">
  <div class="flex items-center justify-between">
    <h2 class="text-xl font-semibold">Automation Rules</h2>
    <button onclick={openCreate} class="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-sm transition-colors">+ New Rule</button>
  </div>

  {#if error}
    <div class="text-center py-12 text-sm text-red-400">{error}</div>
  {:else if loading}
    <div class="text-center py-12 text-sm text-muted">Loading...</div>
  {:else if rules.length === 0}
    <div class="text-center py-12 text-sm text-muted">No automation rules yet. Create one to get started.</div>
  {:else}
    <div class="bg-surface border border-line rounded-xl overflow-hidden">
      <div class="grid grid-cols-[1fr_100px_100px_100px_100px_120px] gap-3 px-4 py-2 border-b border-line bg-background-input text-xs text-muted">
        <span>Name</span><span>Platform</span><span>Trigger</span><span>Response</span><span>Status</span><span>Actions</span>
      </div>
      {#each rules as rule (rule.id)}
        <div class="grid grid-cols-[1fr_100px_100px_100px_100px_120px] gap-3 px-4 py-3 border-b border-line last:border-0 hover:bg-surface-hover transition-colors items-center">
          <div>
            <div class="text-sm font-medium">{rule.name}</div>
            {#if rule.last_triggered}
              <div class="text-[10px] text-muted">Last: {new Date(rule.last_triggered).toLocaleDateString()}</div>
            {/if}
          </div>
          <span class="text-xs text-indigo-400 capitalize">{rule.platform}</span>
          <span class="text-xs text-muted capitalize">{rule.trigger_type}</span>
          <span class="text-xs text-muted capitalize">{rule.response_type.replace("_", " ")}</span>
          <button onclick={() => toggleActive(rule)} class="w-fit">
            {#if rule.is_active}
              <span class="px-2 py-0.5 text-xs rounded bg-green-500/20 text-green-400">Active</span>
            {:else}
              <span class="px-2 py-0.5 text-xs rounded bg-[#6b7280]/20 text-muted">Inactive</span>
            {/if}
          </button>
          <div class="flex items-center gap-2">
            <button onclick={() => openEdit(rule)} class="text-xs text-muted hover:text-indigo-400">Edit</button>
            <button onclick={() => viewLogs(rule.id)} class="text-xs text-muted hover:text-indigo-400">Logs</button>
            <button onclick={() => deleteRule(rule.id)} class="text-xs text-muted hover:text-red-400">Del</button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- Create/Edit Modal -->
{#if showModal}
  <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50" role="dialog">
    <div class="bg-background-input border border-line rounded-xl p-6 w-full max-w-md">
      <h3 class="text-lg font-semibold mb-4">{editingRule ? "Edit" : "Create"} Automation Rule</h3>
      
      <label class="block text-sm text-muted mb-1">Name</label>
      <input type="text" bind:value={formName} placeholder="Auto-reply to comments" class="w-full mb-3 px-3 py-2 bg-[#161b22] border border-[#30363d] rounded text-sm" />

      <label class="block text-sm text-muted mb-1">Platform</label>
      <select bind:value={formPlatform} class="w-full mb-3 px-3 py-2 bg-[#161b22] border border-[#30363d] rounded text-sm">
        {#each platforms as p}
          <option value={p}>{p}</option>
        {/each}
      </select>

      <label class="block text-sm text-muted mb-1">Trigger Type</label>
      <select bind:value={formTrigger} class="w-full mb-3 px-3 py-2 bg-[#161b22] border border-[#30363d] rounded text-sm">
        {#each triggers as t}
          <option value={t}>{t}</option>
        {/each}
      </select>

      <label class="block text-sm text-muted mb-1">Response Type</label>
      <select bind:value={formResponseType} class="w-full mb-3 px-3 py-2 bg-[#161b22] border border-[#30363d] rounded text-sm">
        {#each responseTypes as rt}
          <option value={rt}>{rt.replace("_", " ")}</option>
        {/each}
      </select>

      <label class="block text-sm text-muted mb-1">Response Template</label>
      <textarea bind:value={formTemplate} placeholder="Thanks for your comment! {user} said: {comment}" rows="4" class="w-full mb-4 px-3 py-2 bg-[#161b22] border border-[#30363d] rounded text-sm font-mono"></textarea>

      {#if error}<p class="text-red-400 text-sm mb-3">{error}</p>{/if}

      <div class="flex gap-3 justify-end">
        <button onclick={() => showModal = false} class="px-4 py-2 text-sm text-muted hover:text-white">Cancel</button>
        <button onclick={saveRule} disabled={saving || !formName.trim()} class="px-4 py-2 text-sm bg-indigo-600 hover:bg-indigo-500 rounded disabled:opacity-50">
          {saving ? "Saving..." : editingRule ? "Update" : "Create"}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Logs Modal -->
{#if showLogs}
  <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50" role="dialog">
    <div class="bg-background-input border border-line rounded-xl p-6 w-full max-w-lg max-h-[80vh] overflow-hidden flex flex-col">
      <div class="flex items-center justify-between mb-4">
        <h3 class="text-lg font-semibold">Execution Logs</h3>
        <button onclick={() => showLogs = null} class="text-muted hover:text-white">✕</button>
      </div>
      {#if loadingLogs}
        <div class="text-center py-8 text-sm text-muted">Loading logs...</div>
      {:else if logs.length === 0}
        <div class="text-center py-8 text-sm text-muted">No execution logs yet</div>
      {:else}
        <div class="flex-1 overflow-y-auto space-y-2">
          {#each logs as log (log.id)}
            <div class="bg-[#161b22] border border-[#30363d] rounded-lg p-3">
              <div class="flex items-center gap-2 mb-1">
                <span class="text-xs text-muted">{new Date(log.created_at).toLocaleString()}</span>
                <span class="px-1.5 py-0.5 text-[10px] rounded {log.status === 'success' ? 'bg-green-500/20 text-green-400' : 'bg-red-500/20 text-red-400'}">{log.status}</span>
              </div>
              <div class="text-xs text-muted mb-1">Input: {log.input_text}</div>
              <div class="text-xs text-content-secondary">Output: {log.output_text}</div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
{/if}
