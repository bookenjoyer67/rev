<script lang="ts">
	import { register, onAuthComplete, showOnboarding } from '$lib/stores/auth';

	let displayName = $state('');
	let passphrase = $state('');
	let showPassphrase = $state(false);
	let error = $state('');
	let loading = $state(false);

	async function handleSubmit() {
		if (!displayName.trim()) {
			error = 'Enter a name';
			return;
		}
		loading = true;
		error = '';
		const ok = await register(
			displayName.trim(),
			passphrase.trim() || undefined
		);
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
		<h2>Create your identity</h2>
		<p>Your identity is a cryptographic keypair. No email or password needed.</p>

		<form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>
			<input
				type="text"
				placeholder="What should people call you?"
				bind:value={displayName}
				disabled={loading}
				maxlength="50"
			/>

			{#if !showPassphrase}
				<button type="button" class="passphrase-toggle" onclick={() => showPassphrase = true}>
					Set a recovery passphrase (recommended)
				</button>
			{:else}
				<div class="passphrase-section">
					<label>
						<span>Recovery passphrase</span>
						<input
							type="password"
							placeholder="A memorable phrase for key recovery"
							bind:value={passphrase}
							disabled={loading}
						/>
					</label>
					<p class="hint">This encrypts your keys so you can recover them from any device. Without it, losing this device means losing your identity.</p>
				</div>
			{/if}

			{#if error}
				<p class="error">{error}</p>
			{/if}
			<button type="submit" class="submit-btn" disabled={loading}>
				{loading ? 'Creating...' : 'Create Identity'}
			</button>
		</form>
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
		max-width: 420px;
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

	h2 { margin-bottom: 0.5rem; font-size: 1.3rem; }

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

	input[type="text"], input[type="password"] {
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.75rem;
		color: var(--text);
		font-size: 1rem;
		width: 100%;
		box-sizing: border-box;
	}

	input:focus { outline: none; border-color: var(--accent); }

	.passphrase-toggle {
		background: none;
		color: var(--accent);
		font-size: 0.85rem;
		text-align: left;
		text-decoration: underline;
		padding: 0;
	}

	.passphrase-section {
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.75rem;
	}

	.passphrase-section label {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}

	.passphrase-section label span {
		font-size: 0.8rem;
		font-weight: 600;
		color: var(--text-muted);
	}

	.hint {
		font-size: 0.75rem;
		color: var(--text-muted);
		margin: 0.5rem 0 0;
	}

	.submit-btn {
		background: var(--accent);
		color: white;
		padding: 0.75rem;
		border-radius: var(--radius);
		font-weight: 600;
		font-size: 1rem;
	}

	.submit-btn:disabled { opacity: 0.6; cursor: not-allowed; }

	.error { color: var(--critical); font-size: 0.85rem; margin: 0; }
</style>
