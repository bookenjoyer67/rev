# Hub request — A2b (Agent A): three frontend files outside my write set

A2b.3 replaces the whole client auth surface. Three files I do **not** own still carry the old
one. I have not touched them. Each diff below is exact against the tree as of this card.

**Why this is not optional polish:** the A2b frontend gate is

```
grep -rn "passphrase" web/src | grep -v test      # expect empty
```

and it **cannot reach empty** without these three edits. The 17 remaining hits are all in files
listed below (13 in `Onboarding.svelte`, 4 in `connect/+page.svelte`). Everything in my own write
set is already clean.

I kept `register`, `recover`, `showOnboarding` and `onAuthComplete` exported from
`web/src/lib/stores/auth.ts` as documented shims precisely so these three files keep compiling
until the diffs land — nothing here is load-bearing, and nothing here breaks in the meantime. The
shims navigate to the new pages instead of posting to `/auth/register` and `/auth/recover`, which
no longer exist on the server.

---

## 1. DELETION REQUEST — `web/src/lib/components/Onboarding.svelte`

Delete the file. `rm` is denied in my sandbox.

It is the passphrase-prompt modal: display name + optional "recovery passphrase", posting to
`/api/auth/register`. That endpoint is gone, the second secret is gone, and signup now needs an
email address and a password, neither of which this modal collects. Its replacement is
`web/src/routes/account/signup/+page.svelte`.

Its only importer is `web/src/routes/+layout.svelte` (diff 2). Its only other caller,
`RespondModal.svelte`, imports `register` from the store, not the component — that call now
redirects to `/account/signup`, which is the correct behaviour for an anonymous user trying to
respond to a post. `RespondModal.svelte` belongs to A6 and is left broken-but-unchanged as the
card instructs; the shim keeps it compiling and gives it sane runtime behaviour in the meantime.

## 2. `web/src/routes/+layout.svelte` — drop the modal and point "Join" at the signup page

```diff
@@ -2,7 +2,6 @@
 	import '../app.css';
 	import '$lib/design/tokens.css';
 	import { onMount } from 'svelte';
-	import Onboarding from '$lib/components/Onboarding.svelte';
 	import SearchBar from '$lib/components/SearchBar.svelte';
-	import { auth, isAuthenticated, getToken, refreshRole, initAuth, showOnboarding } from '$lib/stores/auth';
+	import { auth, isAuthenticated, getToken, refreshRole, initAuth } from '$lib/stores/auth';
 	import { serverState, getActiveServer } from '$lib/stores/server';
 	import { initTheme } from '$lib/stores/theme';
@@ -116,7 +115,7 @@
 			<a href="/connect" onclick={closeMenu}>Connect</a>
 			{#if $serverState.active && !$auth.servers?.[$serverState.active]}
-				<button class="join-link" onclick={() => { closeMenu(); showOnboarding.set(true); }}>Join</button>
+				<a href="/account/signup" class="join-link" onclick={closeMenu}>Join</a>
 			{/if}
 			{#if $serverState.active && $auth.servers?.[$serverState.active]}
 				<a href="/account" class="identity" onclick={closeMenu}>{$auth.servers[$serverState.active].displayName}</a>
@@ -143,8 +142,6 @@
 	</div>
 {/if}
 
-<Onboarding />
-
 <style>
```

`.join-link` was styled on a `<button>`; as an `<a>` it will want `text-decoration: none;` and
`display: inline-block;` added to its rule, or it picks up the nav link underline. I have not
written that into the diff because I cannot see the rendered result to check it.

Leaving `showOnboarding` imported but unused would be a new `svelte-check` warning in a file I did
not touch, which is why the import goes in the same diff.

## 3. `web/src/routes/connect/+page.svelte` — remove the recover panel

`/connect` is about choosing a server. Recovery is now the password-reset flow, which starts from
`/account/forgot` and needs an email address rather than a server URL and a second secret.

