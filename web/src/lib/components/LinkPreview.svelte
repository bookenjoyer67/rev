<script lang="ts">
	import { onMount } from 'svelte';

	interface Props {
		url: string;
	}

	let { url }: Props = $props();
	let loading = $state(true);
	let title: string | null = $state(null);
	let description: string | null = $state(null);
	let image: string | null = $state(null);

	onMount(async () => {
		try {
			const res = await fetch(`/api/link-preview?url=${encodeURIComponent(url)}`);
			if (res.ok) {
				const data = await res.json();
				title = data.title || null;
				description = data.description || null;
				image = data.image || null;
			}
		} catch {}
		loading = false;
	});
</script>

{#if !loading && (title || description)}
	<a href={url} target="_blank" rel="noopener" class="link-preview">
		{#if image}
			<img src={image} alt="" class="preview-img" />
		{/if}
		<div class="preview-text">
			{#if title}
				<span class="preview-title">{title}</span>
			{/if}
			{#if description}
				<span class="preview-desc">{description}</span>
			{/if}
			<span class="preview-url">{url.replace(/^https?:\/\//, '').replace(/\/$/, '')}</span>
		</div>
	</a>
{/if}

<style>
	.link-preview {
		display: flex;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		overflow: hidden;
		margin: 0.5rem 0;
		text-decoration: none;
		color: var(--text);
		transition: border-color var(--transition-fast);
	}
	.link-preview:hover {
		border-color: var(--accent);
		text-decoration: none;
	}
	.preview-img {
		width: 100px;
		height: 100px;
		object-fit: cover;
		flex-shrink: 0;
		border-right: 1px solid var(--border);
	}
	.preview-text {
		padding: 0.5rem 0.75rem;
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		overflow: hidden;
		min-width: 0;
	}
	.preview-title {
		font-size: 0.8rem;
		font-weight: 600;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.preview-desc {
		font-size: 0.7rem;
		color: var(--text-muted);
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.preview-url {
		font-size: 0.65rem;
		color: var(--text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
</style>
