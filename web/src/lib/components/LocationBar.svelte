<script lang="ts">
	import { location, geocode, clearLocation } from '$lib/stores/location';

	let query = $state('');
	let loading = $state(false);
	let error = $state('');

	interface Props {
		onLocationSet?: () => void;
	}

	let { onLocationSet }: Props = $props();

	async function handleSearch() {
		if (!query.trim()) { error = 'Enter a city, zip, or neighborhood'; return; }
		loading = true;
		error = '';
		const ok = await geocode(query.trim());
		loading = false;
		if (ok) {
			onLocationSet?.();
		} else {
			error = 'Location not found. Try a different search.';
		}
	}

	function handleClear() {
		clearLocation();
		query = '';
	}
</script>

{#if $location.lat}
	<div class="location-display">
		<span class="pin">&#x1f4cd;</span>
		<span class="name">{$location.name}</span>
		<button class="change-btn" onclick={handleClear}>Change</button>
	</div>
{:else}
	<form class="location-form" onsubmit={(e) => { e.preventDefault(); handleSearch(); }}>
		<input
			type="text"
			bind:value={query}
			placeholder="Enter city, zip, or neighborhood"
			disabled={loading}
		/>
		<button type="submit" disabled={loading}>
			{loading ? '...' : 'Search'}
		</button>
	</form>
	{#if error}
		<p class="error">{error}</p>
	{/if}
{/if}

<style>
	.location-display {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.6rem 1rem;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
	}

	.pin { font-size: 1rem; }
	.name { font-size: 0.9rem; color: var(--text); }

	.change-btn {
		margin-left: auto;
		background: none;
		color: var(--accent);
		font-size: 0.8rem;
	}

	.location-form {
		display: flex;
		gap: 0.5rem;
	}

	input {
		flex: 1;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.7rem 1rem;
		color: var(--text);
		font-size: 0.95rem;
	}

	input:focus { outline: none; border-color: var(--accent); }

	button[type="submit"] {
		background: var(--accent);
		color: var(--text-on-accent);
		padding: 0.7rem 1.2rem;
		border-radius: var(--radius);
		font-weight: 600;
		font-size: 0.9rem;
	}

	button:disabled { opacity: 0.6; cursor: not-allowed; }

	.error { color: var(--critical); font-size: 0.8rem; margin-top: 0.4rem; }
</style>
