<div align="center">

# 🫱🏾‍🫲🏼 Komun

**Federated mutual aid discovery — needs meet resources, encrypted.**

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://rust-lang.org)
[![SvelteKit](https://img.shields.io/badge/SvelteKit-5-ff3e00.svg)](https://svelte.dev)
[![Tests](https://img.shields.io/badge/tests-110%20passed-brightgreen.svg)](https://git.komun.buzz/Book-Enjoyer/rev)

</div>

---

Communities post what they need and what they can give. Komun matches them through encrypted conversations. No surveillance. No central authority. Just people helping people.

---

## 🧩 Architecture

```
     ┌──────────────┐
     │   Browser    │  SvelteKit 5 SPA
     └──────┬───────┘
            │ HTTPS
     ┌──────▼───────────────────────┐
     │       Axum HTTP API          │
     │         :3000                │
     │                              │
     │  ┌──────────┐  ┌──────────┐  │
     │  │ JWT auth │  │ REST API │  │
     │  │ challenge│  │ CRUD     │  │
     │  └──────────┘  └──────────┘  │
     │                              │
     │  ┌──────────────────────┐    │
     │  │  piggPin relay :9001 │────┼─── WebSocket
     │  │  real-time map pins  │    │
     │  └──────────────────────┘    │
     └──────┬───────────────────────┘
            │
     ┌──────▼──────┐     ┌────────────┐
     │  PostgreSQL │     │ WASM Crypto│
     │     :5432   │     │ (client)   │
     └─────────────┘     └────────────┘
```

---

## 📦 Crates

| Crate | Role |
|:------|:-----|
| `crates/core` | ◇ Shared data models — Community, Post, Member, MatchThread |
| `crates/server` | ◆ Axum API + JWT auth + DB queries + relay bridge + REPL |
| `crates/wasm` | ◇ Client crypto — ed25519, x25519, ChaCha20Poly1305, Argon2, BIP39 |
| `crates/relay` | ◆ piggPin WebSocket relay — per-community map pin sharing |

---

## 🚀 Quickstart

### Prerequisites

- **Rust** stable
- **Node.js** 18+
- **PostgreSQL** 16
- **wasm-pack** — `cargo install wasm-pack`

### Local dev

```bash
git clone https://git.komun.buzz/Book-Enjoyer/rev.git
cd rev

# Database
docker compose up db -d
cp config.example.toml config.toml     # ← edit this

# Build
wasm-pack build crates/wasm --target web
cd web && npm install && npm run build && cd ..

# Launch
cargo run --bin komun-server
```

→ Open `http://localhost:3000`

### Docker (one command)

```bash
cp config.example.toml config.toml
docker compose up --build
```

### TLS / Production

Komun does **not** handle TLS itself — it expects a reverse proxy in front.

**nginx example:**

```nginx
server {
    listen 443 ssl;
    server_name komun.your-domain.org;

    ssl_certificate     /etc/letsencrypt/live/komun/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/komun/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto https;
    }

    location /relay {
        proxy_pass http://127.0.0.1:9001;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

**Cloudflare:** Point DNS at Cloudflare, enable "Full (strict)" SSL, no further config needed.

**Environment variables for production:**

```bash
export JWT_SECRET="your-strong-random-secret-at-least-32-chars"
export DATABASE_URL="postgres://user:pass@host:5432/komun"
```

`JWT_SECRET` env var is **required** for production — the server warns on startup if it's missing and errors if the secret is too short.

---

## 🔐 Security Model

> **Secret keys never leave the client. Period.**

| Layer | Technology |
|:------|:-----------|
| Signing | ed25519 keypair → challenge-auth → JWT (HS256) |
| Encryption | x25519 ECDH → ChaCha20Poly1305 per-conversation keys |
| Key storage | Passphrase → Argon2 → AES-256-GCM key bundle |
| Recovery | BIP39 12-word code → Argon2 hash → server verification |
| TLS | Terminated at reverse proxy / Cloudflare |

### Key hierarchy

```
Passphrase ──▶ Argon2 ──▶ wrap key ──▶ decrypts key bundle
                                           │
                    ┌──────────────────────┘
                    ▼
            ed25519 secret ──▶ signs challenges ──▶ JWT
            x25519 secret  ──▶ ECDH ──▶ conversation keys
```

### Auth flow

1. Client generates ed25519 + x25519 keypair in WASM
2. Server issues challenge → client signs → server verifies → JWT issued
3. JWT carries `user_id` + `role` — zero DB round-trips for authorization
4. (Optional) Passphrase encrypts key bundle for server-side recovery
5. (Optional) BIP39 recovery code as backup identity factor

---

## 📁 Project Layout

```
rev/
├── crates/
│   ├── core/src/models/       Data models
│   ├── server/src/
│   │   ├── api/               REST handlers
│   │   ├── auth/              JWT + challenge-auth + middleware
│   │   ├── db/                SQL queries per entity
│   │   └── tasks/             Background jobs
│   ├── wasm/src/              Client-side crypto (→ .wasm)
│   └── relay/src/             piggPin WebSocket relay
├── web/                       SvelteKit 5 SPA
├── migrations/                SQLx migrations (additive)
├── docker/                    Multi-stage Dockerfile
├── config.example.toml        Reference config
└── scripts/                   Utilities
```

---

## ✅ Tests

```bash
cargo test --workspace
```

**110 tests, all green.** Server tests cover auth tokens, challenge flow, ed25519, and config parsing. Core tests cover model serialization, enum roundtrips, and type safety. DB integration tests require a running PostgreSQL instance.

---

## 📐 Conventions

| Rule | Because |
|:-----|:--------|
| UUIDv7 for all PKs | Time-sortable, no collisions |
| `cargo check` clean before commit | Don't merge broken builds |
| Migrations are additive | Never edit existing `.sql` files |
| Secrets never logged | `tracing::info!()` == metadata only |
| `config.toml` + `.env` gitignored | Secrets stay local |
| Svelte 5 runes only | `$state()` `$derived()` `$effect()` `$props()` — no `$:` |

---

## 📜 License

**AGPL-3.0-or-later** — [LICENSE](LICENSE)

<div align="center">
<br>
<b>Solidarity, not charity.</b>
</div>
