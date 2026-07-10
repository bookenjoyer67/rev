<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { isConnected, getActiveServer } from '$lib/stores/server';
	import { auth } from '$lib/stores/auth';
	import { api } from '$lib/api/client';
	import RespondModal from '$lib/components/RespondModal.svelte';

	interface Post {
		id: string;
		kind: 'resource' | 'need' | 'offer';
		category: string;
		title: string;
		body?: string;
		location_name?: string;
		urgency?: 'critical' | 'high' | 'medium' | 'low';
		status: string;
		author_id: string;
	}

	interface Community {
		slug: string;
		name: string;
	}

	let posts: Post[] = $state([]);
	let communities: Community[] = $state([]);
	let selectedCommunity = $state('');
	let filter = $state('all');
	let searchQuery = $state('');
	let loading = $state(true);
	let respondingTo: Post | null = $state(null);
	let editingId: string | null = $state(null);
	let editTitle = $state('');
	let editBody = $state('');

	async function fulfillPost(postId: string) {
		if (!selectedCommunity) return;
		await api.posts.fulfill(selectedCommunity, postId);
		await loadPosts();
	}

	async function deletePost(postId: string) {
		if (!selectedCommunity || !confirm('Delete this post?')) return;
		await api.posts.withdraw(selectedCommunity, postId);
		await loadPosts();
	}

	function startEdit(post: Post) {
		editingId = post.id;
		editTitle = post.title;
		editBody = post.body || '';
	}

	async function saveEdit() {
		if (!selectedCommunity || !editingId) return;
		await api.posts.update(selectedCommunity, editingId, {
			title: editTitle.trim(),
			body: editBody.trim() || undefined,
		});
		editingId = null;
		await loadPosts();
	}

	let myUserId = $derived((() => {
		const server = getActiveServer();
		if (!server) return null;
		return $auth.servers?.[server]?.userId || null;
	})());

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
			if (searchQuery.trim()) filters.q = searchQuery.trim();
			posts = await api.posts.list(selectedCommunity, filters);
		} catch (e) {
			posts = [];
		}
	}

	function handleSearch() {
		loadPosts();
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
		{#if communities.length > 0}
			<a href="/aid/new" class="btn btn-primary">Post</a>
		{/if}
	</header>

	{#if communities.length > 1}
		<div class="community-select">
			{#each communities as c}
				<button class:active={selectedCommunity === c.slug} onclick={() => setCommunity(c.slug)}>{c.name}</button>
			{/each}
		</div>
	{/if}

	<form class="search-bar" onsubmit={(e) => { e.preventDefault(); handleSearch(); }}>
		<input type="text" bind:value={searchQuery} placeholder="Search posts..." />
		{#if searchQuery}
			<button type="button" class="clear-search" onclick={() => { searchQuery = ''; handleSearch(); }}>&times;</button>
		{/if}
	</form>

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
					{#if editingId === post.id}
						<form class="edit-form" onsubmit={(e) => { e.preventDefault(); saveEdit(); }}>
							<input type="text" bind:value={editTitle} placeholder="Title" />
							<textarea bind:value={editBody} placeholder="Details" rows="2"></textarea>
							<div class="edit-actions">
								<button type="submit" class="save-btn">Save</button>
								<button type="button" class="cancel-btn" onclick={() => editingId = null}>Cancel</button>
							</div>
						</form>
					{:else}
						<h3>{post.title}</h3>
						{#if post.body}
							<p class="body">{post.body}</p>
						{/if}
					{/if}
					<div class="post-footer">
						{#if post.location_name}
							<span class="location">{post.location_name}</span>
						{/if}
						{#if post.author_id !== myUserId}
							{#if post.kind === 'need'}
								<button class="respond-btn" onclick={() => respondingTo = post}>I can help</button>
							{:else if post.kind === 'offer'}
								<button class="respond-btn" onclick={() => respondingTo = post}>Request this</button>
							{/if}
						{:else if post.status === 'fulfilled'}
							<span class="fulfilled-badge">Fulfilled</span>
						{:else}
							<div class="author-actions">
								<button class="action-btn fulfill" onclick={() => fulfillPost(post.id)}>Fulfill</button>
								<button class="action-btn edit" onclick={() => startEdit(post)}>Edit</button>
								<button class="action-btn delete" onclick={() => deletePost(post.id)}>Delete</button>
							</div>
						{/if}
					</div>
				</li>
			{/each}
		</ul>
	{/if}
</div>

{#if respondingTo}
	<RespondModal
		post={{ id: respondingTo.id, title: respondingTo.title, kind: respondingTo.kind, server_url: getActiveServer() || '', community_slug: selectedCommunity, author_id: respondingTo.author_id }}
		onClose={() => respondingTo = null}
	/>
{/if}

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

	.search-bar { display: flex; position: relative; margin-bottom: 0.75rem; }
	.search-bar input { flex: 1; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 0.6rem 2rem 0.6rem 0.75rem; color: var(--text); font-size: 0.9rem; }
	.search-bar input:focus { outline: none; border-color: var(--accent); }
	.clear-search { position: absolute; right: 0.5rem; top: 50%; transform: translateY(-50%); background: none; color: var(--text-muted); font-size: 1.2rem; min-height: 30px; min-width: 30px; }

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

	.kind-need { background: var(--kind-need-soft); color: var(--critical); }
	.kind-offer { background: var(--kind-offer-soft); color: var(--success); }
	.kind-resource { background: var(--kind-resource-soft); color: var(--kind-resource); }

	.category { color: var(--text-muted); text-transform: capitalize; }
	.urgency { font-weight: 600; text-transform: uppercase; }

	h3 { font-size: 1.05rem; margin-bottom: 0.3rem; }
	.body { color: var(--text-muted); font-size: 0.9rem; }
	.location { color: var(--text-muted); font-size: 0.8rem; }

	.post-footer {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-top: 0.75rem;
		padding-top: 0.75rem;
		border-top: 1px solid var(--border);
	}

	.respond-btn {
		background: var(--accent);
		color: var(--text-on-accent);
		padding: 0.4rem 0.8rem;
		border-radius: var(--radius);
		font-weight: 600;
		font-size: 0.8rem;
		margin-left: auto;
	}

	.author-actions { display: flex; gap: 0.4rem; margin-left: auto; }
	.action-btn { padding: 0.3rem 0.6rem; border-radius: var(--radius); font-size: 0.75rem; font-weight: 600; border: 1px solid; }
	.action-btn.fulfill { background: var(--success-softer); color: var(--success); border-color: var(--success); }
	.action-btn.edit { background: var(--bg-elevated); color: var(--text-muted); border-color: var(--border); }
	.action-btn.delete { background: var(--critical-softer); color: var(--critical); border-color: var(--critical); }
	.fulfilled-badge { font-size: 0.75rem; color: var(--success); font-weight: 600; margin-left: auto; }

	.edit-form { display: flex; flex-direction: column; gap: 0.5rem; margin-bottom: 0.5rem; }
	.edit-form input, .edit-form textarea { background: var(--bg); border: 1px solid var(--border); border-radius: var(--radius); padding: 0.5rem; color: var(--text); font-size: 0.9rem; font-family: inherit; }
	.edit-actions { display: flex; gap: 0.4rem; }
	.save-btn { background: var(--accent); color: var(--text-on-accent); padding: 0.3rem 0.8rem; border-radius: var(--radius); font-size: 0.8rem; font-weight: 600; }
	.cancel-btn { background: var(--bg-elevated); color: var(--text-muted); padding: 0.3rem 0.8rem; border-radius: var(--radius); font-size: 0.8rem; border: 1px solid var(--border); }

	.status {
		text-align: center;
		color: var(--text-muted);
		padding: 3rem 0;
	}

	@media (max-width: 480px) {
		.page-header { flex-direction: column; gap: 0.5rem; align-items: flex-start; }
		.filters { flex-wrap: wrap; }
		.community-select { flex-wrap: wrap; }
		.post-footer { flex-direction: column; align-items: flex-start; gap: 0.5rem; }
		.author-actions { margin-left: 0; }
		.respond-btn { margin-left: 0; }
	}
</style>
