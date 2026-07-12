import { writable, get } from 'svelte/store';

export interface NodeInfo {
	name: string;
	description: string;
	version: string;
	domain?: string;
	location?: { name?: string; lat?: number; lon?: number };
	communities_count: number;
	listed: boolean;
	federation_enabled: boolean;
}

export interface KnownServer {
	url: string;
	name: string;
	description: string;
	domain?: string;
	lastSeen: number;
}

interface ServerState {
	active: string | null;
	known: KnownServer[];
}

const STORAGE_KEY = 'komun_servers';

function loadFromStorage(): ServerState {
	if (typeof localStorage === 'undefined') return { active: null, known: [] };
	const raw = localStorage.getItem(STORAGE_KEY);
	if (!raw) return { active: null, known: [] };
	try {
		const parsed = JSON.parse(raw) as ServerState;
		if (parsed.known) {
			for (const s of parsed.known) {
				if (!s.domain) s.domain = extractDomain(s.url);
			}
		}
		return parsed;
	} catch {
		return { active: null, known: [] };
	}
}

function saveToStorage(state: ServerState) {
	if (typeof localStorage === 'undefined') return;
	localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
}

function extractDomain(url: string): string {
	const host = url.replace(/^https?:\/\//, '').split('/')[0];
	return host.split(':')[0];
}

export const serverState = writable<ServerState>(loadFromStorage());

serverState.subscribe(saveToStorage);

export function getActiveServer(): string | null {
	return get(serverState).active;
}

export function getServerByDomain(domain: string): KnownServer | null {
	const state = get(serverState);
	return state.known.find(s => s.domain === domain) || null;
}

export function parseSlug(rawSlug: string): { localSlug: string; domain: string | null } {
	const atIdx = rawSlug.lastIndexOf('@');
	if (atIdx === -1) return { localSlug: rawSlug, domain: null };
	return {
		localSlug: rawSlug.substring(0, atIdx),
		domain: rawSlug.substring(atIdx + 1),
	};
}

export async function resolveSlug(rawSlug: string): Promise<{ localSlug: string; serverUrl: string }> {
	const { localSlug, domain } = parseSlug(rawSlug);

	if (!domain || domain === 'localhost') {
		const active = getActiveServer();
		if (!active) throw new Error('Not connected to a server');
		return { localSlug, serverUrl: active };
	}

	// Check known servers for this domain
	const known = getServerByDomain(domain);
	if (known) {
		// Switch to this server if not already active
		const active = getActiveServer();
		if (active !== known.url) {
			try { await connectToServer(known.url); } catch { /* keep going */ }
		}
		return { localSlug, serverUrl: known.url };
	}

	// Construct URL from domain — try https first
	const url = `https://${domain}`;
	try {
		const info = await connectToServer(url);
		return { localSlug, serverUrl: url };
	} catch {
		// Try http
		const urlHttp = `http://${domain}`;
		await connectToServer(urlHttp);
		return { localSlug, serverUrl: urlHttp };
	}
}

export function isConnected(): boolean {
	return get(serverState).active !== null;
}

export async function connectToServer(url: string): Promise<NodeInfo> {
	const normalized = url.replace(/\/+$/, '');

	const res = await fetch(`${normalized}/api/node`);
	if (!res.ok) throw new Error('Not a valid Komun server');

	const info: NodeInfo = await res.json();

	serverState.update((state) => {
		const existing = state.known.findIndex((s) => s.url === normalized);
		const entry: KnownServer = {
			url: normalized,
			name: info.name,
			description: info.description,
			domain: info.domain,
			lastSeen: Date.now(),
		};

		if (existing >= 0) {
			state.known[existing] = entry;
		} else {
			state.known.push(entry);
		}

		return { active: normalized, known: state.known };
	});

	return info;
}

export function disconnectServer() {
	serverState.update((state) => ({ ...state, active: null }));
}

export function removeServer(url: string) {
	serverState.update((state) => ({
		active: state.active === url ? null : state.active,
		known: state.known.filter((s) => s.url !== url),
	}));
}
