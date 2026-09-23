#!/usr/bin/env node
/**
 * A3 runtime gate against komun_a on port 3011.
 *
 * Every call below is the real server over HTTP. The signup payload's crypto is the same
 * stand-in A2b used (scrypt where the real client uses Argon2id, node's own x25519 and
 * ChaCha20-Poly1305) — the server stores those fields opaquely, so the substitution changes
 * nothing about what is being asserted here, which is the shape of the flat post API, the
 * sealed conversation, and the absence of any tenant key in any response body.
 *
 *   node .dispatch/A3-live.mjs
 */

import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';

const BASE = 'http://127.0.0.1:3011';
const DB = ['-h', 'komun-db-a', '-U', 'komun', '-d', 'komun_a'];

let failures = 0;

function ok(name, cond, detail = '') {
	if (cond) console.log(`  PASS  ${name}${detail ? ` — ${detail}` : ''}`);
	else { failures++; console.log(`  FAIL  ${name}${detail ? ` — ${detail}` : ''}`); }
}

function sql(query) {
	return execFileSync('psql', [...DB, '-tAc', query], {
		env: { ...process.env, PGPASSWORD: 'komun' }, encoding: 'utf8',
	}).trim();
}

const b64 = (buf) => Buffer.from(buf).toString('base64');
const b = (s) => Buffer.from(s, 'base64');

function newX25519() {
	const { publicKey, privateKey } = crypto.generateKeyPairSync('x25519');
	return {
		pub: publicKey.export({ type: 'spki', format: 'der' }).subarray(-32),
		sec: privateKey.export({ type: 'pkcs8', format: 'der' }).subarray(-32),
	};
}
const deriveKey = (password, salt) =>
	crypto.scryptSync(Buffer.from(password, 'utf8'), salt, 32, { N: 16384, r: 8, p: 1 });
function seal(plaintext, key) {
	const nonce = crypto.randomBytes(12);
	const c = crypto.createCipheriv('chacha20-poly1305', key, nonce, { authTagLength: 16 });
	return Buffer.concat([nonce, c.update(plaintext), c.final(), c.getAuthTag()]);
}
function wrapSecret(secret, password) {
	const salt = crypto.randomBytes(16);
	return { bundle: seal(secret, deriveKey(password, salt)), salt };
}
const verifier = (password, authSaltB64) => deriveKey(password, b(authSaltB64)).toString('base64');

// --- http -------------------------------------------------------------------

/** Every body this run receives, so the tenant-key scan at the end sees all of them. */
const bodies = [];

async function req(method, path, { body, token, label } = {}) {
	const headers = {};
	if (body !== undefined) headers['Content-Type'] = 'application/json';
	if (token) headers['Authorization'] = `Bearer ${token}`;
	const res = await fetch(`${BASE}${path}`, {
		method, headers, body: body === undefined ? undefined : JSON.stringify(body),
	});
	const text = await res.text();
	let json = null;
	try { json = JSON.parse(text); } catch { /* not json */ }
	bodies.push({ label: label ?? `${method} ${path}`, status: res.status, text });
	return { status: res.status, json, text };
}

function show(label) {
	const e = bodies.find((x) => x.label === label);
	console.log(`\n--- ${label} → HTTP ${e.status}\n${e.text}`);
}

let seq = 0;
async function signup(tag) {
	const password = 'correct horse battery staple';
	const email = `a3-${tag}-${Date.now()}-${++seq}@example.test`;
	const keys = newX25519();
	const authSalt = crypto.randomBytes(16);
	const wrapped = wrapSecret(keys.sec, password);
	const rec = wrapSecret(keys.sec, 'recovery code words here');
	const res = await req('POST', '/api/auth/signup', {
		label: `signup ${tag}`,
		body: {
			email, display_name: `A3 ${tag}`,
			verifier: verifier(password, b64(authSalt)),
			auth_salt: b64(authSalt), password_length: password.length,
			encryption_public_key: b64(keys.pub),
			encrypted_key_bundle: b64(wrapped.bundle), bundle_salt: b64(wrapped.salt),
			encrypted_recovery_bundle: b64(rec.bundle), recovery_bundle_salt: b64(rec.salt),
		},
	});
	if (res.status !== 200) throw new Error(`signup ${tag} failed: ${res.status} ${res.text}`);
	return { email, token: res.json.token ?? res.json.session_token, id: res.json.user?.id ?? res.json.user_id };
}

