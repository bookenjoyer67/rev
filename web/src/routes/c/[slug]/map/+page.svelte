<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { isConnected, getActiveServer } from '$lib/stores/server';
	import { auth } from '$lib/stores/auth';

	let slug = $state('');
	let relayUrl = $state('');
	let loading = $state(true);

	let displayName = $derived((() => {
		const server = getActiveServer();
		if (!server) return null;
		return $auth.servers?.[server]?.displayName || null;
	})());

	onMount(() => {
		if (!isConnected()) { goto('/connect'); return; }
		slug = $page.params.slug as string;
		const server = getActiveServer();
		if (server) {
			const wsUrl = server.replace('https://', 'wss://').replace('http://', 'ws://');
			relayUrl = wsUrl.replace(/:\d+/, ':9001');
			if (!relayUrl.includes(':9001')) {
				relayUrl = wsUrl + ':9001';
			}
		}
		loading = false;
	});

	let iframeSrc = $derived(
		`https://app.piggpin.space/?embed=1`
	);

	function handleMessage(event: MessageEvent) {
		if (event.origin !== 'https://app.piggpin.space') return;
		if (event.data?.type === 'piggpin:ready' && displayName) {
			const iframe = document.querySelector('iframe');
			iframe?.contentWindow?.postMessage({
				type: 'komun:identity',
				displayName
			}, 'https://app.piggpin.space');
		}
	}
</script>

<svelte:window onmessage={handleMessage} />

<div class="map-page">
	<header class="map-header">
		<a href="/c/{slug}" class="back">&larr; Back to community</a>
		<div class="relay-info">
			<span>Relay: {relayUrl}</span>
		</div>
	</header>

	{#if loading}
		<p class="status">Loading map...</p>
	{:else}
		<iframe
			src={iframeSrc}
			title="Community Map"
			allow="geolocation; clipboard-write"
			sandbox="allow-scripts allow-same-origin allow-popups allow-forms"
		></iframe>
	{/if}
</div>

<style>
	.map-page {
		display: flex;
		flex-direction: column;
		height: calc(100dvh - 60px);
	}

	.map-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.5rem 1rem;
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.back {
		color: var(--text-muted);
		font-size: 0.85rem;
	}

	.relay-info {
		font-size: 0.7rem;
		color: var(--text-muted);
	}

	iframe {
		flex: 1;
		width: 100%;
		border: none;
	}

	.status {
		text-align: center;
		color: var(--text-muted);
		padding: 3rem;
	}
</style>
