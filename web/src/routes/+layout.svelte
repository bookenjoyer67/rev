<script lang="ts">
	import '../app.css';
	import Onboarding from '$lib/components/Onboarding.svelte';
	import { auth } from '$lib/stores/auth';
	import { serverState } from '$lib/stores/server';
	import { location } from '$lib/stores/location';

	let { children } = $props();
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
