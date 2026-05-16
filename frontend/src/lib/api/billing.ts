import { api } from '$lib/api/client';

export interface Subscription {
	id: string;
	user_id: string;
	stripe_subscription_id: string | null;
	stripe_customer_id: string | null;
	plan: 'free' | 'pro' | 'business';
	status: string;
	current_period_start: string | null;
	current_period_end: string | null;
	cancel_at_period_end: boolean;
	created_at: string;
	updated_at: string;
}

export interface Invoice {
	id: string;
	amount: number;
	currency: string;
	status: string;
	invoice_url: string | null;
	paid_at: string | null;
	created_at: string;
}

export interface SubscriptionResponse {
	subscription: Subscription;
	plan_name: string;
	plan_features: string[];
}

export const billingApi = {
	createCheckout: (plan: string, interval: string, successUrl: string, cancelUrl: string) =>
		api.post<{ url: string }>('/api/billing/create-checkout', {
			plan,
			interval,
			success_url: successUrl,
			cancel_url: cancelUrl,
		}),

	getSubscription: () => api.get<SubscriptionResponse>('/api/billing/subscription'),

	getInvoices: (limit = 10, offset = 0) =>
		api.get<{ invoices: Invoice[] }>(`/api/billing/invoices?${new URLSearchParams({ limit: String(limit), offset: String(offset) })}`),

	createPortalSession: () => api.post<{ url: string }>('/api/billing/portal-session', {}),
};
