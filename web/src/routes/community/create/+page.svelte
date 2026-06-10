<script lang="ts">
	import { goto } from '$app/navigation';
	import { requireAuth } from '$lib/stores/auth';
	import { api } from '$lib/api/client';

	let name = $state('');
	let slug = $state('');
	let description = $state('');
	let locationName = $state('');
	let error = $state('');
	let loading = $state(false);

	function slugify(s: string): string {
		return s.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
	}

	function handleNameInput() {
		if (!slug || slug === slugify(name.slice(0, -1))) {
			slug = slugify(name);
		}
	}

	function submit() {
		requireAuth(async () => {
			if (!name.trim()) { error = 'Name is required'; return; }
			if (!slug.trim()) { error = 'Slug is required'; return; }
			loading = true;
			error = '';
			try {
				const community = await api.communities.create({
					name: name.trim(),
					slug: slug.trim(),
					description: description.trim() || undefined,
					location_name: locationName.trim() || undefined,
				});
				goto(`/c/${community.slug}`);
			} catch (e: any) {
				error = e.message || 'Failed to create community';
			} finally {
				loading = false;
			}
		});
	}
</script>

<div class="container">
	<h1>Start a community</h1>

	<form onsubmit={(e) => { e.preventDefault(); submit(); }}>
		<label>
			<span>Name</span>
			<input type="text" bind:value={name} oninput={handleNameInput} placeholder="East Oakland Mutual Aid" maxlength="100" />
		</label>

		<label>
			<span>Slug</span>
			<input type="text" bind:value={slug} placeholder="east-oakland" maxlength="50" />
			<small>Used in the URL: /c/{slug || '...'}</small>
		</label>

		<label>
			<span>Description</span>
			<textarea bind:value={description} placeholder="What is this community about?" rows="3"></textarea>
		</label>

		<label>
			<span>Location (optional)</span>
			<input type="text" bind:value={locationName} placeholder="East Oakland, CA" />
		</label>

		{#if error}
			<p class="error">{error}</p>
		{/if}

		<button type="submit" disabled={loading}>
			{loading ? 'Creating...' : 'Create Community'}
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

	input, textarea {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.75rem;
		color: var(--text);
		font-size: 1rem;
		font-family: inherit;
	}

	input:focus, textarea:focus {
		outline: none;
		border-color: var(--accent);
	}

	small {
		color: var(--text-muted);
		font-size: 0.8rem;
	}

	button {
		background: var(--accent);
		color: white;
		padding: 0.75rem;
		border-radius: var(--radius);
		font-weight: 600;
		font-size: 1rem;
		margin-top: 0.5rem;
	}

	button:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.error {
		color: var(--critical);
		font-size: 0.85rem;
	}
</style>
