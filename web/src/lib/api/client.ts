import { getActiveServer } from '$lib/stores/server';
import { getToken } from '$lib/stores/auth';

function getBase(): string {
	const server = getActiveServer();
	if (!server) throw new Error('Not connected to a server');
	return `${server}/api`;
}

async function request<T>(path: string, options?: RequestInit & { auth?: boolean }): Promise<T> {
	const base = getBase();
	console.log('[api]', options?.method || 'GET', path);
	return requestOn<T>(base, path, options);
}

async function requestOn<T>(base: string, path: string, options?: RequestInit & { auth?: boolean }): Promise<T> {
	const headers: Record<string, string> = {
		'Content-Type': 'application/json',
	};

	if (options?.auth) {
		const token = getToken();
		if (token) {
			headers['Authorization'] = `Bearer ${token}`;
		}
	}

	const res = await fetch(`${base}${path}`, {
		...options,
		headers: { ...headers, ...(options?.headers as Record<string, string> || {}) },
	});

	if (!res.ok) {
		const err = await res.json().catch(() => ({ error: res.statusText }));
		console.error('[api] error:', res.status, path, err.error || res.statusText);
		throw new Error(err.error || 'Request failed');
	}

	return res.json();
}

export const api = {
	communities: {
		list: () => request<any[]>('/communities'),
		get: (slug: string) => request<any>(`/communities/${slug}`, { auth: true }),
		members: (slug: string) => request<any[]>(`/communities/${slug}/members`),
		update: (slug: string, data: { name?: string; description?: string; visibility?: string; location_name?: string | null; location_lat?: number | null; location_lon?: number | null }) =>
			request<any>(`/communities/${slug}`, { method: 'PATCH', body: JSON.stringify(data), auth: true }),
		listInvites: (slug: string) => request<any[]>(`/communities/${slug}/invites`, { auth: true }),
		deleteInvite: (slug: string, code: string) =>
			request<any>(`/communities/${slug}/invites/${code}`, { method: 'DELETE', auth: true }),
		create: (data: { name: string; slug: string; description?: string; location_name?: string; location_lat?: number | null; location_lon?: number | null }) =>
			request<any>('/communities', {
				method: 'POST',
				body: JSON.stringify(data),
				auth: true,
			}),
		createInvite: (slug: string) =>
			request<any>(`/communities/${slug}/invite`, {
				method: 'POST',
				auth: true,
			}),
		join: (slug: string, code: string) =>
			request<any>(`/communities/${slug}/join`, {
				method: 'POST',
				body: JSON.stringify({ code }),
				auth: true,
			}),
	},

	posts: {
		list: (slug: string, filters?: Record<string, string>) => {
			const params = new URLSearchParams(
				Object.fromEntries(Object.entries(filters || {}).filter(([, v]) => v))
			).toString();
			const qs = params ? `?${params}` : '';
			return request<any[]>(`/communities/${slug}/posts${qs}`);
		},
		get: (slug: string, id: string) => request<any>(`/communities/${slug}/posts/${id}`),
		create: (slug: string, data: any) =>
			request<any>(`/communities/${slug}/posts`, {
				method: 'POST',
				body: JSON.stringify(data),
				auth: true,
			}),
		respond: (postId: string, message: string, serverUrl?: string) => {
			const base = serverUrl ? `${serverUrl}/api` : getBase();
			return requestOn<{ match_id: string }>( base, `/posts/${postId}/respond`, {
				method: 'POST',
				body: JSON.stringify({ message }),
				auth: true,
			});
		},
		update: (slug: string, id: string, data: { title?: string; body?: string; urgency?: string; status?: string }) =>
			request<any>(`/communities/${slug}/posts/${id}`, {
				method: 'PATCH',
				body: JSON.stringify(data),
				auth: true,
			}),
		fulfill: (slug: string, id: string) =>
			request<any>(`/communities/${slug}/posts/${id}`, {
				method: 'PATCH',
				body: JSON.stringify({ status: 'fulfilled' }),
				auth: true,
			}),
		withdraw: (slug: string, id: string) =>
			request<any>(`/communities/${slug}/posts/${id}`, {
				method: 'DELETE',
				auth: true,
			}).catch((e: Error) => { console.error('[delete] withdraw failed:', e.message); throw e; }),
	},

	conversations: {
		list: () => request<any[]>('/me/conversations', { auth: true }),
		get: (matchId: string) => request<any>(`/conversations/${matchId}`, { auth: true }),
		sendMessage: (matchId: string, body: string) =>
			request<any>(`/conversations/${matchId}/messages`, {
				method: 'POST',
				body: JSON.stringify({ body }),
				auth: true,
			}),
		updateStatus: (matchId: string, status: string) =>
			request<any>(`/conversations/${matchId}/status`, {
				method: 'PATCH',
				body: JSON.stringify({ status }),
				auth: true,
			}),
	},

	alliances: {
		list: () => request<any[]>('/alliances'),
	},

	admin: {
		stats: () => request<any>('/admin/stats', { auth: true }),
		listUsers: () => request<any[]>('/admin/users', { auth: true }),
		deleteUser: (id: string) => request<any>(`/admin/users/${id}`, { method: 'DELETE', auth: true }),
		changeRole: (id: string, role: string) => request<any>(`/admin/users/${id}/role`, {
			method: 'PATCH',
			body: JSON.stringify({ role }),
			auth: true,
		}),
		listCommunities: () => request<any[]>('/admin/communities', { auth: true }),
		deleteCommunity: (id: string) => request<any>(`/admin/communities/${id}`, { method: 'DELETE', auth: true }),
		listDirectory: () => request<any[]>('/admin/directory', { auth: true }),
		removeDirectoryEntry: (url: string) => request<any>(`/admin/directory/${encodeURIComponent(url)}`, { method: 'DELETE', auth: true }),
	},
};
