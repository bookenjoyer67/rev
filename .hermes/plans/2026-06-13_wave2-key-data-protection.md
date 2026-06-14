# Wave 2 — Key & Data Protection (Blast Radius Reduction)

> **For Hermes:** Delegate each fix to OpenCode via `opencode run`, then review with requesting-code-review skill.

**Goal:** Protect crypto keys at rest so a disk-level or XSS compromise doesn't immediately yield full identity theft. Six fixes, ~55 lines, 6 files.

**Architecture:** Add a passphrase-derived encryption layer between memory and persistent storage in both Komun (localStorage) and piggPin (IndexedDB). Tighten cache and iframe boundaries. Redact push secrets from relay snapshots. Add early-abort for oversized WebSocket messages.

**Tech Stack:** JavaScript/TypeScript (SvelteKit, Vanilla JS), Rust (Tokio, Tungstenite, Serde)

---

## Current State (verified from source)

| Fix | Project | File(s) | Current state |
|-----|---------|---------|---------------|
| 2.1 | Komun | `web/src/lib/stores/auth.ts` | Auth state saved to localStorage as plain JSON. Keypair (ed25519 secret, x25519 secret) and server tokens in cleartext on disk. |
| 2.2 | Piggpin | `db.js`, `peer.js` | Signing key stored in IndexedDB "profile" store as plain {signing_secret_key, signing_public_key}. No encryption at rest. DEKs stored in "layer_deks" store. |
| 2.3 | Piggpin | `signal-server/src/storage.rs` | `save_snapshot()` writes `push_subscriptions` map to disk. Each `PushSubscription` has `p256dh` and `auth` fields in cleartext. `secret_key` and `wrapped_dek` already redacted from communities but push secrets are not. |
| 2.4 | Komun | `web/src/service-worker.ts` | Current SW caches ALL `/api/` responses indiscriminately. Need whitelist: only `/api/node`, `/api/health`, `/api/communities`, `/api/directory`. |
| 2.5 | Komun | `web/src/routes/c/[slug]/map/+page.svelte:108`, `web/src/routes/aid/new/+page.svelte:243` | Both iframe sandbox attributes include `allow-same-origin`. Need removed (it lets the iframe access Komun's origin context). |
| 2.6 | Piggpin | `signal-server/src/handler.rs:173` | Message size check AFTER tungstenite has already buffered the full message into a `String`. Need a streaming check that aborts early before the full 10MB is in memory. |

---

### Task 1: Redact push subscription secrets from snapshot (2.3) — easiest, Rust-side

**Objective:** Strip `p256dh` and `auth` from `PushSubscription` entries when serializing to `community_data.json`.

**Files:**
- Modify: `signal-server/src/storage.rs:243` (the push_subscriptions clone in save_snapshot)

**Step 1: Redact in save_snapshot**

In `save_snapshot()`, after cloning `push_subscriptions`, iterate through all entries and blank out `p256dh` and `auth`:

```rust
let push_subscriptions = {
    let subs = self.push_subscriptions.read().await.clone();
    subs.into_iter().map(|(k, list)| {
        (k, list.into_iter().map(|mut sub| {
            sub.p256dh = String::new();
            sub.auth = String::new();
            sub
        }).collect())
    }).collect()
};
```

Replace the current line 243:
```rust
let push_subscriptions = self.push_subscriptions.read().await.clone();
```
with the above block.

**Step 2: Verify with cargo check**

```bash
cd signal-server && cargo check 2>&1
```
Expected: compiles cleanly.

**Step 3: Commit**

```bash
cd ~/team-pins && git add signal-server/src/storage.rs
git commit -m "fix: redact p256dh/auth from push subscriptions in relay snapshot"
```

---

### Task 2: Tighten iframe sandbox — remove allow-same-origin (2.5)

**Objective:** Remove `allow-same-origin` from piggPin iframe sandbox attributes across all Komun pages.

**Files:**
- Modify: `web/src/routes/c/[slug]/map/+page.svelte:108`
- Modify: `web/src/routes/aid/new/+page.svelte:243`

**Step 1: Edit map page**

In `web/src/routes/c/[slug]/map/+page.svelte`, line 108, change:
```svelte
sandbox="allow-scripts allow-same-origin allow-popups allow-forms"
```
to:
```svelte
sandbox="allow-scripts allow-popups allow-forms"
```

**Step 2: Edit aid/new page**

In `web/src/routes/aid/new/+page.svelte`, line 243, change:
```svelte
sandbox="allow-scripts allow-same-origin allow-popups"
```
to:
```svelte
sandbox="allow-scripts allow-popups"
```

**Step 3: Verify nothing breaks**

The piggPin iframe uses `postMessage` for cross-origin communication (already gated with `event.origin !== 'https://app.piggpin.space'` check). Removing `allow-same-origin` does not affect `postMessage`. The iframe won't be able to access Komun's cookies/localStorage/dom — which is the whole point.

**Step 4: Commit**

```bash
cd ~/rev && git add web/src/routes/c/[slug]/map/+page.svelte web/src/routes/aid/new/+page.svelte
git commit -m "fix: remove allow-same-origin from piggpin iframe sandbox"
```

---

### Task 3: Service Worker cache whitelist (2.4)

**Objective:** Only cache specific API endpoint responses instead of all `/api/*`.

**Files:**
- Modify: `web/src/service-worker.ts:43-63`

**Step 1: Replace the broad /api/ cache block**

Current (lines 43-63):
```typescript
if (url.href.includes('/api/')) {
    event.respondWith(
        fetch(request)
            .then((response) => {
                if (response.ok) {
                    const clone = response.clone();
                    caches.open(CACHE_NAME).then((cache) => cache.put(request, clone));
                }
                return response;
            })
            .catch(() =>
                caches.match(request).then((cached) =>
                    cached || new Response(JSON.stringify({ error: 'offline' }), {
                        status: 503,
                        headers: { 'Content-Type': 'application/json' },
                    })
                )
            )
    );
    return;
}
```

Replace with whitelist version:
```typescript
const CACHEABLE_API_PATHS = ['/api/node', '/api/health', '/api/communities', '/api/directory'];
const isCacheableApi = CACHEABLE_API_PATHS.some(p => url.pathname.startsWith(p));

if (isCacheableApi) {
    event.respondWith(
        fetch(request)
            .then((response) => {
                if (response.ok) {
                    const clone = response.clone();
                    caches.open(CACHE_NAME).then((cache) => cache.put(request, clone));
                }
                return response;
            })
            .catch(() =>
                caches.match(request).then((cached) =>
                    cached || new Response(JSON.stringify({ error: 'offline' }), {
                        status: 503,
                        headers: { 'Content-Type': 'application/json' },
                    })
                )
            )
    );
    return;
}
```

Non-whitelisted `/api/` requests pass through to the network-only default handler at the bottom of the fetch listener (lines 74-77).

**Step 2: Verify**

No build step needed for SW changes in dev (vite handles it). But confirm no syntax errors:
```bash
cd ~/rev/web && npx tsc --noEmit src/service-worker.ts 2>&1 | head -20
```

**Step 3: Commit**

```bash
cd ~/rev && git add web/src/service-worker.ts
git commit -m "fix: restrict service worker api cache to whitelist endpoints"
```

---

### Task 4: Streaming WebSocket message size check (2.6)

**Objective:** Abort oversized WebSocket text messages early — before tungstenite buffers the full payload into memory. Currently line 173 does `txt.len() > max_message_size` but `txt` is already fully buffered.

**Files:**
- Modify: `signal-server/src/handler.rs` — the WebSocket read loop around line 170-173

**Approach:** Configure tungstenite's `max_message_size` at the WebSocket stream level via `WebSocketConfig`. This causes tungstenite itself to abort the read mid-stream when the frame payload exceeds the limit, returning an error instead of a full `Message::Text`.

**Step 1: Add WebSocketConfig import and apply to accept_async**

In handler.rs, change the `accept_async` call (line 26) from:
```rust
let mut ws_stream = match timeout(Duration::from_secs(10), accept_async(stream)).await {
```
to:
```rust
let ws_config = tokio_tungstenite::tungstenite::protocol::WebSocketConfig {
    max_message_size: Some(state.config.security.max_message_size),
    ..Default::default()
};
let mut ws_stream = match timeout(Duration::from_secs(10), accept_async_with_config(stream, Some(ws_config))).await {
```

Add the import (line 9):
```rust
use tokio_tungstenite::{accept_async, accept_async_with_config, tungstenite::Message};
```

**Step 2: Remove the redundant post-buffer check**

Line 173 currently has:
```rust
if txt.len() > state.config.security.max_message_size { continue; }
```

This is now redundant — `accept_async_with_config` will return an error for oversized messages instead of a `Message::Text`. Remove this line. The error path at line 209-210 (`Some(Err(e)) => { ... }`) will handle the oversized message error cleanly (disconnect the client).

**Step 3: Update the error handling for oversized messages**

The existing error handler at ~line 209 handles `Some(Err(_))` from the read stream. When tungstenite rejects an oversized message, it returns a `tungstenite::Error::Protocol` or `Capacity` error. Add logging:

```rust
Some(Err(e)) => {
    warn!("[relay] ws error from {}: {}", read_ip, e);
    break;
}
```

**Step 4: Verify with cargo check**

```bash
cd ~/team-pins/signal-server && cargo check 2>&1
```
Expected: compiles cleanly.

**Step 5: Commit**

```bash
cd ~/team-pins && git add signal-server/src/handler.rs
git commit -m "fix: abort oversized websocket messages at stream level instead of post-buffer"
```

---

### Task 5: Encrypt Komun auth localStorage with passphrase-derived wrap key (2.1)

**Objective:** Auth data (token, user_id, keypair secrets) currently stored as plain JSON in localStorage. Encrypt with a key derived from the user's passphrase, only unwrap into sessionStorage on login.

**Files:**
- Modify: `web/src/lib/stores/auth.ts` — loadFromStorage, saveToStorage, register, recover, logout
- Modify: `web/src/lib/crypto.ts` — add wrap/unwrap functions

**Step 1: Add wrap/unwrap functions to crypto.ts**

```typescript
// Derive a wrap key from the passphrase (separate derivation from key bundle)
export async function deriveWrapKey(passphrase: string, salt: string): Promise<string> {
    await ensureInit();
    const passphraseBytes = new TextEncoder().encode(passphrase);
    const saltBytes = base64ToBytes(salt);
    const derived = derive_key_from_passphrase(passphraseBytes, saltBytes);
    return bytesToBase64(new Uint8Array(derived));
}

// Encrypt auth state with wrap key
export async function wrapAuthState(state: string, wrapKeyBase64: string): Promise<string> {
    await ensureInit();
    const stateBytes = new TextEncoder().encode(state);
    const key = base64ToBytes(wrapKeyBase64);
    const encrypted = encrypt_with_shared_key(stateBytes, key);
    return bytesToBase64(new Uint8Array(encrypted));
}

// Decrypt auth state with wrap key
export async function unwrapAuthState(encryptedBase64: string, wrapKeyBase64: string): Promise<string> {
    await ensureInit();
    const data = base64ToBytes(encryptedBase64);
    const key = base64ToBytes(wrapKeyBase64);
    const plaintext = decrypt_with_shared_key(data, key);
    return new TextDecoder().decode(new Uint8Array(plaintext));
}
```

**Step 2: Modify auth.ts storage layer**

Change `STORAGE_KEY` to two keys:
```typescript
const STORAGE_KEY = 'komun_auth';           // encrypted blob
const SALT_KEY = 'komun_auth_salt';         // salt for wrap key derivation
const SESSION_KEY = 'komun_auth_session';   // unwrapped into sessionStorage
```

Add passphrase management:
```typescript
let _wrapKey: string | null = null;
let _passphrase: string | null = null;

export function unlockAuth(passphrase: string) {
    _passphrase = passphrase;
}

export function lockAuth() {
    _passphrase = null;
    _wrapKey = null;
    sessionStorage.removeItem(SESSION_KEY);
}

async function getWrapKey(): Promise<string | null> {
    if (_wrapKey) return _wrapKey;
    if (!_passphrase) return null;
    const salt = localStorage.getItem(SALT_KEY);
    if (!salt) return null;
    _wrapKey = await deriveWrapKey(_passphrase, salt);
    return _wrapKey;
}
```

**Step 3: Modify loadFromStorage**

```typescript
async function loadFromStorage(): Promise<AuthState> {
    if (typeof localStorage === 'undefined') return { keypair: null, servers: {} };

    // Try sessionStorage first (already unwrapped this session)
    const sessionRaw = sessionStorage.getItem(SESSION_KEY);
    if (sessionRaw) {
        try { return JSON.parse(sessionRaw); } catch {}
    }

    // Try to unwrap from localStorage
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { keypair: null, servers: {} };

    const wrapKey = await getWrapKey();
    if (!wrapKey) return { keypair: null, servers: {} };

    try {
        const decrypted = await unwrapAuthState(raw, wrapKey);
        const parsed = JSON.parse(decrypted);
        // Cache in sessionStorage for this session
        sessionStorage.setItem(SESSION_KEY, decrypted);
        return parsed;
    } catch {
        return { keypair: null, servers: {} };
    }
}
```

**Step 4: Modify saveToStorage**

```typescript
async function saveToStorage(state: AuthState) {
    if (typeof localStorage === 'undefined') return;
    const json = JSON.stringify(state);
    // Always update sessionStorage
    sessionStorage.setItem(SESSION_KEY, json);
    // Encrypt for localStorage
    const wrapKey = await getWrapKey();
    if (wrapKey) {
        const salt = localStorage.getItem(SALT_KEY) || bytesToBase64(crypto.getRandomValues(new Uint8Array(32)));
        if (!localStorage.getItem(SALT_KEY)) {
            localStorage.setItem(SALT_KEY, salt);
        }
        const encrypted = await wrapAuthState(json, wrapKey);
        localStorage.setItem(STORAGE_KEY, encrypted);
    }
}
```

**Step 5: Update auth store initialization**

Change from sync to async-aware. The `auth` writable must be initialized with a promise or lazy-loaded. Replace:

```typescript
export const auth = writable<AuthState>(loadFromStorage());
auth.subscribe(saveToStorage);
```

With a pattern that sets initial state to {keypair: null, servers: {}}, then loads async:

```typescript
export const auth = writable<AuthState>({ keypair: null, servers: {} });

// Async initialization
let _initPromise: Promise<void> | null = null;
export async function initAuth(passphrase?: string): Promise<void> {
    if (_initPromise) return _initPromise;
    if (passphrase) unlockAuth(passphrase);
    _initPromise = loadFromStorage().then(state => {
        auth.set(state);
        auth.subscribe(s => { saveToStorage(s); });
    });
    return _initPromise;
}
```

**Step 6: Update register, recover, setPassphrase to call unlockAuth**

In `register()`: after successful registration with a passphrase, call `unlockAuth(passphrase)`.  
In `recover()`: before recovering, call `unlockAuth(passphrase)`.  
In `setPassphrase()`: after setting, call `unlockAuth(passphrase)`.  

**Step 7: Update logout to clear sessionStorage**

```typescript
export function logout() {
    sessionStorage.removeItem(SESSION_KEY);
    // ... existing server removal logic
}
```

**Step 8: Verify build compiles**

```bash
cd ~/rev/web && npm run build 2>&1 | tail -10
```
Expected: builds cleanly. May need to adjust component files that import `auth` sync.

**Step 9: Commit**

```bash
cd ~/rev && git add web/src/lib/crypto.ts web/src/lib/stores/auth.ts
git commit -m "fix: encrypt auth localStorage with passphrase-derived wrap key"
```

---

### Task 6: Encrypt Piggpin IndexedDB keys (2.2)

**Objective:** ed25519 signing key and X25519 keys currently stored as plain strings in IndexedDB "profile" store. Encrypt with passphrase-derived wrap key before `saveSigningKey()`, decrypt on `getSigningKey()`.

**Files:**
- Modify: `db.js` — saveSigningKey, getSigningKey
- Modify: `peer.js` or new utility — add passphrase prompt + wrap key derivation

**Step 1: Add crypto helper in db.js or core**

```javascript
// Wrap/unwrap using the Rust WASM crypto
import { compress_gzip_to_base64, decompress_gzip } from "./core/pkg/e2e_core.js";

// Simple XOR-based (until we add proper WASM AEAD for this)
// For now: use SubtleCrypto AES-GCM with PBKDF2
async function deriveWrapKey(passphrase, salt) {
    const enc = new TextEncoder();
    const keyMaterial = await crypto.subtle.importKey(
        "raw", enc.encode(passphrase), "PBKDF2", false, ["deriveKey"]
    );
    return crypto.subtle.deriveKey(
        { name: "PBKDF2", salt, iterations: 100000, hash: "SHA-256" },
        keyMaterial,
        { name: "AES-GCM", length: 256 },
        false,
        ["encrypt", "decrypt"]
    );
}

async function encryptValue(value, passphrase, salt) {
    const key = await deriveWrapKey(passphrase, salt);
    const iv = crypto.getRandomValues(new Uint8Array(12));
    const enc = new TextEncoder();
    const ciphertext = await crypto.subtle.encrypt(
        { name: "AES-GCM", iv }, key, enc.encode(value)
    );
    return { ciphertext: bytesToBase64(new Uint8Array(ciphertext)), iv: bytesToBase64(iv) };
}

async function decryptValue(encrypted, passphrase, salt) {
    const key = await deriveWrapKey(passphrase, salt);
    const ciphertext = base64ToBytes(encrypted.ciphertext);
    const iv = base64ToBytes(encrypted.iv);
    const plaintext = await crypto.subtle.decrypt(
        { name: "AES-GCM", iv }, key, ciphertext
    );
    return new TextDecoder().decode(plaintext);
}
```

**Step 2: Modify saveSigningKey to encrypt**

```javascript
export async function saveSigningKey(kp, passphrase) {
    await openDB();
    const p = await promisify(tx("profile").get("me")).catch(() => null);
    const profile = p || { key: "me", user_id: generateUUIDCompat(), display_name: "Me" };

    if (passphrase) {
        const salt = crypto.getRandomValues(new Uint8Array(16));
        const json = JSON.stringify({ public: kp.public, secret: kp.secret });
        const encrypted = await encryptValue(json, passphrase, salt);
        profile.signing_keys_encrypted = encrypted;
        profile.signing_keys_salt = bytesToBase64(salt);
        // Clear plaintext fields
        delete profile.signing_public_key;
        delete profile.signing_secret_key;
    } else {
        profile.signing_public_key = kp.public;
        profile.signing_secret_key = kp.secret;
    }
    return promisify(tx("profile", "readwrite").put(profile));
}
```

**Step 3: Modify getSigningKey to decrypt**

```javascript
export async function getSigningKey(passphrase) {
    await openDB();
    const p = await promisify(tx("profile").get("me"));
    if (!p) return null;
    // Encrypted path
    if (p.signing_keys_encrypted && passphrase) {
        try {
            const salt = base64ToBytes(p.signing_keys_salt);
            const json = await decryptValue(p.signing_keys_encrypted, passphrase, salt);
            const kp = JSON.parse(json);
            return kp;
        } catch { return null; }
    }
    // Legacy plaintext path
    if (p.signing_public_key && p.signing_secret_key) {
        return { public: p.signing_public_key, secret: p.signing_secret_key };
    }
    return null;
}
```

**Step 4: Add passphrase prompt to app startup**

In `main.js` (or wherever piggPin initializes), add a passphrase prompt that calls `getSigningKey(passphrase)` and `saveSigningKey(kp, passphrase)`:

```javascript
import { getSigningKey, saveSigningKey } from './db.js';

async function unlockApp() {
    const passphrase = prompt("Enter your passphrase to unlock keys:");
    if (!passphrase) return false;
    const keys = await getSigningKey(passphrase);
    if (keys) {
        window.signingKeys = keys;
        return true;
    }
    return false;
}
```

**Step 5: Migrate existing plaintext keys on first unlock**

If `getSigningKey` finds plaintext keys but a passphrase is provided, encrypt them:

```javascript
export async function getSigningKey(passphrase) {
    // ... after loading profile ...
    if (p.signing_public_key && p.signing_secret_key && passphrase) {
        // Migrate: encrypt existing plaintext keys
        const kp = { public: p.signing_public_key, secret: p.signing_secret_key };
        await saveSigningKey(kp, passphrase);
        return kp;
    }
    // ... rest
}
```

**Step 6: Verify**

No formal test suite for piggPin. Manual verification by running the app and confirming keys work after passphrase entry.

**Step 7: Commit**

```bash
cd ~/team-pins && git add db.js
git commit -m "fix: encrypt signing keys in IndexedDB with passphrase-derived wrap key"
```

---

## Risks & Tradeoffs

- **2.1 & 2.2 (wrap key):** If user forgets passphrase, auth data is lost. Acceptable — same passphrase used for server-side key bundle recovery. Session is the escape hatch: data persists in sessionStorage until tab closes.
- **2.4 (SW cache):** Non-cached API calls won't work offline. `/api/conversations`, `/api/posts`, `/api/auth/*` etc. will fail when offline — acceptable, these contain sensitive personal data that shouldn't be cached.
- **2.5 (iframe sandbox):** Removing `allow-same-origin` prevents the piggPin iframe from accessing Komun's cookies/localStorage. Communication is via `postMessage` only (already the case). No functional impact.
- **2.6 (streaming abort):** `accept_async_with_config` with `max_message_size` causes tungstenite to close the connection on oversized messages with a protocol error. This is cleaner than the current behavior which buffers then drops. The old `max_message_size` config field is still used for the limit value.

## Verification

After all 6 fixes:
```bash
# Komun
cd ~/rev && cargo build --release --bin komun-server 2>&1
cd web && npm run build 2>&1

# Piggpin
cd ~/team-pins/signal-server && cargo check 2>&1
cd ~/team-pins && npm run build 2>&1
```

Then run the pre-commit review pipeline via `requesting-code-review` skill on the combined diff.
