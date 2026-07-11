import { describe, it, expect } from 'vitest';
import {
	bytesToBase64,
	generateFullKeypair,
	createKeyBundle,
	generateRecoveryCode,
	hashRecoveryCode,
	computeRecoveryId,
	signRegisterChallenge
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

describe('generateFullKeypair', () => {
	it('returns keys in base64', async () => {
		const kp = await generateFullKeypair();
		expect(kp.ed25519PublicKey).toBeTruthy();
		expect(kp.ed25519SecretKey).toBeTruthy();
		expect(kp.x25519PublicKey).toBeTruthy();
		expect(kp.x25519SecretKey).toBeTruthy();
		expect(() => atob(kp.ed25519PublicKey)).not.toThrow();
		expect(() => atob(kp.x25519PublicKey)).not.toThrow();
	});

	it('generates different keys each call', async () => {
		const kp1 = await generateFullKeypair();
		const kp2 = await generateFullKeypair();
		expect(kp1.ed25519PublicKey).not.toBe(kp2.ed25519PublicKey);
	});
});

describe('createKeyBundle', () => {
	it('returns encrypted bundle with salt and recovery id', async () => {
		const kp = await generateFullKeypair();
		const bundle = await createKeyBundle(kp.ed25519SecretKey, kp.x25519SecretKey, 'test-passphrase');
		expect(bundle.encryptedBundle).toBeTruthy();
		expect(bundle.salt).toBeTruthy();
		expect(bundle.recoveryId).toBeTruthy();
		expect(() => atob(bundle.encryptedBundle)).not.toThrow();
		expect(() => atob(bundle.salt)).not.toThrow();
		expect(() => atob(bundle.recoveryId)).not.toThrow();
	});
});

describe('generateRecoveryCode', () => {
	it('generates BIP39-compatible 12-word phrase', async () => {
		const code = await generateRecoveryCode();
		const words = code.split(' ');
		expect(words).toHaveLength(12);
		words.forEach(w => expect(w).toMatch(/^[a-z]+$/));
	});
});

describe('hashRecoveryCode', () => {
	it('produces base64 hash', async () => {
		const hash = await hashRecoveryCode('test code');
		expect(hash).toBeTruthy();
		expect(() => atob(hash)).not.toThrow();
	});

	it('same input = same hash', async () => {
		const h1 = await hashRecoveryCode('same code');
		const h2 = await hashRecoveryCode('same code');
		expect(h1).toBe(h2);
	});
});

describe('computeRecoveryId', () => {
	it('returns base64 recovery id', async () => {
		const id = await computeRecoveryId('test');
		expect(id).toBeTruthy();
		expect(() => atob(id)).not.toThrow();
	});

	it('same passphrase = same recovery id', async () => {
		const id1 = await computeRecoveryId('my passphrase');
		const id2 = await computeRecoveryId('my passphrase');
		expect(id1).toBe(id2);
	});
});

describe('signRegisterChallenge', () => {
	it('returns challenge and signature', async () => {
		const kp = await generateFullKeypair();
		const result = await signRegisterChallenge(kp.ed25519SecretKey);
		expect(result.challenge).toBeTruthy();
		expect(result.signature).toBeTruthy();
		expect(() => atob(result.challenge)).not.toThrow();
		expect(() => atob(result.signature)).not.toThrow();
	});

	it('generates 32-byte challenges', async () => {
		const kp = await generateFullKeypair();
		const result = await signRegisterChallenge(kp.ed25519SecretKey);
		const decoded = Uint8Array.from(atob(result.challenge), c => c.charCodeAt(0));
		expect(decoded.length).toBe(32);
	});

	it('generates different challenges each call', async () => {
		const kp = await generateFullKeypair();
		const r1 = await signRegisterChallenge(kp.ed25519SecretKey);
		const r2 = await signRegisterChallenge(kp.ed25519SecretKey);
		expect(r1.challenge).not.toBe(r2.challenge);
	});
});
