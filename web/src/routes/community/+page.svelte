<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { isConnected } from '$lib/stores/server';
	import { api } from '$lib/api/client';

	interface Community {
		slug: string;
		name: string;
		description?: string;
		location_name?: string;
		image_path?: string;
	}

	let communities: Community[] = $state([]);
	let loading = $state(true);

	onMount(() => {
		if (!isConnected()) { goto('/connect'); return; }
		loadCommunities();
	});

	async function loadCommunities() {
		try {
			communities = await api.communities.list();
		} catch (e) {}
		loading = false;
	}
</script>

<div class="container">
	<header class="page-header">
		<h1>Communities</h1>
		<a href="/community/create" class="btn btn-primary">Create</a>
	</header>

	{#if loading}
		<p class="status">Loading...</p>
	{:else if communities.length === 0}
		<div class="empty">
			<p>No communities on this server yet.</p>
			<div class="actions">
				<a href="/community/create" class="btn btn-primary">Create a community</a>
			</div>
		</div>
	{:else}
		<ul class="community-list">
			{#each communities as c}
				<li>
					<a href="/c/{c.slug}">
						{#if c.image_path}
							<img src={'/community-images/' + c.image_path} alt="" class="community-thumb" />
						{/if}
						<div>
							<strong>{c.name}</strong>
							{#if c.description}
								<p>{c.description}</p>
							{/if}
						</div>
						{#if c.location_name}
							<span class="location">{c.location_name}</span>
						{/if}
					</a>
				</li>
			{/each}
		</ul>
	{/if}
</div>

<style>
	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1.5rem;
	}

	h1 { font-size: 1.5rem; }

	.btn-primary {
		background: var(--accent);
		color: var(--text-on-accent);
		padding: 0.5rem 1rem;
		border-radius: var(--radius);
		font-weight: 600;
		font-size: 0.9rem;
	}

	.btn-primary:hover { text-decoration: none; }

	.community-list {
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	li {
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		overflow: hidden;
	}

	li a {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 1rem;
		color: var(--text);
	}

	li a:hover {
		background: var(--bg-surface);
		text-decoration: none;
	}

	li p {
		color: var(--text-muted);
		font-size: 0.85rem;
		margin-top: 0.2rem;
	}

	.location {
		color: var(--text-muted);
		font-size: 0.8rem;
		margin-left: auto;
	}

	.community-thumb {
		width: 48px;
		height: 48px;
		border-radius: var(--radius-md);
		object-fit: cover;
		border: 1px solid var(--border);
		flex-shrink: 0;
	}

	.empty {
		text-align: center;
		padding: 3rem 0;
		color: var(--text-muted);
	}

	.empty .actions {
		margin-top: 1.5rem;
	}

	.status {
		text-align: center;
		color: var(--text-muted);
		padding: 3rem 0;
	}
</style>
