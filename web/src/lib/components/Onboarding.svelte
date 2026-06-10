<script lang="ts">
	import { register, onAuthComplete, showOnboarding } from '$lib/stores/auth';

	let displayName = $state('');
	let error = $state('');
	let loading = $state(false);

	async function handleSubmit() {
		if (!displayName.trim()) {
			error = 'Enter a name';
			return;
		}
		loading = true;
		error = '';
		const ok = await register(displayName.trim());
		loading = false;
		if (ok) {
			onAuthComplete();
		} else {
			error = 'Registration failed. Try again.';
		}
	}

	function close() {
		showOnboarding.set(false);
	}
</script>

{#if $showOnboarding}
<div class="overlay" role="dialog" aria-modal="true">
	<div class="modal">
		<button class="close-btn" onclick={close} aria-label="Close">&times;</button>
		<h2>Identify yourself</h2>
		<p>To contribute, pick a name. No email or password needed.</p>

		<form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>
			<input
				type="text"
				placeholder="What should people call you?"
				bind:value={displayName}
				disabled={loading}
				maxlength="50"
			/>
			{#if error}
				<p class="error">{error}</p>
			{/if}
			<button type="submit" class="submit-btn" disabled={loading}>
				{loading ? 'Creating...' : 'Join'}
			</button>
		</form>

		<p class="note">Your identity is bound to this device. No tracking, no accounts.</p>
	</div>
</div>
{/if}

<style>
	.overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.7);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
		padding: 1rem;
	}

	.modal {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		padding: 2rem;
		max-width: 400px;
		width: 100%;
		position: relative;
	}

	.close-btn {
		position: absolute;
		top: 0.75rem;
		right: 1rem;
		background: none;
		color: var(--text-muted);
		font-size: 1.5rem;
	}

	h2 {
		margin-bottom: 0.5rem;
		font-size: 1.3rem;
	}

	p {
		color: var(--text-muted);
		font-size: 0.9rem;
		margin-bottom: 1rem;
	}

	form {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	input {
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.75rem;
		color: var(--text);
		font-size: 1rem;
	}

	input:focus {
		outline: none;
		border-color: var(--accent);
	}

	.submit-btn {
		background: var(--accent);
		color: white;
		padding: 0.75rem;
		border-radius: var(--radius);
		font-weight: 600;
		font-size: 1rem;
	}

	.submit-btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.error {
		color: var(--critical);
		font-size: 0.85rem;
		margin: 0;
	}

	.note {
		margin-top: 1rem;
		font-size: 0.8rem;
		color: var(--text-muted);
		margin-bottom: 0;
	}
</style>