// --- run --------------------------------------------------------------------

const alice = await signup('alice');
const bob = await signup('bob');
ok('two accounts signed up', !!alice.token && !!bob.token);

// 1. POST /api/posts — a need and a marketplace listing.
const need = await req('POST', '/api/posts', {
	label: 'POST /api/posts (need)', token: alice.token,
	body: {
		kind: 'need', category: 'food', title: 'Winter coats for the warming centre',
		body: 'We need adult coats, sizes M through XXL, by Friday.',
		urgency: 'high', location_name: 'Southside', tags: ['coats', 'winter'],
	},
});
ok('create a need → 200', need.status === 200, `status ${need.status}`);

const listing = await req('POST', '/api/posts', {
	label: 'POST /api/posts (listing)', token: alice.token,
	body: {
		kind: 'listing', category: 'bikes-vehicles', title: 'Road bike, 54cm',
		body: 'Serviced last month.', market_listed: true, price_cents: 12000,
		currency: 'EUR', price_negotiable: true, item_condition: 'good',
	},
});
ok('create a listing → 200', listing.status === 200, `status ${listing.status}`);
ok('a listing comes back as kind=listing, not need (the From<PostRow> bug)',
	listing.json?.kind === 'listing', `kind=${listing.json?.kind}`);

// The market-field guard.
const bad = await req('POST', '/api/posts', {
	label: 'POST /api/posts (price on a need)', token: alice.token,
	body: { kind: 'need', category: 'food', title: 'x', price_cents: 500 },
});
ok('market fields on a non-market kind → 400', bad.status === 400, `status ${bad.status}`);

// 2. GET /api/posts — the feed.
const feed = await req('GET', '/api/posts', { label: 'GET /api/posts' });
ok('feed → 200', feed.status === 200, `status ${feed.status}`);
ok('feed has both posts', Array.isArray(feed.json) && feed.json.length === 2,
	`${feed.json?.length} entries`);

// 3. GET /api/posts/{id}.
const one = await req('GET', `/api/posts/${need.json.id}`, { label: 'GET /api/posts/{id}' });
ok('one post → 200', one.status === 200, `status ${one.status}`);
const missing = await req('GET', `/api/posts/${crypto.randomUUID()}`, { label: 'GET /api/posts/{unknown}' });
ok('an unknown post → 404, not 500', missing.status === 404, `status ${missing.status}`);

// 4. Search.
const search = await req('GET', '/api/search?q=coats', { label: 'GET /api/search?q=coats' });
ok('search → 200 (tags decode from TEXT[])', search.status === 200, `status ${search.status}`);
ok('search finds the need', search.json?.[0]?.id === need.json.id);

// 5. A conversation thread, sealed.
const ct1 = crypto.randomBytes(48), n1 = crypto.randomBytes(12);
const respond = await req('POST', `/api/posts/${need.json.id}/respond`, {
	label: 'POST /api/posts/{id}/respond', token: bob.token,
	body: { ciphertext: b64(ct1), nonce: b64(n1) },
});
ok('respond → 200', respond.status === 200, `status ${respond.status}`);
const matchId = respond.json?.match_id ?? respond.json?.id;

const plaintextAttempt = await req('POST', `/api/posts/${listing.json.id}/respond`, {
	label: 'POST respond with a plaintext body', token: bob.token,
	body: { body: 'hello there' },
});
ok('a plaintext `body` is no longer an accepted shape',
	plaintextAttempt.status === 400 || plaintextAttempt.status === 422,
	`status ${plaintextAttempt.status}`);

