<script lang="ts">
	import { goto } from '$app/navigation';
	import { connectToServer, serverState, type NodeInfo } from '$lib/stores/server';

	let url = $state('');
	let error = $state('');
	let loading = $state(false);
	let nodeInfo: NodeInfo | null = $state(null);

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

		<p class="footer-note">
			Don't have a server? Ask your community organizer, or
			<a href="https://github.com/komun" target="_blank">run your own</a>.
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
		color: white;
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

	.footer-note {
		color: var(--text-muted);
		font-size: 0.8rem;
		margin-top: 2rem;
	}
</style>
