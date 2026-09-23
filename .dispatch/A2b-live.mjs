#!/usr/bin/env node
/**
 * A2b live assertions against komun_a on port 3011.
 *
 * Run one phase per invocation; the caller restarts the server between phases because the rate
 * limiter is in-process and several assertions deliberately exhaust a bucket.
 *
 *   node .dispatch/A2b-live.mjs rate
 *   node .dispatch/A2b-live.mjs core
 *   node .dispatch/A2b-live.mjs reset-with-code
 *   node .dispatch/A2b-live.mjs reset-without-code
 *   node .dispatch/A2b-live.mjs token-reuse
 *   node .dispatch/A2b-live.mjs admin
 *
 * What is real here and what is a stand-in, stated up front:
 *   - every HTTP call, every session, every bundle stored and read back is the real server;
 *   - the x25519 keys and the ChaCha20-Poly1305 wrapping are real (node's own primitives);
 *   - the password-to-key derivation is scrypt rather than Argon2id, because there is no Argon2
 *     binding available to this harness. The server never sees either one — it stores the
 *     verifier and the ciphertext opaquely — so the substitution does not affect what is being
 *     asserted, which is that the *right* secret comes back out.
 */

import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';

const BASE = 'http://127.0.0.1:3011';
const DB = ['-h', 'komun-db-a', '-U', 'komun', '-d', 'komun_a'];

let failures = 0;
let checks = 0;

function ok(name, cond, detail = '') {
	checks++;
	if (cond) {
		console.log(`  PASS  ${name}${detail ? ` — ${detail}` : ''}`);
	} else {
		failures++;
		console.log(`  FAIL  ${name}${detail ? ` — ${detail}` : ''}`);
	}
}

function sql(query) {
	return execFileSync('psql', [...DB, '-tAc', query], {
		env: { ...process.env, PGPASSWORD: 'komun' },
		encoding: 'utf8',
	}).trim();
}

// --- crypto helpers ---------------------------------------------------------

const SPKI = Buffer.from('302a300506032b656e032100', 'hex');
const PKCS8 = Buffer.from('302e020100300506032b656e04220420', 'hex');

function newX25519() {
	const { publicKey, privateKey } = crypto.generateKeyPairSync('x25519');
	return {
		pub: publicKey.export({ type: 'spki', format: 'der' }).subarray(-32),
		sec: privateKey.export({ type: 'pkcs8', format: 'der' }).subarray(-32),
	};
}

function pubFromRaw(raw) {
	return crypto.createPublicKey({
		key: Buffer.concat([SPKI, raw]),
		format: 'der',
		type: 'spki',
	});
}

function secFromRaw(raw) {
	return crypto.createPrivateKey({
		key: Buffer.concat([PKCS8, raw]),
		format: 'der',
		type: 'pkcs8',
	});
}

function sharedKey(secRaw, pubRaw) {
	const s = crypto.diffieHellman({ privateKey: secFromRaw(secRaw), publicKey: pubFromRaw(pubRaw) });
	return crypto.createHash('sha256').update(s).digest();
}

/** Stand-in for Argon2id(password, salt) -> 32 bytes. */
function deriveKey(password, salt) {
	return crypto.scryptSync(Buffer.from(password, 'utf8'), salt, 32, { N: 16384, r: 8, p: 1 });
}

function seal(plaintext, key) {
	const nonce = crypto.randomBytes(12);
	const c = crypto.createCipheriv('chacha20-poly1305', key, nonce, { authTagLength: 16 });
	const body = Buffer.concat([c.update(plaintext), c.final()]);
	return Buffer.concat([nonce, body, c.getAuthTag()]);
}

function open(blob, key) {
	const nonce = blob.subarray(0, 12);
	const tag = blob.subarray(blob.length - 16);
	const body = blob.subarray(12, blob.length - 16);
	const d = crypto.createDecipheriv('chacha20-poly1305', key, nonce, { authTagLength: 16 });
	d.setAuthTag(tag);
	return Buffer.concat([d.update(body), d.final()]);
}

