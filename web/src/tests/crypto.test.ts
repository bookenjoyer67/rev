import { describe, it, expect } from 'vitest';
import {
	bytesToBase64,
	generateIdentityKeypair,
	generateSaltB64,
	deriveVerifier,
	wrapSecret,
	unwrapSecret,
	generateRecoveryCode,
	normalizeRecoveryCode,
} from '$lib/crypto';

describe('bytesToBase64', () => {
	it('encodes empty bytes', () => {
		expect(bytesToBase64(new Uint8Array(0))).toBe('');
	});

	it('encodes known bytes', () => {
		const bytes = new Uint8Array([72, 101, 108, 108, 111]);
		expect(bytesToBase64(bytes)).toBe('SGVsbG8=');
	});

	it('roundtrips with atob', () => {
		const original = 'komun test data';
		const bytes = new TextEncoder().encode(original);
		const b64 = bytesToBase64(bytes);
		const decoded = new TextDecoder().decode(Uint8Array.from(atob(b64), c => c.charCodeAt(0)));
		expect(decoded).toBe(original);
	});

	it('handles binary data', () => {
		const bytes = new Uint8Array([0x00, 0xFF, 0x80, 0x7F]);
		const b64 = bytesToBase64(bytes);
		expect(b64).toBe('AP+Afw==');
	});
});

describe('generateIdentityKeypair', () => {
	it('returns a 32-byte x25519 pair in base64', async () => {
		const kp = await generateIdentityKeypair();
		expect(() => atob(kp.publicKey)).not.toThrow();
		expect(() => atob(kp.secretKey)).not.toThrow();
		expect(atob(kp.publicKey).length).toBe(32);
		expect(atob(kp.secretKey).length).toBe(32);
	});

	it('generates different keys each call', async () => {
		const kp1 = await generateIdentityKeypair();
		const kp2 = await generateIdentityKeypair();
		expect(kp1.publicKey).not.toBe(kp2.publicKey);
		expect(kp1.secretKey).not.toBe(kp2.secretKey);
	});

	// A2.10: there is no second keypair any more. Asserting on the absent fields is the point —
	// a reintroduced signature key would show up here rather than silently in the wire format.
	it('carries no signature key', async () => {
		const kp = await generateIdentityKeypair();
		expect(Object.keys(kp).sort()).toEqual(['publicKey', 'secretKey']);
	});
});

describe('deriveVerifier', () => {
	it('is deterministic in password and salt', async () => {
		const salt = await generateSaltB64();
		const a = await deriveVerifier('correct horse battery staple', salt);
		const b = await deriveVerifier('correct horse battery staple', salt);
		expect(a).toBe(b);
	});

	it('changes when the password changes', async () => {
		const salt = await generateSaltB64();
		const a = await deriveVerifier('correct horse battery staple', salt);
		const b = await deriveVerifier('correct horse battery stapleX', salt);
		expect(a).not.toBe(b);
	});

	it('changes when the salt changes', async () => {
		const a = await deriveVerifier('same password', await generateSaltB64());
		const b = await deriveVerifier('same password', await generateSaltB64());
		expect(a).not.toBe(b);
	});

	it('is long enough for the server to accept', async () => {
		// The server rejects anything shorter than 43 base64 chars as implausibly small for an
		// Argon2id output; 32 raw bytes encode to 44.
		const verifier = await deriveVerifier('a password', await generateSaltB64());
		expect(verifier.length).toBeGreaterThanOrEqual(43);
	});
});

describe('wrapSecret / unwrapSecret', () => {
	it('round-trips the whole secret', async () => {
		const kp = await generateIdentityKeypair();
		const { bundle, salt } = await wrapSecret(kp.secretKey, 'a strong password');
		const recovered = await unwrapSecret(bundle, salt, 'a strong password');
		expect(recovered).toBe(kp.secretKey);
	});

	it('returns 32 bytes, not a half of a 64-byte blob', async () => {
		// The old two-secret bundle was split with slice(0, 32) / slice(32, 64). With one secret
		// left there is nothing to split, and a caller taking a slice would get a wrong key that
		// still looked like a key.
		const kp = await generateIdentityKeypair();
		const { bundle, salt } = await wrapSecret(kp.secretKey, 'pw');
		const recovered = await unwrapSecret(bundle, salt, 'pw');
		expect(atob(recovered).length).toBe(32);
	});

	it('rejects the wrong secret', async () => {
		const kp = await generateIdentityKeypair();
		const { bundle, salt } = await wrapSecret(kp.secretKey, 'right password');
		await expect(unwrapSecret(bundle, salt, 'wrong password')).rejects.toBeTruthy();
	});

	it('uses a fresh salt per wrap, so the same input gives a different bundle', async () => {
		const kp = await generateIdentityKeypair();
		const a = await wrapSecret(kp.secretKey, 'pw');
		const b = await wrapSecret(kp.secretKey, 'pw');
		expect(a.salt).not.toBe(b.salt);
		expect(a.bundle).not.toBe(b.bundle);
	});

	it('wraps the same secret under a recovery code as under a password', async () => {
		// This is what makes the code a real second way in: two wrappings, one secret.
		const kp = await generateIdentityKeypair();
		const code = await generateRecoveryCode();
		const byPassword = await wrapSecret(kp.secretKey, 'pw');
		const byCode = await wrapSecret(kp.secretKey, code);
		expect(await unwrapSecret(byPassword.bundle, byPassword.salt, 'pw')).toBe(kp.secretKey);
		expect(await unwrapSecret(byCode.bundle, byCode.salt, code)).toBe(kp.secretKey);
	});
});

describe('generateRecoveryCode', () => {
	it('generates a 12-word phrase', async () => {
		const code = await generateRecoveryCode();
		const words = code.split(' ');
		expect(words).toHaveLength(12);
		words.forEach(w => expect(w).toMatch(/^[a-z]+$/));
	});
});

describe('normalizeRecoveryCode', () => {
	it('collapses spacing and case so a retyped code still matches', () => {
		expect(normalizeRecoveryCode('  Abandon   ABILITY\nable  ')).toBe('abandon ability able');
	});

	it('leaves an already-clean code alone', () => {
		expect(normalizeRecoveryCode('abandon ability able')).toBe('abandon ability able');
	});
});