const ct2 = crypto.randomBytes(64), n2 = crypto.randomBytes(12);
const msg = await req('POST', `/api/conversations/${matchId}/messages`, {
	label: 'POST /api/conversations/{id}/messages', token: alice.token,
	body: { ciphertext: b64(ct2), nonce: b64(n2) },
});
ok('send a sealed message → 200', msg.status === 200, `status ${msg.status}`);

const thread = await req('GET', `/api/conversations/${matchId}`, {
	label: 'GET /api/conversations/{id}', token: alice.token,
});
ok('thread → 200', thread.status === 200, `status ${thread.status}`);
const msgs = thread.json?.messages ?? [];
ok('the thread returns two sealed messages', msgs.length === 2, `${msgs.length} messages`);
ok('message 1 round-trips as the exact ciphertext that was sent',
	msgs[0]?.ciphertext === b64(ct1), msgs[0]?.ciphertext);
ok('message 1 round-trips the nonce', msgs[0]?.nonce === b64(n1));
ok('no message carries a `body` key', msgs.every((m) => m.body === undefined));

const convos = await req('GET', '/api/me/conversations', {
	label: 'GET /api/me/conversations', token: bob.token,
});
ok('conversation list → 200', convos.status === 200, `status ${convos.status}`);
ok('the list preview is ciphertext, not text',
	convos.json?.[0]?.last_message_ciphertext === b64(ct2),
	convos.json?.[0]?.last_message_ciphertext);

// The DB side of A3.3: nothing readable was stored.
ok('matches.message is NULL — no plaintext copy of the opening message',
	sql(`SELECT message IS NULL FROM matches WHERE id = '${matchId}'`) === 't');
ok('messages has no `body` column at all',
	sql(`SELECT count(*) FROM information_schema.columns
	     WHERE table_name = 'messages' AND column_name = 'body'`) === '0');

// 6. /auth/me.
const me = await req('GET', '/api/auth/me', { label: 'GET /api/auth/me', token: bob.token });
ok('/auth/me → 200', me.status === 200, `status ${me.status}`);

// 7. The mounts.
const alliances = await req('GET', '/api/alliances', { label: 'GET /api/alliances' });
ok('GET /api/alliances → 404', alliances.status === 404, `status ${alliances.status}`);
const health = await req('GET', '/api/health', { label: 'GET /api/health' });
ok('GET /api/health → 200', health.status === 200, `status ${health.status}`);

// 8. No tenant key in any body this run produced.
//
// Keys only: a *value* containing the word (a post whose title says "community garden") is not
// a schema leak, and the card asks about keys.
function keysIn(node, acc = new Set()) {
	if (Array.isArray(node)) node.forEach((n) => keysIn(n, acc));
	else if (node && typeof node === 'object') {
		for (const [k, v] of Object.entries(node)) { acc.add(k); keysIn(v, acc); }
	}
	return acc;
}
const offenders = [];
for (const e of bodies) {
	let parsed = null;
	try { parsed = JSON.parse(e.text); } catch { continue; }
	for (const k of keysIn(parsed)) {
		if (/communit|slug/i.test(k)) offenders.push(`${e.label}: "${k}"`);
	}
}
ok(`no community/slug key in any of the ${bodies.length} response bodies`,
	offenders.length === 0, offenders.join(', '));

// --- raw bodies the card asks to see ----------------------------------------

console.log('\n================ RAW BODIES ================');
[
	'GET /api/posts',
	'GET /api/posts/{id}',
	'GET /api/search?q=coats',
	'GET /api/conversations/{id}',
	'GET /api/me/conversations',
	'GET /api/auth/me',
	'GET /api/alliances',
	'GET /api/health',
].forEach(show);

console.log(`\n${failures === 0 ? 'ALL PASS' : `${failures} FAILED`}`);
process.exit(failures === 0 ? 0 : 1);
