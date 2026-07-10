# Komun

Federated mutual aid discovery platform. Communities post needs, offers,
and resources — then match them through encrypted conversations.

Komun is one component of the intercommunal software ecosystem, alongside
[Sweeet](https://github.com/user/sweeet) (community logistics) and
[piggPin](https://github.com/user/team-pins) (spatial mesh relay).

## What it does

A community running Komun can:

- **Post needs and offers.** Food, housing, transport, childcare, tools.
- **Match automatically.** Offers get routed to matching needs.
- **Converse securely.** End-to-end encrypted threads between matched parties.
- **Discover other communities.** Federated directory so mutual aid networks
  can find each other.
- **Operate a relay.** Built-in piggPin map relay for spatial coordination.

## Architecture

```
Browser (SvelteKit SPA + WASM crypto)
        |
    HTTPS + WSS
        |
  ┌─────┴──────────────────────┐
  │     Komun Node              │
  │                              │
  │  Axum HTTP API ──────────┐   │
  │  WebSocket Relay ────────┤   │
  │  PostgreSQL ─────────────┤   │
  │  JWT Auth ───────────────┤   │
  │  Federation Engine ──────┤   │
  └──────────────────────────┘   │
              ↕ P2P              │
  ┌──────────────────────────┐   │
  │     Other Komun Nodes    │   │
  │     Sweeet Instances     │   │
  │     piggPin Peers        │   │
  └──────────────────────────┘   │
```

- **Backend:** Rust (Axum 0.8), PostgreSQL, SQLx
- **Frontend:** SvelteKit SPA, static adapter, PWA
- **Crypto:** Client-side WASM (ed25519, x25519, ChaCha20Poly1305, Argon2)
- **Auth:** Challenge-response with ed25519 signing, JWT tokens
- **Federation:** P2P mesh with capability advertisement

## Security model

Secret keys and passphrases never leave the client. The server stores only
public keys and encrypted key bundles. All conversation encryption is
end-to-end via x25519 ECDH + ChaCha20Poly1305.

See [AGENTS.md](AGENTS.md) for the full security model and threat analysis.

## Quick start

Requirements: Rust 1.80+, Node.js 20+, PostgreSQL 16+

```bash
# Start the database
docker compose up db -d

# Build WASM crypto module
wasm-pack build crates/wasm --target web

# Build the frontend
cd web && npm install && npm run build && cd ..

# Copy and edit config
cp config.example.toml config.toml
# Edit config.toml — set node.name, database.url, auth.jwt_secret

# Run
cargo run --release --bin komun-server
```

Open http://localhost:3000

### Docker

```bash
docker compose up --build
```

## Configuration

See `config.example.toml` for all options — every value is documented.
Key sections:

| Section | What it controls |
|---------|-----------------|
| `[server]` | Bind address, port |
| `[database]` | PostgreSQL connection, pool size |
| `[node]` | Community name, description, location |
| `[discovery]` | Directory listing, federation registration |
| `[auth]` | JWT secret, token lifetime, rate limits |
| `[federation]` | P2P mesh enablement, domain, alliances |
| `[security]` | Rate limits, CORS origins |
| `[relay]` | piggPin WebSocket map relay |
| `[posts]` | Default TTL for needs/offers/resources |

## Project structure

```
rev/
├── Cargo.toml              # Workspace
├── config.example.toml      # Documented config template
├── docker-compose.yml       # PostgreSQL + app
├── Dockerfile               # Multi-stage build
├── crates/
│   ├── core/                # Shared data models (Community, Member, Post)
│   ├── server/              # Axum HTTP API, DB, auth, tasks
│   ├── wasm/                # Client-side crypto (ed25519, x25519, Argon2)
│   └── relay/               # piggPin WebSocket map relay
├── web/                     # SvelteKit SPA frontend (PWA)
├── migrations/              # SQLx migrations
└── scripts/                 # Utility scripts
```

## Federation

Komun speaks the Intercommunal Federation Protocol — the same protocol
used by Sweeet and piggPin. Communities discover each other, share surplus,
and coordinate cross-community aid without a central platform.

See [Sweeet docs/PROTOCOL.md](../sweeet/docs/PROTOCOL.md) for the spec.

## License

AGPL-3.0-or-later. See [LICENSE](LICENSE).

The empire can't fork and close. If they use it, they release their changes.
