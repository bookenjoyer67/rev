# Coding conventions

## Rust

### Error handling patterns

**Bootstrap errors** — `.expect()` panics for unrecoverable failures:
```rust
// config loading, DB connection, migrations, server bind
let pool = PgPoolOptions::new().connect(&url).await.expect("db connect");
```

**API handlers** — `Result<Json<T>, (StatusCode, Json<serde_json::Value>)>`:
```rust
async fn handler(
    State(state): State<AppState>,
    ...
) -> Result<Json<MyResponse>, (StatusCode, Json<serde_json::Value>)> {
    // ...
    Ok(Json(response))
}
```
The `(StatusCode, Json<Value>)` tuple implements Axum's `IntoResponse`.

**Config loading** — `anyhow::Result` with `?` propagation:
```rust
fn load() -> anyhow::Result<Self> { ... }
```

**WASM** — `Result<T, JsValue>` with `JsValue::from_str("error message")`.

**DB errors** — Mapped to 500:
```rust
.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))
```

### Module and router patterns

Each API submodule exports a `router(state: AppState) -> Router`:
```rust
// api/health.rs
use axum::{Router, routing::get};
use crate::AppState;

pub fn router() -> Router {
    Router::new().route("/health", get(health_handler))
}

// api/communities.rs
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{slug}", get(get_one).put(update).delete(delete))
        .with_state(state)
}
```

Routes are composed in `api/mod.rs`:
```rust
pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(health::router())
        .nest("/auth", auth::router(state.clone()))
        .nest("/communities", communities::router(state.clone()))
        .with_state(state)
}
```

- `merge` — flat route (no prefix)
- `nest` — prefix all routes in the sub-router

### State management

`AppState` is `Clone` via `Arc`:
```rust
#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub config: Arc<Config>,
    pub relay_store: Option<Arc<komun_relay::storage::PersistentStore>>,
}
```

Passed to handlers via Axum's `State` extractor:
```rust
async fn handler(State(state): State<AppState>) -> ...
```

### Database queries

All database access through `db/` modules, using SQLx:
```rust
// In db/posts.rs
pub async fn get_post(pool: &PgPool, id: Uuid) -> Result<Post, sqlx::Error> {
    sqlx::query_as!(Post, "SELECT * FROM posts WHERE id = $1", id)
        .fetch_one(pool)
        .await
}
```

### Auth middleware

Two middleware levels in `auth/mod.rs`:
```rust
pub async fn require_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response { ... }

pub async fn require_superadmin(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response { ... }
```

User identity is stored in request extensions:
```rust
request.extensions().insert(AuthUser { user_id });
```

### Naming conventions

- **Crates**: `komun-core`, `komun-server`, `komun-relay`, `komun-wasm`
- **Modules**: lowercase, underscore-separated (`relay_bridge.rs`, `relay_ops.rs`)
- **Structs**: PascalCase (`AppState`, `ServerConfig`, `AuthUser`)
- **Functions**: snake_case (`spawn_relay`, `create_token`, `wrap_dek`)
- **Config serde fields**: snake_case in TOML, matching Rust field names

### Feature flags (relay crate)

Optional bridges are behind features:
```toml
[features]
mqtt-bridge = ["dep:rumqttc", "dep:meshtastic_protobufs", "dep:prost"]
rnode-bridge = ["dep:serialport"]
reticulum-bridge = ["dep:reticulum-rs-core", "dep:reticulum-rs-transport"]
peer-relay = []
tls = ["dep:rustls", "dep:rustls-pemfile", "dep:tokio-rustls"]
hot-reload = ["dep:notify"]
```

Module gating:
```rust
#[cfg(feature = "mqtt-bridge")]
pub mod mqtt_bridge;
```

The server crate depends on relay with `default-features = false` (no bridges by default).

### Base64/hex encoding

- Public keys, encrypted blobs on the wire: **base64** (standard, no padding variation)
- Relay community keys in storage/config: **hex** (lowercase)
- WASM types return `Vec<u8>`; the TypeScript wrapper does base64 conversion

---

## SvelteKit (frontend)

### Svelte 5 runes (required — no legacy syntax)

```svelte
<script lang="ts">
    // State
    let count = $state(0);
    let name = $state('');

    // Derived
    let doubled = $derived(count * 2);

    // Effects
    $effect(() => {
        console.log('count changed:', count);
    });

    // Props (in components)
    let { title, onsave } = $props();
</script>

<!-- Render children -->
{@render children()}

<!-- Event handlers -->
<button onclick={() => count++}>Click</button>

<!-- Conditional -->
{#if count > 0}
    <p>Count: {count}</p>
{/if}

<!-- Each loop -->
{#each items as item (item.id)}
    <li>{item.name}</li>
{/each}
```

**Never use**: `$:`, `export let`, `on:click`, `bind:value` (use `bind:value` only where needed, but prefer `oninput`).

### SPA mode

```ts
// +layout.ts
export const ssr = false;
export const prerender = false;
```

All rendering happens client-side. The `adapter-static` generates `index.html` with a SPA fallback.

### Store conventions

