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

	let myUserId = $derived((() => {
		const server = getActiveServer();
		if (!server) return null;
		return $auth.servers?.[server]?.userId || null;
	})());

	const kindLabels: Record<string, string> = { resource: 'Resource', need: 'Need', offer: 'Offer' };
	const urgencyColors: Record<string, string> = { critical: 'var(--critical)', high: 'var(--warning)', medium: 'var(--text-muted)', low: 'var(--text-muted)' };

	function timeAgo(dateStr: string): string {
		const seconds = Math.floor((Date.now() - new Date(dateStr).getTime()) / 1000);
		if (seconds < 60) return 'just now';
		if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
		if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
		return `${Math.floor(seconds / 86400)}d ago`;
	}
</script>

<article class="aid-card">
	<div class="meta">
		<span class="kind kind-{post.kind}">{kindLabels[post.kind]}</span>
		<span class="category">{post.category}</span>
		{#if post.urgency}
			<span class="urgency" style="color: {urgencyColors[post.urgency]}">{post.urgency}</span>
		{/if}
		<span class="time">{timeAgo(post.created_at)}</span>
	</div>

	<h3>{post.title}</h3>

	{#if post.body}
		<p class="body">{post.body}</p>
	{/if}

	<div class="footer">
		<div class="origin">
			<span class="community">{post.community_name}</span>
			<span class="server">{post.server_name}</span>
		</div>

		{#if post.author_id !== myUserId}
			{#if post.kind === 'need'}
				<button class="respond-btn" onclick={() => showModal = true}>I can help</button>
			{:else if post.kind === 'offer'}
				<button class="respond-btn" onclick={() => showModal = true}>Request this</button>
			{/if}
		{:else}
			<span class="your-post">Your post</span>
		{/if}
	</div>
</article>

{#if showModal}
	<RespondModal
		post={{ id: post.id, title: post.title, kind: post.kind, server_url: post.server_url, community_slug: post.community_slug, author_id: post.author_id }}
		onClose={() => showModal = false}
	/>
{/if}

<style>
	.aid-card {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		padding: 1rem;
	}

	.meta {
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

	.body { color: var(--text-muted); font-size: 0.9rem; margin-bottom: 0.75rem; }

	.footer {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-top: 0.75rem;
		padding-top: 0.75rem;
		border-top: 1px solid var(--border);
	}

	.origin {
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
	}

	.community { font-size: 0.8rem; color: var(--text); font-weight: 500; }
	.server { font-size: 0.7rem; color: var(--text-muted); }

	.respond-btn {
		background: var(--accent);
		color: white;
		padding: 0.4rem 0.8rem;
		border-radius: var(--radius);
		font-weight: 600;
		font-size: 0.8rem;
	}

	.respond-btn:hover { opacity: 0.9; }

	.your-post {
		color: var(--text-muted);
		font-size: 0.75rem;
		font-style: italic;
	}

	@media (max-width: 480px) {
		.footer { flex-direction: column; align-items: flex-start; gap: 0.5rem; }
		.respond-btn { align-self: flex-start; }
	}
</style>
