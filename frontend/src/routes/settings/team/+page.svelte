<script lang="ts">
  import { onMount } from 'svelte';
  import { teamsApi, type Team, type TeamMember } from '$lib/api/teams';
  import { toast } from '$lib/stores/toast';
  import { currentUser } from '$lib/stores/auth';
  import TeamMemberRow from '$lib/team/TeamMemberRow.svelte';
  import InviteDialog from '$lib/team/InviteDialog.svelte';

  let teams = $state<Team[]>([]);
  let loading = $state(true);
  let selectedTeam = $state<Team | null>(null);
  let members = $state<TeamMember[]>([]);
  let membersLoading = $state(false);

  // Create team form
  let showCreate = $state(false);
  let newName = $state('');
  let newSlug = $state('');
  let creating = $state(false);
  let createError = $state('');

  // Edit team
  let editing = $state(false);
  let editName = $state('');
  let editSlug = $state('');
  let saving = $state(false);

  // Delete confirmation
  let deleteConfirm = $state('');

  async function loadTeams() {
    loading = true;
    const res = await teamsApi.list();
    if (res.data) {
      teams = res.data;
    } else {
      toast(res.error || 'Failed to load teams', 'error');
    }
    loading = false;
  }

  async function loadMembers(teamId: string) {
    membersLoading = true;
    const res = await teamsApi.members(teamId);
    if (res.data) {
      members = res.data;
    } else {
      toast(res.error || 'Failed to load members', 'error');
    }
    membersLoading = false;
  }

  async function handleCreate() {
    const name = newName.trim();
    const slug = newSlug.trim();
    if (!name) { createError = 'Team name is required'; return; }
    if (!slug) { createError = 'Team slug is required'; return; }
    if (!/^[a-z0-9-]+$/.test(slug)) { createError = 'Slug must be lowercase alphanumeric with hyphens only'; return; }

    creating = true;
    createError = '';
    const res = await teamsApi.create({ name, slug });
    creating = false;
    if (res.error) {
      createError = res.error;
      toast(res.error, 'error');
      return;
    }
    toast('Team created!', 'success');
    showCreate = false;
    newName = '';
    newSlug = '';
    await loadTeams();
  }

  function selectTeam(team: Team) {
    selectedTeam = team;
    editing = false;
    deleteConfirm = '';
    loadMembers(team.id);
  }

  function startEdit() {
    if (!selectedTeam) return;
    editName = selectedTeam.name;
    editSlug = selectedTeam.slug;
    editing = true;
  }

  async function handleSaveEdit() {
    if (!selectedTeam) return;
    const name = editName.trim();
    const slug = editSlug.trim();
    if (!name || !slug) { toast('Name and slug are required', 'error'); return; }

    saving = true;
    const res = await teamsApi.update(selectedTeam.id, { name, slug });
    saving = false;
    if (res.error) { toast(res.error, 'error'); return; }
    toast('Team updated!', 'success');
    editing = false;
    await loadTeams();
    selectedTeam = res.data || { ...selectedTeam, name, slug };
  }

  async function handleDelete() {
    if (!selectedTeam) return;
    const res = await teamsApi.delete(selectedTeam.id);
    if (res.error) { toast(res.error, 'error'); return; }
    toast('Team deleted', 'success');
    selectedTeam = null;
    members = [];
    await loadTeams();
  }

  async function handleRemoveMember(userId: string) {
    if (!selectedTeam) return;
    const res = await teamsApi.removeMember(selectedTeam.id, userId);
    if (res.error) { toast(res.error, 'error'); return; }
    toast('Member removed', 'success');
    await loadMembers(selectedTeam.id);
  }

  function autoSlug(name: string) {
    newSlug = name.toLowerCase().replace(/[^a-z0-9-]/g, '-').replace(/-+/g, '-').replace(/^-|-$/g, '');
  }

  onMount(loadTeams);
</script>

