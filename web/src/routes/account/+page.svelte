<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { auth, getActiveAuth, getPublicKey, isAuthenticated, setPassphrase, updateDisplayName, logout } from '$lib/stores/auth';
	import { isConnected, serverState, disconnectServer, removeServer } from '$lib/stores/server';
	import { themeName } from '$lib/stores/theme';
	import ThemePicker from '$lib/components/ThemePicker.svelte';

	let displayName = $state('');
	let passphrase = $state('');
	let passphraseConfirm = $state('');
	let saving = $state(false);
	let saved = $state(false);
	let passphraseSaving = $state(false);
	let passphraseSaved = $state(false);
	let error = $state('');
	let passphraseError = $state('');
	let copied = $state(false);
	let showTheme = $state(false);

	let publicKey = $derived(getPublicKey() || '');
	let hasBundle = $derived(!!$auth.keypair?.ed25519SecretKey);

	onMount(() => {
		if (!isConnected() || !isAuthenticated()) { goto('/'); return; }
		displayName = getActiveAuth()?.displayName || '';
	});

	async function saveName() {
		if (!displayName.trim()) { error = 'Name is required'; return; }
		saving = true;
		error = '';
		const ok = await updateDisplayName(displayName.trim());
		saving = false;
		if (ok) { saved = true; setTimeout(() => saved = false, 2000); }
		else { error = 'Failed to update name'; }
	}

	async function savePassphrase() {
		if (!passphrase) { passphraseError = 'Enter a passphrase'; return; }
		if (passphrase !== passphraseConfirm) { passphraseError = 'Passphrases do not match'; return; }
		if (passphrase.length < 6) { passphraseError = 'Passphrase must be at least 6 characters'; return; }
		passphraseSaving = true;
		passphraseError = '';
		const ok = await setPassphrase(passphrase);
		passphraseSaving = false;
		if (ok) {
			passphraseSaved = true;
			passphrase = '';
			passphraseConfirm = '';
			setTimeout(() => passphraseSaved = false, 3000);
		} else {
			passphraseError = 'Failed to save passphrase';
		}
	}

	function copyPublicKey() {
		navigator.clipboard.writeText(publicKey);
		copied = true;
		setTimeout(() => copied = false, 2000);
	}

	function handleLogout() {
		logout();
		goto('/');
	}
</script>

