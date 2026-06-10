<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { isSuperadmin } from '$lib/stores/auth';
	import { isConnected } from '$lib/stores/server';
	import { api } from '$lib/api/client';

	interface Community {
		id: string;
		slug: string;
		name: string;
		description?: string;
		visibility: string;
		created_at: string;
	}

	let communities: Community[] = $state([]);
	let loading = $state(true);

	onMount(() => {
		if (!isConnected() || !isSuperadmin()) { goto('/'); return; }
		loadCommunities();
	});

	async function loadCommunities() {
		try { communities = await api.admin.listCommunities(); } catch {}
		loading = false;
	}

	async function deleteCommunity(comm: Community) {
		if (!confirm(`Delete community "${comm.name}"? All posts and data will be lost.`)) return;
		await api.admin.deleteCommunity(comm.id);
		communities = communities.filter((c) => c.id !== comm.id);
	}
</script>

<div class="container">
	<header class="page-header">
		<div>
			<a href="/admin" class="back">&larr; Admin</a>
			<h1>Communities</h1>
		</div>
	</header>

	{#if loading}
		<p class="status">Loading...</p>
	{:else if communities.length === 0}
		<p class="status">No communities.</p>
	{:else}
		<table>
			<thead>
				<tr>
					<th>Name</th>
					<th>Slug</th>
					<th>Visibility</th>
					<th>Created</th>
					<th>Actions</th>
				</tr>
			</thead>
			<tbody>
				{#each communities as comm}
					<tr>
						<td>
							<a href="/c/{comm.slug}">{comm.name}</a>
							{#if comm.description}
								<span class="desc">{comm.description}</span>
							{/if}
						</td>
						<td class="muted">/c/{comm.slug}</td>
						<td><span class="vis vis-{comm.visibility}">{comm.visibility}</span></td>
						<td class="muted">{new Date(comm.created_at).toLocaleDateString()}</td>
						<td>
							<button class="delete-btn" onclick={() => deleteCommunity(comm)}>Delete</button>
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	{/if}
</div>

<style>
	.page-header { margin-bottom: 1.5rem; }
	.back { color: var(--text-muted); font-size: 0.8rem; }
	h1 { font-size: 1.5rem; margin-top: 0.25rem; }

	table {
		width: 100%;
		border-collapse: collapse;
	}

	th {
		text-align: left;
		font-size: 0.75rem;
		color: var(--text-muted);
		text-transform: uppercase;
		padding: 0.5rem;
		border-bottom: 1px solid var(--border);
	}

	td {
		padding: 0.75rem 0.5rem;
		border-bottom: 1px solid var(--border);
		font-size: 0.9rem;
	}

	.muted { color: var(--text-muted); font-size: 0.8rem; }
	.desc { display: block; color: var(--text-muted); font-size: 0.75rem; }

	.vis {
		font-size: 0.75rem;
		padding: 0.15rem 0.4rem;
		border-radius: 4px;
	}

	.vis-public { background: #2ec4b620; color: var(--success); }
	.vis-federated { background: #6366f120; color: #818cf8; }
	.vis-private { background: #f4a26120; color: var(--warning); }

	.delete-btn {
		background: none;
		color: var(--critical);
		font-size: 0.8rem;
		padding: 0.2rem 0.5rem;
		border: 1px solid var(--critical);
		border-radius: var(--radius);
	}

	.delete-btn:hover { background: #e6394620; }

	.status { text-align: center; color: var(--text-muted); padding: 3rem 0; }
</style>
