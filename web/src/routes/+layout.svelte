<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import Onboarding from '$lib/components/Onboarding.svelte';
	import { auth, isAuthenticated, getToken, refreshRole } from '$lib/stores/auth';
	import { serverState, getActiveServer } from '$lib/stores/server';

	let { children } = $props();
	let unreadCount = $state(0);
	let menuOpen = $state(false);

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

	function closeMenu() {
		menuOpen = false;
	}
</script>

<svelte:head>
	<title>Komun</title>
	<meta name="description" content="Federated mutual aid for intercommunal survival" />
</svelte:head>

<header>
	<nav class="container">
		<a href="/" class="logo" onclick={closeMenu}>komun</a>

		<button class="hamburger" class:open={menuOpen} onclick={() => menuOpen = !menuOpen} aria-label="Menu">
			<span></span><span></span><span></span>
		</button>

		<div class="nav-links" class:show={menuOpen}>
			{#if $serverState.active}
				<a href="/aid" onclick={closeMenu}>Aid</a>
				<a href="/community" onclick={closeMenu}>Community</a>
				{#if $auth.servers?.[$serverState.active]}
					<a href="/messages" onclick={closeMenu}>Messages</a>
					<a href="/notifications" class="notif-link" onclick={closeMenu}>
						Alerts
						{#if unreadCount > 0}
							<span class="notif-badge">{unreadCount}</span>
						{/if}
					</a>
					{#if $auth.servers[$serverState.active].role === 'superadmin'}
						<a href="/admin" class="admin-link" onclick={closeMenu}>Admin</a>
					{/if}
				{/if}
			{/if}
			<a href="/connect" onclick={closeMenu}>Connect</a>
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
		padding: 0.75rem 0;
		position: sticky;
		top: 0;
		background: var(--bg);
		z-index: 100;
	}

	nav {
		display: flex;
		align-items: center;
		justify-content: space-between;
		position: relative;
	}

	.logo {
		font-size: 1.4rem;
		font-weight: 700;
		color: var(--text);
		letter-spacing: -0.5px;
		z-index: 2;
	}

	.logo:hover {
		text-decoration: none;
		color: var(--accent);
	}

	.hamburger {
		display: none;
		flex-direction: column;
		gap: 5px;
		background: none;
		padding: 8px;
		z-index: 2;
		min-height: 44px;
		min-width: 44px;
		align-items: center;
		justify-content: center;
	}

	.hamburger span {
		display: block;
		width: 22px;
		height: 2px;
		background: var(--text);
		transition: all 0.2s;
	}

	.hamburger.open span:nth-child(1) {
		transform: rotate(45deg) translate(5px, 5px);
	}

	.hamburger.open span:nth-child(2) {
		opacity: 0;
	}

	.hamburger.open span:nth-child(3) {
		transform: rotate(-45deg) translate(5px, -5px);
	}

	.nav-links {
		display: flex;
		gap: 1.5rem;
		align-items: center;
	}

	.nav-links a {
		color: var(--text-muted);
		font-size: 0.9rem;
		padding: 0.25rem 0;
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
		padding: 1.5rem 0;
	}

	@media (max-width: 768px) {
		.hamburger {
			display: flex;
		}

		.nav-links {
			display: none;
			position: fixed;
			top: 50px;
			left: 0;
			right: 0;
			flex-direction: column;
			background: var(--bg-surface);
			border-bottom: 1px solid var(--border);
			padding: 1rem;
			gap: 0;
			z-index: 100;
		}

		.nav-links.show {
			display: flex;
		}

		.nav-links a {
			padding: 0.75rem 0;
			font-size: 1rem;
			border-bottom: 1px solid var(--border);
			width: 100%;
		}

		.nav-links a:last-of-type {
			border-bottom: none;
		}

		.identity {
			margin-top: 0.75rem;
			text-align: center;
		}

		.notif-badge {
			position: static;
			display: inline-flex;
			margin-left: 0.5rem;
		}
	}
</style>