Svelte `writable` with localStorage persistence:
```ts
// lib/stores/auth.ts
import { writable } from 'svelte/store';

const STORAGE_KEY = 'komun_auth';

function loadFromStorage(): AuthState { ... }
function saveToStorage(state: AuthState) { ... }

const stored = loadFromStorage();
export const auth = writable<AuthState>(stored);

auth.subscribe(saveToStorage);
```

Auto-subscription in components with `$` prefix:
```svelte
{#if $auth}
    <p>Logged in as {$auth.displayName}</p>
{/if}
```

Getter functions for derived state (not stores, just functions):
```ts
export function getToken(): string | null { ... }
export function isAuthenticated(): boolean { ... }
export function getActiveAuth(): PerServerAuth | null { ... }
```

### API client pattern

Typed `request<T>` wrapper with auth opt-in:
```ts
// lib/api/client.ts
async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
    const base = getActiveServer();
    const headers: Record<string, string> = { 'Content-Type': 'application/json' };
    if (options.auth) {
        const token = getToken();
        if (token) headers['Authorization'] = `Bearer ${token}`;
    }
    const res = await fetch(`${base}${path}`, {
        method: options.method || 'GET',
        headers,
        body: options.body ? JSON.stringify(options.body) : undefined,
    });
    if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error || res.statusText);
    }
    return res.json();
}
```

**Auth opt-in per call** — never auto-inject tokens. Each method explicitly passes `{ auth: true }`:
```ts
export const api = {
    communities: {
        list: () => request('/api/communities'),
        create: (data) => request('/api/communities', { method: 'POST', body: data, auth: true }),
    },
};
```

### `requestOn` for cross-server calls

When hitting a different server (e.g., responding to a post discovered via directory):
```ts
async function requestOn<T>(base: string, path: string, options: RequestOptions = {}): Promise<T>
```

### CSS conventions

All design tokens in `app.css` as custom properties:
```css
:root {
    --bg: #0f0f1a;
    --bg-surface: #1a1a2e;
    --bg-elevated: #25253e;
    --text: #e8e8f0;
    --text-muted: #8888aa;
    --accent: #e63946;
    --accent-soft: #e6394620;
    --success: #2ec4b6;
    --warning: #f4a261;
    --critical: #e63946;
    --border: #2a2a4a;
    --radius: 8px;
    --radius-lg: 12px;
}
```

**No Tailwind, no CSS framework.** All styling via scoped `<style>` blocks in each component:
```svelte
<style>
    .my-class {
        background: var(--bg-surface);
        border: 1px solid var(--border);
        border-radius: var(--radius);
        color: var(--text);
    }
</style>
```

Basic reset in `app.css` (box-sizing, margin, font). Input elements get `font-size: 16px` (prevents iOS zoom). `.container` utility: `max-width: 800px`, centered, `1rem` padding, `word-break: break-word`.

### Component conventions

- **PascalCase filenames**: `AidCard.svelte`, `LocationBar.svelte`, `Onboarding.svelte`
- **Location**: `src/lib/components/`
- **Imports**: `import AidCard from '$lib/components/AidCard.svelte';`
- **Props**: Svelte 5 `$props()` rune
- **Stores**: Imported directly, subscribed with `$` prefix in template
- **Styles**: Scoped `<style>` blocks in each `.svelte` file

### Auth gating pattern

Components that require authentication use `requireAuth`:
```ts
import { requireAuth } from '$lib/stores/auth';

function handleCreatePost() {
    requireAuth(() => {
        // Only runs if authenticated
        goto('/aid/new');
    });
}
```

If not authenticated, `requireAuth` shows the `Onboarding` modal and queues the action. After auth completes, the action runs automatically.

### Service worker

```ts
// service-worker.ts
import { build, files, version } from '$service-worker';

const CACHE = `komun-${version}`;

// install: cache all build assets + static files
// activate: delete old caches
// fetch:
//   - Non-GET: passthrough
//   - Static assets: cache-first
//   - /api/*: network-first, cache fallback, offline 503 JSON
//   - Navigation: network-first, fallback to cached index.html
```

### PWA manifest

```json
{
    "name": "Komun",
    "short_name": "Komun",
    "start_url": "/",
    "display": "standalone",
    "background_color": "#0f0f1a",
    "theme_color": "#1a1a2e",
    "icons": [{ "src": "/favicon.svg", "sizes": "any", "type": "image/svg+xml" }]
}
```

### TypeScript

- Strict mode enabled (`"strict": true`)
- Module resolution: `bundler`
- `$lib/*` path alias (SvelteKit convention)
- No ESLint, no Prettier, no Biome — only `svelte-check` for type/lint checking

### Directory conventions

```
web/src/
  app.html           HTML shell (SvelteKit template)
  app.css            Global CSS custom properties + reset
  service-worker.ts  Offline-capable service worker
  lib/
    api/             API client + server discovery
    components/      Reusable Svelte components
    stores/          Svelte writable stores
    crypto.ts        WASM crypto wrapper
    wasm/            (empty — wasm files referenced via ../crates/wasm/pkg)
  routes/
    +layout.svelte   Persistent shell
    +page.svelte     Home page
    */+page.svelte   Route pages (SvelteKit file-based routing)
```
