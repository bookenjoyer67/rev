<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import { getActiveServer, resolveSlug } from '$lib/stores/server';
	import LinkPreview from '$lib/components/LinkPreview.svelte';

	interface Post {
		id: string;
		kind: string;
		category: string;
		title: string;
		body?: string;
		location_name?: string;
		urgency?: string;
		status?: string;
		author_id: string;
		images?: string[];
		contact_method?: string;
		created_at: string;
	}

	let post: Post | null = $state(null);
	let communityName = $state('');
	let error = $state('');
	let loading = $state(true);

	const urls: string[] = $derived(post?.body
		? [...post.body.matchAll(/https?:\/\/[^\s<>"]+/g)].map(m => m[0].replace(/[.,;:!?)]+$/, ''))
		: []);

	onMount(async () => {
		const rawSlug = $page.params.slug as string;
		const rawId = $page.params.id as string;
		try {
			const { localSlug } = await resolveSlug(rawSlug);
			const community = await api.communities.get(localSlug);
			communityName = community.name;
			post = await api.posts.get(localSlug, rawId);
		} catch (e: any) {
			error = e.message || 'Post not found';
		}
		loading = false;
	});

	function timeAgo(dateStr: string): string {
		const seconds = Math.floor((Date.now() - new Date(dateStr).getTime()) / 1000);
		if (seconds < 60) return 'just now';
		if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
		if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
		return `${Math.floor(seconds / 86400)}d ago`;
	}
</script>

<svelte:head>
	<title>{post?.title || 'Post'} — Komun</title>
	<meta name="description" content={post?.body?.slice(0, 200) || 'A post on Komun'} />
	<meta property="og:title" content={post?.title || 'Post'} />
	<meta property="og:description" content={post?.body?.slice(0, 200) || 'A post on Komun'} />
	<meta property="og:type" content="article" />
	<meta name="twitter:card" content="summary" />
</svelte:head>

<div class="container">
	{#if loading}
		<p class="status">Loading...</p>
	{:else if error}
		<p class="status error">{error}</p>
	{:else if post}
		<a href="/c/{$page.params.slug}" class="back">&larr; {communityName}</a>

		<article class="post-detail">
			<div class="meta">
				<span class="kind kind-{post.kind}">{post.kind}</span>
				<span class="category">{post.category}</span>
				{#if post.urgency}
					<span class="urgency">{post.urgency}</span>
				{/if}
				<span class="time">{timeAgo(post.created_at)}</span>
			</div>

			<h1>{post.title}</h1>

			{#if post.body}
				<p class="body">{post.body}</p>
			{/if}

			{#each urls as url}
				<LinkPreview {url} />
			{/each}

			{#if post.location_name}
				<p class="location">📍 {post.location_name}</p>
			{/if}

			{#if post.contact_method}
				<p class="contact">📞 {post.contact_method}</p>
			{/if}

			{#if post.images?.length}
				<div class="images">
					{#each post.images as img}
						<img src={'/post-images/' + img} alt="" />
					{/each}
				</div>
			{/if}
		</article>
	{/if}
</div>

<style>
	.container { max-width: 640px; margin: 0 auto; padding: 2rem 1rem; }
	.back { color: var(--text-muted); font-size: 0.85rem; display: inline-block; margin-bottom: 1.5rem; }
	.post-detail { background: var(--bg-surface); border: 1px solid var(--border); border-radius: 2px 8px 2px 8px; padding: var(--space-5); }
	.meta { display: flex; gap: 0.5rem; align-items: center; margin-bottom: 0.75rem; font-size: var(--text-xs); }
	.kind { padding: 0.15rem 0.5rem; border-radius: var(--radius-full); font-weight: 700; text-transform: uppercase; font-size: 0.65rem; letter-spacing: 0.3px; }
	.kind-need { background: var(--kind-need-soft); color: var(--kind-need, var(--critical)); }
	.kind-offer { background: var(--kind-offer-soft); color: var(--kind-offer, var(--success)); }
	.kind-resource { background: var(--kind-resource-soft); color: var(--kind-resource); }
	.category { color: var(--text-muted); text-transform: capitalize; }
	.urgency { font-weight: 700; text-transform: uppercase; color: var(--text-muted); }
	.time { color: var(--text-muted); margin-left: auto; }
	h1 { font-size: var(--text-2xl); margin-bottom: 0.75rem; }
	.body { color: var(--text); font-size: var(--text-base); line-height: 1.7; margin-bottom: var(--space-4); white-space: pre-wrap; }
	.location { color: var(--text-muted); font-size: var(--text-sm); margin-bottom: var(--space-2); }
	.contact { color: var(--text-muted); font-size: var(--text-sm); margin-bottom: var(--space-2); }
	.images { display: flex; flex-wrap: wrap; gap: 0.5rem; margin-top: var(--space-3); }
	.images img { max-width: 100%; max-height: 400px; border-radius: var(--radius-md); border: 1px solid var(--border); }
	.status { text-align: center; color: var(--text-muted); padding: 3rem 0; }
	.error { color: var(--critical); }
</style>
