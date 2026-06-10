import { writable, get } from 'svelte/store';
import { getActiveServer } from './server';
import { generateFullKeypair, createKeyBundle, recoverFromBundle, computeRecoveryId } from '$lib/crypto';

interface PerServerAuth {
	token: string;
	userId: string;
	displayName: string;
	role: string;
}

interface KeyPair {
	ed25519PublicKey: string;
	ed25519SecretKey: string;
	x25519PublicKey: string;
	x25519SecretKey: string;
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
			return { keypair: null, servers: {} };
		}
		if (parsed.keypair && !parsed.keypair.x25519SecretKey) {
			parsed.keypair = null;
			parsed.servers = {};
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
	return get(auth).keypair?.x25519SecretKey || null;
}

export function getEncryptionPublicKey(): string | null {
	return get(auth).keypair?.x25519PublicKey || null;
}

export function getPublicKey(): string | null {
	return get(auth).keypair?.ed25519PublicKey || null;
}

export async function register(displayName: string, passphrase?: string): Promise<boolean> {
	const server = getActiveServer();
	if (!server) return false;

	let state = get(auth);
	if (!state.keypair) {
		const kp = await generateFullKeypair();
		state = { ...state, keypair: kp };
		auth.set(state);
	}

	const body: Record<string, string> = {
		display_name: displayName,
		public_key: state.keypair!.ed25519PublicKey,
		encryption_public_key: state.keypair!.x25519PublicKey,
	};

	if (passphrase && passphrase.length > 0) {
		const bundle = await createKeyBundle(
			state.keypair!.ed25519SecretKey,
			state.keypair!.x25519SecretKey,
			passphrase
		);
		body.encrypted_key_bundle = bundle.encryptedBundle;
		body.bundle_salt = bundle.salt;
		body.recovery_id = bundle.recoveryId;
	}

	const res = await fetch(`${server}/api/auth/register`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(body),
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
				role: data.role || 'user',
			},
		},
	}));

	return true;
}

export function isSuperadmin(): boolean {
	const auth_data = getActiveAuth();
	return auth_data?.role === 'superadmin';
}

export async function recover(serverUrl: string, passphrase: string): Promise<boolean> {
	try {
		const recoveryId = await computeRecoveryId(passphrase);

		const res = await fetch(`${serverUrl}/api/auth/recover`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ recovery_id: recoveryId }),
		});

		if (!res.ok) return false;

		const data = await res.json();
		const keys = await recoverFromBundle(data.encrypted_key_bundle, data.bundle_salt, passphrase);

		const keypair: KeyPair = {
			ed25519PublicKey: data.public_key,
			ed25519SecretKey: keys.ed25519Secret,
			x25519PublicKey: data.encryption_public_key || '',
			x25519SecretKey: keys.x25519Secret,
		};

		auth.set({ keypair, servers: {} });

		const regRes = await fetch(`${serverUrl}/api/auth/register`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({
				display_name: data.display_name,
				public_key: data.public_key,
				encryption_public_key: data.encryption_public_key,
			}),
		});

		if (regRes.ok) {
			const regData = await regRes.json();
			auth.update((s) => ({
				...s,
				servers: {
					...s.servers,
					[serverUrl]: {
						token: regData.token,
						userId: regData.user_id,
						displayName: regData.display_name,
						role: regData.role || 'user',
					},
				},
			}));
		}

		return true;
	} catch {
		return false;
	}
}

export async function setPassphrase(passphrase: string): Promise<boolean> {
	const server = getActiveServer();
	if (!server) return false;
	const state = get(auth);
	if (!state.keypair) return false;

	try {
		const bundle = await createKeyBundle(
			state.keypair.ed25519SecretKey,
			state.keypair.x25519SecretKey,
			passphrase
		);

		const res = await fetch(`${server}/api/auth/register`, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				'Authorization': `Bearer ${getToken()}`,
			},
			body: JSON.stringify({
				display_name: getActiveAuth()?.displayName,
				public_key: state.keypair.ed25519PublicKey,
				encryption_public_key: state.keypair.x25519PublicKey,
				encrypted_key_bundle: bundle.encryptedBundle,
				bundle_salt: bundle.salt,
				recovery_id: bundle.recoveryId,
			}),
		});

		return res.ok;
	} catch {
		return false;
	}
}

export async function updateDisplayName(newName: string): Promise<boolean> {
	const server = getActiveServer();
	if (!server) return false;
	const state = get(auth);
	if (!state.keypair) return false;

	try {
		const res = await fetch(`${server}/api/auth/register`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({
				display_name: newName,
				public_key: state.keypair.ed25519PublicKey,
				encryption_public_key: state.keypair.x25519PublicKey,
			}),
		});

		if (!res.ok) return false;
		const data = await res.json();
		auth.update((s) => ({
			...s,
			servers: {
				...s.servers,
				[server]: { ...s.servers[server], displayName: data.display_name },
			},
		}));
		return true;
	} catch {
		return false;
	}
}

export async function refreshRole(): Promise<void> {
	const server = getActiveServer();
	const token = getToken();
	if (!server || !token) return;
	try {
		const res = await fetch(`${server}/api/auth/me`, {
			headers: { 'Authorization': `Bearer ${token}` }
		});
		if (!res.ok) return;
		const data = await res.json();
		if (data.role) {
			auth.update((s) => ({
				...s,
				servers: {
					...s.servers,
					[server]: { ...s.servers[server], role: data.role },
				},
			}));
		}
	} catch {}
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