/** Wrap a 32-byte secret under a password (or a recovery code). */
function wrapSecret(secret, password) {
	const salt = crypto.randomBytes(16);
	return { bundle: seal(secret, deriveKey(password, salt)), salt };
}

function unwrapSecret(bundleB64, saltB64, password) {
	return open(b(bundleB64), deriveKey(password, b(saltB64)));
}

/** The verifier: an Argon2id output in the real client, any stable 32 bytes here. */
function verifier(password, authSaltB64) {
	return deriveKey(password, b(authSaltB64)).toString('base64');
}

const b64 = (buf) => Buffer.from(buf).toString('base64');
const b = (s) => Buffer.from(s, 'base64');

// --- http -------------------------------------------------------------------

async function req(method, path, { body, token, xff } = {}) {
	const headers = {};
	if (body !== undefined) headers['Content-Type'] = 'application/json';
	if (token) headers['Authorization'] = `Bearer ${token}`;
	if (xff) headers['X-Forwarded-For'] = xff;
	const res = await fetch(`${BASE}${path}`, {
		method,
		headers,
		body: body === undefined ? undefined : JSON.stringify(body),
	});
	const text = await res.text();
	let json = null;
	try { json = JSON.parse(text); } catch { /* not json */ }
	return { status: res.status, json, text };
}

// --- account helper ---------------------------------------------------------

let seq = 0;
function freshEmail(tag) {
	seq++;
	return `a2b-${tag}-${Date.now()}-${seq}@example.test`;
}

async function signup(tag, password) {
	const email = freshEmail(tag);
	const keys = newX25519();
	const authSalt = crypto.randomBytes(16);
	const wrapped = wrapSecret(keys.sec, password);
	const recoveryCode = Array.from({ length: 12 }, () => crypto.randomBytes(4).toString('hex')).join(' ');
	const recoveryWrapped = wrapSecret(keys.sec, recoveryCode);

	const res = await req('POST', '/api/auth/signup', {
		body: {
			email,
			display_name: `A2b ${tag}`,
			verifier: verifier(password, b64(authSalt)),
			auth_salt: b64(authSalt),
			password_length: password.length,
			encryption_public_key: b64(keys.pub),
			encrypted_key_bundle: b64(wrapped.bundle),
			bundle_salt: b64(wrapped.salt),
			encrypted_recovery_bundle: b64(recoveryWrapped.bundle),
			recovery_bundle_salt: b64(recoveryWrapped.salt),
		},
	});
	return { email, keys, password, recoveryCode, authSalt: b64(authSalt), res };
}

/**
 * `GET /auth/salt` shares the SignIn rate-limit bucket, so every sign-in that has to look the salt
 * up costs two units rather than one. Where the harness already knows the salt it passes it in;
 * the lookup path is still exercised, just not fourteen times over.
 */
async function signin(email, password, deviceLabel, knownSalt) {
	let authSalt = knownSalt;
	if (!authSalt) {
		const salt = await req('GET', `/api/auth/salt?email=${encodeURIComponent(email)}`);
		authSalt = salt.json.auth_salt;
	}
	const res = await req('POST', '/api/auth/signin', {
		body: {
			email,
			verifier: verifier(password, authSalt),
			device_label: deviceLabel ?? null,
		},
	});
	return res;
}

/** Mint a password-reset token the way the server would, since no SMTP is reachable here. */
function mintResetToken(userId, minutes = 30) {
	const raw = crypto.randomBytes(32).toString('base64url');
	const hash = crypto.createHash('sha256').update(raw).digest('hex');
	sql(`INSERT INTO one_time_tokens (id, user_id, kind, token_hash, expires_at)
	     VALUES (gen_random_uuid(), '${userId}', 'password_reset', decode('${hash}','hex'),
	             now() + interval '${minutes} minutes')`);
	return raw;
}

