<script lang="ts">
  import { teamsApi } from '$lib/api/teams';
  import { toast } from '$lib/stores/toast';

  let {
    teamId,
    onInvite,
  }: {
    teamId: string;
    onInvite: () => void;
  } = $props();

  let email = $state('');
  let role = $state('member');
  let submitting = $state(false);
  let error = $state('');

  async function handleSubmit() {
    const trimmed = email.trim();
    if (!trimmed) {
      error = 'Email is required';
      return;
    }
    if (!trimmed.includes('@')) {
      error = 'Invalid email address';
      return;
    }
    submitting = true;
    error = '';
    const res = await teamsApi.invite(teamId, { email: trimmed, role });
    submitting = false;
    if (res.error) {
      error = res.error;
      toast(res.error, 'error');
      return;
    }
    toast('Invitation sent!', 'success');
    email = '';
    role = 'member';
    onInvite();
  }
</script>

<div class="space-y-3">
  <h4 class="text-sm font-medium text-[#e8edf5]">Invite Member</h4>
  <div class="flex gap-2">
    <input
      type="email"
      bind:value={email}
      placeholder="email@example.com"
      class="flex-1 px-3 py-2 bg-[#0b0e14] border border-[#1e2435] rounded-lg text-sm text-[#e8edf5] placeholder-[#4b5563] focus:outline-none focus:border-indigo-500/50"
    />
    <select
      bind:value={role}
      class="px-3 py-2 bg-[#0b0e14] border border-[#1e2435] rounded-lg text-sm text-[#e8edf5] focus:outline-none focus:border-indigo-500/50"
    >
      <option value="admin">Admin</option>
      <option value="member">Member</option>
      <option value="viewer">Viewer</option>
    </select>
    <button
      onclick={handleSubmit}
      disabled={submitting}
      class="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:bg-indigo-800 disabled:text-[#6b7280] text-white text-sm rounded-lg transition-colors"
    >
      {submitting ? '...' : 'Invite'}
    </button>
  </div>
  {#if error}
    <p class="text-xs text-red-400">{error}</p>
  {/if}
</div>
