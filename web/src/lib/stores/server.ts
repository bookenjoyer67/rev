import { writable, get } from 'svelte/store';

export interface NodeInfo {
	name: string;
	description: string;
	version: string;
	location?: { name?: string; lat?: number; lon?: number };
	communities_count: number;
	listed: boolean;
	federation_enabled: boolean;
}

export interface KnownServer {
	url: string;
	name: string;
	description: string;
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
		return JSON.parse(raw);
	} catch {
		return { active: null, known: [] };
	}
}

function saveToStorage(state: ServerState) {
	if (typeof localStorage === 'undefined') return;
	localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
}

export const serverState = writable<ServerState>(loadFromStorage());

serverState.subscribe(saveToStorage);

export function getActiveServer(): string | null {
	return get(serverState).active;
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