// --- phases -----------------------------------------------------------------

async function phaseRate() {
	console.log('\n[rate] sign-in rate limiting and X-Forwarded-For handling');

	const password = 'correct horse battery staple';
	const acct = await signup('rate', password);
	ok('signup succeeds', acct.res.status === 200, `status ${acct.res.status}`);

	// Deliberately no `GET /auth/salt` here: it draws on the same bucket, and one extra unit would
	// move the observed threshold by one and make the number below impossible to read.
	const bad = { email: acct.email, verifier: verifier('wrong password', acct.authSalt) };

	// Every attempt carries a DIFFERENT spoofed X-Forwarded-For. The peer (127.0.0.1) is not in
	// trusted_proxies, so the header must be ignored and all of them must share one bucket.
	const statuses = [];
	for (let i = 0; i < 14; i++) {
		const r = await req('POST', '/api/auth/signin', { body: bad, xff: `203.0.113.${i + 1}` });
		statuses.push(r.status);
	}
	console.log(`        statuses: ${statuses.join(' ')}`);

	const first429 = statuses.indexOf(429);
	ok('rapid sign-ins from one IP are eventually refused with 429', first429 !== -1,
		first429 === -1 ? 'never saw 429' : `first 429 at attempt ${first429 + 1}`);
	ok('a spoofed X-Forwarded-For from an untrusted peer does not open a new bucket',
		first429 !== -1 && statuses.slice(first429).every((s) => s === 429),
		`attempts ${first429 + 1}..14 all 429 despite 14 distinct forwarded addresses`);
	console.log(`        NOTE: RouteClass::SignIn quota is 10 per 300s, so the first refusal lands on`);
	console.log(`        attempt 11, not attempt 6. The card says "5 rapid logins -> 429".`);
}

