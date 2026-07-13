<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import { auth } from '$lib/stores/auth';
	import { getActiveServer, resolveSlug, parseSlug } from '$lib/stores/server';
	import RespondModal from '$lib/components/RespondModal.svelte';

	import { requireAuth, getToken } from '$lib/stores/auth';

	interface Community { slug: string; name: string; description?: string; location_name?: string; image_path?: string; is_member: boolean; member_role?: string; }
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
		images?: string[];
		created_at: string;
	}

	interface Member { user_id: string; display_name: string; role: string; joined_at: string; }

	let community: Community | null = $state(null);
	let posts: Post[] = $state([]);
	let members: Member[] = $state([]);
	let filter = $state('all');
	let searchQuery = $state('');
	let showMembers = $state(false);
	let loading = $state(true);
	let error = $state('');
	let respondingTo: Post | null = $state(null);
	let editingId: string | null = $state(null);
	let editTitle = $state('');
	let editBody = $state('');
	let editImageFiles: File[] = $state([]);
	let editImagePreviews: string[] = $state([]);
	let editExistingImages: string[] = $state([]);
	let inviteCode = $state('');
	let joining = $state(false);
	let joinError = $state('');

	async function joinCommunity() {
		requireAuth(async () => {
			if (!community) return;
			joining = true;
			joinError = '';
			try {
				await api.communities.join(community.slug, inviteCode.trim());
				await load(community.slug);
			} catch (e: any) {
				joinError = e.message || 'Failed to join';
			}
			joining = false;
		});
	}

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
		editExistingImages = post.images || [];
		editImageFiles = [];
		editImagePreviews = [];
	}

	function handleEditImages(e: Event) {
		const files = (e.target as HTMLInputElement).files;
		if (!files) return;
		for (const file of files) {
			if (editImageFiles.length >= 5) break;
			if (!file.type.match(/image\/(png|jpeg|webp)/)) continue;
			editImageFiles = [...editImageFiles, file];
			editImagePreviews = [...editImagePreviews, URL.createObjectURL(file)];
		}
	}

	function removeEditImage(i: number) {
		URL.revokeObjectURL(editImagePreviews[i]);
		editImageFiles = editImageFiles.filter((_, j) => j !== i);
		editImagePreviews = editImagePreviews.filter((_, j) => j !== i);
	}

	async function saveEdit() {
		if (!community || !editingId) return;
		await api.posts.update(community.slug, editingId, {
			title: editTitle.trim(),
			body: editBody.trim() || undefined,
		});
		if (editImageFiles.length > 0) {
			const server = getActiveServer();
			const token = getToken();
			if (server && token) {
				const formData = new FormData();
				for (const file of editImageFiles) formData.append('file', file);
				await fetch(`${server}/api/communities/${community.slug}/posts/${editingId}/images`, {
					method: 'POST',
					headers: { 'Authorization': `Bearer ${token}` },
					body: formData,
				});
			}
		}
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

	let localSlug = $state('');
	let serverUrl = $state('');

	$effect(() => {
		const rawSlug = $page.params.slug;
		if (rawSlug) load(rawSlug);
	});

	async function load(rawSlug: string) {
		loading = true;
		error = '';
		try {
			const resolved = await resolveSlug(rawSlug);
			localSlug = resolved.localSlug;
			serverUrl = resolved.serverUrl;
			community = await api.communities.get(localSlug);
			await loadPosts(localSlug);
			members = await api.communities.members(localSlug);
		} catch (e: any) {
			error = e.message || 'Community not found';
		} finally {
			loading = false;
		}
	}

	async function loadPosts(slug: string) {
		const filters: Record<string, string> = {};
		if (filter !== 'all') filters.kind = filter;
		if (searchQuery.trim()) filters.q = searchQuery.trim();
		posts = await api.posts.list(slug, filters);
	}

	function handleSearch() {
		if (community) loadPosts(community.slug);
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
			<div class="community-info">
				{#if community.image_path}
					<img src={'/community-images/' + community.image_path} alt={community.name} class="community-image" />
				{/if}
				<div>
					<h1>{community.name}</h1>
					{#if community.description}
						<p class="desc">{community.description}</p>
					{/if}
					{#if community.location_name}
						<span class="location">{community.location_name}</span>
					{/if}
				</div>
			</div>
			<div class="header-actions">
				<a href="/c/{localSlug}@{serverUrl.replace(/^https?:\/\//, '').split('/')[0].split(':')[0]}/map" class="btn-map">Map</a>
				{#if community.is_member}
					<a href="/aid/new" class="btn-post">Post</a>
				{/if}
				{#if community.member_role === 'admin'}
					<a href="/c/{localSlug}@{serverUrl.replace(/^https?:\/\//, '').split('/')[0].split(':')[0]}/settings" class="btn-settings">Settings</a>
				{/if}
			</div>
		</header>

		{#if !community.is_member}
			<div class="join-banner">
				<p>Join this community to post and participate.</p>
				<div class="join-form">
					<input type="text" bind:value={inviteCode} placeholder="Invite code (if required)" />
					<button class="join-btn" onclick={joinCommunity} disabled={joining}>
						{joining ? 'Joining...' : 'Join'}
					</button>
				</div>
				{#if joinError}<p class="join-error">{joinError}</p>{/if}
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

		{#if posts.length === 0}
			<div class="empty-state">
					<p class="empty-title">Nothing here yet</p>
					<p class="empty-body">That means there's room for you. Be the first to share what your community needs — or what you can offer.</p>
				</div>
		{:else}
			<ul class="post-list">
				{#each posts as post}
					<li class="post-card kind-{post.kind}" class:post-urgent={post.urgency === 'urgent' || post.urgency === 'critical'}>
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
							{#if editExistingImages.length > 0}
								<div class="edit-images">
									{#each editExistingImages as img}
										<img src={'/post-images/' + img} alt="" class="post-thumb" />
									{/each}
								</div>
							{/if}
							<div class="edit-image-previews">
								{#each editImagePreviews as preview, i}
									<div class="preview-item">
										<img src={preview} alt="" />
										<button type="button" class="remove-img" onclick={() => removeEditImage(i)}>&times;</button>
									</div>
								{/each}
							</div>
							{#if (editExistingImages.length + editImageFiles.length) < 5}
								<input type="file" accept="image/png,image/jpeg,image/webp" multiple onchange={handleEditImages} class="edit-file-input" />
								<button type="button" class="btn-ghost add-img-btn" onclick={() => (document.querySelector('.edit-file-input') as HTMLInputElement)?.click()}>
									Add images
								</button>
							{/if}
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
						{#if post.images?.length}
							<div class="post-images">
								{#each post.images.slice(0, 3) as img}
									<img src={'/post-images/' + img} alt="" class="post-thumb" />
								{/each}
								{#if post.images.length > 3}
									<span class="more-images">+{post.images.length - 3}</span>
								{/if}
							</div>
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
		{#if members.length > 0}
			<button class="members-toggle" onclick={() => showMembers = !showMembers}>
				{showMembers ? 'Hide' : 'Show'} members ({members.length})
			</button>
			{#if showMembers}
				<ul class="member-list">
					{#each members as member}
						<li>
							<span class="member-name">
								{#if member.user_id}
									<a href="/users/{member.user_id}">{member.display_name}</a>
								{:else}
									{member.display_name}
								{/if}
							</span>
							{#if member.role !== 'member'}
								<span class="member-role">{member.role}</span>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
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
	.community-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 1.5rem; gap: 1rem; }
	h1 { font-size: 1.5rem; margin-bottom: 0.25rem; }
	.desc { color: var(--text-muted); font-size: 0.9rem; }
	.location { color: var(--text-muted); font-size: 0.8rem; }
	.community-info { display: flex; align-items: flex-start; gap: 1rem; }
	.community-image { width: 72px; height: 72px; border-radius: var(--radius-md); object-fit: cover; border: 2px solid var(--border); flex-shrink: 0; }
	.header-actions { display: flex; gap: 0.5rem; }
	.btn-map { background: var(--kind-resource-soft); color: var(--kind-resource); padding: 0.5rem 1rem; border-radius: var(--radius); font-weight: 600; font-size: 0.9rem; border: 1px solid var(--kind-resource); }
	.btn-map:hover { text-decoration: none; }
	.btn-settings { background: var(--bg-elevated); color: var(--text); padding: 0.5rem 1rem; border-radius: var(--radius); font-weight: 600; font-size: 0.9rem; border: 1px solid var(--border); }
	.btn-settings:hover { text-decoration: none; border-color: var(--accent); }
	.btn-post { background: var(--accent); color: var(--text-on-accent); padding: 0.5rem 1rem; border-radius: var(--radius); font-weight: 600; font-size: 0.9rem; white-space: nowrap; }
	.btn-post:hover { text-decoration: none; }

	.post-card { background: var(--bg-surface); border: 1px solid transparent; border-radius: 2px 8px 2px 8px; padding: var(--space-4); box-shadow: 2px 3px 0 rgba(0,0,0,0.15), 4px 6px 12px rgba(0,0,0,0.2); transition: transform var(--transition-base), box-shadow var(--transition-base), border-color var(--transition-fast); }
	.post-card.kind-need:hover { border-color: var(--kind-need, var(--critical)); }
	.post-card.kind-offer:hover { border-color: var(--kind-offer, var(--success)); }
	.post-card.kind-resource:hover { border-color: var(--kind-resource); }
	.post-card:hover { transform: translateY(-3px) rotate(0deg); box-shadow: 3px 5px 0 rgba(0,0,0,0.2), 6px 10px 20px rgba(0,0,0,0.3); border-color: var(--accent); }
	.post-urgent { animation: urgent-pulse 2s ease-in-out infinite; }
	@keyframes urgent-pulse { 0%, 100% { box-shadow: 0 0 0 0 rgba(239, 68, 68, 0); } 50% { box-shadow: 0 0 0 4px rgba(239, 68, 68, 0.15); } }

	.post-meta { display: flex; gap: 0.5rem; align-items: center; margin-bottom: 0.5rem; font-size: 0.8rem; }
	.kind { padding: 0.15rem 0.5rem; border-radius: var(--radius-full); font-weight: 700; text-transform: uppercase; font-size: 0.65rem; letter-spacing: 0.3px; }
	.kind-need { background: var(--kind-need-soft); color: var(--critical); }
	.kind-offer { background: var(--kind-offer-soft); color: var(--success); }
	.kind-resource { background: var(--kind-resource-soft); color: var(--kind-resource); }
	.category { color: var(--text-muted); text-transform: capitalize; }
	.urgency { font-weight: 600; text-transform: uppercase; }
	.time { color: var(--text-muted); margin-left: auto; }
	h3 { font-size: 1.05rem; margin-bottom: 0.3rem; }
	.body { color: var(--text-muted); font-size: 0.9rem; }
	.loc { color: var(--text-muted); font-size: 0.8rem; }
	.post-images { display: flex; gap: 0.4rem; margin: 0.5rem 0; align-items: center; }
	.post-thumb { width: 80px; height: 80px; object-fit: cover; border-radius: 6px; border: 1px solid var(--border); }
	.more-images { font-size: 0.75rem; color: var(--text-muted); background: var(--bg-elevated); padding: 0.2rem 0.5rem; border-radius: 4px; }

	.join-banner p { color: var(--text-muted); font-size: 0.9rem; margin-bottom: 0.75rem; }
	.join-form { display: flex; gap: 0.5rem; }
	.join-form input { flex: 1; background: var(--bg); border: 1px solid var(--border); border-radius: var(--radius); padding: 0.5rem 0.75rem; color: var(--text); font-size: 0.9rem; }
	.join-btn { background: var(--accent); color: var(--text-on-accent); padding: 0.5rem 1rem; border-radius: var(--radius); font-weight: 600; font-size: 0.9rem; }
	.join-btn:disabled { opacity: 0.6; }
	.join-error { color: var(--critical); font-size: 0.8rem; margin-top: 0.5rem; }

	.search-bar { display: flex; position: relative; margin-bottom: var(--space-4); }
	.search-bar input { flex: 1; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 0.6rem 2rem 0.6rem 0.75rem; color: var(--text); font-size: 0.9rem; }
	.search-bar input:focus { outline: none; border-color: var(--accent); }
	.clear-search { position: absolute; right: 0.5rem; top: 50%; transform: translateY(-50%); background: none; color: var(--text-muted); font-size: 1.2rem; min-height: 30px; min-width: 30px; }

	.filters { display: flex; gap: 0.5rem; margin-bottom: var(--space-5); }
	.filters button { background: var(--bg-surface); color: var(--text-muted); padding: 0.4rem 0.8rem; border-radius: var(--radius-full); font-size: var(--text-xs); border: 1px solid var(--border); }
	.filters button.active { background: var(--accent-soft); color: var(--accent); border-color: var(--accent); }

	.post-list { list-style: none; display: flex; flex-direction: column; gap: 0.75rem; }
	.post-list :global(li:nth-child(odd) .post-card) { transform: rotate(-0.4deg); }
	.post-list :global(li:nth-child(even) .post-card) { transform: rotate(0.3deg); }
	.post-list :global(.post-card:hover) { transform: translateY(-3px) rotate(0deg) !important; }

	.post-footer { display: flex; justify-content: space-between; align-items: center; margin-top: 0.75rem; padding-top: 0.75rem; border-top: 1px solid var(--border); }
	.respond-btn { background: var(--accent); color: var(--text-on-accent); padding: 0.4rem 0.8rem; border-radius: var(--radius-full); font-weight: 600; font-size: 0.8rem; margin-left: auto; transition: transform var(--transition-fast); }
	.respond-btn:hover { transform: translateY(-1px); }
	.author-actions { display: flex; gap: 0.4rem; margin-left: auto; }
	.action-btn { padding: 0.3rem 0.6rem; border-radius: var(--radius-full); font-size: 0.75rem; font-weight: 600; border: 1px solid; }
	.action-btn.fulfill { background: var(--success-softer); color: var(--success); border-color: var(--success); }
	.action-btn.edit { background: var(--bg-elevated); color: var(--text-muted); border-color: var(--border); }
	.action-btn.delete { background: var(--critical-softer); color: var(--critical); border-color: var(--critical); }
	.fulfilled-badge { font-size: 0.75rem; color: var(--success); font-weight: 600; margin-left: auto; }

	.edit-form { display: flex; flex-direction: column; gap: 0.5rem; margin-bottom: 0.5rem; }
	.edit-form input, .edit-form textarea { background: var(--bg); border: 1px solid var(--border); border-radius: var(--radius); padding: 0.5rem; color: var(--text); font-size: 0.9rem; font-family: inherit; }
	.edit-images, .edit-image-previews { display: flex; flex-wrap: wrap; gap: 0.4rem; }
	.preview-item { position: relative; width: 64px; height: 64px; }
	.preview-item img { width: 100%; height: 100%; object-fit: cover; border-radius: 4px; border: 1px solid var(--border); }
	.remove-img { position: absolute; top: -6px; right: -6px; background: var(--critical); color: var(--text-on-critical); border-radius: 50%; width: 20px; height: 20px; font-size: 12px; line-height: 1; display: flex; align-items: center; justify-content: center; padding: 0; min-height: unset; min-width: unset; }
	.edit-file-input { display: none; }
	.add-img-btn { font-size: var(--text-xs); }
	.edit-actions { display: flex; gap: 0.4rem; }
	.save-btn { background: var(--accent); color: var(--text-on-accent); padding: 0.3rem 0.8rem; border-radius: var(--radius); font-size: 0.8rem; font-weight: 600; }
	.cancel-btn { background: var(--bg-elevated); color: var(--text-muted); padding: 0.3rem 0.8rem; border-radius: var(--radius); font-size: 0.8rem; border: 1px solid var(--border); }

	.members-toggle { background: var(--bg-surface); color: var(--text-muted); border: 1px solid var(--border); border-radius: var(--radius); padding: 0.5rem 1rem; font-size: 0.85rem; width: 100%; margin-top: 1.5rem; }
	.member-list { list-style: none; margin-top: 0.75rem; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-lg); overflow: hidden; }
	.member-list li { padding: 0.6rem 1rem; display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid var(--border); font-size: 0.9rem; }
	.member-list li:last-child { border-bottom: none; }
	.member-role { font-size: 0.7rem; color: var(--warning); text-transform: uppercase; font-weight: 600; }

	.status { text-align: center; color: var(--text-muted); padding: 3rem 0; }
	.error { color: var(--critical); }

	@media (max-width: 480px) {
		.community-header { flex-direction: column; gap: 0.75rem; }
		.btn-post { align-self: flex-start; }
		.filters { flex-wrap: wrap; }
		.post-footer { flex-direction: column; align-items: flex-start; gap: 0.5rem; }
		.author-actions { margin-left: 0; }
		.respond-btn { margin-left: 0; }
	}
</style>
