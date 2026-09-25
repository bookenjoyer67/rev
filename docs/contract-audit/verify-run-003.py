#!/usr/bin/env python3
"""Re-execute Run 003's citations against the repository: every claimed path:line must
contain the quoted text, and every claimed count must reproduce."""
import re, subprocess, sys, os

R = "/home/computing/rev"
os.chdir(R)

def lines(path):
    try:
        return open(path, encoding="utf-8", errors="replace").read().split("\n")
    except FileNotFoundError:
        return None

# (label, path, line_number, expected substring)
CITES = [
    ("gitignore line 8",            ".gitignore", 8, "config.toml"),
    ("gitignore line 6",            ".gitignore", 6, ".env"),
    ("gitignore 12/13",             ".gitignore", 12, "data/avatars/"),
    ("gitignore 13",                ".gitignore", 13, "data/post-images/"),
    ("package.json:33 wasm dep",    "web/package.json", 33, "komun-wasm"),
    ("main.rs:123 router",          "crates/server/src/main.rs", 123, "Router::new()"),
    ("main.rs:125 avatars",         "crates/server/src/main.rs", 125, "nest_service(\"/avatars\""),
    ("main.rs:126 post-images",     "crates/server/src/main.rs", 126, "nest_service(\"/post-images\""),
    ("nginx:30 sveltekit build",    "deploy/nginx-komun.conf", 30, "SvelteKit static build"),
    ("nginx:33 try_files",          "deploy/nginx-komun.conf", 33, "try_files $uri $uri/ /index.html;"),
    ("Dockerfile:5 FROM",           "docker/Dockerfile", 5, "FROM rust:1.95-slim-bookworm AS builder"),
    ("Dockerfile:2 comment",        "docker/Dockerfile", 2, "1.82 until 2026-09-25"),
    ("Dockerfile:14 ENV",           "docker/Dockerfile", 14, "ENV SQLX_OFFLINE=true"),
    ("DEVELOPMENT.md:12 toolchain", "docs/DEVELOPMENT.md", 12, "1.95.0"),
    ("DEVELOPMENT.md:98 FROZEN",    "docs/DEVELOPMENT.md", 98, "001_schema.sql` is FROZEN"),
    ("002 additive :6",             "migrations/002_directory_open_registration.sql", 6, "ALTER TABLE directory_entries"),
    ("002 additive :7",             "migrations/002_directory_open_registration.sql", 7, "ADD COLUMN open_registration BOOLEAN NOT NULL DEFAULT true;"),
    ("001:25 enc public key",       "migrations/001_schema.sql", 25, "encryption_public_key BYTEA"),
    ("001:227 messages comment",    "migrations/001_schema.sql", 227, "ciphertext only, no plaintext body"),
    ("001:228 messages table",      "migrations/001_schema.sql", 228, "CREATE TABLE messages ("),
    ("001:232 ciphertext",          "migrations/001_schema.sql", 232, "ciphertext BYTEA NOT NULL"),
    ("001:212 matches.message",     "migrations/001_schema.sql", 212, "message TEXT"),
    ("003:22 ALTER",                "migrations/003_drop_matches_message.sql", 22, "ALTER TABLE matches DROP COLUMN message;"),
    ("001:160 chk_posts_kind",      "migrations/001_schema.sql", 160, "chk_posts_kind CHECK (kind IN ('resource', 'need', 'offer', 'listing', 'want'))"),
    ("001:151 price_cents",         "migrations/001_schema.sql", 151, "price_cents BIGINT"),
    ("001:185 posts_search fn",     "migrations/001_schema.sql", 185, "posts_search_update() RETURNS TRIGGER"),
    ("001:199 trigger",             "migrations/001_schema.sql", 199, "CREATE TRIGGER trg_posts_search"),
    ("001:96 chk_categories_scope", "migrations/001_schema.sql", 96, "chk_categories_scope CHECK (scope IN ('aid', 'market', 'both'))"),
    ("001:124 last category row",   "migrations/001_schema.sql", 124, "('transport',"),
    ("001:315 avatar_uploads id",   "migrations/001_schema.sql", 315, "BIGSERIAL"),
    ("auth/mod.rs:138 verifier",    "crates/server/src/auth/mod.rs", 138, "The password itself is never sent."),
    ("auth/mod.rs:1619 session",    "crates/server/src/auth/mod.rs", 1619, "pub async fn require_session("),
    ("auth/mod.rs:1640 auth",       "crates/server/src/auth/mod.rs", 1640, "pub async fn require_auth("),
    ("auth/mod.rs:1664 role db",    "crates/server/src/auth/mod.rs", 1664, "role comes from the database on this request"),
    ("auth/mod.rs:1686 superadmin", "crates/server/src/auth/mod.rs", 1686, "pub async fn require_superadmin("),
    ("auth/mod.rs:113 /me route",   "crates/server/src/auth/mod.rs", 113, ".route(\"/me\","),
    ("auth/mod.rs:692 rehash log",  "crates/server/src/auth/mod.rs", 692, "password rehash for {} failed: {e}"),
    ("api/mod.rs:31 router fn",     "crates/server/src/api/mod.rs", 31, "pub fn router(state: AppState) -> Router {"),
    ("api/mod.rs:33 geocode route", "crates/server/src/api/mod.rs", 33, ".route(\"/geocode\","),
    ("api/mod.rs:62 link-preview",  "crates/server/src/api/mod.rs", 62, ".route(\"/link-preview\","),
    ("api/mod.rs:21 alliances",     "crates/server/src/api/mod.rs", 21, "`alliances` is gone"),
    ("api/mod.rs:64 directory if",  "crates/server/src/api/mod.rs", 64, "if state.config.discovery.directory_enabled {"),
    ("health.rs:5 route",           "crates/server/src/api/health.rs", 5, "route(\"/health\""),
    ("node.rs:27 route",            "crates/server/src/api/node.rs", 27, ".route(\"/node\","),
    ("search.rs:16 route",          "crates/server/src/api/search.rs", 16, ".route(\"/search\","),
    ("search.rs:17 users route",    "crates/server/src/api/search.rs", 17, ".route(\"/search/users\","),
    ("categories.rs:36 route",      "crates/server/src/api/categories.rs", 36, "route(\"/categories\""),
    ("reports.rs:16 report route",  "crates/server/src/api/reports.rs", 16, ".route(\"/posts/{post_id}/report\","),
    ("reports.rs:18 hide route",    "crates/server/src/api/reports.rs", 18, ".route(\"/posts/{post_id}/hide\","),
    ("reviews.rs:44 route",         "crates/server/src/api/reviews.rs", 44, ".route(\"/matches/{match_id}/reviews\","),
    ("reviews.rs:59 SPEC B4",       "crates/server/src/db/reviews.rs", 59, "writable only against a completed deal"),
    ("conversations.rs:24 route",   "crates/server/src/api/conversations.rs", 24, ".route(\"/me/conversations\","),
    ("notifications.rs:15 route",   "crates/server/src/api/notifications.rs", 15, ".route(\"/me/notifications\","),
    ("directory.rs:30 route",       "crates/server/src/api/directory.rs", 30, ".route(\"/directory\","),
    ("admin.rs:20 stats route",     "crates/server/src/api/admin.rs", 20, ".route(\"/admin/stats\","),
    ("config.rs:59 JWT obituary",   "crates/server/src/config.rs", 59, "signing-key setting is gone with the JWTs"),
    ("config.rs:158 require_verif", "crates/server/src/config.rs", 158, "require_email_verification: true,"),
    ("config.rs:180 default_curr",  "crates/server/src/config.rs", 180, "pub default_currency: Option<String>,"),
    ("config.rs:291 KOMUN_CONFIG",  "crates/server/src/config.rs", 291, "std::env::var(\"KOMUN_CONFIG\")"),
    ("config.rs:316 refusal",       "crates/server/src/config.rs", 316, "if self.registration.require_email_verification && !self.email.is_configured() {"),
    ("main.rs:78 migrate!",         "crates/server/src/main.rs", 78, "sqlx::migrate!"),
    ("main.rs:89 pepper",           "crates/server/src/main.rs", 89, "salt_pepper: Arc::new(sessions::generate_pepper())"),
    ("main.rs:150 IsTerminal",      "crates/server/src/main.rs", 150, "IsTerminal::is_terminal"),
    ("repl.rs:26 help",             "crates/server/src/repl.rs", 26, "\"help\" | \"?\" => print_help()"),
    ("sessions.rs:13 sha2",         "crates/server/src/sessions.rs", 13, "use sha2::{Digest, Sha256};"),
    ("sessions.rs:47 hash_token",   "crates/server/src/sessions.rs", 47, "pub fn hash_token(raw: &str) -> Vec<u8> {"),
    ("sessions.rs:115 pepper doc",  "crates/server/src/sessions.rs", 115, "per-deployment pepper, generated at startup"),
    ("sessions.rs:120 generate",    "crates/server/src/sessions.rs", 120, "pub fn generate_pepper() -> Vec<u8> {"),
    ("sessions.rs:107 cleanup log", "crates/server/src/sessions.rs", 107, "session cleanup removed"),
    ("password.rs:4 verifier doc",  "crates/server/src/auth/password.rs", 4, "Argon2id(password, auth_salt)"),
    ("password.rs:23 params",       "crates/server/src/auth/password.rs", 23, "Argon2::new(Algorithm::Argon2id, Version::V0x13, params)"),
    ("email.rs:57 starttls_relay",  "crates/server/src/auth/email.rs", 57, "starttls_relay"),
    ("match_thread.rs:46 relays",   "crates/core/src/models/match_thread.rs", 46, "stores and relays ciphertext only"),
    ("core models/mod.rs:7 macro",  "crates/core/src/models/mod.rs", 7, "macro_rules! db_enum {"),
    ("core tests.rs:11 comment",    "crates/core/src/tests.rs", 11, "read out of migrations/001_schema.sql at test time"),
    ("core tests.rs:15 path",       "crates/core/src/tests.rs", 15, "concat!(env!(\"CARGO_MANIFEST_DIR\")"),
    ("wasm Cargo.toml:15 x25519",   "crates/wasm/Cargo.toml", 15, "x25519-dalek"),
    ("wasm lib.rs:4 XChaCha",       "crates/wasm/src/lib.rs", 4, "XChaCha20Poly1305"),
    ("svelte.config.js:1 adapter",  "web/svelte.config.js", 1, "adapter-static"),
    ("+layout.ts:1 ssr",            "web/src/routes/+layout.ts", 1, "export const ssr = false;"),
    ("service-worker.ts:43 paths",  "web/src/service-worker.ts", 43, "CACHEABLE_API_PATHS = ['/api/node', '/api/health', '/api/posts', '/api/directory'];"),
    ("manifest.json:6 standalone",  "web/static/manifest.json", 6, "\"display\": \"standalone\","),
    ("auth.ts:434 multiline call",  "web/src/lib/stores/auth.ts", 434, "/api/auth/password-reset/bundle"),
    ("api/reviews.ts:65 fetch",     "web/src/lib/api/reviews.ts", 65, "fetch(`${server}/api${path}`"),
    ("api/offers.ts:72 fetch",      "web/src/lib/api/offers.ts", 72, "fetch(`${server}/api${path}`"),
    ("clippy-report.md:71 warning", "docs/clippy-report.md", 71, "sqlx-postgres v0.8.0"),
    ("clippy-report.md:62 proceed", "docs/clippy-report.md", 62, "Proceed."),
    ("platform DOCS: DATABASE.md:3","docs/DATABASE.md", 3, "UUIDv7"),
    ("compose:3 postgres",          "docker-compose.yml", 3, "postgres:16-alpine"),
    ("Cargo.toml:8 license",        "Cargo.toml", 8, "AGPL-3.0-or-later"),
    ("LICENSE:1",                   "LICENSE", 1, "GNU AFFERO GENERAL PUBLIC LICENSE"),
    ("agent-summary.md:3",          "agent-summary.md", 3, "Federated mutual aid discovery platform"),
    ("agent-summary.md:11",         "agent-summary.md", 11, "Community, Member, Post, MatchThread, User"),
    ("setup.md:386",                "setup.md", 386, "community::Community"),
    ("config.example.toml:98",      "config.example.toml", 98, "require_email_verification = false"),
    ("config.example.toml:129",     "config.example.toml", 129, "# default_currency = \"USD\""),
]

