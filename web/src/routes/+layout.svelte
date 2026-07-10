<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import Onboarding from '$lib/components/Onboarding.svelte';
	import { auth, isAuthenticated, getToken, refreshRole, initAuth } from '$lib/stores/auth';
	import { serverState, getActiveServer } from '$lib/stores/server';
	import { initTheme } from '$lib/stores/theme';

	let { children } = $props();
	let unreadCount = $state(0);
	let menuOpen = $state(false);
	let online = $state(true);
	let offlineDismissed = $state(false);
	let installPrompt: any = $state(null);
	let showInstall = $state(false);

	onMount(() => {
		online = navigator.onLine;
		window.addEventListener('online', () => { online = true; offlineDismissed = false; });
		window.addEventListener('offline', () => { online = false; offlineDismissed = false; });

		initTheme();
		initAuth();

		if (!window.matchMedia('(display-mode: standalone)').matches) {
			window.addEventListener('beforeinstallprompt', (e: Event) => {
				e.preventDefault();
				installPrompt = e;
				showInstall = true;
			});
		}

		refreshRole();
		pollNotifications();
		const interval = setInterval(pollNotifications, 15000);
		return () => clearInterval(interval);
	});

	function handleInstall() {
		installPrompt?.prompt();
		showInstall = false;
	}

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
				<a href="/account" class="identity" onclick={closeMenu}>{$auth.servers[$serverState.active].displayName}</a>
			{/if}
		</div>
	</nav>
</header>

{#if !online && !offlineDismissed}
	<div class="offline-banner">
		<span>You're offline. Showing cached data.</span>
		<button class="dismiss-btn" onclick={() => offlineDismissed = true}>&times;</button>
	</div>
{/if}

<main>
	{@render children()}
</main>

{#if showInstall}
	<div class="install-banner">
		<span>Install Komun for faster access</span>
		<button class="install-btn" onclick={handleInstall}>Install</button>
		<button class="dismiss-btn" onclick={() => showInstall = false}>&times;</button>
	</div>
{/if}

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
		color: var(--text-on-critical);
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
		color: var(--success) !important;
		font-size: 0.85rem;
		font-weight: 600;
		padding: 0.25rem 0.6rem;
		background: var(--success-softer);
		border-radius: var(--radius);
	}

	.identity:hover {
		text-decoration: none !important;
		background: var(--success-soft);
	}

	main {
		padding: 1.5rem 0;
	}

	.offline-banner {
		background: var(--warning);
		color: var(--text-on-warning);
		padding: 0.5rem 1rem;
		display: flex;
		justify-content: center;
		align-items: center;
		gap: 1rem;
		font-size: 0.85rem;
		font-weight: 600;
	}

	.install-banner {
		position: fixed;
		bottom: 0;
		left: 0;
		right: 0;
		background: var(--bg-surface);
		border-top: 1px solid var(--border);
		padding: 0.75rem 1rem;
		display: flex;
		justify-content: center;
		align-items: center;
		gap: 1rem;
		font-size: 0.9rem;
		z-index: 200;
	}

	.install-btn {
		background: var(--accent);
		color: var(--text-on-accent);
		padding: 0.4rem 1rem;
		border-radius: var(--radius);
		font-weight: 600;
		font-size: 0.85rem;
	}

	.dismiss-btn {
		background: none;
		color: inherit;
		font-size: 1.2rem;
		opacity: 0.7;
		min-height: 30px;
		min-width: 30px;
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
