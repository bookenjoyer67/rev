<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { isConnected } from '$lib/stores/server';
	import { api } from '$lib/api/client';

	interface Post {
		id: string;
		kind: 'resource' | 'need' | 'offer';
		category: string;
		title: string;
		body?: string;
		location_name?: string;
		urgency?: 'critical' | 'high' | 'medium' | 'low';
	}

	interface Community {
		slug: string;
		name: string;
	}

	let posts: Post[] = $state([]);
	let communities: Community[] = $state([]);
	let selectedCommunity = $state('');
	let filter = $state('all');
	let loading = $state(true);

	const kindLabels: Record<string, string> = { resource: 'Resource', need: 'Need', offer: 'Offer' };
	const urgencyColors: Record<string, string> = { critical: 'var(--critical)', high: 'var(--warning)', medium: 'var(--text-muted)', low: 'var(--text-muted)' };

	onMount(async () => {
		if (!isConnected()) { goto('/connect'); return; }
		try {
			communities = await api.communities.list();
			if (communities.length > 0) {
				selectedCommunity = communities[0].slug;
				await loadPosts();
			}
		} catch (e) {}
		loading = false;
	});

	async function loadPosts() {
		if (!selectedCommunity) return;
		try {
			const filters: Record<string, string> = {};
			if (filter !== 'all') filters.kind = filter;
			posts = await api.posts.list(selectedCommunity, filters);
		} catch (e) {
			posts = [];
		}
	}

	function setFilter(f: string) {
		filter = f;
		loadPosts();
	}

	function setCommunity(slug: string) {
		selectedCommunity = slug;
		loadPosts();
	}
</script>

<div class="container">
	<header class="page-header">
		<h1>Mutual Aid</h1>
		<a href="/aid/new" class="btn btn-primary">Post</a>
	</header>

	{#if communities.length > 1}
		<div class="community-select">
			{#each communities as c}
				<button class:active={selectedCommunity === c.slug} onclick={() => setCommunity(c.slug)}>{c.name}</button>
			{/each}
		</div>
	{/if}

	<div class="filters">
		<button class:active={filter === 'all'} onclick={() => setFilter('all')}>All</button>
		<button class:active={filter === 'need'} onclick={() => setFilter('need')}>Needs</button>
		<button class:active={filter === 'offer'} onclick={() => setFilter('offer')}>Offers</button>
		<button class:active={filter === 'resource'} onclick={() => setFilter('resource')}>Resources</button>
	</div>

	{#if loading}
		<p class="status">Loading...</p>
	{:else if communities.length === 0}
		<p class="status">No communities yet. <a href="/community/create">Create one</a> to start posting.</p>
	{:else if posts.length === 0}
		<p class="status">No posts yet. Be the first to share a need, offer, or resource.</p>
	{:else}
		<ul class="post-list">
			{#each posts as post}
				<li class="post-card">
					<div class="post-meta">
						<span class="kind kind-{post.kind}">{kindLabels[post.kind]}</span>
						<span class="category">{post.category}</span>
						{#if post.urgency}
							<span class="urgency" style="color: {urgencyColors[post.urgency]}">{post.urgency}</span>
						{/if}
					</div>
					<h3>{post.title}</h3>
					{#if post.body}
						<p class="body">{post.body}</p>
					{/if}
					{#if post.location_name}
						<span class="location">{post.location_name}</span>
					{/if}
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
		color: white;
		padding: 0.5rem 1rem;
		border-radius: var(--radius);
		font-weight: 600;
		font-size: 0.9rem;
	}

	.btn-primary:hover { text-decoration: none; }

	.community-select {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 1rem;
		flex-wrap: wrap;
	}

	.community-select button {
		background: var(--bg-surface);
		color: var(--text-muted);
		padding: 0.4rem 0.8rem;
		border-radius: var(--radius);
		font-size: 0.85rem;
		border: 1px solid var(--border);
	}

	.community-select button.active {
		background: var(--bg-elevated);
		color: var(--text);
		border-color: var(--text-muted);
	}

	.filters {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 1.5rem;
	}

	.filters button {
		background: var(--bg-surface);
		color: var(--text-muted);
		padding: 0.4rem 0.8rem;
		border-radius: var(--radius);
		font-size: 0.85rem;
		border: 1px solid var(--border);
	}

	.filters button.active {
		background: var(--accent-soft);
		color: var(--accent);
		border-color: var(--accent);
	}

	.post-list {
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.post-card {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		padding: 1rem;
	}

	.post-meta {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 0.5rem;
		font-size: 0.8rem;
	}

	.kind {
		padding: 0.15rem 0.5rem;
		border-radius: 4px;
		font-weight: 600;
		text-transform: uppercase;
		font-size: 0.7rem;
	}

	.kind-need { background: #e6394620; color: var(--critical); }
	.kind-offer { background: #2ec4b620; color: var(--success); }
	.kind-resource { background: #6366f120; color: #818cf8; }

	.category { color: var(--text-muted); text-transform: capitalize; }
	.urgency { font-weight: 600; text-transform: uppercase; }

	h3 { font-size: 1.05rem; margin-bottom: 0.3rem; }
	.body { color: var(--text-muted); font-size: 0.9rem; }
	.location { color: var(--text-muted); font-size: 0.8rem; }

	.status {
		text-align: center;
		color: var(--text-muted);
		padding: 3rem 0;
	}
</style>
