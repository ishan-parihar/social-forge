<script lang="ts">
  import type { TeamMember } from '$lib/api/teams';

  let {
    member,
    isOwner,
    onRemove,
  }: {
    member: TeamMember;
    isOwner: boolean;
    onRemove: (userId: string) => void;
  } = $props();

  const roleColors: Record<string, string> = {
    owner: 'bg-red-900/40 text-red-300 border-red-700/40',
    admin: 'bg-yellow-900/40 text-yellow-300 border-yellow-700/40',
    member: 'bg-green-900/40 text-green-300 border-green-700/40',
    viewer: 'bg-gray-800/40 text-gray-400 border-gray-700/40',
  };
</script>

<div class="flex items-center justify-between px-4 py-3 bg-[#131720] border border-[#1e2435] rounded-lg">
  <div class="flex items-center gap-3 min-w-0">
    <div class="w-8 h-8 rounded-full bg-indigo-900/50 flex items-center justify-center text-xs text-indigo-300 font-medium flex-shrink-0">
      {member.name.charAt(0).toUpperCase()}
    </div>
    <div class="min-w-0">
      <div class="text-sm text-[#e8edf5] truncate">{member.name}</div>
      <div class="text-xs text-[#6b7280] truncate">{member.email}</div>
    </div>
  </div>
  <div class="flex items-center gap-2 flex-shrink-0">
    <span class="px-2 py-0.5 text-xs rounded border {roleColors[member.role] || 'bg-gray-800 text-gray-400 border-gray-700'}">
      {member.role}
    </span>
    {#if isOwner && member.role !== 'owner'}
      <button
        onclick={() => onRemove(member.user_id)}
        class="text-xs text-[#6b7280] hover:text-red-400 transition-colors px-2 py-1"
      >
        Remove
      </button>
    {/if}
  </div>
</div>
