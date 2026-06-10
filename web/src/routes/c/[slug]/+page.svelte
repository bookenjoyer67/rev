<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import { auth } from '$lib/stores/auth';
	import { getActiveServer } from '$lib/stores/server';
	import RespondModal from '$lib/components/RespondModal.svelte';

	interface Community { slug: string; name: string; description?: string; location_name?: string; }
	interface Post {
		id: string;
		kind: 'resource' | 'need' | 'offer';
		category: string;
		title: string;
		body?: string;
		location_name?: string;
		urgency?: string;
		status: string;
		author_id: string;
		created_at: string;
	}

	let community: Community | null = $state(null);
	let posts: Post[] = $state([]);
	let filter = $state('all');
	let loading = $state(true);
	let error = $state('');
	let respondingTo: Post | null = $state(null);
	let editingId: string | null = $state(null);
	let editTitle = $state('');
	let editBody = $state('');

	async function fulfillPost(postId: string) {
		if (!community) return;
		await api.posts.fulfill(community.slug, postId);
		await loadPosts(community.slug);
	}

	async function deletePost(postId: string) {
		if (!community || !confirm('Delete this post?')) return;
		await api.posts.withdraw(community.slug, postId);
		await loadPosts(community.slug);
	}

	function startEdit(post: Post) {
		editingId = post.id;
		editTitle = post.title;
		editBody = post.body || '';
	}

	async function saveEdit() {
		if (!community || !editingId) return;
		await api.posts.update(community.slug, editingId, {
			title: editTitle.trim(),
			body: editBody.trim() || undefined,
		});
		editingId = null;
		await loadPosts(community.slug);
	}

	let myUserId = $derived((() => {
		const server = getActiveServer();
		if (!server) return null;
		return $auth.servers?.[server]?.userId || null;
	})());

	const kindLabels: Record<string, string> = { resource: 'Resource', need: 'Need', offer: 'Offer' };
	const urgencyColors: Record<string, string> = { critical: 'var(--critical)', high: 'var(--warning)', medium: 'var(--text-muted)', low: 'var(--text-muted)' };

	$effect(() => {
		const slug = $page.params.slug;
		if (slug) load(slug);
	});

	async function load(slug: string) {
		loading = true;
		error = '';
		try {
			community = await api.communities.get(slug);
			await loadPosts(slug);
		} catch (e: any) {
			error = e.message || 'Community not found';
		} finally {
			loading = false;
		}
	}

	async function loadPosts(slug: string) {
		const filters: Record<string, string> = {};
		if (filter !== 'all') filters.kind = filter;
		posts = await api.posts.list(slug, filters);
	}

	function setFilter(f: string) {
		filter = f;
		if (community) loadPosts(community.slug);
	}

	function timeAgo(dateStr: string): string {
		const seconds = Math.floor((Date.now() - new Date(dateStr).getTime()) / 1000);
		if (seconds < 60) return 'just now';
		if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
		if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
		return `${Math.floor(seconds / 86400)}d ago`;
	}
</script>

<div class="container">
	{#if loading}
		<p class="status">Loading...</p>
	{:else if error}
		<p class="status error">{error}</p>
	{:else if community}
		<header class="community-header">
			<div>
				<h1>{community.name}</h1>
				{#if community.description}
					<p class="desc">{community.description}</p>
				{/if}
				{#if community.location_name}
					<span class="location">{community.location_name}</span>
				{/if}
			</div>
			<a href="/aid/new" class="btn-post">Post</a>
		</header>

		<div class="filters">
			<button class:active={filter === 'all'} onclick={() => setFilter('all')}>All</button>
			<button class:active={filter === 'need'} onclick={() => setFilter('need')}>Needs</button>
			<button class:active={filter === 'offer'} onclick={() => setFilter('offer')}>Offers</button>
			<button class:active={filter === 'resource'} onclick={() => setFilter('resource')}>Resources</button>
		</div>

		{#if posts.length === 0}
			<p class="status">No posts yet. Be the first to share.</p>
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
							<span class="time">{timeAgo(post.created_at)}</span>
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
								<span class="loc">{post.location_name}</span>
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
	{/if}
</div>

{#if respondingTo && community}
	<RespondModal
		post={{ id: respondingTo.id, title: respondingTo.title, kind: respondingTo.kind, server_url: getActiveServer() || '', community_slug: community.slug, author_id: respondingTo.author_id }}
		onClose={() => respondingTo = null}
	/>
{/if}

<style>
	.community-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 1.5rem;
		gap: 1rem;
	}

	h1 { font-size: 1.5rem; margin-bottom: 0.25rem; }
	.desc { color: var(--text-muted); font-size: 0.9rem; }
	.location { color: var(--text-muted); font-size: 0.8rem; }

	.btn-post {
		background: var(--accent);
		color: white;
		padding: 0.5rem 1rem;
		border-radius: var(--radius);
		font-weight: 600;
		font-size: 0.9rem;
		white-space: nowrap;
	}

	.btn-post:hover { text-decoration: none; }

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
		align-items: center;
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
	.time { color: var(--text-muted); margin-left: auto; }

	h3 { font-size: 1.05rem; margin-bottom: 0.3rem; }
	.body { color: var(--text-muted); font-size: 0.9rem; }
	.loc { color: var(--text-muted); font-size: 0.8rem; }

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
		color: white;
		padding: 0.4rem 0.8rem;
		border-radius: var(--radius);
		font-weight: 600;
		font-size: 0.8rem;
		margin-left: auto;
	}

	.author-actions {
		display: flex;
		gap: 0.4rem;
		margin-left: auto;
	}

	.action-btn {
		padding: 0.3rem 0.6rem;
		border-radius: var(--radius);
		font-size: 0.75rem;
		font-weight: 600;
		border: 1px solid;
	}

	.action-btn.fulfill { background: #2ec4b615; color: var(--success); border-color: var(--success); }
	.action-btn.edit { background: var(--bg-elevated); color: var(--text-muted); border-color: var(--border); }
	.action-btn.delete { background: #e6394610; color: var(--critical); border-color: var(--critical); }

	.fulfilled-badge {
		font-size: 0.75rem;
		color: var(--success);
		font-weight: 600;
		margin-left: auto;
	}

	.edit-form {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		margin-bottom: 0.5rem;
	}

	.edit-form input, .edit-form textarea {
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.5rem;
		color: var(--text);
		font-size: 0.9rem;
		font-family: inherit;
	}

	.edit-actions { display: flex; gap: 0.4rem; }
	.save-btn { background: var(--accent); color: white; padding: 0.3rem 0.8rem; border-radius: var(--radius); font-size: 0.8rem; font-weight: 600; }
	.cancel-btn { background: var(--bg-elevated); color: var(--text-muted); padding: 0.3rem 0.8rem; border-radius: var(--radius); font-size: 0.8rem; border: 1px solid var(--border); }

	.status { text-align: center; color: var(--text-muted); padding: 3rem 0; }
	.error { color: var(--critical); }
</style>