async function phaseCore() {
	console.log('\n[core] key continuity across change-password, and recovery-code reissue');

	const password = 'correct horse battery staple';
	const acct = await signup('core', password);
	ok('signup succeeds', acct.res.status === 200, `status ${acct.res.status}`);
	const userId = acct.res.json.user_id;

	// A message sent to this account before anything changes.
	const peer = newX25519();
	const preResetCipher = seal(Buffer.from('a message sent before the change'),
		sharedKey(peer.sec, acct.keys.pub));

	ok('signup response does not contain the recovery code',
		!acct.res.text.includes(acct.recoveryCode) && !acct.res.text.includes('recovery_code'),
		'checked the raw body');

	// --- change password ---
	const newPassword = 'a different long password';
	const rewrapped = wrapSecret(acct.keys.sec, newPassword);
	const newAuthSalt = crypto.randomBytes(16);
	const sess1 = await signin(acct.email, password, 'device-one');
	const sess2 = await signin(acct.email, password, 'device-two', acct.authSalt);
	ok('two more sessions established', sess1.status === 200 && sess2.status === 200);
	// Signup signs you in, so the account is on three sessions here, not two.
	ok('signup itself issued a session',
		sql(`SELECT COUNT(*) FROM sessions WHERE user_id='${userId}' AND revoked_at IS NULL`) === '3');
	ok('GET /auth/salt returns the salt the account signed up with',
		(await req('GET', `/api/auth/salt?email=${encodeURIComponent(acct.email)}`)).json?.auth_salt
			=== acct.authSalt);

	const change = await req('POST', '/api/auth/password/change', {
		token: sess1.json.token,
		body: {
			current_verifier: verifier(password, acct.authSalt),
			verifier: verifier(newPassword, b64(newAuthSalt)),
			auth_salt: b64(newAuthSalt),
			password_length: newPassword.length,
			encrypted_key_bundle: b64(rewrapped.bundle),
			bundle_salt: b64(rewrapped.salt),
		},
	});
	ok('POST /auth/password/change succeeds', change.status === 200, `status ${change.status} ${change.text}`);
	ok('change-password revokes every other session and only those',
		change.json?.sessions_revoked === 2, `sessions_revoked=${change.json?.sessions_revoked} of 3`);
	const stale = await req('GET', '/api/auth/me', { token: sess2.json.token });
	ok('the other device is actually signed out', stale.status === 401, `status ${stale.status}`);
	const current = await req('GET', '/api/auth/me', { token: sess1.json.token });
	ok('the device that changed the password stays signed in', current.status === 200);

	// A wrong current password must not be enough.
	const badChange = await req('POST', '/api/auth/password/change', {
		token: sess1.json.token,
		body: {
			current_verifier: verifier('not the password', acct.authSalt),
			verifier: verifier('yet another password', b64(newAuthSalt)),
			auth_salt: b64(newAuthSalt),
			password_length: 20,
			encrypted_key_bundle: b64(rewrapped.bundle),
			bundle_salt: b64(rewrapped.salt),
		},
	});
	ok('a session token alone cannot change the password', badChange.status === 401,
		`status ${badChange.status}`);

	// --- the key survived ---
	const after = await signin(acct.email, newPassword, 'after-change', b64(newAuthSalt));
	ok('sign-in with the new password works', after.status === 200, `status ${after.status}`);
	const recovered = unwrapSecret(after.json.encrypted_key_bundle, after.json.bundle_salt, newPassword);
	ok('change-password keeps the same x25519 secret',
		recovered.equals(acct.keys.sec));
	const keys = await req('GET', `/api/auth/users/${userId}/keys`, { token: after.json.token });
	ok('the published public key is unchanged',
		keys.json?.encryption_public_key === b64(acct.keys.pub));
	const stillReadable = open(preResetCipher, sharedKey(recovered, peer.pub));
	ok('a message sent before the change is still readable',
		stillReadable.toString() === 'a message sent before the change');

	// A2b.4
	ok('GET /auth/users/{id}/keys no longer returns public_key',
		!('public_key' in (keys.json ?? {})), `fields: ${Object.keys(keys.json ?? {}).join(', ')}`);

	// --- recovery code reissue ---
	const newCode = Array.from({ length: 12 }, () => crypto.randomBytes(4).toString('hex')).join(' ');
	const newWrapped = wrapSecret(acct.keys.sec, newCode);
	const reissue = await req('POST', '/api/auth/recovery/reissue', {
		token: after.json.token,
		body: {
			current_verifier: verifier(newPassword, b64(newAuthSalt)),
			encrypted_recovery_bundle: b64(newWrapped.bundle),
			recovery_bundle_salt: b64(newWrapped.salt),
		},
	});
	ok('POST /auth/recovery/reissue succeeds', reissue.status === 200, `status ${reissue.status} ${reissue.text}`);
	ok('the reissue response contains no recovery code',
		!reissue.text.includes(newCode) && !reissue.text.includes('recovery'),
		`body: ${reissue.text}`);

	const badReissue = await req('POST', '/api/auth/recovery/reissue', {
		token: after.json.token,
		body: {
			current_verifier: verifier('wrong', b64(newAuthSalt)),
			encrypted_recovery_bundle: b64(newWrapped.bundle),
			recovery_bundle_salt: b64(newWrapped.salt),
		},
	});
	ok('a session token alone cannot overwrite the recovery bundle', badReissue.status === 401,
		`status ${badReissue.status}`);

	// The stored bundle is now the new one, and only the new code opens it.
	const stored = sql(`SELECT encode(encrypted_recovery_bundle,'base64') || '|' ||
	                           encode(recovery_bundle_salt,'base64')
	                    FROM users WHERE id = '${userId}'`).replace(/\s+/g, '');
	const [sb, ss] = stored.split('|');
	let oldWorks = true;
	try { unwrapSecret(sb, ss, acct.recoveryCode); } catch { oldWorks = false; }
	ok('reissuing invalidates the previous recovery code', !oldWorks);
	ok('the new recovery code opens the stored bundle',
		unwrapSecret(sb, ss, newCode).equals(acct.keys.sec));

	// --- the code is never handed back ---
	const me = await req('GET', '/api/auth/me', { token: after.json.token });
	const sessions = await req('GET', '/api/auth/sessions', { token: after.json.token });
	const haystack = [after.text, me.text, sessions.text, keys.text].join('\n');
	ok('no endpoint returns a recovery code after signup',
		!haystack.includes(newCode) && !haystack.includes(acct.recoveryCode)
			&& !/recovery_code|recovery_bundle/.test(haystack),
		'checked signin, /me, /sessions and /users/{id}/keys');

	console.log(`        userId=${userId}`);
}

