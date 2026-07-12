# Komun Soft Launch — Alpine Deployment + Search + Federation

> **For Hermes:** Use this plan directly. No need for subagent-driven-development — execute phase by phase.

**Goal:** Deploy Komun on the Alpine server (192.168.1.115), implement full-text search, wire up federation (alliance handshake + cross-server post sharing), finish endorsement UI, and seed realistic test data so people can actually find each other.

**Architecture:** Rust/Axum API binary (musl-static) on Alpine behind nginx + Cloudflare Tunnel. PostgreSQL 18 with FTS indexes. SvelteKit SPA frontend deployed to CF Pages or served via nginx static. Federation via alliance handshake protocol + directory-based server discovery. WebSocket relay for map sharing.

**Current State:**
- Auth: ✅ Ed25519 keypair, JWT, recovery, profiles (complete)
- Communities + posts + matches + messaging: ✅ 
- Directory (server discovery): ✅ geosearch, registration, listing
- Alliances table: ✅ schema + read-only API
- Endorsements: ✅ backend complete — ❌ frontend only shows count, no buttons/list/API client
- Search: ❌ only basic ILIKE — no FTS, no community/user search
- Federation protocol: ❌ no alliance handshake, no cross-server post sharing
- Deployment: ❌ no musl build, no service config, no nginx config
- Seed data: ❌ no test data for soft launch

**Server capacity check:**
- 3.6GB RAM, 1.4GB used, 1.1GB swap (tight — Komun needs ~200-300MB)
- Running: PostgreSQL, Redis, PeerTube, Forgejo, nginx, Conduit
- Akkoma is STOPPED (frees ~500MB)
- Have ~1GB headroom after Akkoma stopped

---

## Phase 0: Endorsement UI (Backend Done — Wire Frontend)

### Task 0.1: Add endorsements to API client

**File:** `web/src/lib/api/client.ts`

Add `endorsements` module:
```ts
endorsements: {
    list: (userId: string) => request<{count: number, endorsements: any[]}>(`/users/${userId}/endorsements`),
    endorse: (userId: string, note?: string) =>
        request<any>(`/users/${userId}/endorse`, {
            method: 'POST',
            body: JSON.stringify({ note }),
            auth: true,
        }),
    unendorse: (userId: string) =>
        request<any>(`/users/${userId}/endorse`, {
            method: 'DELETE',
            auth: true,
        }),
},
```

### Task 0.2: Add endorse/unendorse button to profile page

**File:** `web/src/routes/users/[id]/+page.svelte`

Add below the stats row:
- If NOT own profile + logged in: "Endorse" button (calls POST, updates count)
- If already endorsed: "Endorsed ✓" button (calls DELETE to remove)
- Show loading state during API call
- Show error on self-endorse or conflict

### Task 0.3: Show endorsements list on profile

**File:** `web/src/routes/users/[id]/+page.svelte`

After the stats row, add an "Endorsements" section that fetches `api.endorsements.list()` and renders:
- Each endorser's display name (linked to their profile)
- Endorsement note (if any)
- Relative timestamp
- Empty state: "No endorsements yet."

### Task 0.4: Fetch endorsements in page load

**File:** `web/src/routes/users/[id]/+page.ts`

Add a second fetch to `/api/users/{id}/endorsements` and return `endorsements` alongside `profile`.

---

## Phase 1: Search (PostgreSQL FTS + API + Frontend)

### Task 1.1: Add PostgreSQL full-text search index on posts

**File:** `migrations/014_post_fts.sql`