fails = []
for label, path, ln, want in CITES:
    ls = lines(path)
    if ls is None:
        fails.append((label, path, ln, "FILE MISSING", want)); continue
    got = ls[ln-1] if 0 < ln <= len(ls) else "<line does not exist>"
    ok = want in got
    if not ok:
        fails.append((label, path, ln, got.strip()[:100], want))
print(f"citations checked: {len(CITES)}   mismatches: {len(fails)}")
for f in fails:
    print(f"  MISMATCH {f[0]:32} {f[1]}:{f[2]}\n     want: {f[4]}\n     got : {f[3]}")

print("\n=== counts re-run ===")
def sh(cmd):
    return subprocess.run(cmd, shell=True, capture_output=True, text=True).stdout.strip()

gl = lines(".gitignore")
print("gitignore lines:", len(gl) - (1 if gl[-1] == "" else 0))
print("fetch() in web/src with run003's corrected pattern:",
      sh("grep -rE 'fetch\\(.*/api/' web/src | wc -l"),
      "in files:", sh("grep -rEl 'fetch\\(.*/api/' web/src | wc -l"))
print("auth.ts with that pattern:", sh("grep -cE 'fetch\\(.*/api/' web/src/lib/stores/auth.ts"))
print("fetch( to /api/ per-file (full listing):")
print(sh("grep -rcE 'fetch\\(' web/src --include='*.ts' --include='*.svelte' | grep -v ':0$' | sort -t: -k2 -rn | head -5"))
print("#[test]/#[tokio::test] in crates/core:", sh("grep -rE '#\\[(tokio::)?test\\]' crates/core/src | wc -l"),
      "crates/server:", sh("grep -rE '#\\[(tokio::)?test\\]' crates/server/src | wc -l"),
      "crates/wasm:", sh("grep -rE '#\\[(tokio::)?test\\]' crates/wasm/src | wc -l"))
