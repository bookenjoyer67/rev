import { getDirectories } from '$lib/stores/directories';
import { getLocation } from '$lib/stores/location';

export interface NearbyServer {
	url: string;
	name: string;
	description?: string;
	location_name?: string;
	communities_count: number;
	distance_km?: number;
	matched_community?: {
		slug: string;
		name: string;
		location_name?: string;
	};
}

export interface DiscoveredCommunity {
	slug: string;
	name: string;
	description?: string;
	server_url: string;
	server_name: string;
}

export interface AggregatedPost {
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
	server_url: string;
	server_name: string;
	server_location?: string;
	community_slug: string;
	community_name: string;
}

export interface DiscoveryResult {
	servers: NearbyServer[];
	communities: DiscoveredCommunity[];
	posts: AggregatedPost[];
}

export async function discoverNearbyServers(): Promise<NearbyServer[]> {
	const dirs = getDirectories();
	const loc = getLocation();

	if (!loc.lat || !loc.lon) return [];

	const results = await Promise.allSettled(
		dirs.map(async (dirUrl) => {
			const url = `${dirUrl}/api/directory?lat=${loc.lat}&lon=${loc.lon}&radius=50`;
			const res = await fetch(url);
			if (!res.ok) return [];
			const data = await res.json();
			return data.map((entry: any) => ({
				url: entry.url,
				name: entry.name,
				description: entry.description,
				location_name: entry.location_name,
				communities_count: entry.communities_count || 0,
				distance_km: entry.distance_km,
				matched_community: entry.matched_community,
			}));
		})
	);

	const allServers: NearbyServer[] = [];
	const seen = new Set<string>();

	for (const result of results) {
		if (result.status === 'fulfilled') {
			for (const server of result.value) {
				const mc = server.matched_community
					? { slug: server.matched_community.slug, name: server.matched_community.name, location_name: server.matched_community.location_name }
					: undefined;
				const key = mc ? `${server.url}#${mc.slug}` : server.url;
				if (!seen.has(key)) {
					seen.add(key);
					allServers.push(server);
				}
			}
		}
	}

	allServers.sort((a, b) => (a.distance_km ?? 999) - (b.distance_km ?? 999));
	return allServers.slice(0, 5);
}

export async function discoverAllServers(): Promise<NearbyServer[]> {
	const dirs = getDirectories();

	const results = await Promise.allSettled(
		dirs.map(async (dirUrl) => {
			const res = await fetch(`${dirUrl}/api/directory`);
			if (!res.ok) return [];
			return (await res.json()).map((entry: any) => ({
				url: entry.url,
				name: entry.name,
				description: entry.description,
				location_name: entry.location_name,
				communities_count: entry.communities_count || 0,
			}));
		})
	);

	const allServers: NearbyServer[] = [];
	const seen = new Set<string>();

	for (const result of results) {
		if (result.status === 'fulfilled') {
			for (const server of result.value) {
				if (!seen.has(server.url)) {
					seen.add(server.url);
					allServers.push(server);
				}
			}
		}
	}

	return allServers;
}

export async function fetchFromServers(servers: NearbyServer[]): Promise<DiscoveryResult> {
	const allPosts: AggregatedPost[] = [];
	const allCommunities: DiscoveredCommunity[] = [];

	const results = await Promise.allSettled(
		servers.map(async (server) => {
			const commRes = await fetch(`${server.url}/api/communities`);
			if (!commRes.ok) return { posts: [], communities: [] };

			const communities: any[] = await commRes.json();
			const posts: AggregatedPost[] = [];
			const comms: DiscoveredCommunity[] = [];

			const toFetch: { slug: string; name: string; description?: string }[] = communities.slice(0, 5).map((c: any) => ({
				slug: c.slug,
				name: c.name,
				description: c.description,
			}));

			if (server.matched_community) {
				const mc = server.matched_community;
				if (!toFetch.some((c) => c.slug === mc.slug)) {
					toFetch.push({ slug: mc.slug, name: mc.name });
				}
			}

			for (const comm of toFetch) {
				if (!comms.some((c) => c.slug === comm.slug)) {
					comms.push({
						slug: comm.slug,
						name: comm.name,
						description: comm.description,
						server_url: server.url,
						server_name: server.name,
					});
				}

				try {
					const postsRes = await fetch(`${server.url}/api/communities/${comm.slug}/posts`);
					if (!postsRes.ok) continue;
					const communityPosts: any[] = await postsRes.json();

					for (const p of communityPosts.slice(0, 10)) {
						posts.push({
							...p,
							server_url: server.url,
							server_name: server.name,
							server_location: server.location_name,
							community_slug: comm.slug,
							community_name: comm.name,
						});
					}
				} catch { /* skip */ }
			}

			return { posts, communities: comms };
		})
	);

	for (const result of results) {
		if (result.status === 'fulfilled') {
			allPosts.push(...result.value.posts);
			allCommunities.push(...result.value.communities);
		}
	}

	allPosts.sort((a, b) => {
		const urgencyOrder: Record<string, number> = { critical: 0, high: 1, medium: 2, low: 3 };
		const ua = urgencyOrder[a.urgency || 'low'] ?? 3;
		const ub = urgencyOrder[b.urgency || 'low'] ?? 3;
		if (ua !== ub) return ua - ub;
		return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
	});

	return { servers, communities: allCommunities, posts: allPosts };
}
