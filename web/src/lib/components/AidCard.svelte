<script lang="ts">
	import type { AggregatedPost } from '$lib/api/discovery';
	import RespondModal from './RespondModal.svelte';
	import { auth } from '$lib/stores/auth';
	import { getActiveServer } from '$lib/stores/server';

	interface Props {
		post: AggregatedPost;
	}

	let { post }: Props = $props();
	let showModal = $state(false);
	let showMap = $state(false);

	let myUserId = $derived((() => {
		const server = getActiveServer();
		if (!server) return null;
		return $auth.servers?.[server]?.userId || null;
	})());

	const kindLabels: Record<string, string> = { resource: 'Resource', need: 'Need', offer: 'Offer' };

	function timeAgo(dateStr: string): string {
		const seconds = Math.floor((Date.now() - new Date(dateStr).getTime()) / 1000);
		if (seconds < 60) return 'just now';
		if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
		if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
		return `${Math.floor(seconds / 86400)}d ago`;
	}
</script>

<article class="aid-card">
	<div class="card-top">
		<span class="kind kind-{post.kind}">{kindLabels[post.kind]}</span>
		<span class="category">{post.category}</span>
		{#if post.urgency}
			<span class="urgency" data-level={post.urgency}>{post.urgency}</span>
		{/if}
		<span class="time">{timeAgo(post.created_at)}</span>
	</div>

	<h3>{post.title}</h3>

	{#if post.body}
		<p class="body">{post.body}</p>
	{/if}

	{#if post.location_name}
		<p class="location">📍 {post.location_name}</p>
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

	<div class="footer">
		<span class="community">{post.community_name}</span>
		<div class="footer-actions">
			{#if post.location_name || (post.location_lat != null && post.location_lon != null)}
				<button class="map-btn" onclick={() => showMap = true} title="View on map">📍</button>
			{/if}
			{#if post.author_id !== myUserId}
				<button class="btn-primary respond-btn" onclick={() => showModal = true}>
					{#if post.kind === 'need'}I can help{:else if post.kind === 'offer'}Request this{:else}Respond{/if}
				</button>
				{:else}
				<span class="your-post">Your post</span>
				{/if}
				</div>
				</div>
				</article>

				{#if showMap}
				<div class="map-overlay" role="dialog" onclick={() => showMap = false}>
				<div class="map-popout" onclick={(e) => e.stopPropagation()}>
				<button class="map-close" onclick={() => showMap = false}>&times;</button>
				<iframe
					title="Post location"
					src={post.location_lat != null && post.location_lon != null
						? `https://www.openstreetmap.org/export/embed.html?bbox=${post.location_lon - 0.01},${post.location_lat - 0.005},${post.location_lon + 0.01},${post.location_lat + 0.005}&layer=mapnik&marker=${post.location_lat},${post.location_lon}`
						: `https://www.openstreetmap.org/export/embed.html?bbox=-0.01,-0.005,0.01,0.005&layer=mapnik`}
				></iframe>
				</div>
				</div>
				{/if}

{#if showModal}
	<RespondModal
		post={{ id: post.id, title: post.title, kind: post.kind, server_url: post.server_url, community_slug: post.community_slug, author_id: post.author_id }}
		onClose={() => showModal = false}
	/>
{/if}

<style>
	.aid-card {
		background: var(--bg-surface);
		border: 1px solid transparent;
		border-radius: 2px 8px 2px 8px;
		padding: var(--space-4);
		box-shadow: 2px 3px 0 rgba(0,0,0,0.15), 4px 6px 12px rgba(0,0,0,0.2);
		transition: transform var(--transition-base), box-shadow var(--transition-base), border-color var(--transition-fast);
	}

	.aid-card:hover {
		transform: translateY(-3px) rotate(0deg);
		box-shadow: 3px 5px 0 rgba(0,0,0,0.2), 6px 10px 20px rgba(0,0,0,0.3);
		border-color: var(--accent);
	}

	.card-top {
		display: flex;
		gap: 0.5rem;
		align-items: center;
		margin-bottom: 0.5rem;
		font-size: var(--text-xs);
	}

	.kind {
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-full);
		font-weight: 700;
		text-transform: uppercase;
		font-size: 0.65rem;
		letter-spacing: 0.3px;
	}

	.kind-need { background: var(--kind-need-soft); color: var(--kind-need, var(--critical)); }
	.kind-offer { background: var(--kind-offer-soft); color: var(--kind-offer, var(--success)); }
	.kind-resource { background: var(--kind-resource-soft); color: var(--kind-resource); }

	.category { color: var(--text-muted); text-transform: capitalize; font-size: 0.7rem; }

	.urgency {
		font-weight: 700;
		text-transform: uppercase;
		font-size: 0.65rem;
	}

	.urgency[data-level="critical"] { color: var(--critical); }
	.urgency[data-level="high"] { color: var(--warning); }
	.urgency[data-level="medium"],
	.urgency[data-level="low"] { color: var(--text-muted); }

	.time {
		color: var(--text-muted);
		margin-left: auto;
	}

	h3 {
		font-size: var(--text-lg);
		margin-bottom: 0.3rem;
		font-weight: 700;
	}

	.body {
		color: var(--text-muted);
		font-size: var(--text-sm);
		margin-bottom: var(--space-3);
		line-height: 1.5;
	}

	.location {
		color: var(--text-muted);
		font-size: var(--text-xs);
		margin-bottom: var(--space-2);
	}

	.footer {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-top: var(--space-3);
		padding-top: var(--space-3);
		border-top: 1px solid var(--border);
	}

	.footer-actions {
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}

	.map-btn {
		background: var(--bg-elevated);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		padding: 0.25rem 0.5rem;
		font-size: 0.9rem;
		min-height: unset;
		min-width: unset;
		cursor: pointer;
		transition: border-color var(--transition-fast);
	}

	.map-btn:hover { border-color: var(--accent); }

	.map-overlay {
		position: fixed;
		inset: 0;
		background: var(--overlay);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
		padding: 1rem;
	}

	.map-popout {
		position: relative;
		width: 100%;
		max-width: 500px;
		aspect-ratio: 1;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		overflow: hidden;
	}

	.map-popout iframe {
		width: 100%;
		height: 100%;
		border: none;
	}

	.map-close {
		position: absolute;
		top: 0.5rem;
		right: 0.5rem;
		z-index: 1;
		background: var(--bg-surface);
		color: var(--text);
		border: 1px solid var(--border);
		border-radius: 50%;
		width: 28px;
		height: 28px;
		font-size: 1rem;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0;
		min-height: unset;
		min-width: unset;
	}

	.community {
		font-size: var(--text-sm);
		color: var(--text);
		font-weight: 600;
	}

	.respond-btn {
		font-size: var(--text-xs);
		padding: var(--space-1) var(--space-3);
	}

	.your-post {
		color: var(--text-muted);
		font-size: var(--text-xs);
		font-style: italic;
	}

	.post-images { display: flex; gap: 0.4rem; margin: 0.5rem 0; align-items: center; }
	.post-thumb { width: 72px; height: 72px; object-fit: cover; border-radius: 6px; border: 1px solid var(--border); }
	.more-images { font-size: 0.75rem; color: var(--text-muted); background: var(--bg-elevated); padding: 0.2rem 0.5rem; border-radius: 4px; }

	@media (max-width: 480px) {
		.footer {
			flex-direction: column;
			align-items: flex-start;
			gap: 0.5rem;
		}
	}
</style>
