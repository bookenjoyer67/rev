<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import LocationBar from '$lib/components/LocationBar.svelte';
	import AidCard from '$lib/components/AidCard.svelte';
	import { location, hasLocation } from '$lib/stores/location';
	import { isConnected, connectToServer } from '$lib/stores/server';
	import { discoverNearbyServers, fetchFromServers, type AggregatedPost, type NearbyServer, type DiscoveredCommunity } from '$lib/api/discovery';

	let posts: AggregatedPost[] = $state([]);
	let servers: NearbyServer[] = $state([]);
	let communities: DiscoveredCommunity[] = $state([]);
	let loading = $state(false);
	let searched = $state(false);
	let filter = $state('all');

	onMount(() => {
		if (hasLocation()) {
			loadFeed();
		}
	});

	async function loadFeed() {
		loading = true;
		searched = true;
		try {
			servers = await discoverNearbyServers();
			if (servers.length > 0) {
				const result = await fetchFromServers(servers);
				posts = result.posts;
				communities = result.communities;
			} else {
				posts = [];
				communities = [];
			}
		} catch (e) {
			posts = [];
			communities = [];
		}
		loading = false;
	}

	function onLocationSet() {
		loadFeed();
	}

	$effect(() => {
		if ($location.lat && $location.lon && !searched) {
			loadFeed();
		}
	});

	let filteredPosts = $derived(
		filter === 'all' ? posts : posts.filter((p) => p.kind === filter)
	);

	async function startCommunity() {
		if (isConnected()) {
			goto('/community/create');
		} else if (servers.length > 0) {
			await connectToServer(servers[0].url);
			goto('/community/create');
		} else {
			goto('/connect');
		}
	}
</script>

<div class="container">
	{#if !hasLocation() && !$location.lat}
		<section class="hero">
			<h1>komun</h1>
			<p class="tagline">Find mutual aid near you</p>
			<div class="location-prompt">
				<LocationBar {onLocationSet} />
			</div>
			<p class="manual-link">
				<a href="/connect">Or connect to a specific server</a>
			</p>
			<p class="manual-link">
				Looking to organize? <button class="link-btn" onclick={startCommunity}>Start a community</button>
			</p>
		</section>
	{:else}
		<div class="feed-header">
			<LocationBar {onLocationSet} />
		</div>

		<div class="filters">
			<button class:active={filter === 'all'} onclick={() => filter = 'all'}>All</button>
			<button class:active={filter === 'need'} onclick={() => filter = 'need'}>Needs</button>
			<button class:active={filter === 'offer'} onclick={() => filter = 'offer'}>Offers</button>
			<button class:active={filter === 'resource'} onclick={() => filter = 'resource'}>Resources</button>
		</div>

		{#if loading}
			<p class="status">Searching for aid nearby...</p>
		{:else if servers.length === 0}
			<div class="empty">
				<p>No communities found nearby. Be the first to organize mutual aid.</p>
				<button class="start-btn" onclick={startCommunity}>Start a community</button>
				<p class="sub">Or <a href="/connect">browse available servers</a> to join an existing one.</p>
			</div>
		{:else if filteredPosts.length === 0 && communities.length === 0}
			<div class="empty">
				<p>No communities here yet. Be the first to organize mutual aid.</p>
				<button class="start-btn" onclick={startCommunity}>Start a community</button>
				<p class="sub">Found {servers.length} server{servers.length > 1 ? 's' : ''} nearby.</p>
			</div>
		{:else if filteredPosts.length === 0}
			<div class="empty">
				<p>Found {communities.length} communit{communities.length > 1 ? 'ies' : 'y'} nearby, but no posts yet.</p>
				<div class="community-links">
					{#each communities as comm}
						<a href="/c/{comm.slug}" class="community-link" onclick={() => connectToServer(comm.server_url)}>
							{comm.name}
						</a>
					{/each}
				</div>
				<button class="start-btn" onclick={startCommunity}>Start another community</button>
			</div>
		{:else}
			<ul class="feed">
				{#each filteredPosts as post}
					<li><AidCard {post} /></li>
				{/each}
			</ul>
			<p class="feed-footer">
				Showing aid from {servers.length} nearby communit{servers.length > 1 ? 'ies' : 'y'}
				&middot; <button class="link-btn" onclick={startCommunity}>Start a community</button>
			</p>
		{/if}
	{/if}
</div>

<style>
	.hero {
		text-align: center;
		padding: 4rem 0 2rem;
		max-width: 440px;
		margin: 0 auto;
	}

	h1 {
		font-size: 2.5rem;
		font-weight: 800;
		letter-spacing: -1px;
		margin-bottom: 0.5rem;
	}

	.tagline {
		color: var(--text-muted);
		font-size: 1.1rem;
		margin-bottom: 2rem;
	}

	.location-prompt {
		margin-bottom: 1.5rem;
	}

	.manual-link {
		font-size: 0.85rem;
		color: var(--text-muted);
	}

	.feed-header {
		margin-bottom: 1rem;
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

	.feed {
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.feed-footer {
		text-align: center;
		color: var(--text-muted);
		font-size: 0.8rem;
		margin-top: 1.5rem;
		padding-top: 1rem;
		border-top: 1px solid var(--border);
	}

	.status {
		text-align: center;
		color: var(--text-muted);
		padding: 3rem 0;
	}

	.empty {
		text-align: center;
		padding: 3rem 0;
	}

	.empty p { color: var(--text-muted); }
	.empty .sub { font-size: 0.85rem; margin-top: 0.5rem; }

	.community-links {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		justify-content: center;
		margin: 1rem 0;
	}

	.community-link {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.5rem 1rem;
		font-size: 0.9rem;
		color: var(--text);
	}

	.community-link:hover {
		border-color: var(--accent);
		text-decoration: none;
	}

	.start-btn {
		background: var(--accent);
		color: white;
		padding: 0.75rem 1.5rem;
		border-radius: var(--radius);
		font-weight: 600;
		font-size: 1rem;
		margin-top: 1rem;
	}

	.link-btn {
		background: none;
		color: var(--accent);
		font-size: inherit;
		text-decoration: underline;
		padding: 0;
	}

	.link-btn:hover { opacity: 0.8; }
</style>