<div class="space-y-6">
  <div class="flex items-center justify-between">
    <h2 class="text-xl font-semibold text-[#e8edf5]">Team Management</h2>
    <button
      onclick={() => { showCreate = !showCreate; createError = ''; }}
      class="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 text-white text-sm rounded-lg transition-colors"
    >
      {showCreate ? 'Cancel' : 'Create Team'}
    </button>
  </div>

  {#if showCreate}
    <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5 space-y-3">
      <h3 class="text-sm font-medium text-[#e8edf5]">New Team</h3>
      <div>
        <label class="text-xs text-[#6b7280] block mb-1">Name</label>
        <input
          type="text"
          bind:value={newName}
          oninput={() => autoSlug(newName)}
          placeholder="My Team"
          class="w-full px-3 py-2 bg-[#0b0e14] border border-[#1e2435] rounded-lg text-sm text-[#e8edf5] placeholder-[#4b5563] focus:outline-none focus:border-indigo-500/50"
        />
      </div>
      <div>
        <label class="text-xs text-[#6b7280] block mb-1">Slug</label>
        <input
          type="text"
          bind:value={newSlug}
          placeholder="my-team"
          class="w-full px-3 py-2 bg-[#0b0e14] border border-[#1e2435] rounded-lg text-sm text-[#e8edf5] placeholder-[#4b5563] focus:outline-none focus:border-indigo-500/50"
        />
      </div>
      {#if createError}
        <p class="text-xs text-red-400">{createError}</p>
      {/if}
      <button
        onclick={handleCreate}
        disabled={creating}
        class="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:bg-indigo-800 disabled:text-[#6b7280] text-white text-sm rounded-lg transition-colors"
      >
        {creating ? 'Creating...' : 'Create'}
      </button>
    </div>
  {/if}

  <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
    <div class="lg:col-span-1">
      <div class="bg-[#131720] border border-[#1e2435] rounded-xl">
        <div class="px-5 py-3 border-b border-[#1e2435]">
          <h3 class="text-sm font-medium text-[#e8edf5]">Your Teams</h3>
        </div>
        <div class="p-2 space-y-1">
          {#if loading}
            <p class="text-xs text-[#6b7280] px-3 py-4">Loading...</p>
          {:else if teams.length === 0}
            <p class="text-xs text-[#6b7280] px-3 py-4">No teams yet. Create one above.</p>
          {:else}
            {#each teams as team (team.id)}
              <button
                onclick={() => selectTeam(team)}
                class="w-full text-left px-3 py-2.5 rounded-lg text-sm transition-colors
                  {selectedTeam?.id === team.id ? 'bg-[#1a1f2e] text-indigo-400' : 'text-[#6b7280] hover:text-[#e8edf5] hover:bg-[#1a1f2e]'}"
              >
                <div class="font-medium">{team.name}</div>
                <div class="text-xs text-[#4b5563]">{team.slug} &middot; {team.member_count} {team.member_count === 1 ? 'member' : 'members'}</div>
              </button>
            {/each}
          {/if}
        </div>
      </div>
    </div>

    <div class="lg:col-span-2">
      {#if selectedTeam}
        <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5 space-y-4">
          {#if editing}
            <div class="space-y-3">
              <h3 class="text-sm font-medium text-[#e8edf5]">Edit Team</h3>
              <div>
                <label class="text-xs text-[#6b7280] block mb-1">Name</label>
                <input
                  type="text"
                  bind:value={editName}
                  class="w-full px-3 py-2 bg-[#0b0e14] border border-[#1e2435] rounded-lg text-sm text-[#e8edf5] focus:outline-none focus:border-indigo-500/50"
                />
              </div>
              <div>
                <label class="text-xs text-[#6b7280] block mb-1">Slug</label>
                <input
                  type="text"
                  bind:value={editSlug}
                  class="w-full px-3 py-2 bg-[#0b0e14] border border-[#1e2435] rounded-lg text-sm text-[#e8edf5] focus:outline-none focus:border-indigo-500/50"
                />
              </div>
              <div class="flex gap-2">
                <button
                  onclick={handleSaveEdit}
                  disabled={saving}
                  class="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:bg-indigo-800 text-white text-sm rounded-lg transition-colors"
                >
                  {saving ? 'Saving...' : 'Save'}
                </button>
                <button
                  onclick={() => { editing = false; }}
                  class="px-4 py-2 bg-[#1e2435] hover:bg-[#2a3045] text-[#e8edf5] text-sm rounded-lg transition-colors"
                >
                  Cancel
                </button>
              </div>
            </div>
          {:else}
            <div class="flex items-center justify-between">
              <div>
                <h3 class="text-base font-medium text-[#e8edf5]">{selectedTeam.name}</h3>
                <p class="text-xs text-[#6b7280]">{selectedTeam.slug} &middot; {selectedTeam.member_count} {selectedTeam.member_count === 1 ? 'member' : 'members'}</p>
              </div>
              <div class="flex gap-2">
                <button onclick={startEdit} class="text-xs text-[#6b7280] hover:text-indigo-400 transition-colors px-3 py-1.5 bg-[#1e2435] rounded-lg">Edit</button>
                {#if $currentUser?.id === selectedTeam.owner_id}
                  <button
                    onclick={() => deleteConfirm = 'confirm'}
                    class="text-xs text-red-400 hover:text-red-300 transition-colors px-3 py-1.5 bg-[#1e2435] rounded-lg"
                  >
                    Delete
                  </button>
                {/if}
              </div>
            </div>
          {/if}

          {#if deleteConfirm === 'confirm'}
            <div class="bg-red-900/20 border border-red-800/40 rounded-lg p-4 space-y-2">
              <p class="text-sm text-red-300">Are you sure you want to delete this team? This action cannot be undone.</p>
              <div class="flex gap-2">
                <button onclick={handleDelete} class="px-4 py-2 bg-red-700 hover:bg-red-600 text-white text-sm rounded-lg transition-colors">Delete Team</button>
                <button onclick={() => deleteConfirm = ''} class="px-4 py-2 bg-[#1e2435] hover:bg-[#2a3045] text-[#e8edf5] text-sm rounded-lg transition-colors">Cancel</button>
              </div>
            </div>
          {/if}

          <hr class="border-[#1e2435]" />

          <InviteDialog
            teamId={selectedTeam.id}
            onInvite={() => loadMembers(selectedTeam!.id)}
          />

          <hr class="border-[#1e2435]" />

          <div>
            <h4 class="text-sm font-medium text-[#e8edf5] mb-3">Members ({members.length})</h4>
            {#if membersLoading}
              <p class="text-xs text-[#6b7280]">Loading...</p>
            {:else if members.length === 0}
              <p class="text-xs text-[#6b7280]">No members</p>
            {:else}
              <div class="space-y-2">
                {#each members as member (member.id)}
                  <TeamMemberRow
                    member={member}
                    isOwner={$currentUser?.id === selectedTeam.owner_id}
                    onRemove={handleRemoveMember}
                  />
                {/each}
              </div>
            {/if}
          </div>
        </div>
      {:else}
        <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-10 text-center">
          <p class="text-sm text-[#6b7280]">Select a team to manage</p>
        </div>
      {/if}
    </div>
  </div>
</div>
