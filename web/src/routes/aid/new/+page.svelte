<script lang="ts">
	import { goto } from '$app/navigation';
	import { requireAuth } from '$lib/stores/auth';
	import { api } from '$lib/api/client';
	import { getActiveServer } from '$lib/stores/server';
	import { onMount } from 'svelte';

	interface Community { slug: string; name: string; }

	let communities: Community[] = $state([]);
	let selectedCommunity = $state('');
	let kind = $state('need');
	let category = $state('other');
	let title = $state('');
	let body = $state('');
	let urgency = $state('medium');
	let expiresIn = $state('7');
	let locationName = $state('');
	let locationLat: number | null = $state(null);
	let locationLon: number | null = $state(null);
	let contactMethod = $state('');
	let error = $state('');
	let loading = $state(false);
	let showPicker = $state(false);
	let mapIframeSrc: string | null = $state(null);

	const expiryDefaults: Record<string, string> = { need: '7', offer: '14', resource: '0' };

	function setKind(k: string) {
		kind = k;
		expiresIn = expiryDefaults[k] || '7';
	}

	function getExpiresAt(): string | null {
		const days = parseInt(expiresIn);
		if (!days) return null;
		return new Date(Date.now() + days * 86400000).toISOString();
	}

	onMount(async () => {
		try {
			communities = await api.communities.list();
			if (communities.length > 0) {
				selectedCommunity = communities[0].slug;
			}
		} catch (e) {}

		const server = getActiveServer();
		if (server) {
			try {
				const nodeInfo = await fetch(`${server}/api/node`).then(r => r.json());
				const relayUrl = nodeInfo.relay_url || '';
				if (relayUrl) {
					for (const c of communities) {
						try {
							const community = await api.communities.get(c.slug);
							if (community.map_community_id) {
								const payload: Record<string, string> = {
									cid: community.map_community_id,
									n: community.name,
									r: relayUrl,
									pw: 'false',
								};
								if (community.map_secret_hex) {
									payload.sk = community.map_secret_hex;
								}
								if (community.location_lat != null && community.location_lon != null) {
									payload.lat = String(community.location_lat);
									payload.lon = String(community.location_lon);
									payload.zoom = '14';
								} else if (nodeInfo.location?.lat != null) {
									payload.lat = String(nodeInfo.location.lat);
									payload.lon = String(nodeInfo.location.lon);
									payload.zoom = '10';
								}
								const b64 = btoa(JSON.stringify(payload)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
							mapIframeSrc = `https://localhost:5174/?embed=1&picker=1#community=${b64}`;
								break;
							}
						} catch (_) {}
					}
				}
			} catch (_) {}
		}
	});

	function handlePickMessage(event: MessageEvent) {
		console.log('[rev] ANY msg:', event.origin, typeof event.data, event.data?.type);
		if (event.origin !== 'https://localhost:5174') return;
		console.log('[rev] pick message:', event.data);
		if (event.data?.type === 'piggpin:location-picked') {
			locationLat = event.data.lat;
			locationLon = event.data.lng;
			console.log('[rev] picked location:', locationLat, locationLon);
		}
	}

	function sendPinDetails() {
		const iframe = document.querySelector('iframe');
		if (iframe?.contentWindow) {
			try {
				iframe.contentWindow.postMessage({
					type: 'komun:pin-details',
					title: title,
					body: body,
					kind,
					category,
					urgency: kind === 'need' ? urgency : null,
					contact: contactMethod
				}, 'https://localhost:5174');
			} catch (_) {}
		}
	}

	function submit() {
		requireAuth(async () => {
			if (!selectedCommunity) { error = 'Select a community'; return; }
			if (!title.trim()) { error = 'Title is required'; return; }
			loading = true;
			error = '';
			try {
				await api.posts.create(selectedCommunity, {
					kind,
					category,
					title: title.trim(),
					body: body.trim() || null,
					urgency: kind === 'need' ? urgency : null,
					expires_at: getExpiresAt(),
					location_name: locationName.trim() || null,
					location_lat: locationLat,
					location_lon: locationLon,
					contact_method: contactMethod.trim() || null,
				});
				sendPinDetails();
				const iframe = document.querySelector('iframe');
				if (iframe?.contentWindow) {
					try { iframe.contentWindow.postMessage({ type: 'komun:submit' }, 'https://localhost:5174'); } catch (_) {}
				}
				goto(`/c/${selectedCommunity}`);
			} catch (e: any) {
				error = e.message || 'Failed to create post';
			} finally {
				loading = false;
			}
		});
	}
</script>

<svelte:window onmessage={handlePickMessage} />

<div class="container">
	<h1>New Post</h1>

	<form onsubmit={(e) => { e.preventDefault(); submit(); }}>
		<label>
			<span>Community</span>
			<select bind:value={selectedCommunity}>
				{#each communities as c}
					<option value={c.slug}>{c.name}</option>
				{/each}
			</select>
		</label>

		<label>
			<span>Type</span>
			<div class="kind-selector">
				<button type="button" class:active={kind === 'need'} onclick={() => setKind('need')}>Need</button>
				<button type="button" class:active={kind === 'offer'} onclick={() => setKind('offer')}>Offer</button>
				<button type="button" class:active={kind === 'resource'} onclick={() => setKind('resource')}>Resource</button>
			</div>
		</label>

		<label>
			<span>Category</span>
			<select bind:value={category}>
				<option value="food">Food</option>
				<option value="shelter">Shelter</option>
				<option value="health">Health</option>
				<option value="transport">Transport</option>
				<option value="education">Education</option>
				<option value="labor">Labor</option>
				<option value="legal">Legal</option>
				<option value="other">Other</option>
			</select>
		</label>

		<label>
			<span>Title</span>
			<input type="text" bind:value={title} oninput={sendPinDetails} placeholder="What do you need or offer?" maxlength="200" />
		</label>

		<label>
			<span>Details (optional)</span>
			<textarea bind:value={body} oninput={sendPinDetails} placeholder="More info..." rows="3"></textarea>
		</label>

		{#if kind === 'need'}
			<label>
				<span>Urgency</span>
				<select bind:value={urgency}>
					<option value="critical">Critical</option>
					<option value="high">High</option>
					<option value="medium">Medium</option>
					<option value="low">Low</option>
				</select>
			</label>
		{/if}

		<label>
			<span>Expires in</span>
			<select bind:value={expiresIn}>
				<option value="1">1 day</option>
				<option value="3">3 days</option>
				<option value="7">1 week</option>
				<option value="14">2 weeks</option>
				<option value="30">1 month</option>
				<option value="0">Never</option>
			</select>
		</label>

		<label>
			<span>Location (optional)</span>
			<input type="text" bind:value={locationName} placeholder="Neighborhood or area" />
		</label>

		{#if mapIframeSrc}
			<div class="picker-section">
				{#if locationLat != null && locationLon != null}
					<div class="picked-coords">
						Pinned: {locationLat.toFixed(5)}, {locationLon.toFixed(5)}
						<button type="button" class="clear-pin" onclick={() => { locationLat = null; locationLon = null; }}>×</button>
					</div>
				{:else}
					<button type="button" class="pick-toggle" onclick={() => { showPicker = !showPicker; if (showPicker) setTimeout(sendPinDetails, 2000); }}>
						📌 {showPicker ? 'Hide map' : 'Pin on map'}
					</button>
				{/if}
				{#if showPicker}
					<iframe
						src={mapIframeSrc}
						title="Pick a location on the map"
						allow="geolocation; clipboard-write"
					sandbox="allow-scripts allow-popups allow-same-origin"
					></iframe>
				{/if}
			</div>
		{/if}

		<label>
			<span>Contact method (optional)</span>
			<input type="text" bind:value={contactMethod} placeholder="How should people reach you?" />
		</label>

		{#if error}
			<p class="error">{error}</p>
		{/if}

		<button type="submit" disabled={loading}>
			{loading ? 'Posting...' : 'Post'}
		</button>
	</form>
</div>

<style>
	h1 {
		font-size: 1.5rem;
		margin-bottom: 1.5rem;
	}

	form {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
		max-width: 500px;
	}

	label {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}

	label span {
		font-size: 0.85rem;
		font-weight: 600;
		color: var(--text-muted);
	}

	input, textarea, select {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.75rem;
		color: var(--text);
		font-size: 1rem;
		font-family: inherit;
	}

	input:focus, textarea:focus, select:focus {
		outline: none;
		border-color: var(--accent);
	}

	.kind-selector {
		display: flex;
		gap: 0.5rem;
	}

	.kind-selector button {
		flex: 1;
		padding: 0.5rem;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		color: var(--text-muted);
		font-size: 0.9rem;
	}

	.kind-selector button.active {
		border-color: var(--accent);
		color: var(--accent);
		background: var(--accent-soft);
	}

	.picker-section {
		margin: -0.5rem 0;
	}

	.pick-toggle {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.5rem 0.75rem;
		color: var(--text);
		font-size: 0.85rem;
		cursor: pointer;
		width: 100%;
		text-align: left;
	}
	.pick-toggle:hover { border-color: var(--accent); }

	.picker-section iframe {
		width: 100%;
		height: 250px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		margin-top: 0.5rem;
	}

	.picked-coords {
		font-size: 0.8rem;
		color: var(--accent);
		margin-bottom: 0;
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.clear-pin {
		background: none;
		border: 1px solid var(--border);
		border-radius: 3px;
		color: var(--text-muted);
		cursor: pointer;
		font-size: 0.75rem;
		padding: 0 4px;
		line-height: 1.2;
	}

	button[type="submit"] {
		background: var(--accent);
		color: var(--text-on-accent);
		padding: 0.75rem;
		border-radius: var(--radius);
		font-weight: 600;
		font-size: 1rem;
		margin-top: 0.5rem;
	}

	button[type="submit"]:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.error {
		color: var(--critical);
		font-size: 0.85rem;
	}
</style>
