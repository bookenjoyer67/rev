import init, {
	generate_keypair,
	generate_x25519_keypair,
	encrypt_message,
	decrypt_message,
	derive_shared_key,
	encrypt_with_shared_key,
	decrypt_with_shared_key,
	generate_salt,
	derive_key_from_passphrase,
	encrypt_key_bundle,
	decrypt_key_bundle,
	compute_recovery_id,
} from 'komun-wasm';

let initialized = false;

async function ensureInit() {
	if (!initialized) {
		await init();
		initialized = true;
	}
}

export interface FullKeypair {
	ed25519PublicKey: string;
	ed25519SecretKey: string;
	x25519PublicKey: string;
	x25519SecretKey: string;
}

function bytesToBase64(bytes: Uint8Array): string {
	return btoa(String.fromCharCode(...bytes));
}

function base64ToBytes(b64: string): Uint8Array {
	const binary = atob(b64);
	const bytes = new Uint8Array(binary.length);
	for (let i = 0; i < binary.length; i++) {
		bytes[i] = binary.charCodeAt(i);
	}
	return bytes;
}

export async function generateFullKeypair(): Promise<FullKeypair> {
	await ensureInit();
	const edKp = generate_keypair();
	const xKp = generate_x25519_keypair();
	return {
		ed25519PublicKey: bytesToBase64(new Uint8Array(edKp.public_key)),
		ed25519SecretKey: bytesToBase64(new Uint8Array(edKp.secret_key)),
		x25519PublicKey: bytesToBase64(new Uint8Array(xKp.public_key)),
		x25519SecretKey: bytesToBase64(new Uint8Array(xKp.secret_key)),
	};
}

export interface KeyBundle {
	encryptedBundle: string;
	salt: string;
	recoveryId: string;
}

export async function createKeyBundle(
	ed25519Secret: string,
	x25519Secret: string,
	passphrase: string
): Promise<KeyBundle> {
	await ensureInit();
	const passphraseBytes = new TextEncoder().encode(passphrase);
	const salt = new Uint8Array(generate_salt());
	const derivedKey = derive_key_from_passphrase(passphraseBytes, salt);
	const encrypted = encrypt_key_bundle(
		base64ToBytes(ed25519Secret),
		base64ToBytes(x25519Secret),
		new Uint8Array(derivedKey)
	);
	const recoveryIdBytes = compute_recovery_id(passphraseBytes);

	return {
		encryptedBundle: bytesToBase64(new Uint8Array(encrypted)),
		salt: bytesToBase64(salt),
		recoveryId: bytesToBase64(new Uint8Array(recoveryIdBytes)),
	};
}

export async function recoverFromBundle(
	encryptedBundle: string,
	salt: string,
	passphrase: string
): Promise<{ ed25519Secret: string; x25519Secret: string }> {
	await ensureInit();
	const passphraseBytes = new TextEncoder().encode(passphrase);
	const derivedKey = derive_key_from_passphrase(passphraseBytes, base64ToBytes(salt));
	const decrypted = decrypt_key_bundle(base64ToBytes(encryptedBundle), new Uint8Array(derivedKey));
	const bytes = new Uint8Array(decrypted);

	return {
		ed25519Secret: bytesToBase64(bytes.slice(0, 32)),
		x25519Secret: bytesToBase64(bytes.slice(32, 64)),
	};
}

export async function computeRecoveryId(passphrase: string): Promise<string> {
	await ensureInit();
	const passphraseBytes = new TextEncoder().encode(passphrase);
	const id = compute_recovery_id(passphraseBytes);
	return bytesToBase64(new Uint8Array(id));
}

export async function deriveConversationKey(
	mySecretKeyBase64: string,
	theirPublicKeyBase64: string
): Promise<string> {
	await ensureInit();
	const mySecret = base64ToBytes(mySecretKeyBase64);
	const theirPublic = base64ToBytes(theirPublicKeyBase64);
	const key = derive_shared_key(mySecret, theirPublic);
	return bytesToBase64(new Uint8Array(key));
}

export async function encryptMessage(
	plaintext: string,
	sharedKeyBase64: string
): Promise<string> {
	await ensureInit();
	const plaintextBytes = new TextEncoder().encode(plaintext);
	const key = base64ToBytes(sharedKeyBase64);
	const encrypted = encrypt_with_shared_key(plaintextBytes, key);
	return bytesToBase64(new Uint8Array(encrypted));
}

export async function decryptMessage(
	encryptedBase64: string,
	sharedKeyBase64: string
): Promise<string> {
	await ensureInit();
	const data = base64ToBytes(encryptedBase64);
	const key = base64ToBytes(sharedKeyBase64);
	const plaintext = decrypt_with_shared_key(data, key);
	return new TextDecoder().decode(new Uint8Array(plaintext));
}