async function phaseResetWithCode() {
	console.log('\n[reset-with-code] recovery code preserves the pre-reset key');

	const password = 'correct horse battery staple';
	const acct = await signup('rwc', password);
	const userId = acct.res.json.user_id;
	ok('signup succeeds', acct.res.status === 200, `status ${acct.res.status}`);

	const peer = newX25519();
	const preResetCipher = seal(Buffer.from('sent before the reset'), sharedKey(peer.sec, acct.keys.pub));

	// forgot-password for an address nobody has: 200, and nothing is written.
	const tokensBefore = Number(sql(`SELECT COUNT(*) FROM one_time_tokens`));
	const unknown = await req('POST', '/api/auth/password-reset', {
		body: { email: `nobody-${Date.now()}@example.test` },
	});
	const tokensAfter = Number(sql(`SELECT COUNT(*) FROM one_time_tokens`));
	ok('forgot-password for an unknown address answers 200', unknown.status === 200,
		`status ${unknown.status}`);
	ok('forgot-password for an unknown address sends and stores nothing',
		tokensAfter === tokensBefore, `one_time_tokens ${tokensBefore} -> ${tokensAfter}`);
	ok('the response gives no hint that the address is unknown',
		/if that address has an account/i.test(unknown.text), unknown.text);

	// A live session that must not survive the reset.
	const before = await signin(acct.email, password, 'pre-reset', acct.authSalt);
	ok('session established before the reset', before.status === 200);

	const token = mintResetToken(userId);
	const bundleRes = await req('GET', `/api/auth/password-reset/bundle?token=${encodeURIComponent(token)}`);
	ok('the reset flow can fetch the recovery bundle', bundleRes.status === 200,
		`status ${bundleRes.status}`);

	const secret = unwrapSecret(
		bundleRes.json.encrypted_recovery_bundle,
		bundleRes.json.recovery_bundle_salt,
		acct.recoveryCode
	);
	ok('the recovery code opens it', secret.equals(acct.keys.sec));

	const newPassword = 'a brand new long password';
	const rewrapped = wrapSecret(secret, newPassword);
	const authSalt = crypto.randomBytes(16);
	const confirm = await req('POST', '/api/auth/password-reset/confirm', {
		body: {
			token,
			verifier: verifier(newPassword, b64(authSalt)),
			auth_salt: b64(authSalt),
			password_length: newPassword.length,
			encrypted_key_bundle: b64(rewrapped.bundle),
			bundle_salt: b64(rewrapped.salt),
		},
	});
	ok('the reset is accepted', confirm.status === 200, `status ${confirm.status} ${confirm.text}`);

	const stale = await req('GET', '/api/auth/me', { token: before.json.token });
	ok('every session is gone after a password reset', stale.status === 401,
		`status ${stale.status}, sessions_revoked=${confirm.json?.sessions_revoked}`);
	ok('no session rows survive for the account',
		sql(`SELECT COUNT(*) FROM sessions WHERE user_id = '${userId}' AND revoked_at IS NULL`) === '0');

	const after = await signin(acct.email, newPassword, 'post-reset', b64(authSalt));
	ok('sign-in with the new password works', after.status === 200, `status ${after.status}`);
	const recovered = unwrapSecret(after.json.encrypted_key_bundle, after.json.bundle_salt, newPassword);
	ok('the account still holds its original x25519 secret', recovered.equals(acct.keys.sec));

	const keys = await req('GET', `/api/auth/users/${userId}/keys`, { token: after.json.token });
	ok('the published public key is unchanged',
		keys.json?.encryption_public_key === b64(acct.keys.pub));

	const readable = open(preResetCipher, sharedKey(recovered, peer.pub));
	ok('reset WITH the recovery code makes a pre-reset message readable',
		readable.toString() === 'sent before the reset');
}