```sql
-- Add tsvector column
ALTER TABLE posts ADD COLUMN search_vector tsvector;

-- Populate from title + body + tags + category
UPDATE posts SET search_vector = 
    setweight(to_tsvector('english', COALESCE(title, '')), 'A') ||
    setweight(to_tsvector('english', COALESCE(body, '')), 'B') ||
    setweight(to_tsvector('english', COALESCE(category, '')), 'C') ||
    setweight(to_tsvector('english', COALESCE(array_to_string(tags, ' '), '')), 'C');

-- GIN index for fast search
CREATE INDEX idx_posts_search ON posts USING GIN(search_vector);

-- Trigger to keep search_vector updated on insert/update
CREATE FUNCTION posts_search_update() RETURNS TRIGGER AS $$
BEGIN
    NEW.search_vector := 
        setweight(to_tsvector('english', COALESCE(NEW.title, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.body, '')), 'B') ||
        setweight(to_tsvector('english', COALESCE(NEW.category, '')), 'C') ||
        setweight(to_tsvector('english', COALESCE(array_to_string(NEW.tags, ' '), '')), 'C');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_posts_search
    BEFORE INSERT OR UPDATE ON posts
    FOR EACH ROW EXECUTE FUNCTION posts_search_update();
```

### Task 1.2: Add search endpoints to API

**Files:**
- Create: `crates/server/src/api/search.rs`
- Modify: `crates/server/src/api/mod.rs`

New endpoints:
- `GET /api/search?q=food&kind=need&community=eastside` — search posts
- `GET /api/search/communities?q=east` — search communities
- `GET /api/search/users?q=alex` — search users

Search uses PostgreSQL `ts_rank` for relevance scoring:
```sql
SELECT *, ts_rank(search_vector, plainto_tsquery('english', $1)) AS rank
FROM posts WHERE search_vector @@ plainto_tsquery('english', $1)
ORDER BY rank DESC LIMIT 20
```

### Task 1.3: Add search bar to frontend

**Files:**
- Create: `web/src/lib/components/SearchBar.svelte`
- Modify: `web/src/routes/+layout.svelte` — add search bar to nav
- Create: `web/src/routes/search/+page.svelte` — search results page

Search bar in nav header, expands on click. Results page shows tabs: Posts / Communities / Users. Posts show kind badge, community name, urgency. Communities show location + member count. Users show display name + endorsement count.

### Task 1.4: Add community search filter to aid page

**File:** `web/src/routes/aid/+page.svelte`

Add search input above the filter controls. Client-side filter (all data is already loaded on that page) plus option to full-text search on server for cross-community queries.

---

## Phase 2: Federation (Alliance Handshake + Cross-Server Post Discovery)

### Task 2.1: Alliance creation and acceptance flow

**Files:**
- Create: `crates/server/src/api/alliances/create.rs`
- Modify: `crates/server/src/api/alliances.rs`
- Create: `crates/server/src/db/alliances/create.rs`

New endpoints:
- `POST /api/alliances` — propose alliance to remote server
- `POST /api/alliances/{id}/accept` — accept pending alliance
- `POST /api/alliances/{id}/reject` — reject
- `DELETE /api/alliances/{id}` — sever alliance

Handshake flow:
1. Server A admin POSTs `{remote_domain: "b.mutualaid.org"}` 
2. Komun A creates alliance with status=pending, `initiated_by=outgoing`
3. Komun A sends POST to `https://b.mutualaid.org/api/alliances/propose` with `{domain, name, public_key}`
4. Komun B creates matching alliance with status=pending, `initiated_by=incoming`
5. Komun B admin accepts → POST to A's `/api/alliances/{id}/accept`
6. Both sides update to status=accepted

### Task 2.2: Remote node info + federation handshake

**Files:**
- Create: `crates/server/src/federation/handshake.rs`
- Create: `crates/server/src/federation/mod.rs`

When an alliance is accepted, each side fetches the other's `/api/node` endpoint to get:
- Server name, description, location
- Community count
- Federation status
- Relay URL

Store this in `directory_entries` (reuse existing schema) so the remote server appears in directory results.

### Task 2.3: Cross-server post discovery

**Files:**
- Create: `crates/server/src/federation/discovery.rs`
- Modify: `crates/server/src/tasks/mod.rs` — add background sync task

Background task (every 5 minutes) for each accepted alliance:
1. Fetch `GET https://{remote}/api/communities` — list of communities
2. For each community, fetch `GET https://{remote}/api/communities/{slug}/posts?since={last_sync}`
3. Import posts as `federated` with `federated_id` + `origin_node` set
4. Store `last_synced_at` on the alliance row

