<script lang="ts">
	import { goto } from '$app/navigation';
	import { requireAuth } from '$lib/stores/auth';
	import { api } from '$lib/api/client';
	import { onMount } from 'svelte';

	interface Community { slug: string; name: string; }

	let communities: Community[] = $state([]);
	let selectedCommunity = $state('');
	let kind = $state('need');
	let category = $state('other');
	let title = $state('');
	let body = $state('');
	let urgency = $state('medium');
	let locationName = $state('');
	let contactMethod = $state('');
	let error = $state('');
	let loading = $state(false);

	onMount(async () => {
		try {
			communities = await api.communities.list();
			if (communities.length > 0) {
				selectedCommunity = communities[0].slug;
			}
		} catch (e) {}
	});

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
					location_name: locationName.trim() || null,
					contact_method: contactMethod.trim() || null,
				});
				goto(`/c/${selectedCommunity}`);
			} catch (e: any) {
				error = e.message || 'Failed to create post';
			} finally {
				loading = false;
			}
		});
	}
</script>

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
				<button type="button" class:active={kind === 'need'} onclick={() => kind = 'need'}>Need</button>
				<button type="button" class:active={kind === 'offer'} onclick={() => kind = 'offer'}>Offer</button>
				<button type="button" class:active={kind === 'resource'} onclick={() => kind = 'resource'}>Resource</button>
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
			<input type="text" bind:value={title} placeholder="What do you need or offer?" maxlength="200" />
		</label>

		<label>
			<span>Details (optional)</span>
			<textarea bind:value={body} placeholder="More info..." rows="3"></textarea>
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
			<span>Location (optional)</span>
			<input type="text" bind:value={locationName} placeholder="Neighborhood or area" />
		</label>

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

	button[type="submit"] {
		background: var(--accent);
		color: white;
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