async function phaseResetWithoutCode() {
	console.log('\n[reset-without-code] no code means the old key is gone for good');

	const password = 'correct horse battery staple';
	const acct = await signup('rwoc', password);
	const userId = acct.res.json.user_id;
	ok('signup succeeds', acct.res.status === 200, `status ${acct.res.status}`);

	const peer = newX25519();
	const preResetCipher = seal(Buffer.from('sent before the reset'), sharedKey(peer.sec, acct.keys.pub));

	const token = mintResetToken(userId);
	const newPassword = 'a brand new long password';
	const fresh = newX25519();
	const wrapped = wrapSecret(fresh.sec, newPassword);
	const newCode = Array.from({ length: 12 }, () => crypto.randomBytes(4).toString('hex')).join(' ');
	const recoveryWrapped = wrapSecret(fresh.sec, newCode);
	const authSalt = crypto.randomBytes(16);

	// A new public key without a matching recovery bundle must be refused outright.
	const partial = await req('POST', '/api/auth/password-reset/confirm', {
		body: {
			token,
			verifier: verifier(newPassword, b64(authSalt)),
			auth_salt: b64(authSalt),
			password_length: newPassword.length,
			encryption_public_key: b64(fresh.pub),
			encrypted_key_bundle: b64(wrapped.bundle),
			bundle_salt: b64(wrapped.salt),
		},
	});
	ok('rotating the identity key without a new recovery bundle is refused',
		partial.status === 400, `status ${partial.status} ${partial.text}`);
	ok('the refusal did not consume the token',
		sql(`SELECT used_at IS NULL FROM one_time_tokens WHERE user_id='${userId}' AND kind='password_reset'`) === 't');

	const confirm = await req('POST', '/api/auth/password-reset/confirm', {
		body: {
			token,
			verifier: verifier(newPassword, b64(authSalt)),
			auth_salt: b64(authSalt),
			password_length: newPassword.length,
			encryption_public_key: b64(fresh.pub),
			encrypted_key_bundle: b64(wrapped.bundle),
			bundle_salt: b64(wrapped.salt),
			encrypted_recovery_bundle: b64(recoveryWrapped.bundle),
			recovery_bundle_salt: b64(recoveryWrapped.salt),
		},
	});
	ok('the reset is accepted', confirm.status === 200, `status ${confirm.status} ${confirm.text}`);

	const after = await signin(acct.email, newPassword, 'post-reset', b64(authSalt));
	ok('sign-in with the new password works', after.status === 200, `status ${after.status}`);
	const recovered = unwrapSecret(after.json.encrypted_key_bundle, after.json.bundle_salt, newPassword);
	ok('the account now holds a different x25519 secret', !recovered.equals(acct.keys.sec));

	const keys = await req('GET', `/api/auth/users/${userId}/keys`, { token: after.json.token });
	ok('the published public key was rotated',
		keys.json?.encryption_public_key === b64(fresh.pub)
			&& keys.json?.encryption_public_key !== b64(acct.keys.pub));

	let readable = true;
	try {
		open(preResetCipher, sharedKey(recovered, peer.pub));
	} catch {
		readable = false;
	}
	ok('reset WITHOUT the recovery code leaves a pre-reset message unreadable', !readable);
}

