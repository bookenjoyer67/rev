<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import Onboarding from '$lib/components/Onboarding.svelte';
	import { auth, isAuthenticated, getToken, refreshRole } from '$lib/stores/auth';
	import { serverState, getActiveServer } from '$lib/stores/server';

	let { children } = $props();
	let unreadCount = $state(0);

	onMount(() => {
		refreshRole();
		pollNotifications();
		const interval = setInterval(pollNotifications, 15000);
		return () => clearInterval(interval);
	});

	async function pollNotifications() {
		if (!isAuthenticated()) return;
		const server = getActiveServer();
		const token = getToken();
		if (!server || !token) return;
		try {
			const res = await fetch(`${server}/api/me/notifications/count`, {
				headers: { 'Authorization': `Bearer ${token}` }
			});
			if (res.ok) {
				const data = await res.json();
				unreadCount = data.unread || 0;
			}
		} catch {}
	}
</script>

<svelte:head>
	<title>Komun</title>
	<meta name="description" content="Federated mutual aid for intercommunal survival" />
</svelte:head>

<header>
	<nav class="container">
		<a href="/" class="logo">komun</a>
		<div class="nav-links">
			{#if $serverState.active}
				<a href="/aid">Aid</a>
				<a href="/community">Community</a>
				{#if $auth.servers?.[$serverState.active]}
					<a href="/messages">Messages</a>
					<a href="/notifications" class="notif-link">
						Alerts
						{#if unreadCount > 0}
							<span class="notif-badge">{unreadCount}</span>
						{/if}
					</a>
					{#if $auth.servers[$serverState.active].role === 'superadmin'}
						<a href="/admin" class="admin-link">Admin</a>
					{/if}
				{/if}
			{/if}
			<a href="/connect">Connect</a>
			{#if $serverState.active && $auth.servers?.[$serverState.active]}
				<span class="identity">{$auth.servers[$serverState.active].displayName}</span>
			{/if}
		</div>
	</nav>
</header>

<main>
	{@render children()}
</main>

<Onboarding />

<style>
	header {
		border-bottom: 1px solid var(--border);
		padding: 1rem 0;
		position: sticky;
		top: 0;
		background: var(--bg);
		z-index: 100;
	}

	nav {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.logo {
		font-size: 1.4rem;
		font-weight: 700;
		color: var(--text);
		letter-spacing: -0.5px;
	}

	.logo:hover {
		text-decoration: none;
		color: var(--accent);
	}

	.nav-links {
		display: flex;
		gap: 1.5rem;
		align-items: center;
	}

	.nav-links a {
		color: var(--text-muted);
		font-size: 0.9rem;
	}

	.nav-links a:hover {
		color: var(--text);
		text-decoration: none;
	}

	.notif-link {
		position: relative;
	}

	.notif-badge {
		position: absolute;
		top: -8px;
		right: -12px;
		background: var(--critical);
		color: white;
		font-size: 0.65rem;
		font-weight: 700;
		min-width: 16px;
		height: 16px;
		border-radius: 8px;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0 4px;
	}

	.admin-link {
		color: var(--warning) !important;
		font-weight: 600;
	}

	.identity {
		color: var(--success);
		font-size: 0.85rem;
		font-weight: 600;
		padding: 0.25rem 0.6rem;
		background: #2ec4b615;
		border-radius: var(--radius);
	}

	main {
		padding: 2rem 0;
	}
</style>
