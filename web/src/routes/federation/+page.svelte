<script lang="ts">
	import { onMount } from 'svelte';
	import { getActiveServer } from '$lib/stores/server';
	import { getToken } from '$lib/stores/auth';

	interface Alliance {
		id: string;
		remote_domain: string;
		remote_name?: string;
		status: string;
		initiated_by: string;
		created_at: string;
	}

	let alliances: Alliance[] = $state([]);
	let loading = $state(true);
	let error = $state('');

	onMount(async () => {
		const server = getActiveServer();
		if (!server) {
			loading = false;
			return;
		}

		try {
			const headers: Record<string, string> = {};
			const token = getToken();
			if (token) headers['Authorization'] = `Bearer ${token}`;

			const res = await fetch(`${server}/api/alliances`, { headers });
			if (res.ok) {
				alliances = await res.json();
			} else {
				error = `Server returned ${res.status}`;
			}
		} catch (e) {
			error = 'Could not reach server';
		}
		loading = false;
	});
</script>

<div class="container">
	<header class="page-header">
		<h1>Federation</h1>
		<p class="subtitle">Komun nodes federate to share resources, needs, and offers across communities.</p>
	</header>

	{#if loading}
		<div class="empty"><p>Loading alliances…</p></div>
	{:else if error}
		<div class="empty"><p>{error}</p></div>
	{:else if alliances.length === 0}
		<div class="empty">
			<p>No federated nodes yet.</p>
			<p class="hint">Share your node address with other communities to form alliances.</p>
		</div>
	{:else}
		<ul class="alliance-list">
			{#each alliances as a (a.id)}
				<li>
					<div class="alliance-info">
						<strong>{a.remote_name || a.remote_domain}</strong>
						<span class="domain">{a.remote_domain}</span>
					</div>
					<span class="status status-{a.status}">{a.status}</span>
				</li>
			{/each}
		</ul>
	{/if}
</div>

<style>
	.container {
		max-width: 640px;
		margin: 0 auto;
		padding: 1rem;
	}

	.page-header {
		margin-bottom: 2rem;
	}

	h1 { font-size: 1.5rem; margin-bottom: 0.25rem; }

	.subtitle {
		color: var(--text-muted);
		font-size: 0.9rem;
	}

	.empty {
		text-align: center;
		padding: 3rem 0;
		color: var(--text-muted);
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
	}

	.hint {
		font-size: 0.8rem;
		margin-top: 0.5rem;
	}

	.alliance-list {
		list-style: none;
	}

	li {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		margin-bottom: 0.5rem;
		background: var(--bg-surface);
	}

	.alliance-info {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}

	.domain {
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	.status {
		font-size: 0.75rem;
		padding: 0.2rem 0.6rem;
		border-radius: 4px;
		font-weight: 600;
		text-transform: uppercase;
	}

	.status-accepted { background: var(--success-soft); color: var(--success); }
	.status-pending { background: var(--warning-soft); color: var(--warning); }
	.status-rejected { background: var(--critical-soft); color: var(--critical); }
	.status-severed { background: var(--text-muted); color: var(--bg); }
</style>