/** Build a reset-confirm body for an account whose secret we already hold. */
function resetBody(acct, token, pw) {
	const salt = crypto.randomBytes(16);
	const wrapped = wrapSecret(acct.keys.sec, pw);
	return {
		body: {
			token,
			verifier: verifier(pw, b64(salt)),
			auth_salt: b64(salt),
			password_length: pw.length,
			encrypted_key_bundle: b64(wrapped.bundle),
			bundle_salt: b64(wrapped.salt),
		},
		authSalt: b64(salt),
	};
}

async function phaseTokenReuse() {
	console.log('\n[token-reuse] a reset token is good exactly once');

	const password = 'correct horse battery staple';
	const acct = await signup('reuse', password);
	const userId = acct.res.json.user_id;
	ok('signup succeeds', acct.res.status === 200, `status ${acct.res.status}`);

	const token = mintResetToken(userId);
	const one = resetBody(acct, token, 'first new password');
	const first = await req('POST', '/api/auth/password-reset/confirm', { body: one.body });
	ok('the first use of the token succeeds', first.status === 200, `status ${first.status}`);

	const two = resetBody(acct, token, 'second new password');
	const second = await req('POST', '/api/auth/password-reset/confirm', { body: two.body });
	ok('the same token cannot be used twice', second.status === 400, `status ${second.status} ${second.text}`);

	const peek = await req('GET', `/api/auth/password-reset/bundle?token=${encodeURIComponent(token)}`);
	ok('a used token cannot fetch the recovery bundle either', peek.status === 400,
		`status ${peek.status}`);

	const signedIn = await signin(acct.email, 'first new password', null, one.authSalt);
	ok('the password is the one from the first, successful reset', signedIn.status === 200,
		`status ${signedIn.status}`);
	const rejected = await signin(acct.email, 'second new password', null, two.authSalt);
	ok('the refused second reset changed nothing', rejected.status === 401,
		`status ${rejected.status}`);
}

async function phaseTokenExpired() {
	// Its own phase because RouteClass::PasswordReset allows 3 per hour and the reuse phase
	// already spends all three.
	console.log('\n[token-expired] an expired reset token is refused');

	const password = 'correct horse battery staple';
	const acct = await signup('expired', password);
	const userId = acct.res.json.user_id;
	ok('signup succeeds', acct.res.status === 200, `status ${acct.res.status}`);

	const expired = mintResetToken(userId, -1);
	const onExpired = await req('POST', '/api/auth/password-reset/confirm', {
		body: resetBody(acct, expired, 'a password that must not stick').body,
	});
	ok('an expired token is refused', onExpired.status === 400,
		`status ${onExpired.status} ${onExpired.text}`);
	ok('the expired token was not consumed either',
		sql(`SELECT used_at IS NULL FROM one_time_tokens WHERE user_id='${userId}'
		     AND kind='password_reset'`) === 't');

	const still = await signin(acct.email, password, null, acct.authSalt);
	ok('the original password still works', still.status === 200, `status ${still.status}`);
}

