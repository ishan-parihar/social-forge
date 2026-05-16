<script lang="ts">
	let {
		name,
		price,
		interval,
		features,
		highlighted = false,
		current = false,
		onSelect,
		loading = false,
	}: {
		name: string;
		price: string;
		interval: string;
		features: string[];
		highlighted?: boolean;
		current?: boolean;
		onSelect?: () => void;
		loading?: boolean;
	} = $props();
</script>

<div
	class="rounded-lg border {highlighted ? 'border-indigo-500 ring-1 ring-indigo-500' : 'border-[#1e2435]'} {current ? 'bg-indigo-500/10' : 'bg-[#131720]'} p-6 flex flex-col"
>
	<h3 class="text-lg font-semibold text-[#d1d5db]">{name}</h3>
	<div class="mt-4">
		<span class="text-3xl font-bold text-white">{price}</span>
		{#if interval}
			<span class="text-sm text-[#6b7280]">/{interval}</span>
		{/if}
	</div>
	<ul class="mt-6 space-y-3 flex-1">
		{#each features as feature, i (i)}
			<li class="flex items-center gap-2 text-sm text-[#9ca3af]">
				<span class="text-green-400">&check;</span>
				{feature}
			</li>
		{/each}
	</ul>
	<div class="mt-6">
		{#if current}
			<button disabled class="w-full rounded-md bg-[#1e2435] text-[#6b7280] px-4 py-2 text-sm font-medium cursor-not-allowed">
				Current Plan
			</button>
		{:else if onSelect}
			<button
				onclick={onSelect}
				disabled={loading}
				class="w-full rounded-md {highlighted ? 'bg-indigo-600 hover:bg-indigo-500' : 'bg-[#1e2435] hover:bg-[#2a3045]'} text-white px-4 py-2 text-sm font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
			>
				{loading ? 'Redirecting...' : 'Upgrade'}
			</button>
		{/if}
	</div>
</div>
