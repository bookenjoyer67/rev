import { vi } from 'vitest';

let counter = 0;
const nextByte = () => { counter++; return counter % 256; };

const mockEdKeypair = () => {
	const pub = new Uint8Array(32);
	const sec = new Uint8Array(64);
	for (let i = 0; i < 32; i++) pub[i] = nextByte();
	for (let i = 0; i < 64; i++) sec[i] = nextByte();
	return { public_key: pub.buffer, secret_key: sec.buffer };
};

const mockXKeypair = () => {
	const pub = new Uint8Array(32);
	const sec = new Uint8Array(32);
	for (let i = 0; i < 32; i++) { pub[i] = nextByte(); sec[i] = nextByte(); }
	return { public_key: pub.buffer, secret_key: sec.buffer };
};

export const generate_keypair = vi.fn(mockEdKeypair);
export const generate_x25519_keypair = vi.fn(mockXKeypair);
export const sign = vi.fn(() => new Uint8Array(64).fill(nextByte()).buffer);
export const encrypt_message = vi.fn((data: Uint8Array) => data.buffer);
export const decrypt_message = vi.fn((data: Uint8Array) => data.buffer);
export const derive_shared_key = vi.fn(() => new Uint8Array(32).fill(nextByte()).buffer);
export const encrypt_with_shared_key = vi.fn((data: Uint8Array) => data.buffer);
export const decrypt_with_shared_key = vi.fn((data: Uint8Array) => data.buffer);
export const generate_salt = vi.fn(() => new Uint8Array(16).fill(nextByte()).buffer);
export const derive_key_from_passphrase = vi.fn(() => new Uint8Array(32).fill(nextByte()).buffer);
export const encrypt_key_bundle = vi.fn(() => new Uint8Array(64).fill(nextByte()).buffer);
export const decrypt_key_bundle = vi.fn(() => new Uint8Array(64).fill(nextByte()).buffer);
export const compute_recovery_id = vi.fn(() => new Uint8Array(32).fill(0x55).buffer);
export const generate_recovery_code = vi.fn(() => 'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about');
export const hash_recovery_code = vi.fn(() => new Uint8Array(32).fill(0x66).buffer);

export default function init(): Promise<void> {
	counter = 0;
	return Promise.resolve();
}
