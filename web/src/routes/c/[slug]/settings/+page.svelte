<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { isConnected, getActiveServer, resolveSlug } from '$lib/stores/server';
	import { isAuthenticated, getToken } from '$lib/stores/auth';
	import { api } from '$lib/api/client';

	interface Invite { code: string; uses_remaining?: number; expires_at?: string; created_at: string; }

	let name = $state('');
	let description = $state('');
	let visibility = $state('federated');
	let locationName = $state('');
	let locationLat: number | null = $state(null);
	let locationLon: number | null = $state(null);
	let resolvedName: string | null = $state(null);
	let resolvedLat: number | null = $state(null);
	let resolvedLon: number | null = $state(null);
	let geocoding = $state(false);
	let debounceTimer: ReturnType<typeof setTimeout> | null = $state(null);
	let invites: Invite[] = $state([]);
	let loading = $state(true);
	let saving = $state(false);
	let saved = $state(false);
	let error = $state('');
	let slug = $state('');
	let localSlug = $state('');
	let imagePath: string | null = $state(null);
	let imageFile: File | null = $state(null);
	let imagePreview: string | null = $state(null);
	let uploadingImage = $state(false);

	async function geocodeCoords(query: string): Promise<{ lat: number; lon: number; display_name: string } | null> {
		const serverUrl = getActiveServer();
		if (!serverUrl) return null;
		try {
			const res = await fetch(`${serverUrl}/api/geocode?q=${encodeURIComponent(query)}`);
			if (!res.ok) return null;
			const result = await res.json();
			if (!result.lat || !result.lon) return null;
			return {
				lat: parseFloat(result.lat),
				lon: parseFloat(result.lon),
				display_name: result.display_name.split(',').slice(0, 2).join(',').trim(),
			};
		} catch {
			return null;
		}
	}

	function handleLocationInput() {
		if (debounceTimer) clearTimeout(debounceTimer);
		const query = locationName.trim();
		if (!query) {
			resolvedName = null;
			resolvedLat = null;
			resolvedLon = null;
			geocoding = false;
			return;
		}
		geocoding = true;
		debounceTimer = setTimeout(async () => {
			const result = await geocodeCoords(query);
			if (result) {
				resolvedName = result.display_name;
				resolvedLat = result.lat;
				resolvedLon = result.lon;
			} else {
				resolvedName = null;
				resolvedLat = null;
				resolvedLon = null;
			}
			geocoding = false;
		}, 500);
	}

	onMount(async () => {
		const rawSlug = $page.params.slug as string;
		slug = rawSlug;
		let resolved;
		try {
			resolved = await resolveSlug(rawSlug);
			localSlug = resolved.localSlug;
		} catch {
			goto('/connect');
			return;
		}
		if (!isAuthenticated()) { goto('/'); return; }
		try {
			const community = await api.communities.get(localSlug);
			if (community.member_role !== 'admin') { goto(`/c/${slug}`); return; }
			name = community.name;
			description = community.description || '';
			visibility = community.visibility;
			locationName = community.location_name || '';
			locationLat = community.location_lat ?? null;
			locationLon = community.location_lon ?? null;
			if (community.location_lat != null && community.location_lon != null) {
				resolvedLat = community.location_lat;
				resolvedLon = community.location_lon;
				resolvedName = community.location_name || null;
			}
			invites = await api.communities.listInvites(localSlug);
			imagePath = community.image_path ? '/community-images/' + community.image_path : null;
			if (imagePath) imagePreview = imagePath;
		} catch { goto(`/c/${slug}`); }
		loading = false;
	});

	async function save() {
		saving = true;
		error = '';
		try {
			await api.communities.update(localSlug, {
				name: name.trim(),
				description: description.trim() || undefined,
				visibility,
				location_name: locationName.trim() || null,
				location_lat: resolvedLat,
				location_lon: resolvedLon,
			});
			locationLat = resolvedLat;
			locationLon = resolvedLon;
			saved = true;
			setTimeout(() => saved = false, 2000);
		} catch (e: any) { error = e.message; }
		saving = false;
	}

	async function createInvite() {
		try {
			const invite = await api.communities.createInvite(localSlug);
			invites = [invite, ...invites];
		} catch (e: any) { error = e.message; }
	}

	async function removeInvite(code: string) {
		await api.communities.deleteInvite(localSlug, code);
		invites = invites.filter(i => i.code !== code);
	}

	function copyCode(code: string) {
		navigator.clipboard.writeText(code);
	}

	function handleImage(e: Event) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (!file) return;
		if (!file.type.match(/image\/(png|jpeg|webp)/)) {
			error = 'Image must be PNG, JPEG, or WebP';
			return;
		}
		imageFile = file;
		imagePreview = URL.createObjectURL(file);
	}

	async function uploadImage() {
		if (!imageFile) return;
		uploadingImage = true;
		const server = getActiveServer();
		const token = getToken();
		if (!server || !token) { uploadingImage = false; return; }
		const formData = new FormData();
		formData.append('file', imageFile);
		try {
			const res = await fetch(`${server}/api/communities/${localSlug}/image`, {
				method: 'POST',
				headers: { 'Authorization': `Bearer ${token}` },
				body: formData,
			});
			if (res.ok) {
				const data = await res.json();
				imagePath = data.image_url;
				imageFile = null;
			}
		} catch {}
		uploadingImage = false;
	}
