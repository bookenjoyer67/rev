<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { connectToServer, serverState, type NodeInfo } from '$lib/stores/server';
	import { recover } from '$lib/stores/auth';
	import { discoverAllServers, type NearbyServer } from '$lib/api/discovery';

	let url = $state('');
	let error = $state('');
	let loading = $state(false);
	let nodeInfo: NodeInfo | null = $state(null);
	let browseServers: NearbyServer[] = $state([]);
	let browseLoading = $state(false);

	let showRecover = $state(false);

	onMount(async () => {
		browseLoading = true;
		try {
			browseServers = await discoverAllServers();
		} catch { browseServers = []; }
		browseLoading = false;
	});
	let recoverUrl = $state('');
	let recoverPassphrase = $state('');
	let recoverCode = $state('');
	let recoverError = $state('');
	let recoverLoading = $state(false);
	let recoverSuccess = $state(false);

	async function handleRecover() {
		if (!recoverUrl.trim()) { recoverError = 'Enter a server URL'; return; }
		if (!recoverPassphrase.trim()) { recoverError = 'Enter your passphrase'; return; }
		recoverLoading = true;
		recoverError = '';
		const ok = await recover(recoverUrl.trim().replace(/\/+$/, ''), recoverPassphrase, recoverCode.trim() || undefined);
		recoverLoading = false;
		if (ok) {
			recoverSuccess = true;
			try { await connectToServer(recoverUrl.trim()); } catch {}
			setTimeout(() => goto('/'), 1000);
		} else {
			recoverError = 'Recovery failed. Check your passphrase and server URL.';
		}
	}

	async function handleConnect() {
		if (!url.trim()) { error = 'Enter a server URL'; return; }
		loading = true;
		error = '';
		nodeInfo = null;
		try {
			const info = await connectToServer(url.trim());
			nodeInfo = info;
			setTimeout(() => goto('/'), 500);
		} catch (e: any) {
			error = e.message || 'Could not connect to server';
		} finally {
			loading = false;
		}
	}

	function connectToKnown(serverUrl: string) {
		url = serverUrl;
		handleConnect();
	}
</script>