Add `federated` filter to the aid page and community pages — show a "Federated" tab/section for posts from allied servers.

### Task 2.4: Admin federation UI

**Files:**
- Modify: `web/src/routes/federation/+page.svelte` — full management page

In addition to the current read-only list, add:
- "Propose alliance" form (enter remote domain)
- Accept/reject buttons for pending alliances
- Last synced timestamp
- Community count from remote
- Status badges (accepted/pending/rejected/severed)

---

## Phase 3: Deployment (Alpine Server)

### Task 3.1: Build musl-static binary

```bash
rustup target add x86_64-unknown-linux-musl
cd ~/rev
cargo build --release --target x86_64-unknown-linux-musl --bin komun-server
# → target/x86_64-unknown-linux-musl/release/komun-server
```

### Task 3.2: Create production config

**File:** Create `config.toml` (not committed — contains secrets)

```toml
[server]
bind_address = "127.0.0.1"
port = 3001

[database]
url = "postgres://komun:***@localhost:5432/komun"
max_connections = 10

[node]
name = "STL Mutual Aid"
description = "St. Louis mutual aid discovery — find needs, offers, and resources near you."
public_url = "https://komun.mutualaid.org"
location_name = "St. Louis, MO"
location_lat = 38.6270
location_lon = -90.1994

[discovery]
listed = true
directory_enabled = true
registration_mode = "open"

[auth]
jwt_secret = "REDACTED-at-least-32-chars"
token_lifetime_days = 30
max_registrations_per_hour = 20

[federation]
enabled = true
domain = "komun.mutualaid.org"
max_alliances = 50

[relay]
enabled = true
port = 9001
bind_address = "127.0.0.1"
storage_path = "data/relay"
external_url = "wss://komun.mutualaid.org/relay-ws"

[media]
avatar_dir = "data/avatars"
max_avatar_bytes = 1048576
post_images_dir = "data/post-images"
max_post_image_bytes = 5242880
max_post_images = 5
```

### Task 3.3: Create OpenRC service script

**File:** Create `deploy/komun.initd` (scp to server)

```sh
#!/sbin/openrc-run
name="komun"
description="Komun mutual aid API server"
command="/opt/komun/komun-server"
command_args=""
pidfile="/var/run/komun.pid"
command_background=true
command_user="komun"
directory="/opt/komun"

depend() {
    need net
    need postgresql
    use nginx
}
```

### Task 3.4: Set up server

```bash
# On Alpine server:
addgroup komun
adduser -S -s /bin/false -h /opt/komun -H -G komun komun
mkdir -p /opt/komun/data/avatars /opt/komun/data/post-images /opt/komun/data/relay
chown -R komun:komun /opt/komun

# Create DB
su -s /bin/sh - postgres -c "psql -c \"CREATE USER komun WITH PASSWORD 'REDACTED';\""
su -s /bin/sh - postgres -c "psql -c \"CREATE DATABASE komun OWNER komun;\""
su -s /bin/sh - postgres -c "psql -d komun -c \"GRANT ALL ON SCHEMA public TO komun;\""
su -s /bin/sh - postgres -c "psql -d komun -c \"GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO komun;\""
su -s /bin/sh - postgres -c "psql -d komun -c \"GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO komun;\""

# Run migrations
scp migrations/*.sql root@192.168.1.115:/opt/komun/migrations/
ssh root@192.168.1.115 'for f in /opt/komun/migrations/*.sql; do su -s /bin/sh - postgres -c "psql -d komun -f $f"; done'

# Deploy binary
scp target/x86_64-unknown-linux-musl/release/komun-server root@192.168.1.115:/opt/komun/
scp config.toml root@192.168.1.115:/opt/komun/

# Install service
scp deploy/komun.initd root@192.168.1.115:/etc/init.d/komun
chmod +x /etc/init.d/komun
rc-update add komun
rc-service komun start
```

### Task 3.5: Nginx reverse proxy config

**File:** Create `deploy/nginx-komun.conf` (scp to server)

