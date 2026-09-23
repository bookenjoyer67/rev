# Crypto architecture

## What changed

There is **one** keypair now. The old ed25519 signing key, the signed-challenge login, the
passphrase prompt, the fixed-salt `recovery_id` and the server-side recovery oracle are all
gone (SPEC A10–A13). What remains is a single **x25519** key per account: it never leaves the
browser unwrapped, and it is what conversation encryption is built on.

All crypto runs client-side in WASM (`crates/wasm/src/lib.rs`) behind the TypeScript wrapper
`web/src/lib/crypto.ts`. The server stores public keys and wrapped bundles, never the secret.

## Primitives

| Primitive | Where | Use |
|---|---|---|
| X25519 (`x25519-dalek`) | wasm | account keypair; ECDH for conversation keys |
| SHA-256 (`sha2`) | wasm | ECDH output → shared key |
| XChaCha20-Poly1305 (`chacha20poly1305`) | wasm | message + bundle AEAD, 24-byte random nonce |
| Argon2id (`argon2`) | wasm + server | password verifier and wrapping keys; server-side verifier hashing |
| BIP39 12-word phrase | wasm | user-held recovery code (generated locally) |

## Account keys and the password

```
browser:  x25519 keypair  ── public key  ──▶ POST /api/auth/signup (encryption_public_key)
                          └─ secret key  ── wrapped, stored on the server (see below)

browser:  password ──Argon2id(password, auth_salt)──▶ verifier ──▶ server
server:   Argon2id(verifier, its own per-user salt) ──▶ users.password_hash
```

`deriveVerifier` (`crypto.ts`) is the **only** value derived from the password that is
transmitted. The server stretches the verifier again before storing it, so a database dump
yields neither the password nor a replayable verifier. The plaintext password and the key derived
from it never leave the browser; there is no separate passphrase on the wire.

## The wrapped key bundle

The account's x25519 secret is wrapped **twice**, under two independently derived keys:

```
wrap_key = Argon2id(password  |  recovery code, per-bundle salt)   # Argon2id(4096 KiB, 3, 1, 32B)
bundle   = XChaCha20-Poly1305(x25519_secret, wrap_key, nonce)       # nonce(24) || ct(48)
```

| Server column | Wrapped by | Salt column |
|---|---|---|
| `encrypted_key_bundle` | password-derived key | `bundle_salt` |
| `encrypted_recovery_bundle` | recovery-code-derived key | `recovery_bundle_salt` |

Because the plaintext is exactly the 32-byte x25519 secret, re-wrapping on a password change
or a recovery-code reissue preserves message history with nothing else to keep in sync.
`decrypt_key_bundle` asserts the plaintext is 32 bytes, so a stale two-secret bundle cannot
decrypt into a silently wrong key.

## The recovery code

A 12-word BIP39 phrase generated in the browser and shown to the user once. Nothing derived
from it is ever sent: the server only stores the x25519 secret wrapped under a key derived
from the code. Losing the code (and the password) means losing history — that is the accepted
tradeoff (SPEC A12). Signup never returns it, and no endpoint can retrieve it.

## E2E messaging

```
their x25519_pk + my x25519_sk
      ↓ X25519 Diffie-Hellman
shared secret (32 bytes)  ↓ SHA-256  → conversation_key (32 bytes)

plaintext ──XChaCha20-Poly1305(conversation_key, random 24-byte nonce)──▶ [nonce || ciphertext+tag]
```

`deriveConversationKey` / `encryptMessage` / `decryptMessage` in `crypto.ts`. The ciphertext
is what crosses the API and what the server stores in `messages.ciphertext` — the schema has
no plaintext message column. The server can route and store messages but cannot read them.

## WASM binding map

| `web/src/lib/crypto.ts` | `crates/wasm/src/lib.rs` |
|---|---|
| `generateIdentityKeypair()` | `generate_x25519_keypair()` |
| `generateSaltB64()` | `generate_salt()` |
| `deriveVerifier()` | `derive_key_from_password()` |
| `wrapSecret()` | `derive_key_from_password()` + `encrypt_key_bundle()` |
| `unwrapSecret()` | `derive_key_from_password()` + `decrypt_key_bundle()` |
| `generateRecoveryCode()` | `generate_recovery_code()` |
| `deriveConversationKey()` | `derive_shared_key()` (ECDH + SHA-256) |
| `encryptMessage()` / `decryptMessage()` | `encrypt_with_shared_key()` / `decrypt_with_shared_key()` |

## Honest limitation

Browser-delivered end-to-end encryption cannot protect against a malicious server that
serves modified JavaScript. This design protects against database theft, passive disk reads,
an operator reading message content, and admin snooping. It does **not** protect against a
hostile operator who ships modified client code. Say exactly that in user-facing material.

## Rules

1. Secret keys never leave the client; the server stores public keys and wrapped bundles only.
2. The plaintext password (and the recovery code) never leave the client; no derived verification
   value from the recovery code is transmitted.
3. Never log keys, bundles, passwords, derived keys, or plaintext messages.
4. Every AEAD operation uses a fresh random nonce; never reuse a nonce with the same key.
5. `crates/wasm` is the cryptographic boundary; changes there require rebuilding the wasm
   package and the frontend (see `docs/DEVELOPMENT.md`).