print("it(/test( in web test files:", sh("grep -rE '^\\s*(it|test)\\(' web/src --include='*.test.ts' | wc -l"),
      "files:", sh("grep -rlE '^\\s*(it|test)\\(' web/src --include='*.test.ts' | wc -l"))
print("TRIGGER|RULE |REVOKE in migrations:", sh("grep -rE 'TRIGGER|RULE |REVOKE' migrations/ | wc -l"))
print("relay|piggpin|federation (case-insens) in crates/:",
      sh("grep -riE 'relay|piggpin|federation' crates/ | wc -l"))
print("logging macros with sensitive words in crates/:",
      sh("grep -rE 'tracing::(info|warn|error|debug)!' crates/ | grep -E 'verifier|password|secret|bundle|plaintext|ciphertext|recovery_code|wrap_key|token' | wc -l"))
print("now_v7|Uuid::new_v4 in crates/:",
      sh("grep -rE 'now_v7|Uuid::new_v4' crates/ | wc -l"),
      "files:", sh("grep -rEl 'now_v7|Uuid::new_v4' crates/ | wc -l"))
print("categories seed tuples between 101 and 124:",
      sh("sed -n '101,124p' migrations/001_schema.sql | grep -c \"^    ('\""))
print("route( declarations in crates/server/src (the number run 002 mis-stated as 61):",
      sh("grep -rhoE '\\.route\\(\"' crates/server/src | wc -l"))
print("\n=== the three docs/ artifacts the run called missing ===")
for p in ("docs/agent-rubric.md", "docs/contract-audit", "docs/prd.md", "docs/rubric.md",
          "docs/iteration-log.md", "docs/clippy-report.md", "docs/clippy-gate/prd.md",
          "docs/clippy-gate/rubric.md", "docs/clippy-gate/iteration-log.md"):
    print(f"  {'EXISTS' if os.path.exists(p) else 'MISSING':8} {p}")