async function phaseAdmin() {
	console.log('\n[admin] session visibility and audited role changes');

	const password = 'correct horse battery staple';
	const boss = await signup('admin', password);
	const victim = await signup('victim', password);
	ok('both accounts created', boss.res.status === 200 && victim.res.status === 200);

	sql(`UPDATE users SET role = 'superadmin' WHERE id = '${boss.res.json.user_id}'`);

	const bossSession = await signin(boss.email, password, 'boss', boss.authSalt);
	ok('superadmin signs in', bossSession.status === 200);
	const token = bossSession.json.token;

	await signin(victim.email, password, 'victim-phone', victim.authSalt);
	await signin(victim.email, password, 'victim-laptop', victim.authSalt);
	const victimId = victim.res.json.user_id;

	// Three, not two: signup signed the account in as well.
	const list = await req('GET', `/api/admin/users/${victimId}/sessions`, { token });
	ok('GET /admin/users/{id}/sessions lists them all', list.status === 200 && list.json?.length === 3,
		`status ${list.status}, ${list.json?.length} session(s)`);
	ok('the device labels come through',
		list.json?.some((s) => s.device_label === 'victim-phone')
			&& list.json?.some((s) => s.device_label === 'victim-laptop'),
		(list.json ?? []).map((s) => s.device_label).join(', '));
	ok('the listing carries no token material',
		!/token|hash/i.test(list.text), `fields: ${Object.keys(list.json?.[0] ?? {}).join(', ')}`);

	const revoke = await req('DELETE', `/api/admin/users/${victimId}/sessions`, { token });
	ok('DELETE /admin/users/{id}/sessions revokes them', revoke.status === 200 && revoke.json?.revoked === 3,
		`status ${revoke.status}, revoked=${revoke.json?.revoked}`);
	ok('no live sessions remain',
		sql(`SELECT COUNT(*) FROM sessions WHERE user_id='${victimId}' AND revoked_at IS NULL`) === '0');

	const promote = await req('PATCH', `/api/admin/users/${victimId}/role`, {
		token, body: { role: 'admin' },
	});
	ok('role promotion succeeds', promote.status === 200 && promote.json?.role === 'admin',
		`status ${promote.status} ${promote.text}`);
	ok('it reports the previous role', promote.json?.previous_role === 'user',
		`previous_role=${promote.json?.previous_role}`);

	const demote = await req('PATCH', `/api/admin/users/${victimId}/role`, {
		token, body: { role: 'user' },
	});
	ok('demotion succeeds', demote.status === 200 && demote.json?.previous_role === 'admin',
		`status ${demote.status} ${demote.text}`);

	const self = await req('PATCH', `/api/admin/users/${boss.res.json.user_id}/role`, {
		token, body: { role: 'user' },
	});
	ok('changing your own role is a 400, not a 200 with an error key', self.status === 400,
		`status ${self.status} ${self.text}`);

	const nonsense = await req('PATCH', `/api/admin/users/${victimId}/role`, {
		token, body: { role: 'god' },
	});
	ok('an invalid role is a 400', nonsense.status === 400, `status ${nonsense.status}`);

	const audits = sql(`SELECT action || ':' || COALESCE(detail::text,'') FROM audit_events
	                    WHERE subject_id = '${victimId}' ORDER BY created_at`);
	ok('every role change wrote an audit row',
		(audits.match(/admin\.role_change/g) || []).length === 2, audits.replace(/\n/g, ' | '));
	ok('the revocation was audited too', /admin\.revoke_sessions/.test(audits));

	// A plain user must not reach any of it.
	const victimSession = await signin(victim.email, password, 'victim-again', victim.authSalt);
	const forbidden = await req('GET', `/api/admin/users/${victimId}/sessions`, {
		token: victimSession.json.token,
	});
	ok('a non-superadmin is refused', forbidden.status === 403 || forbidden.status === 401,
		`status ${forbidden.status}`);
}

// --- main -------------------------------------------------------------------

const phases = {
	rate: phaseRate,
	core: phaseCore,
	'reset-with-code': phaseResetWithCode,
	'reset-without-code': phaseResetWithoutCode,
	'token-reuse': phaseTokenReuse,
	'token-expired': phaseTokenExpired,
	admin: phaseAdmin,
};

const name = process.argv[2];
const run = phases[name];
if (!run) {
	console.error(`unknown phase ${name}; expected one of ${Object.keys(phases).join(', ')}`);
	process.exit(2);
}

await run();
console.log(`\n[${name}] ${checks - failures}/${checks} passed`);
process.exit(failures === 0 ? 0 : 1);
