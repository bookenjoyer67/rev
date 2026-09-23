# Hub request — A2b (Agent A): `crates/wasm/pkg/` must be regenerated

`crates/wasm/src/lib.rs` changed in ways that alter the **exported surface**. I cannot rebuild
`pkg/` here (no `wasm32-unknown-unknown` target, no `wasm-bindgen` CLI, no network), so this file
is filed the moment the source edit landed, per A2b.5.

**Action needed:** `wasm-pack build --target web crates/wasm` (or whatever the project's usual
incantation is), then re-run the web gates.

`cargo build --offline -p komun-wasm` → **exit 0** on the edited source, so the crate itself
compiles; only the generated bindings are stale.

---

## Exports REMOVED

| Export | Why |
|---|---|
| `generate_keypair()` | A2.10 — the signature keypair goes with the registration challenge |
| `sign(message, secret_key)` | same |
| `verify(message, signature, public_key)` | same; it had no caller even before A2.10 |
| `KeyPair` (class, with `secret_key` / `public_key` getters) | the return type of `generate_keypair` |
| `hash_recovery_code(phrase)` | derived a server-side verifier for the recovery code under a salt hardcoded into the WASM (`komun-recovery-code-v1`), identical on every deployment. The column it fed (`users.recovery_code_hash`) does not exist in the squashed schema. The recovery code now derives a wrapping key through `derive_key_from_password` under the account's own `recovery_bundle_salt`; nothing derived from it is transmitted. |

## Exports RENAMED

| Was | Now |
|---|---|
| `derive_key_from_passphrase(passphrase, salt)` | `derive_key_from_password(password, salt)` |

Same algorithm and parameters (Argon2id, m=4096 KiB, t=3, p=1, 32-byte output); only the name and
the parameter name changed. The rename is not cosmetic for the gates: A2b's frontend gate is
`grep -rn "passphrase" web/src | grep -v test` expecting **empty**, and an `import { ... }` of the
old name would have matched it.

## Exports with a CHANGED SIGNATURE

```diff
-encrypt_key_bundle(ed25519_secret: &[u8], x25519_secret: &[u8], derived_key: &[u8]) -> Vec<u8>
+encrypt_key_bundle(x25519_secret: &[u8], derived_key: &[u8]) -> Vec<u8>
```

Three arguments to two. The bundle plaintext is now exactly the 32-byte x25519 secret instead of a
64-byte concatenation. `encrypt_key_bundle` rejects a secret that is not 32 bytes rather than
wrapping whatever it was handed.

## Exports with CHANGED BEHAVIOUR (signature unchanged)

`decrypt_key_bundle(encrypted, derived_key) -> Vec<u8>` now returns 32 bytes and **errors** with
`"unexpected key bundle length"` if the authenticated plaintext is any other length. AEAD proves
the plaintext is authentic, not that it has the shape this version expects: a bundle written by the
old two-secret format decrypts cleanly and yields 64 bytes, and a caller taking the first 32 of
those would hold the wrong key and see every message fail to decrypt, with no error to point at.
Its error string also changed from `"wrong passphrase or corrupted bundle"` to
`"wrong password or corrupted bundle"`.

**Migration note for existing accounts:** any `encrypted_key_bundle` written before this change is
64 bytes of plaintext and will now be rejected outright instead of being silently half-read. On
`komun_a` that is only throwaway test data. If any deployment has real accounts, they need a
re-wrap before this ships — flagging it rather than deciding it.

## Exports UNCHANGED

`generate_x25519_keypair`, `X25519KeyPair`, `encrypt_message`, `decrypt_message`,
`derive_shared_key`, `encrypt_with_shared_key`, `decrypt_with_shared_key`, `generate_salt`,
`generate_recovery_code`.

## Also changed in the same crate

`crates/wasm/Cargo.toml`: `ed25519-dalek` removed from `[dependencies]` (it had no remaining
reference). `Cargo.lock` changes accordingly. This is a removal, so no fetch was needed.

---

## Consequence for the web gates, stated up front

Until `pkg/` is regenerated, `web/src/lib/crypto.ts` imports names that the stale artifact does not
export (`derive_key_from_password`) and calls `encrypt_key_bundle` with the new arity. Any web test
that reaches the real `pkg/` will fail on that mismatch. Per A2b.5 those are **waiting on the
rebuild, not defects** — I have named each one individually in my report rather than lumping them
together.
