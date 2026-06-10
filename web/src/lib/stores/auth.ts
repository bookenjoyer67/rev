import { writable, get } from 'svelte/store';
import { getActiveServer } from './server';
import { generateEncryptionKeypair } from '$lib/crypto';

interface PerServerAuth {
	token: string;
	userId: string;
	displayName: string;
}

interface KeyPair {
	publicKey: string;
	encryptionPublicKey: string;
	encryptionSecretKey: string;
}

interface AuthState {
	keypair: KeyPair | null;
	servers: Record<string, PerServerAuth>;
}

const STORAGE_KEY = 'komun_auth';

function loadFromStorage(): AuthState {
	if (typeof localStorage === 'undefined') return { keypair: null, servers: {} };
	const raw = localStorage.getItem(STORAGE_KEY);
	if (!raw) return { keypair: null, servers: {} };
	try {
		const parsed = JSON.parse(raw);
		if (!parsed.servers || typeof parsed.servers !== 'object') {
			return { keypair: parsed.keypair || null, servers: {} };
		}
		return parsed;
	} catch {
		return { keypair: null, servers: {} };
	}
}

function saveToStorage(state: AuthState) {
	if (typeof localStorage === 'undefined') return;
	localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
}

function generateIdentityKey(): string {
	const bytes = new Uint8Array(32);
	crypto.getRandomValues(bytes);
	return btoa(String.fromCharCode(...bytes));
}

export const auth = writable<AuthState>(loadFromStorage());

auth.subscribe(saveToStorage);

export function getActiveAuth(): PerServerAuth | null {
	const server = getActiveServer();
	if (!server) return null;
	return get(auth).servers[server] || null;
}

export function isAuthenticated(): boolean {
	return getActiveAuth() !== null;
}

export function getToken(): string | null {
	return getActiveAuth()?.token || null;
}

export function getDisplayName(): string | null {
	return getActiveAuth()?.displayName || null;
}

export function getEncryptionSecretKey(): string | null {
	return get(auth).keypair?.encryptionSecretKey || null;
}

export function getEncryptionPublicKey(): string | null {
	return get(auth).keypair?.encryptionPublicKey || null;
}

export async function register(displayName: string): Promise<boolean> {
	const server = getActiveServer();
	if (!server) return false;

	let state = get(auth);
	if (!state.keypair) {
		const encKp = await generateEncryptionKeypair();
		state = {
			...state,
			keypair: {
				publicKey: generateIdentityKey(),
				encryptionPublicKey: encKp.publicKey,
				encryptionSecretKey: encKp.secretKey,
			},
		};
		auth.set(state);
	}

	const res = await fetch(`${server}/api/auth/register`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({
			display_name: displayName,
			public_key: state.keypair!.publicKey,
			encryption_public_key: state.keypair!.encryptionPublicKey,
		}),
	});

	if (!res.ok) return false;

	const data = await res.json();
	auth.update((s) => ({
		...s,
		servers: {
			...s.servers,
			[server]: {
				token: data.token,
				userId: data.user_id,
				displayName: data.display_name,
			},
		},
	}));

	return true;
}

export function logout() {
	const server = getActiveServer();
	if (!server) return;
	auth.update((s) => {
		const { [server]: _, ...rest } = s.servers;
		return { ...s, servers: rest };
	});
}

let pendingAction: (() => void) | null = null;
export const showOnboarding = writable(false);

export function requireAuth(action: () => void) {
	if (isAuthenticated()) {
		action();
	} else {
		pendingAction = action;
		showOnboarding.set(true);
	}
}

export function onAuthComplete() {
	showOnboarding.set(false);
	if (pendingAction) {
		pendingAction();
		pendingAction = null;
	}
}
