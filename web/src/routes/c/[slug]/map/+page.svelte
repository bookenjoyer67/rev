<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { isConnected, getActiveServer } from '$lib/stores/server';
	import { auth } from '$lib/stores/auth';
	import { api } from '$lib/api/client';

	let slug = $state('');
	let relayUrl = $state('');
	let mapCommunityId: string | null = $state(null);
	let mapSecretKey: string | null = $state(null);
	let communityName: string | null = $state(null);
	let iframeSrc: string | null = $state(null);
	let loading = $state(true);
	let error: string | null = $state(null);

	let displayName = $derived(getDisplayName());

	function getDisplayName(): string | null {
		const server = getActiveServer();
		if (!server) return null;
		return $auth.servers?.[server]?.displayName || null;
	}

	onMount(async () => {
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

		try {
			const community = await api.communities.get(slug);
			if (community.map_community_id) {
				mapCommunityId = community.map_community_id;
				communityName = community.name;
				if (community.map_secret_key) {
					mapSecretKey = community.map_secret_key;
				}
				const payload = {
					cid: community.map_community_id,
					n: community.name,
					r: relayUrl.replace('wss://', '').replace('ws://', ''),
				};
				const b64 = btoa(JSON.stringify(payload)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
				iframeSrc = `https://app.piggpin.space/?embed=1#community=${b64}`;
			}
		} catch (e: any) {
			error = e.message || 'Failed to load community map data';
		}

		loading = false;
	});

	function handleMessage(event: MessageEvent) {
		if (event.origin !== 'https://app.piggpin.space') return;
		const msg = event.data;
		if (msg?.type === 'piggpin:ready' && displayName) {
			const iframe = document.querySelector('iframe');
			const data: any = {
				type: 'komun:identity',
				displayName,
			};
			if (mapSecretKey) {
				data.communityPassword = mapSecretKey;
			}
			iframe?.contentWindow?.postMessage(data, 'https://app.piggpin.space');
		}
	}
</script>

<svelte:window onmessage={handleMessage} />

<div class="map-page">
	<header class="map-header">
		<a href="/c/{slug}" class="back">&larr; Back to community</a>
		<div class="relay-info">
			{#if relayUrl}
				<span>Relay: {relayUrl}</span>
			{/if}
		</div>
	</header>

	{#if loading}
		<p class="status">Loading map...</p>
	{:else if error}
		<p class="status error">{error}</p>
	{:else if !mapCommunityId}
		<p class="status">This community doesn't have a map yet. Create posts to enable the map feature.</p>
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

	.status.error {
		color: #dc2626;
	}
</style>
