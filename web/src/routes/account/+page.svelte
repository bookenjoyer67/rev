<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { auth, getActiveAuth, getPublicKey, getToken, isAuthenticated, setPassphrase, updateDisplayName, logout } from '$lib/stores/auth';
	import { isConnected, getActiveServer, serverState, disconnectServer, removeServer } from '$lib/stores/server';
	import { themeName } from '$lib/stores/theme';
	import ThemePicker from '$lib/components/ThemePicker.svelte';

	let displayName = $state('');
	let bio = $state('');
	let avatarUrl = $state('');
	let passphrase = $state('');
	let passphraseConfirm = $state('');
	let saving = $state(false);
	let saved = $state(false);
	let bioSaving = $state(false);
	let bioSaved = $state(false);
	let avatarUploading = $state(false);
	let passphraseSaving = $state(false);
	let passphraseSaved = $state(false);
	let error = $state('');
	let bioError = $state('');
	let passphraseError = $state('');
	let copied = $state(false);
	let showTheme = $state(false);

	let publicKey = $derived(getPublicKey() || '');
	let hasBundle = $derived(!!$auth.keypair?.ed25519SecretKey);

	onMount(async () => {
		if (!isConnected() || !isAuthenticated()) { goto('/'); return; }
		const server = getActiveServer();
		const token = getToken();
		if (server && token) {
			try {
				const res = await fetch(`${server}/api/auth/me`, {
					headers: { 'Authorization': `Bearer ${token}` }
				});
				if (res.ok) {
					const data = await res.json();
					displayName = data.display_name || '';
					bio = data.bio || '';
					avatarUrl = data.avatar_url || '';
				}
			} catch {}
		}
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

	async function saveBio() {
		bioSaving = true;
		bioError = '';
		try {
			const server = getActiveServer();
			const token = getToken();
			const res = await fetch(`${server}/api/auth/me`, {
				method: 'PUT',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': `Bearer ${token}`,
				},
				body: JSON.stringify({ bio: bio.trim() || null }),
			});
			bioSaving = false;
			if (res.ok) { bioSaved = true; setTimeout(() => bioSaved = false, 2000); }
			else { bioError = 'Failed to save bio'; }
		} catch {
			bioSaving = false;
			bioError = 'Network error';
		}
	}

	async function handleAvatarUpload(e: Event) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (!file) return;
		avatarUploading = true;
		try {
			const formData = new FormData();
			formData.append('file', file);
			const server = getActiveServer();
			const token = getToken();
			const res = await fetch(`${server}/api/auth/me/avatar`, {
				method: 'POST',
				headers: { 'Authorization': `Bearer ${token}` },
				body: formData,
			});
			if (res.ok) {
				const data = await res.json();
				avatarUrl = data.avatar_url;
			}
		} catch {}
		avatarUploading = false;
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
		<h2>Profile Picture</h2>
		<div class="avatar-section">
			{#if avatarUrl}
				<img src={avatarUrl} alt="Your avatar" class="avatar-preview" />
			{:else}
				<div class="avatar-placeholder">{displayName[0]?.toUpperCase() || '?'}</div>
			{/if}
			<label class="upload-label">
				{avatarUploading ? 'Uploading...' : 'Upload photo'}
				<input type="file" accept="image/png,image/jpeg,image/webp" onchange={handleAvatarUpload} disabled={avatarUploading} class="file-input" />
			</label>
			<p class="hint">PNG, JPEG, or WebP. Max 1MB. EXIF data is stripped.</p>
		</div>
	</section>

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
		<h2>Bio</h2>
		<form onsubmit={(e) => { e.preventDefault(); saveBio(); }}>
			<textarea bind:value={bio} maxlength="500" placeholder="Tell communities about yourself..." rows="4"></textarea>
			{#if bioError}<p class="error">{bioError}</p>{/if}
			<button type="submit" class="save-btn" disabled={bioSaving}>
				{bioSaving ? 'Saving...' : bioSaved ? 'Saved!' : 'Save Bio'}
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
	input, textarea { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 0.75rem; color: var(--text); font-size: 1rem; font-family: inherit; }
	input:focus, textarea:focus { outline: none; border-color: var(--accent); }

	.save-btn { background: var(--accent); color: var(--text-on-accent); padding: 0.6rem; border-radius: var(--radius); font-weight: 600; }
	.save-btn:disabled { opacity: 0.6; }

	.avatar-section {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.75rem;
	}

	.avatar-preview {
		width: 96px;
		height: 96px;
		border-radius: 50%;
		object-fit: cover;
		border: 2px solid var(--border);
	}

	.avatar-placeholder {
		width: 96px;
		height: 96px;
		border-radius: 50%;
		background: var(--bg-elevated);
		border: 2px solid var(--border);
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 2.5rem;
		font-weight: 700;
		color: var(--text-muted);
	}

	.upload-label {
		background: var(--bg-elevated);
		color: var(--text);
		padding: 0.4rem 0.8rem;
		border-radius: var(--radius);
		font-size: 0.85rem;
		border: 1px solid var(--border);
		cursor: pointer;
	}

	.file-input { display: none; }

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