```diff
@@ -2,7 +2,6 @@
 	import { goto } from '$app/navigation';
 	import { onMount } from 'svelte';
 	import { connectToServer, serverState, type NodeInfo } from '$lib/stores/server';
-	import { recover } from '$lib/stores/auth';
 	import { discoverAllServers, type NearbyServer } from '$lib/api/discovery';
 
 	let url = $state('');
@@ -12,8 +11,6 @@
 	let browseServers: NearbyServer[] = $state([]);
 	let browseLoading = $state(false);
 
-	let showRecover = $state(false);
-
 	onMount(async () => {
 		browseLoading = true;
 		try {
@@ -21,29 +18,6 @@
 		} catch { browseServers = []; }
 		browseLoading = false;
 	});
-	let recoverUrl = $state('');
-	let recoverPassphrase = $state('');
-	let recoverCode = $state('');
-	let recoverError = $state('');
-	let recoverLoading = $state(false);
-	let recoverSuccess = $state(false);
-
-	async function handleRecover() {
-		if (!recoverUrl.trim()) { recoverError = 'Enter a server URL'; return; }
-		if (!recoverPassphrase.trim()) { recoverError = 'Enter your passphrase'; return; }
-		recoverLoading = true;
-		recoverError = '';
-		const ok = await recover(recoverUrl.trim().replace(/\/+$/, ''), recoverPassphrase, recoverCode.trim() || undefined);
-		recoverLoading = false;
-		if (ok) {
-			recoverSuccess = true;
-			try { await connectToServer(recoverUrl.trim()); } catch {}
-			setTimeout(() => goto('/'), 1000);
-		} else {
-			recoverError = 'Recovery failed. Check your passphrase and server URL.';
-		}
-	}
 
 	async function handleConnect() {
 		if (!url.trim()) { error = 'Enter a server URL'; return; }
```

Markup — replace the whole `.recover-section` block:

```diff
@@ -143,26 +117,10 @@
 		<div class="recover-section">
-			{#if !showRecover}
-				<button class="recover-toggle" onclick={() => showRecover = true}>
-					Recover an existing identity
-				</button>
-			{:else if recoverSuccess}
-				<p class="recover-success">Identity recovered! Redirecting...</p>
-			{:else}
-				<h3>Recover identity</h3>
-				<p class="recover-hint">Enter a server you've used before and your recovery passphrase.</p>
-				<form onsubmit={(e) => { e.preventDefault(); handleRecover(); }}>
-					<input type="url" bind:value={recoverUrl} placeholder="https://server-url" disabled={recoverLoading} />
-					<input type="password" bind:value={recoverPassphrase} placeholder="Your recovery passphrase" disabled={recoverLoading} />
-					<input type="text" bind:value={recoverCode} placeholder="Recovery code (12 words, if set)" disabled={recoverLoading} />
-					{#if recoverError}
-						<p class="error">{recoverError}</p>
-					{/if}
-					<button type="submit" class="recover-btn" disabled={recoverLoading}>
-						{recoverLoading ? 'Recovering...' : 'Recover'}
-					</button>
-				</form>
-			{/if}
+			<a class="recover-toggle" href="/account/login">Already have an account? Sign in</a>
 		</div>
```

CSS — the rules for the removed elements have to go too, or `svelte-check` reports each as an
unused selector and the "no new diagnostic" criterion fails on a file nobody edited on purpose:

```diff
@@ -300,36 +258,6 @@
-	.recover-section h3 {
-		font-size: 1rem;
-		margin-bottom: 0.25rem;
-	}
-
-	.recover-hint {
-		font-size: 0.8rem;
-		color: var(--text-muted);
-		margin-bottom: 0.75rem;
-	}
-
-	.recover-section form {
-		display: flex;
-		flex-direction: column;
-		gap: 0.5rem;
-	}
-
-	.recover-section input {
-		background: var(--bg-surface);
-		border: 1px solid var(--border);
-		border-radius: var(--radius);
-		padding: 0.6rem 0.8rem;
-		color: var(--text);
-		font-size: 0.9rem;
-	}
-
-	.recover-btn {
-		background: var(--bg-elevated);
-		color: var(--text);
-		border: 1px solid var(--border);
-		padding: 0.6rem;
-		border-radius: var(--radius);
-		font-weight: 600;
-	}
-
-	.recover-success {
-		color: var(--success);
-		font-weight: 600;
-	}
-
```

Keep `.recover-section` and `.recover-toggle` — both are still used by the replacement link.
Keep `.error`; it is still used at line 109 for the connect error.

## 4. After the diffs land, `web/src/lib/stores/auth.ts` can lose its shims

Once nothing imports them, these four exports should be deleted from `web/src/lib/stores/auth.ts`
(a file I do own — I will remove them in a later card rather than breaking the three files above
in this one):

- `register()` — the deprecated redirect shim
- `recover()` — the deprecated redirect shim
- `showOnboarding` — the store that nothing sets any more
- `onAuthComplete()` — only ever called by `Onboarding.svelte`

`requireAuth()` stays; it now routes to `/account/login` instead of opening the modal.