```nginx
server {
    listen 8080;
    server_name komun.mutualaid.org;

    # API
    location /api/ {
        proxy_pass http://127.0.0.1:3001;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }

    # WebSocket relay
    location /relay-ws {
        proxy_pass http://127.0.0.1:9001;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "Upgrade";
    }

    # Static files (avatars, post images)
    location /avatars/ {
        proxy_pass http://127.0.0.1:3001;
    }
    location /post-images/ {
        proxy_pass http://127.0.0.1:3001;
    }
    location /node {
        proxy_pass http://127.0.0.1:3001;
    }

    # Frontend SPA
    location / {
        root /opt/komun/frontend;
        try_files $uri $uri/ /index.html;
    }
}
```

---

## Phase 4: Seed Data

### Task 4.1: Create seed data SQL

**File:** Create `deploy/seed.sql`

Realistic St. Louis mutual aid seed data:
- 3 communities: "Eastside Free Food", "South City Tool Share", "North County Transit Collective"
- 15-20 posts across needs/offers/resources: food pickup shifts, free meals, tool lending, ride shares, housing resources
- 5-8 test users with Ed25519 keys (use `komun-seed-patterns` skill)
- 1 alliance with a test peer
- Cross-endorsements between users (to populate the web of trust)

---

## Phase 5: Smoke Test & Verify

### Task 5.1: End-to-end test

1. Build and deploy binary
2. Run migrations + seed data
3. Start OpenRC service
4. Verify: `curl https://komun.mutualaid.org/api/node`
5. Verify: `curl https://komun.mutualaid.org/api/communities`
6. Verify: `curl "https://komun.mutualaid.org/api/search?q=food"`
7. Verify: `curl "https://komun.mutualaid.org/api/users/{id}/endorsements"`
8. Verify: WebSocket relay `wscat -c wss://komun.mutualaid.org/relay-ws`
9. Frontend: build SPA, deploy to nginx path, verify in browser

---

## Files Changed Summary

| Phase | Create | Modify |
|-------|--------|--------|
| 0. Endorsements | — | `web/src/lib/api/client.ts`, `web/src/routes/users/[id]/+page.svelte`, `web/src/routes/users/[id]/+page.ts` |
| 1. Search | `migrations/014_post_fts.sql`, `crates/server/src/api/search.rs`, `web/src/lib/components/SearchBar.svelte`, `web/src/routes/search/+page.svelte` | `crates/server/src/api/mod.rs`, `web/src/routes/+layout.svelte`, `web/src/routes/aid/+page.svelte` |
| 2. Federation | `crates/server/src/api/alliances/create.rs`, `crates/server/src/db/alliances/create.rs`, `crates/server/src/federation/mod.rs`, `crates/server/src/federation/handshake.rs`, `crates/server/src/federation/discovery.rs` | `crates/server/src/api/alliances.rs`, `crates/server/src/tasks/mod.rs`, `web/src/routes/federation/+page.svelte` |
| 3. Deploy | `config.toml`, `deploy/komun.initd`, `deploy/nginx-komun.conf`, `deploy/seed.sql` | — |
| 4. Seed | `deploy/seed.sql` | — |
| 5. Test | — | — |

## Risks

- **RAM pressure**: 3.6GB server already at 1.4GB + 1.1GB swap. Komun adds ~200-300MB. With Akkoma stopped (~500MB freed), there's headroom. Monitor swap after deploy.
- **PostgreSQL FTS on small server**: GIN indexes are write-heavy. Rate-limit posts per hour already in place (60/hr). Should be fine for low-volume mutual aid usage.
- **Federation security**: Alliance handshake needs Ed25519 signing to prevent spoofing. Use the existing `public_key` field on alliances table. Remote server's `/api/node` response includes its public key for verification.
- **Frontend deployment**: Need to decide — build SPA and serve via nginx on the same Alpine box, or deploy to Cloudflare Pages. nginx is simpler for soft launch.
- **Cloudflare Tunnel**: Existing tunnel already routes `*.uate.social` to port 4000 (Akkoma). Need a new hostname for Komun. Use the same tunnel with a new ingress rule.
