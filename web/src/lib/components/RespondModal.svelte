<script lang="ts">
	import { goto } from '$app/navigation';
	import { isAuthenticated, register, auth, getEncryptionSecretKey } from '$lib/stores/auth';
	import { connectToServer, isConnected, getActiveServer } from '$lib/stores/server';
	import { api } from '$lib/api/client';
	import { deriveConversationKey, encryptMessage } from '$lib/crypto';

	interface Props {
		post: {
			id: string;
			title: string;
			kind: string;
			server_url: string;
			community_slug: string;
			author_id?: string;
		};
		onClose: () => void;
	}

	let { post, onClose }: Props = $props();

	let displayName = $state('');
	let message = $state('');
	let error = $state('');
	let loading = $state(false);
	let success = $state(false);
	let matchId = $state('');

	let needsIdentity = $derived(!isAuthenticated());

	async function handleSubmit() {
		if (needsIdentity && !displayName.trim()) {
			error = 'Enter your name';
			return;
		}
		if (!message.trim()) {
			error = 'Write a message';
			return;
		}

		loading = true;
		error = '';

		try {
			if (getActiveServer() !== post.server_url) {
				await connectToServer(post.server_url);
			}

			if (needsIdentity) {
				const ok = await register(displayName.trim());
				if (!ok) { error = 'Failed to create identity'; loading = false; return; }
			}

			let body = message.trim();

			if (post.author_id) {
				const mySecret = getEncryptionSecretKey();
				if (mySecret) {
					try {
						const server = getActiveServer();
						const res = await fetch(`${server}/api/auth/users/${post.author_id}/keys`);
						if (res.ok) {
							const keys = await res.json();
							if (keys.encryption_public_key) {
								const sharedKey = await deriveConversationKey(mySecret, keys.encryption_public_key);
								body = await encryptMessage(body, sharedKey);
							}
						}
					} catch {
						// fall back to plaintext
					}
				}
			}

			const result = await api.posts.respond(post.id, body, post.server_url);
			matchId = result.match_id;
			success = true;
		} catch (e: any) {
			error = e.message || 'Failed to send response';
		}

		loading = false;
	}

	function viewConversation() {
		onClose();
		goto(`/messages/${matchId}`);
	}
</script>

<div class="overlay" role="dialog" aria-modal="true">
	<div class="modal">
		<button class="close-btn" onclick={onClose} aria-label="Close">&times;</button>

		{#if success}
			<div class="success">
				<h2>Response sent!</h2>
				<p>The requester will be notified.</p>
				<div class="success-actions">
					<button class="btn-primary" onclick={viewConversation}>View conversation</button>
					<button class="btn-secondary" onclick={onClose}>Back to feed</button>
				</div>
			</div>
		{:else}
			<h2>{post.kind === 'need' ? 'Offer help' : 'Request this'}</h2>
			<p class="post-ref">Re: {post.title}</p>

			<form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>
				{#if needsIdentity}
					<label>
						<span>Your name</span>
						<input type="text" bind:value={displayName} placeholder="What should people call you?" maxlength="50" disabled={loading} />
					</label>
				{/if}

				<label>
					<span>Your message</span>
					<textarea bind:value={message} placeholder={post.kind === 'need' ? "What can you offer? When/where can you help?" : "What do you need? How can they reach you?"} rows="4" disabled={loading}></textarea>
				</label>

				{#if error}
					<p class="error">{error}</p>
				{/if}

				<button type="submit" class="btn-primary" disabled={loading}>
					{loading ? 'Sending...' : 'Send Response'}
				</button>
			</form>

			{#if needsIdentity}
				<p class="note">No account needed. Your identity is bound to this device.</p>
			{/if}
		{/if}
	</div>
</div>

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
		max-width: 440px;
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

	h2 { margin-bottom: 0.25rem; font-size: 1.2rem; }

	.post-ref {
		color: var(--text-muted);
		font-size: 0.85rem;
		margin-bottom: 1.25rem;
		font-style: italic;
	}

	form {
		display: flex;
		flex-direction: column;
		gap: 1rem;
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
		background: var(--bg);
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

	.btn-primary {
		background: var(--accent);
		color: white;
		padding: 0.75rem;
		border-radius: var(--radius);
		font-weight: 600;
		font-size: 1rem;
		width: 100%;
	}

	.btn-primary:disabled { opacity: 0.6; cursor: not-allowed; }

	.btn-secondary {
		background: var(--bg-elevated);
		color: var(--text);
		padding: 0.75rem;
		border-radius: var(--radius);
		font-weight: 600;
		font-size: 1rem;
		border: 1px solid var(--border);
		width: 100%;
	}

	.error { color: var(--critical); font-size: 0.85rem; }

	.note {
		text-align: center;
		color: var(--text-muted);
		font-size: 0.8rem;
		margin-top: 1rem;
	}

	.success {
		text-align: center;
		padding: 1rem 0;
	}

	.success h2 { color: var(--success); margin-bottom: 0.5rem; }
	.success p { color: var(--text-muted); margin-bottom: 1.5rem; }

	.success-actions {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
</style>
