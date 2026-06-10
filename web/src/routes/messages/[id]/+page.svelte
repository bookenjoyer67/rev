<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { isConnected, getActiveServer } from '$lib/stores/server';
	import { isAuthenticated, auth, getEncryptionSecretKey } from '$lib/stores/auth';
	import { api } from '$lib/api/client';
	import { deriveConversationKey, encryptMessage, decryptMessage } from '$lib/crypto';

	interface Message {
		id: string;
		sender_id: string;
		body: string;
		created_at: string;
	}

	interface DecryptedMessage {
		id: string;
		sender_id: string;
		body: string;
		created_at: string;
		encrypted: boolean;
	}

	interface Conversation {
		match_id: string;
		post_id: string;
		post_title: string;
		post_kind: string;
		responder_id: string;
		author_id: string;
		responder_name: string;
		author_name: string;
		status: string;
		messages: Message[];
	}

	let convo: Conversation | null = $state(null);
	let decryptedMessages: DecryptedMessage[] = $state([]);
	let newMessage = $state('');
	let loading = $state(true);
	let sending = $state(false);
	let error = $state('');
	let sharedKey: string | null = $state(null);
	let encrypted = $state(false);

	let myUserId = $derived((() => {
		const server = getActiveServer();
		if (!server) return null;
		return $auth.servers?.[server]?.userId || null;
	})());

	onMount(() => {
		if (!isConnected() || !isAuthenticated()) {
			goto('/');
			return;
		}
		loadConversation();
		const interval = setInterval(loadConversation, 5000);
		return () => clearInterval(interval);
	});

	async function loadConversation() {
		const matchId = $page.params.id as string;
		if (!matchId) return;
		try {
			convo = await api.conversations.get(matchId);
			await setupEncryption();
			await decryptMessages();
		} catch (e: any) {
			error = e.message;
		}
		loading = false;
	}

	async function setupEncryption() {
		if (!convo || sharedKey) return;
		const mySecret = getEncryptionSecretKey();
		if (!mySecret) return;

		const otherPartyId = myUserId === convo.author_id ? convo.responder_id : convo.author_id;
		try {
			const server = getActiveServer();
			const res = await fetch(`${server}/api/auth/users/${otherPartyId}/keys`);
			if (!res.ok) return;
			const keys = await res.json();
			if (!keys.encryption_public_key) return;

			sharedKey = await deriveConversationKey(mySecret, keys.encryption_public_key);
			encrypted = true;
		} catch {
			// encryption not available for this conversation
		}
	}

	async function decryptMessages() {
		if (!convo) return;
		const msgs: DecryptedMessage[] = [];
		for (const msg of convo.messages) {
			if (encrypted && sharedKey) {
				try {
					const plaintext = await decryptMessage(msg.body, sharedKey);
					msgs.push({ ...msg, body: plaintext, encrypted: true });
				} catch {
					msgs.push({ ...msg, encrypted: false });
				}
			} else {
				msgs.push({ ...msg, encrypted: false });
			}
		}
		decryptedMessages = msgs;
	}

	async function sendMessage() {
		if (!newMessage.trim() || !convo) return;
		sending = true;
		try {
			let body = newMessage.trim();
			if (encrypted && sharedKey) {
				body = await encryptMessage(body, sharedKey);
			}
			await api.conversations.sendMessage(convo.match_id, body);
			newMessage = '';
			await loadConversation();
		} catch (e: any) {
			error = e.message;
		}
		sending = false;
	}

	async function markFulfilled() {
		if (!convo) return;
		await api.conversations.updateStatus(convo.match_id, 'completed');
		await loadConversation();
	}

	function formatTime(dateStr: string): string {
		return new Date(dateStr).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
	}
</script>

