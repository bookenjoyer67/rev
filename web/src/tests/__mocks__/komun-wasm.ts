import { vi } from 'vitest';

/**
 * Stand-in for the generated `komun-wasm` bindings.
 *
 * Tracks the real surface after A2.10: no signature keypair, no `sign`/`verify`, no
 * `compute_recovery_id`, no `hash_recovery_code`. `derive_key_from_password` replaces
 * `derive_key_from_passphrase`, and `encrypt_key_bundle` takes two arguments instead of three.
 *
 * The bundle mock is a real (if trivial) wrap rather than a constant, so a test can tell an
 * honest round-trip apart from a function that happens to return 32 bytes: `encrypt_key_bundle`
 * XORs the secret with the key and prefixes the key, and `decrypt_key_bundle` undoes exactly that,
 * failing loudly on a key mismatch the way the AEAD does.
 */

let counter = 0;
const nextByte = () => {
	counter++;
	return counter % 256;
};

const mockXKeypair = () => {
	const pub = new Uint8Array(32);
	const sec = new Uint8Array(32);
	for (let i = 0; i < 32; i++) {
		pub[i] = nextByte();
		sec[i] = nextByte();
	}
	return { public_key: pub.buffer, secret_key: sec.buffer };
};

export const generate_x25519_keypair = vi.fn(mockXKeypair);
export const encrypt_message = vi.fn((data: Uint8Array) => data.buffer);
export const decrypt_message = vi.fn((data: Uint8Array) => data.buffer);
export const derive_shared_key = vi.fn(() => new Uint8Array(32).fill(nextByte()).buffer);
export const encrypt_with_shared_key = vi.fn((data: Uint8Array) => data.buffer);
export const decrypt_with_shared_key = vi.fn((data: Uint8Array) => data.buffer);
export const generate_salt = vi.fn(() => new Uint8Array(16).fill(nextByte()).buffer);

/** Deterministic in (password, salt), like the Argon2id call it stands in for. */
export const derive_key_from_password = vi.fn((password: Uint8Array, salt: Uint8Array) => {
	const out = new Uint8Array(32);
	for (let i = 0; i < 32; i++) {
		out[i] = ((password[i % password.length] ?? 0) * 31 + (salt[i % salt.length] ?? 0)) % 256;
	}
	return out.buffer;
});

export const encrypt_key_bundle = vi.fn((secret: Uint8Array, key: Uint8Array) => {
	if (secret.length !== 32) throw new Error('invalid secret key length');
	if (key.length !== 32) throw new Error('invalid derived key length');
	const out = new Uint8Array(64);
	out.set(key, 0);
	for (let i = 0; i < 32; i++) out[32 + i] = secret[i] ^ key[i];
	return out.buffer;
});

export const decrypt_key_bundle = vi.fn((bundle: Uint8Array, key: Uint8Array) => {
	if (bundle.length !== 64) throw new Error('wrong password or corrupted bundle');
	for (let i = 0; i < 32; i++) {
		if (bundle[i] !== key[i]) throw new Error('wrong password or corrupted bundle');
	}
	const out = new Uint8Array(32);
	for (let i = 0; i < 32; i++) out[i] = bundle[32 + i] ^ key[i];
	return out.buffer;
});

export const generate_recovery_code = vi.fn(() =>
	[
		'abandon', 'ability', 'able', 'about', 'above', 'absent',
		'absorb', 'abstract', 'absurd', 'abuse', 'access', 'accident',
	].join(' ')
);

export default function init(): Promise<void> {
	counter = 0;
	return Promise.resolve();
}
