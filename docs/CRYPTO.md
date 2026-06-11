# Crypto architecture

## Overview

Komun uses dual-keypair cryptography for end-to-end encrypted messaging and identity. All crypto operations happen client-side in WASM (`crates/wasm/`) and in the TypeScript wrapper (`web/src/lib/crypto.ts`). The server never sees secret keys.

## Key types

| Key | Algorithm | Purpose | Size |
|---|---|---|---|
| Signing keypair | Ed25519 (ed25519-dalek) | Identity, auth signatures | 32 bytes each |
| Encryption keypair | X25519 (x25519-dalek) | E2E key exchange (ECDH) | 32 bytes each |
| Shared key | X25519 ECDH → SHA-256 | Symmetric message encryption | 32 bytes |
| DEK (Data Encryption Key) | Random (OsRng) | Relay community content encryption | 32 bytes |
| Key bundle key | Argon2id(passphrase, salt) | Encrypting the keypair bundle for recovery | 32 bytes |
| Recovery ID | Argon2id(passphrase, "komun-recovery-v1") | Server-side account lookup | 16 bytes |

## Key lifecycle

### 1. Generation (client-side only)

```
generate_keypair()        → Ed25519  { signing_key, verifying_key }
generate_x25519_keypair() → X25519   { secret_key, public_key }
```

Both happen in `crypto.ts` → `generateFullKeypair()`. Keys never leave the browser in plaintext.

### 2. Registration

The client sends to `POST /api/auth/register`:
- `public_key`: Ed25519 verifying key (base64)
- `encryption_public_key`: X25519 public key (base64)
- `encrypted_key_bundle`: (optional) passphrase-encrypted keypair (base64)
- `bundle_salt`: (optional) salt for key bundle encryption (base64)
- `recovery_id`: (optional) passphrase hash for account recovery (base64)
- `display_name`: string

The server stores:
- `users.public_key` — Ed25519 verifying key
- `users.encryption_public_key` — X25519 public key
- `users.encrypted_key_bundle` — encrypted keypair blob
- `users.bundle_salt` — Argon2 salt
- `users.recovery_id` — recovery identifier

### 3. Passphrase protection (optional but recommended)

Users can add a passphrase to encrypt their keypair for device recovery:

```
passphrase + random_salt
    ↓ Argon2id (4096 KB, 3 iterations, parallelism 1)
derived_key (32 bytes)
    ↓ ChaCha20-Poly1305 encrypt
encrypted_bundle = nonce(12) || ciphertext([ed25519_sk:32][x25519_sk:32])

recovery_id = Argon2id(passphrase, salt="komun-recovery-v1")[0..16]
```

The plaintext bundle layout being encrypted:
```
[ed25519_secret_key: 32 bytes] [x25519_secret_key: 32 bytes]
```

### 4. Recovery

```
1. Client computes recovery_id from passphrase
2. GET /api/auth/recover { recovery_id }
3. Server returns: { encrypted_key_bundle, bundle_salt, public_key, ... }
4. Client derives key from passphrase + salt
5. Client decrypts bundle → recovers ed25519_sk + x25519_sk
6. Client re-registers (upserts) on this device with recovered keys
```

The passphrase never leaves the client. The recovery_id is a one-way hash that the server uses as a lookup key.

## E2E messaging

### Establishing a conversation key

```
Alice's x25519_sk  +  Bob's x25519_pk
    ↓ X25519 Diffie-Hellman
shared_secret (32 bytes)
    ↓ SHA-256
conversation_key (32 bytes)
```

Both parties compute the same `conversation_key`. This is done via `deriveConversationKey()` in `crypto.ts`.

### Message encryption

```
plaintext
    ↓ ChaCha20-Poly1305 with conversation_key + random nonce
[nonce: 12 bytes] [ciphertext + tag: N+16 bytes]
```

Done via `encryptMessage()` / `decryptMessage()` in `crypto.ts`. Nonce is random and prepended to the ciphertext.

### Message format (on wire)

Messages are exchanged via the REST API (`/api/conversations/{match_id}/messages`). The message body is a base64-encoded encrypted blob containing the nonce + ciphertext.

## Relay community crypto (server-side)

When the server creates a relay community for piggPin maps, it generates:

### Community keypair
```
X25519 keypair → community identity (for ECIES DEK wrapping)
```

### DEK (Data Encryption Key)
```
OsRng → 32 random bytes → content encryption key
```

### DEK wrapping (ECIES)
```
dek (32 bytes)
    ↓ encrypted with recipient's public key via ECIES
ephemeral_pk(32) || nonce(12) || ciphertext(N+16)
```

The ECIES scheme uses:
1. Ephemeral X25519 keypair
2. X25519 DH with recipient's public key
3. HKDF-SHA256 with info string `"ecies-v1"` to derive encryption key
4. ChaCha20-Poly1305 AEAD encryption

This is implemented in `relay_ops.rs` for server-side community creation.

## WASM bindings map

| JavaScript (crypto.ts) | WASM (lib.rs) | Purpose |
|---|---|---|
| `generateFullKeypair()` | `generate_keypair()` + `generate_x25519_keypair()` | Create user keypairs |
| `createKeyBundle()` | `generate_salt()` + `derive_key_from_passphrase()` + `encrypt_key_bundle()` | Encrypt keys with passphrase |
| `recoverFromBundle()` | `derive_key_from_passphrase()` + `decrypt_key_bundle()` | Recover keys from passphrase |
| `computeRecoveryId()` | `compute_recovery_id()` | Lookup identifier for recovery |
| `deriveConversationKey()` | `derive_shared_key()` | ECDH key agreement |
| `encryptMessage()` | `encrypt_with_shared_key()` | Symmetric encryption |
| `decryptMessage()` | `decrypt_with_shared_key()` | Symmetric decryption |
| — | `sign()` / `verify()` | Ed25519 signing (available, limited use) |
| — | `encrypt_message()` / `decrypt_message()` | ECDH+encrypt/decrypt (available, not used in current flow) |

## Security rules

1. **Secret keys never leave the client.** The server only stores public keys and encrypted bundles.
2. **Passphrases never leave the client.** Only the recovery ID (one-way hash) is sent to the server.
3. **Never log keys, bundles, passphrases, or derived secrets.**
4. **Every encryption uses a random nonce.** ChaCha20-Poly1305 nonces must never be reused with the same key.
5. **The shared secret DH check** (`all zeros = low-order point`) protects against invalid public keys in ECIES.
6. **The JWT secret in config.toml must be a random string in production.** The default is a known dev value.
7. **The relay DEK is encrypted with the community's public key before storage.** The plaintext DEK exists only in memory during community creation.
