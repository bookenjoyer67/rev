import init, {
	generate_x25519_keypair,
	encrypt_message,
	decrypt_message,
	derive_shared_key,
	encrypt_with_shared_key,
	decrypt_with_shared_key,
} from 'komun-wasm';

let initialized = false;

async function ensureInit() {
	if (!initialized) {
		await init();
		initialized = true;
	}
}

export interface EncryptionKeyPair {
	publicKey: string;
	secretKey: string;
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

export async function generateEncryptionKeypair(): Promise<EncryptionKeyPair> {
	await ensureInit();
	const kp = generate_x25519_keypair();
	return {
		publicKey: bytesToBase64(new Uint8Array(kp.public_key)),
		secretKey: bytesToBase64(new Uint8Array(kp.secret_key)),
	};
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

export async function encryptForRecipient(
	plaintext: string,
	recipientPublicKeyBase64: string
): Promise<string> {
	await ensureInit();
	const plaintextBytes = new TextEncoder().encode(plaintext);
	const recipientPk = base64ToBytes(recipientPublicKeyBase64);
	const envelope = encrypt_message(plaintextBytes, recipientPk);
	return bytesToBase64(new Uint8Array(envelope));
}

export async function decryptFromSender(
	envelopeBase64: string,
	mySecretKeyBase64: string
): Promise<string> {
	await ensureInit();
	const envelope = base64ToBytes(envelopeBase64);
	const mySecret = base64ToBytes(mySecretKeyBase64);
	const plaintext = decrypt_message(envelope, mySecret);
	return new TextDecoder().decode(new Uint8Array(plaintext));
}
