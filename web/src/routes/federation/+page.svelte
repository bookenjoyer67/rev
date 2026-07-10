<script lang="ts">
	interface Alliance {
		remote_domain: string;
		remote_name?: string;
		status: string;
	}

	let alliances: Alliance[] = $state([]);

	// TODO: fetch from /api/alliances
</script>

<div class="container">
	<header class="page-header">
		<h1>Federation</h1>
	</header>

	<section class="info">
		<p>
			Komun nodes federate with each other via ActivityPub.
			Allied communities share resources, needs, and offers across nodes.
		</p>
	</section>

	{#if alliances.length === 0}
		<div class="empty">
			<p>No allied nodes yet.</p>
			<p>Share your node address with other communities to form alliances.</p>
		</div>
	{:else}
		<ul class="alliance-list">
			{#each alliances as a}
				<li>
					<strong>{a.remote_name || a.remote_domain}</strong>
					<span class="status status-{a.status}">{a.status}</span>
				</li>
			{/each}
		</ul>
	{/if}
</div>

<style>
	.page-header {
		margin-bottom: 1.5rem;
	}

	h1 { font-size: 1.5rem; }

	.info {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		padding: 1rem;
		margin-bottom: 1.5rem;
	}

	.info p {
		color: var(--text-muted);
		font-size: 0.9rem;
	}

	.empty {
		text-align: center;
		padding: 3rem 0;
		color: var(--text-muted);
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
	}

	.status {
		font-size: 0.8rem;
		padding: 0.2rem 0.5rem;
		border-radius: 4px;
	}

	.status-accepted { background: var(--success-soft); color: var(--success); }
	.status-pending { background: var(--warning-soft); color: var(--warning); }
</style>