<div class="container">
	<h1>Account</h1>

	<section class="section">
		<h2>Display Name</h2>
		<form onsubmit={(e) => { e.preventDefault(); saveName(); }}>
			<input type="text" bind:value={displayName} maxlength="50" />
			{#if error}<p class="error">{error}</p>{/if}
			<button type="submit" class="save-btn" disabled={saving}>
				{saving ? 'Saving...' : saved ? 'Saved!' : 'Update Name'}
			</button>
		</form>
	</section>

	<section class="section">
		<h2>Your Identity</h2>
		<p class="hint">This is your public key. It identifies you across all Komun servers.</p>
		<div class="key-display">
			<code>{publicKey.slice(0, 20)}...{publicKey.slice(-8)}</code>
			<button class="copy-btn" onclick={copyPublicKey}>{copied ? 'Copied!' : 'Copy'}</button>
		</div>
	</section>

	<section class="section">
		<h2>Recovery Passphrase</h2>
		{#if passphraseSaved}
			<p class="success">Passphrase saved! You can now recover your identity from any device.</p>
		{:else}
			<p class="hint">Set a passphrase to recover your identity on a new device. Without it, losing this device means losing access to your encrypted messages.</p>
			<form onsubmit={(e) => { e.preventDefault(); savePassphrase(); }}>
				<input type="password" bind:value={passphrase} placeholder="Recovery passphrase" />
				<input type="password" bind:value={passphraseConfirm} placeholder="Confirm passphrase" />
				{#if passphraseError}<p class="error">{passphraseError}</p>{/if}
				<button type="submit" class="save-btn" disabled={passphraseSaving}>
					{passphraseSaving ? 'Encrypting...' : 'Set Passphrase'}
				</button>
			</form>
		{/if}
	</section>

	<section class="section">
		<h2>Theme</h2>
		<div class="theme-pick-row">
			<span class="current-theme">{$themeName}</span>
			<button class="change-btn" onclick={() => showTheme = true}>Change</button>
		</div>
	</section>

	<section class="section">
		<h2>Connected Servers</h2>
		{#if $serverState.known.length === 0}
			<p class="hint">No servers connected.</p>
		{:else}
			<ul class="server-list">
				{#each $serverState.known as server}
					<li>
						<div>
							<strong>{server.name}</strong>
							<span class="server-url">{server.url}</span>
							{#if server.url === $serverState.active}
								<span class="active-badge">active</span>
							{/if}
						</div>
						<button class="remove-btn" onclick={() => removeServer(server.url)}>Remove</button>
					</li>
				{/each}
			</ul>
		{/if}
	</section>

	<section class="section danger-zone">
		<button class="logout-btn" onclick={handleLogout}>Log out of this server</button>
	</section>
</div>

<ThemePicker shown={showTheme} onClose={() => showTheme = false} />

<style>
	h1 { font-size: 1.5rem; margin-bottom: 1.5rem; }
	h2 { font-size: 1.1rem; margin-bottom: 0.5rem; }

	.section {
		margin-bottom: 2rem;
		padding-bottom: 2rem;
		border-bottom: 1px solid var(--border);
	}

	.hint { color: var(--text-muted); font-size: 0.85rem; margin-bottom: 0.75rem; }

	form { display: flex; flex-direction: column; gap: 0.6rem; max-width: 400px; }
	input { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 0.75rem; color: var(--text); font-size: 1rem; }
	input:focus { outline: none; border-color: var(--accent); }

	.save-btn { background: var(--accent); color: var(--text-on-accent); padding: 0.6rem; border-radius: var(--radius); font-weight: 600; }
	.save-btn:disabled { opacity: 0.6; }

	.key-display {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.6rem 0.8rem;
		max-width: 400px;
	}

	code { font-size: 0.85rem; color: var(--text-muted); flex: 1; word-break: break-all; }
	.copy-btn { background: var(--bg-elevated); color: var(--text); padding: 0.3rem 0.6rem; border-radius: var(--radius); font-size: 0.8rem; border: 1px solid var(--border); }

	.server-list { list-style: none; max-width: 500px; }
	.server-list li {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.6rem 0.8rem;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		margin-bottom: 0.4rem;
	}

	.server-url { display: block; color: var(--text-muted); font-size: 0.75rem; }
	.active-badge { font-size: 0.65rem; color: var(--success); background: var(--success-softer); padding: 0.1rem 0.4rem; border-radius: 4px; margin-left: 0.4rem; }
	.remove-btn { background: none; color: var(--critical); font-size: 0.8rem; padding: 0.2rem 0.5rem; border: 1px solid var(--critical); border-radius: var(--radius); }

	.error { color: var(--critical); font-size: 0.85rem; }
	.success { color: var(--success); font-weight: 600; font-size: 0.9rem; }

	.danger-zone { border-bottom: none; }
	.logout-btn { background: none; color: var(--critical); padding: 0.6rem 1rem; border: 1px solid var(--critical); border-radius: var(--radius); font-weight: 600; }
	.logout-btn:hover { background: var(--critical-soft); }

	.theme-pick-row {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.current-theme {
		color: var(--text);
		font-weight: 500;
		font-size: 0.95rem;
	}

	.change-btn {
		background: var(--bg-elevated);
		color: var(--text);
		padding: 0.35rem 0.75rem;
		border-radius: var(--radius);
		font-size: 0.8rem;
		border: 1px solid var(--border);
	}

	.change-btn:hover {
		border-color: var(--accent);
	}
</style>