<div class="container">
	<div class="connect-page">
		<h1>komun</h1>
		<p class="tagline">Connect to a server to browse communities and mutual aid.</p>

		{#if browseLoading}
			<p class="browse-status">Loading available servers...</p>
		{:else if browseServers.length > 0}
			<div class="available-servers">
				<h3>Available servers</h3>
				<ul>
					{#each browseServers as server}
						<li>
							<button class="server-entry" onclick={() => connectToKnown(server.url)}>
								<div class="server-info">
									<strong>{server.name}</strong>
									{#if server.description}
										<span class="server-desc">{server.description}</span>
									{/if}
									<span class="server-meta">
										{#if server.location_name}{server.location_name} &middot; {/if}
										{server.communities_count} communit{server.communities_count === 1 ? 'y' : 'ies'}
									</span>
								</div>
								<span class="server-connect">Connect</span>
							</button>
						</li>
					{/each}
				</ul>
			</div>
		{/if}

		<form onsubmit={(e) => { e.preventDefault(); handleConnect(); }}>
			<input
				type="url"
				bind:value={url}
				placeholder="https://your-community-server.org"
				disabled={loading}
			/>
			{#if error}
				<p class="error">{error}</p>
			{/if}
			<button type="submit" disabled={loading}>
				{loading ? 'Connecting...' : 'Connect'}
			</button>
		</form>

		{#if nodeInfo}
			<div class="node-info">
				<h3>{nodeInfo.name}</h3>
				<p>{nodeInfo.description}</p>
				{#if nodeInfo.location?.name}
					<span class="location">{nodeInfo.location.name}</span>
				{/if}
				<span class="meta">{nodeInfo.communities_count} communities</span>
			</div>
		{/if}

		{#if $serverState.known.length > 0}
			<div class="known-servers">
				<h3>Previous servers</h3>
				<ul>
					{#each $serverState.known as server}
						<li>
							<button onclick={() => connectToKnown(server.url)}>
								<strong>{server.name}</strong>
								<span class="server-url">{server.url}</span>
							</button>
						</li>
					{/each}
				</ul>
			</div>
		{/if}

		<div class="recover-section">
			{#if !showRecover}
				<button class="recover-toggle" onclick={() => showRecover = true}>
					Recover an existing identity
				</button>
			{:else if recoverSuccess}
				<p class="recover-success">Identity recovered! Redirecting...</p>
			{:else}
				<h3>Recover identity</h3>
				<p class="recover-hint">Enter a server you've used before and your recovery passphrase.</p>
				<form onsubmit={(e) => { e.preventDefault(); handleRecover(); }}>
					<input type="url" bind:value={recoverUrl} placeholder="https://server-url" disabled={recoverLoading} />
					<input type="password" bind:value={recoverPassphrase} placeholder="Your recovery passphrase" disabled={recoverLoading} />
					<input type="text" bind:value={recoverCode} placeholder="Recovery code (12 words, if set)" disabled={recoverLoading} />
					{#if recoverError}
						<p class="error">{recoverError}</p>
					{/if}
					<button type="submit" class="recover-btn" disabled={recoverLoading}>
						{recoverLoading ? 'Recovering...' : 'Recover'}
					</button>
				</form>
			{/if}
		</div>

		<p class="footer-note">
			Don't have a server? Ask your community organizer, or
			<a href="https://git.komun.buzz/Book-Enjoyer/rev" target="_blank">run your own</a>.
		</p>
	</div>
</div>

<style>
	.connect-page {
		max-width: 440px;
		margin: 0 auto;
		text-align: center;
		padding: 3rem 0;
	}

	h1 {
		font-size: 2.5rem;
		font-weight: 800;
		letter-spacing: -1px;
		margin-bottom: 0.5rem;
	}

	.tagline {
		color: var(--text-muted);
		margin-bottom: 2rem;
	}

	form {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		margin-bottom: 1.5rem;
	}

	input {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.85rem 1rem;
		color: var(--text);
		font-size: 1rem;
		text-align: center;
	}

	input:focus {
		outline: none;
		border-color: var(--accent);
	}

	button[type="submit"] {
		background: var(--accent);
		color: var(--text-on-accent);
		padding: 0.75rem;
		border-radius: var(--radius);
		font-weight: 600;
		font-size: 1rem;
	}

	button[type="submit"]:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.error {
		color: var(--critical);
		font-size: 0.85rem;
	}

	.node-info {
		background: var(--bg-surface);
		border: 1px solid var(--success);
		border-radius: var(--radius-lg);
		padding: 1rem;
		margin-bottom: 1.5rem;
	}

	.node-info h3 { font-size: 1.1rem; margin-bottom: 0.25rem; }
	.node-info p { color: var(--text-muted); font-size: 0.9rem; }
	.node-info .location { color: var(--text-muted); font-size: 0.8rem; display: block; margin-top: 0.25rem; }
	.node-info .meta { color: var(--success); font-size: 0.8rem; }

	.known-servers {
		text-align: left;
		margin-bottom: 1.5rem;
	}

	.known-servers h3 {
		font-size: 0.9rem;
		color: var(--text-muted);
		margin-bottom: 0.5rem;
	}

	.known-servers ul { list-style: none; }

	.known-servers li {
		margin-bottom: 0.4rem;
	}

	.known-servers button {
		width: 100%;
		text-align: left;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.6rem 0.8rem;
		color: var(--text);
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.known-servers button:hover {
		border-color: var(--accent);
	}

	.server-url {
		color: var(--text-muted);
		font-size: 0.8rem;
	}

	.recover-section {
		margin-top: 2rem;
		padding-top: 1.5rem;
		border-top: 1px solid var(--border);
	}

	.recover-toggle {
		background: none;
		color: var(--text-muted);
		font-size: 0.85rem;
		text-decoration: underline;
	}

	.recover-section h3 {
		font-size: 1rem;
		margin-bottom: 0.25rem;
	}

	.recover-hint {
		font-size: 0.8rem;
		color: var(--text-muted);
		margin-bottom: 0.75rem;
	}

	.recover-section form {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.recover-section input {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.6rem 0.8rem;
		color: var(--text);
		font-size: 0.9rem;
	}

	.recover-btn {
		background: var(--bg-elevated);
		color: var(--text);
		border: 1px solid var(--border);
		padding: 0.6rem;
		border-radius: var(--radius);
		font-weight: 600;
	}

	.recover-success {
		color: var(--success);
		font-weight: 600;
	}

	.footer-note {
		color: var(--text-muted);
		font-size: 0.8rem;
		margin-top: 2rem;
	}

	.browse-status {
		color: var(--text-muted);
		font-size: 0.85rem;
		margin-bottom: 1.5rem;
	}

	.available-servers {
		text-align: left;
		margin-bottom: 1.5rem;
	}

	.available-servers h3 {
		font-size: 0.9rem;
		color: var(--text-muted);
		margin-bottom: 0.5rem;
		text-align: left;
	}

	.available-servers ul { list-style: none; }

	.available-servers li { margin-bottom: 0.4rem; }

	.server-entry {
		width: 100%;
		text-align: left;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.7rem 0.8rem;
		color: var(--text);
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.75rem;
	}

	.server-entry:hover { border-color: var(--accent); }

	.server-info {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		min-width: 0;
	}

	.server-info strong { font-size: 0.95rem; }

	.server-desc {
		color: var(--text-muted);
		font-size: 0.8rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.server-meta {
		color: var(--text-muted);
		font-size: 0.75rem;
	}

	.server-connect {
		color: var(--accent);
		font-weight: 600;
		font-size: 0.85rem;
		flex-shrink: 0;
	}
</style>