</script>

<div class="container">
	{#if loading}
		<p class="status">Loading...</p>
	{:else}
		<header class="page-header">
			<a href="/c/{slug}" class="back">&larr; Back</a>
			<h1>Community Settings</h1>
		</header>

		<form onsubmit={(e) => { e.preventDefault(); save(); }}>
			<label>
				<span>Name</span>
				<input type="text" bind:value={name} maxlength="100" />
			</label>
			<label>
				<span>Description</span>
				<textarea bind:value={description} rows="3"></textarea>
			</label>
			<label>
				<span>Location (optional)</span>
				<input type="text" bind:value={locationName} oninput={handleLocationInput} placeholder="East Oakland, CA" />
			</label>

			{#if geocoding}
				<p class="geo-feedback">Geocoding...</p>
			{:else if resolvedLat != null && resolvedLon != null}
				<div class="location-preview">
					<span class="geo-feedback">Resolved to: {resolvedName || locationName.trim()}</span>
					<iframe
						title="Location preview"
						src={`https://www.openstreetmap.org/export/embed.html?bbox=${resolvedLon - 0.01},${resolvedLat - 0.005},${resolvedLon + 0.01},${resolvedLat + 0.005}&layer=mapnik&marker=${resolvedLat},${resolvedLon}`}
						class="map-preview"
					></iframe>
				</div>
			{:else if locationName.trim() && !geocoding}
				<p class="geo-feedback not-found">Location not found</p>
			{/if}

			<label>
				<span>Visibility</span>
				<select bind:value={visibility}>
					<option value="public">Public</option>
					<option value="federated">Federated</option>
					<option value="private">Private</option>
				</select>
			</label>

			<label>
				<span>Community Image</span>
				<div class="image-upload">
					{#if imagePreview}
						<img src={imagePreview} alt="Preview" class="image-preview" />
					{/if}
					{#if imagePath}
						<p class="hint">Current image set. Upload a new one to replace it.</p>
					{/if}
					<input type="file" accept="image/png,image/jpeg,image/webp" onchange={handleImage} class="file-input" />
					<button type="button" class="btn-ghost upload-btn" onclick={() => (document.querySelector('.file-input') as HTMLInputElement)?.click()}>
						{imagePreview ? 'Change' : 'Choose image'}
					</button>
					{#if imageFile}
						<button type="button" class="btn-primary upload-btn" onclick={uploadImage} disabled={uploadingImage}>
							{uploadingImage ? 'Uploading...' : 'Upload'}
						</button>
					{/if}
				</div>
				<small>PNG, JPEG, or WebP. Max 1MB.</small>
			</label>
			{#if error}<p class="error">{error}</p>{/if}
			<button type="submit" class="save-btn" disabled={saving}>
				{saving ? 'Saving...' : saved ? 'Saved!' : 'Save Changes'}
			</button>
		</form>

		<section class="invites">
			<div class="invite-header">
				<h2>Invite Codes</h2>
				<button class="create-btn" onclick={createInvite}>Create Code</button>
			</div>
			<p class="hint">When invite codes exist, new members must use one to join.</p>
			{#if invites.length === 0}
				<p class="empty">No invite codes. Community is open to everyone.</p>
			{:else}
				<ul>
					{#each invites as invite}
						<li>
							<code>{invite.code}</code>
							<div class="invite-actions">
								<button class="copy-btn" onclick={() => copyCode(invite.code)}>Copy</button>
								<button class="remove-btn" onclick={() => removeInvite(invite.code)}>Remove</button>
							</div>
						</li>
					{/each}
				</ul>
			{/if}
		</section>
	{/if}
</div>

<style>
	.page-header { margin-bottom: 1.5rem; }
	.back { color: var(--text-muted); font-size: 0.8rem; }
	h1 { font-size: 1.5rem; margin-top: 0.25rem; }

	form { display: flex; flex-direction: column; gap: 1rem; max-width: 500px; margin-bottom: 2rem; }
	label { display: flex; flex-direction: column; gap: 0.3rem; }
	label span { font-size: 0.85rem; font-weight: 600; color: var(--text-muted); }
	input, textarea, select { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 0.75rem; color: var(--text); font-size: 1rem; font-family: inherit; }
	input:focus, textarea:focus, select:focus { outline: none; border-color: var(--accent); }

	.save-btn { background: var(--accent); color: var(--text-on-accent); padding: 0.75rem; border-radius: var(--radius); font-weight: 600; }
	.save-btn:disabled { opacity: 0.6; }

	.invites { border-top: 1px solid var(--border); padding-top: 1.5rem; }
	.invite-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem; }
	h2 { font-size: 1.2rem; }
	.create-btn { background: var(--success); color: var(--text-on-success); padding: 0.4rem 0.8rem; border-radius: var(--radius); font-weight: 600; font-size: 0.85rem; }
	.hint { color: var(--text-muted); font-size: 0.8rem; margin-bottom: 1rem; }
	.empty { color: var(--text-muted); font-size: 0.85rem; }

	ul { list-style: none; }
	li { display: flex; justify-content: space-between; align-items: center; padding: 0.6rem 0.8rem; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); margin-bottom: 0.4rem; }
	code { font-size: 1.1rem; font-weight: 600; letter-spacing: 1px; }
	.invite-actions { display: flex; gap: 0.4rem; }
	.copy-btn { background: var(--bg-elevated); color: var(--text); padding: 0.3rem 0.6rem; border-radius: var(--radius); font-size: 0.8rem; border: 1px solid var(--border); }
	.remove-btn { background: none; color: var(--critical); padding: 0.3rem 0.6rem; border-radius: var(--radius); font-size: 0.8rem; border: 1px solid var(--critical); }

	.error { color: var(--critical); font-size: 0.85rem; }
	.status { text-align: center; color: var(--text-muted); padding: 3rem 0; }

	.image-upload {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		flex-wrap: wrap;
	}

	.image-preview {
		width: 80px;
		height: 80px;
		border-radius: var(--radius-md);
		object-fit: cover;
		border: 1px solid var(--border);
	}

	.file-input {
		display: none;
	}

	.upload-btn {
		font-size: var(--text-sm);
	}

	.location-preview {
		display: flex;
		flex-direction: column;
	}

	.geo-feedback {
		font-size: 0.8rem;
		color: var(--text-muted);
		margin-top: -0.5rem;
	}

	.geo-feedback.not-found {
		color: var(--warning);
	}

	.map-preview {
		width: 100%;
		height: 200px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		margin-top: 0.4rem;
	}
</style>
