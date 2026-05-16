<script lang="ts">
	import { onMount } from 'svelte';
	import { billingApi, type SubscriptionResponse, type Invoice } from '$lib/api/billing';
	import PlanCard from '$lib/ui/PlanCard.svelte';
	import Spinner from '$lib/ui/Spinner.svelte';

	let subscription = $state<SubscriptionResponse | null>(null);
	let invoices = $state<Invoice[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let upgrading = $state<string | null>(null);

	onMount(async () => {
		await loadData();
	});

	async function loadData() {
		loading = true;
		error = null;
		try {
			const [subRes, invRes] = await Promise.all([
				billingApi.getSubscription(),
				billingApi.getInvoices(10, 0),
			]);
			if (subRes.data) subscription = subRes.data;
			if (invRes.data) invoices = invRes.data.invoices;
		} catch (e) {
			error = 'Failed to load billing data';
		} finally {
			loading = false;
		}
	}

	async function upgrade(plan: string, interval: string) {
		upgrading = plan;
		error = null;
		try {
			const r = await billingApi.createCheckout(
				plan,
				interval,
				`${window.location.origin}/settings/billing?success=true`,
				`${window.location.origin}/settings/billing?canceled=true`,
			);
			if (r.error) {
				error = r.error;
				return;
			}
			if (r.data?.url) window.location.href = r.data.url;
		} catch (e) {
			error = 'Checkout failed. Please try again.';
			console.error('Checkout failed:', e);
		} finally {
			upgrading = null;
		}
	}

	async function manageBilling() {
		error = null;
		try {
			const r = await billingApi.createPortalSession();
			if (r.data?.url) window.location.href = r.data.url;
		} catch (e) {
			error = 'Failed to open billing portal';
		}
	}
</script>

<div class="p-6 max-w-5xl mx-auto">
	<h1 class="text-2xl font-bold text-white mb-2">Billing</h1>
	<p class="text-sm text-[#6b7280] mb-8">Manage your subscription and billing information.</p>

	{#if loading}
		<div class="flex justify-center py-12"><Spinner size="lg" /></div>
	{:else if error}
		<div class="text-red-400 text-center py-12">{error}</div>
	{:else}
		{#if subscription}
			<div class="bg-[#131720] border border-[#1e2435] rounded-lg p-4 mb-8">
				<div class="flex items-center justify-between">
					<div>
						<span class="text-sm text-[#6b7280]">Current Plan</span>
						<p class="text-lg font-semibold text-white">{subscription.plan_name}</p>
					</div>
					<div class="text-right">
						<span class="text-sm text-[#6b7280]">Status</span>
						<p class="text-sm capitalize text-[#9ca3af]">{subscription.subscription.status}</p>
					</div>
					{#if subscription.subscription.stripe_customer_id}
						<button onclick={manageBilling} class="text-sm text-indigo-400 hover:text-indigo-300 underline">
							Manage on Stripe
						</button>
					{/if}
				</div>
			</div>
		{/if}

		<div class="grid grid-cols-1 md:grid-cols-3 gap-6 mb-8">
			<PlanCard
				name="Free"
				price="$0"
				interval=""
				features={['Up to 5 social channels', 'Basic analytics', 'Schedule posts']}
				current={subscription?.subscription.plan === 'free'}
			/>
			<PlanCard
				name="Pro"
				price="$29"
				interval="month"
				features={['Up to 15 social channels', 'Advanced analytics', 'Team members (3)', 'AI suggestions', 'RSS autopost']}
				highlighted={true}
				current={subscription?.subscription.plan === 'pro'}
				onSelect={() => upgrade('pro', 'monthly')}
				loading={upgrading === 'pro'}
			/>
			<PlanCard
				name="Business"
				price="$99"
				interval="month"
				features={['Unlimited social channels', 'Premium analytics', 'Unlimited team members', 'AI assistant', 'Priority support', 'Custom integrations']}
				current={subscription?.subscription.plan === 'business'}
				onSelect={() => upgrade('business', 'monthly')}
				loading={upgrading === 'business'}
			/>
		</div>

		<div class="bg-[#131720] border border-[#1e2435] rounded-lg p-4">
			<h2 class="text-lg font-semibold text-white mb-4">Invoices</h2>
			{#if invoices.length === 0}
				<p class="text-sm text-[#6b7280] text-center py-4">No invoices yet</p>
			{:else}
				<table class="w-full text-sm">
					<thead>
						<tr class="text-[#6b7280] border-b border-[#1e2435]">
							<th class="text-left py-2">Date</th>
							<th class="text-left py-2">Amount</th>
							<th class="text-left py-2">Status</th>
							<th class="text-right py-2">Receipt</th>
						</tr>
					</thead>
					<tbody>
						{#each invoices as invoice (invoice.id)}
							<tr class="border-b border-[#1e2435] text-[#d1d5db]">
								<td class="py-2">{new Date(invoice.created_at).toLocaleDateString()}</td>
								<td class="py-2">${(invoice.amount / 100).toFixed(2)} {invoice.currency.toUpperCase()}</td>
								<td class="py-2"><span class="capitalize text-green-400">{invoice.status}</span></td>
								<td class="py-2 text-right">
									{#if invoice.invoice_url}
										<a href={invoice.invoice_url} target="_blank" rel="noopener noreferrer" class="text-indigo-400 hover:text-indigo-300">View</a>
									{:else}
										<span class="text-[#6b7280]">&mdash;</span>
									{/if}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{/if}
		</div>
	{/if}
</div>
