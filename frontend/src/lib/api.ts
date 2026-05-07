// Vite proxy handles /api/* -> http://localhost:3000/*
// Empty string = same-origin = no CORS issues
const API_BASE = "";
let _token: string | null = null;
const isBrowser =
	typeof window !== "undefined" && typeof localStorage !== "undefined";

export function getToken() {
	return _token;
}
export function setToken(t: string | null) {
	_token = t;
	if (isBrowser) {
		if (t) localStorage.setItem("token", t);
		else localStorage.removeItem("token");
	}
}
export function loadToken() {
	if (isBrowser) {
		const t = localStorage.getItem("token");
		if (t) _token = t;
	}
	return _token;
}

async function request<T>(
	method: string,
	path: string,
	body?: unknown,
	timeoutMs = 10000,
): Promise<{ data?: T; error?: string; status: number }> {
	const headers: Record<string, string> = {};
	let reqBody: BodyInit | undefined;
	if (body && !(body instanceof FormData)) {
		headers["Content-Type"] = "application/json";
		reqBody = JSON.stringify(body);
	} else if (body instanceof FormData) reqBody = body;
	if (_token || loadToken()) headers["Authorization"] = `Bearer ${_token}`;
	try {
		const controller = new AbortController();
		const timeout = setTimeout(() => controller.abort(), timeoutMs);
		const res = await fetch(`${API_BASE}${path}`, {
			method,
			headers,
			body: reqBody,
			signal: controller.signal,
		});
		clearTimeout(timeout);
		const text = await res.text();
		const data = text ? JSON.parse(text) : {};
		if (!res.ok)
			return { error: data.error || `HTTP ${res.status}`, status: res.status };
		return { data, status: res.status };
	} catch (e: any) {
		if (e.name === "AbortError")
			return { error: "Request timed out", status: 0 };
		return { error: e.message, status: 0 };
	}
}

export const auth = {
	register: (e: string, p: string, n: string) =>
		request<{
			token: string;
			user: { id: string; email: string; name: string };
		}>("POST", "/api/auth/register", { email: e, password: p, name: n }),
	login: (e: string, p: string) =>
		request<{
			token: string;
			user: { id: string; email: string; name: string };
		}>("POST", "/api/auth/login", { email: e, password: p }),
	me: () =>
		request<{ id: string; email: string; name: string }>("GET", "/api/auth/me"),
};

export const posts = {
	list: (p?: { state?: string; limit?: number; offset?: number }) => {
		const q = new URLSearchParams();
		if (p?.state) q.set("state", p.state);
		if (p?.limit) q.set("limit", String(p.limit));
		if (p?.offset) q.set("offset", String(p.offset));
		return request<{ posts: PostSummary[]; total: number }>(
			"GET",
			`/api/posts?${q}`,
		);
	},
	get: (id: string) => request<PostDetail>("GET", `/api/posts/${id}`),
	create: (d: {
		integration_id: string;
		content: string;
		title?: string;
		scheduled_at?: string;
	}) => request<PostDetail>("POST", "/api/posts", d),
	schedule: (id: string, at: string) =>
		request<PostDetail>("POST", `/api/posts/${id}/schedule`, {
			scheduled_at: at,
		}),
	delete: (id: string) =>
		request<{ deleted: boolean }>("DELETE", `/api/posts/${id}`),
	findSlot: () => request<{ date: string }>("GET", "/api/posts/find-slot"),
};

export const integrations = {
	list: () =>
		request<{ integrations: Integration[] }>("GET", "/api/integrations"),
	connect: (p: string) =>
		request<{ url: string; state: string }>(
			"GET",
			`/api/integrations/connect/${p}`,
		),
	delete: (id: string) =>
		request<{ deleted: boolean }>("DELETE", `/api/integrations/${id}`),
};

export const calendar = {
	get: (s: string, e: string) =>
		request<{ days: CalendarDay[]; total: number }>(
			"GET",
			`/api/calendar?start=${s}&end=${e}`,
		),
};

export const media = {
	upload: (file: File) => {
		const fd = new FormData();
		fd.append("file", file);
		return request<MediaItem>("POST", "/api/media", fd, 30000);
	},
};

export interface PostSummary {
	id: string;
	integration_name: string;
	state: string;
	content: string;
	title?: string;
	scheduled_at?: string;
	platform_post_url?: string;
	error_message?: string;
	created_at: string;
}
export interface PostDetail {
	id: string;
	integration_id: string;
	state: string;
	content: string;
	title?: string;
	scheduled_at?: string;
	published_at?: string;
	platform_post_url?: string;
	error_message?: string;
	created_at: string;
}
export interface Integration {
	id: string;
	provider_identifier: string;
	provider_name: string;
	profile_name?: string;
	profile_picture?: string;
	disabled: boolean;
	refresh_needed: boolean;
}
export interface CalendarDay {
	date: string;
	posts: PostSummary[];
	post_count: number;
}
export interface MediaItem {
	id: string;
	original_name: string;
	url: string;
	mime_type: string;
	file_size: number;
	width?: number;
	height?: number;
}
