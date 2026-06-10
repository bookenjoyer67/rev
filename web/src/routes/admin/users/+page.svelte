<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { isSuperadmin, auth } from '$lib/stores/auth';
	import { isConnected, getActiveServer } from '$lib/stores/server';
	import { api } from '$lib/api/client';

	interface User {
		id: string;
		display_name: string;
		role: string;
		last_seen?: string;
		created_at: string;
	}

	let users: User[] = $state([]);
	let loading = $state(true);

	let myUserId = $derived((() => {
		const server = getActiveServer();
		if (!server) return null;
		return $auth.servers?.[server]?.userId || null;
	})());

	onMount(() => {
		if (!isConnected() || !isSuperadmin()) { goto('/'); return; }
		loadUsers();
	});

	async function loadUsers() {
		try { users = await api.admin.listUsers(); } catch {}
		loading = false;
	}

	async function changeRole(user: User, newRole: string) {
		if (user.id === myUserId) return;
		await api.admin.changeRole(user.id, newRole);
		user.role = newRole;
		users = [...users];
	}

	async function deleteUser(user: User) {
		if (user.id === myUserId) return;
		if (!confirm(`Delete user "${user.display_name}"? This cannot be undone.`)) return;
		await api.admin.deleteUser(user.id);
		users = users.filter((u) => u.id !== user.id);
	}

	function timeAgo(dateStr?: string): string {
		if (!dateStr) return 'never';
		const seconds = Math.floor((Date.now() - new Date(dateStr).getTime()) / 1000);
		if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
		if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
		return `${Math.floor(seconds / 86400)}d ago`;
	}
</script>

<div class="container">
	<header class="page-header">
		<div>
			<a href="/admin" class="back">&larr; Admin</a>
			<h1>Users</h1>
		</div>
	</header>

	{#if loading}
		<p class="status">Loading...</p>
	{:else}
		<table>
			<thead>
				<tr>
					<th>Name</th>
					<th>Role</th>
					<th>Last Seen</th>
					<th>Created</th>
					<th>Actions</th>
				</tr>
			</thead>
			<tbody>
				{#each users as user}
					<tr>
						<td>
							{user.display_name}
							{#if user.id === myUserId}<span class="you">(you)</span>{/if}
						</td>
						<td>
							<select
								value={user.role}
								onchange={(e) => changeRole(user, (e.target as HTMLSelectElement).value)}
								disabled={user.id === myUserId}
							>
								<option value="user">user</option>
								<option value="admin">admin</option>
								<option value="superadmin">superadmin</option>
							</select>
						</td>
						<td class="muted">{timeAgo(user.last_seen)}</td>
						<td class="muted">{new Date(user.created_at).toLocaleDateString()}</td>
						<td>
							{#if user.id !== myUserId}
								<button class="delete-btn" onclick={() => deleteUser(user)}>Delete</button>
							{/if}
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
	.you { color: var(--text-muted); font-size: 0.75rem; }

	select {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		color: var(--text);
		padding: 0.25rem 0.5rem;
		font-size: 0.8rem;
	}

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