<div class="container">
	{#if loading}
		<p class="status">Loading...</p>
	{:else if error && !convo}
		<p class="status err">{error}</p>
	{:else if convo}
		<header class="thread-header">
			<a href="/messages" class="back">&larr;</a>
			<div class="header-info">
				<h2>{convo.post_title}</h2>
				<span class="participants">
					{convo.author_name} &harr; {convo.responder_name}
				</span>
			</div>
			<div class="header-right">
				{#if encrypted}
					<span class="lock" title="End-to-end encrypted">&#x1f512;</span>
				{/if}
				<span class="status-badge status-{convo.status}">{convo.status}</span>
			</div>
		</header>

		<div class="messages">
			{#each decryptedMessages as msg}
				<div class="bubble" class:mine={msg.sender_id === myUserId} class:theirs={msg.sender_id !== myUserId}>
					<p>{msg.body}</p>
					<span class="msg-time">
						{formatTime(msg.created_at)}
						{#if msg.encrypted}
							<span class="lock-small">&#x1f512;</span>
						{/if}
					</span>
				</div>
			{/each}
		</div>

		{#if convo.status !== 'completed' && convo.status !== 'withdrawn'}
			<form class="send-form" onsubmit={(e) => { e.preventDefault(); sendMessage(); }}>
				<input
					type="text"
					bind:value={newMessage}
					placeholder="Type a message..."
					disabled={sending}
				/>
				<button type="submit" disabled={sending || !newMessage.trim()}>Send</button>
			</form>

			{#if myUserId === convo.author_id}
				<button class="fulfill-btn" onclick={markFulfilled}>Mark as fulfilled</button>
			{/if}
		{:else}
			<p class="closed">This conversation has been marked as {convo.status}.</p>
		{/if}
	{/if}
</div>

<style>
	.thread-header {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 1.5rem;
		padding-bottom: 1rem;
		border-bottom: 1px solid var(--border);
	}

	.back { font-size: 1.2rem; color: var(--text-muted); }
	.header-info { flex: 1; }
	h2 { font-size: 1.1rem; }
	.participants { font-size: 0.8rem; color: var(--text-muted); }

	.header-right { display: flex; align-items: center; gap: 0.5rem; }
	.lock { font-size: 0.9rem; }

	.status-badge {
		font-size: 0.7rem;
		padding: 0.2rem 0.5rem;
		border-radius: 4px;
		text-transform: uppercase;
		font-weight: 600;
	}

	.status-proposed { background: #f4a26120; color: var(--warning); }
	.status-accepted { background: #2ec4b620; color: var(--success); }
	.status-completed { background: #2ec4b640; color: var(--success); }

	.messages {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		margin-bottom: 1.5rem;
		min-height: 200px;
		max-height: 60vh;
		overflow-y: auto;
		padding: 0.5rem 0;
	}

	.bubble {
		max-width: 75%;
		padding: 0.6rem 0.9rem;
		border-radius: var(--radius-lg);
		font-size: 0.9rem;
	}

	.bubble.mine {
		align-self: flex-end;
		background: var(--accent);
		color: white;
	}

	.bubble.theirs {
		align-self: flex-start;
		background: var(--bg-elevated);
		color: var(--text);
	}

	.bubble p { margin: 0; word-break: break-word; }

	.msg-time {
		display: block;
		font-size: 0.65rem;
		opacity: 0.7;
		margin-top: 0.2rem;
	}

	.lock-small { font-size: 0.6rem; }

	.send-form {
		display: flex;
		gap: 0.5rem;
	}

	input {
		flex: 1;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.75rem;
		color: var(--text);
		font-size: 1rem;
	}

	input:focus { outline: none; border-color: var(--accent); }

	button[type="submit"] {
		background: var(--accent);
		color: white;
		padding: 0.75rem 1.2rem;
		border-radius: var(--radius);
		font-weight: 600;
	}

	button[type="submit"]:disabled { opacity: 0.5; cursor: not-allowed; }

	.fulfill-btn {
		width: 100%;
		margin-top: 1rem;
		background: var(--success);
		color: white;
		padding: 0.75rem;
		border-radius: var(--radius);
		font-weight: 600;
	}

	.closed {
		text-align: center;
		color: var(--text-muted);
		padding: 1rem;
		font-style: italic;
	}

	.status { text-align: center; color: var(--text-muted); padding: 3rem 0; }
	.err { color: var(--critical); }

	@media (max-width: 480px) {
		.bubble { max-width: 85%; }
		.thread-header { flex-wrap: wrap; }
		h2 { font-size: 1rem; }
		.send-form { position: sticky; bottom: 0; background: var(--bg); padding: 0.5rem 0; }
	}
</style>
