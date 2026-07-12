<script lang="ts">
    import { onMount } from 'svelte';
    import { getActiveServer } from '$lib/stores/server';
    import { isAuthenticated } from '$lib/stores/auth';
    import { api } from '$lib/api/client';

    interface Alliance {
        id: string;
        remote_domain: string;
        remote_name?: string;
        status: string;
        initiated_by: string;
        last_synced_at?: string;
        created_at: string;
    }

    let alliances: Alliance[] = $state([]);
    let loading = $state(true);
    let error = $state('');
    let proposeDomain = $state('');
    let proposeName = $state('');
    let proposing = $state(false);
    let proposeError = $state('');

    onMount(async () => {
        const server = getActiveServer();
        if (!server) { loading = false; return; }
        await loadAlliances();
    });

    async function loadAlliances() {
        try {
            alliances = await api.alliances.list();
        } catch (e) {
            error = 'Could not reach server';
        }
        loading = false;
    }

    async function handlePropose(e: Event) {
        e.preventDefault();
        if (!proposeDomain.trim()) return;
        proposeError = '';
        proposing = true;
        try {
            await api.alliances.propose(proposeDomain.trim(), proposeName.trim() || undefined);
            proposeDomain = '';
            proposeName = '';
            await loadAlliances();
        } catch (e: any) {
            proposeError = e.message || 'Failed to propose alliance';
        }
        proposing = false;
    }

    async function handleAccept(id: string) {
        try {
            await api.alliances.accept(id);
            await loadAlliances();
        } catch (e: any) {
            alert(e.message || 'Failed');
        }
    }

    async function handleReject(id: string) {
        try {
            await api.alliances.reject(id);
            await loadAlliances();
        } catch (e: any) {
            alert(e.message || 'Failed');
        }
    }

    async function handleDelete(id: string) {
        if (!confirm('Sever this alliance?')) return;
        try {
            await api.alliances.delete(id);
            await loadAlliances();
        } catch (e: any) {
            alert(e.message || 'Failed');
        }
    }

    function relativeTime(iso: string): string {
        const d = new Date(iso);
        const now = Date.now();
        const diff = now - d.getTime();
        const mins = Math.floor(diff / 60000);
        if (mins < 1) return 'just now';
        if (mins < 60) return `${mins}m ago`;
        const hrs = Math.floor(mins / 60);
        if (hrs < 24) return `${hrs}h ago`;
        const days = Math.floor(hrs / 24);
        if (days < 30) return `${days}d ago`;
        return `${Math.floor(days / 30)}mo ago`;
    }
</script>

<div class="container">
    <header class="page-header">
        <h1>Federation</h1>
        <p class="subtitle">Komun nodes federate to share resources, needs, and offers across communities.</p>
    </header>

    {#if isAuthenticated()}
        <form class="propose-form" onsubmit={handlePropose}>
            <h3>Propose alliance</h3>
            <div class="propose-fields">
                <input type="text" bind:value={proposeDomain} placeholder="remote-server.org" disabled={proposing} />
                <input type="text" bind:value={proposeName} placeholder="Name (optional)" disabled={proposing} />
                <button type="submit" disabled={proposing}>
                    {proposing ? 'Proposing...' : 'Propose'}
                </button>
            </div>
            {#if proposeError}
                <p class="error">{proposeError}</p>
            {/if}
        </form>
    {/if}

    {#if loading}
        <div class="empty"><p>Loading alliances...</p></div>
    {:else if error}
        <div class="empty"><p>{error}</p></div>
    {:else if alliances.length === 0}
        <div class="empty">
            <p>No federated nodes yet.</p>
            <p class="hint">Share your node address with other communities to form alliances.</p>
        </div>
    {:else}
        <ul class="alliance-list">
            {#each alliances as a (a.id)}
                <li>
                    <div class="alliance-info">
                        <strong>{a.remote_name || a.remote_domain}</strong>
                        <span class="domain">{a.remote_domain}</span>
                        {#if a.last_synced_at}
                            <span class="synced">Last synced: {relativeTime(a.last_synced_at)}</span>
                        {/if}
                    </div>
                    <div class="alliance-actions">
                        <span class="status status-{a.status}">{a.status}</span>
                        {#if a.status === 'pending'}
                            <button class="action-btn accept" onclick={() => handleAccept(a.id)}>Accept</button>
                            <button class="action-btn reject" onclick={() => handleReject(a.id)}>Reject</button>
                        {/if}
                        {#if a.status === 'accepted'}
                            <button class="action-btn delete" onclick={() => handleDelete(a.id)}>Sever</button>
                        {/if}
                    </div>
                </li>
            {/each}
        </ul>
    {/if}
</div>

<style>
    .container {
        max-width: 640px;
        margin: 0 auto;
        padding: 1rem;
    }

    .page-header {
        margin-bottom: 2rem;
    }

    h1 { font-size: 1.5rem; margin-bottom: 0.25rem; }

    .subtitle {
        color: var(--text-muted);
        font-size: 0.9rem;
    }

    .propose-form {
        background: var(--bg-surface);
        border: 1px solid var(--border);
        border-radius: var(--radius, 8px);
        padding: 1rem;
        margin-bottom: 1.5rem;
    }

    .propose-form h3 {
        font-size: 1rem;
        margin-bottom: 0.5rem;
    }

    .propose-fields {
        display: flex;
        gap: 0.5rem;
    }

    .propose-fields input {
        flex: 1;
        background: var(--bg);
        border: 1px solid var(--border);
        border-radius: var(--radius, 6px);
        padding: 0.5rem 0.7rem;
        color: var(--text);
        font-size: 0.9rem;
    }

    .propose-fields button {
        background: var(--accent);
        color: var(--text-on-accent);
        padding: 0.5rem 1rem;
        border-radius: var(--radius, 6px);
        font-weight: 600;
        font-size: 0.9rem;
        white-space: nowrap;
    }

    .propose-fields button:disabled {
        opacity: 0.5;
    }

    .error {
        color: var(--critical);
        font-size: 0.8rem;
        margin-top: 0.3rem;
    }

    .empty {
        text-align: center;
        padding: 3rem 0;
        color: var(--text-muted);
        background: var(--bg-surface);
        border: 1px solid var(--border);
        border-radius: var(--radius-lg);
    }

    .hint { font-size: 0.8rem; margin-top: 0.5rem; }

    .alliance-list { list-style: none; }

    li {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 1rem;
        border: 1px solid var(--border);
        border-radius: var(--radius);
        margin-bottom: 0.5rem;
        background: var(--bg-surface);
        gap: 0.75rem;
        flex-wrap: wrap;
    }

    .alliance-info {
        display: flex;
        flex-direction: column;
        gap: 0.15rem;
        min-width: 0;
        flex: 1;
    }

    .domain { font-size: 0.75rem; color: var(--text-muted); }
    .synced { font-size: 0.7rem; color: var(--text-muted); }

    .alliance-actions {
        display: flex;
        align-items: center;
        gap: 0.4rem;
        flex-shrink: 0;
    }

    .status {
        font-size: 0.7rem;
        padding: 0.15rem 0.5rem;
        border-radius: 4px;
        font-weight: 600;
        text-transform: uppercase;
    }

    .status-accepted { background: var(--success-soft); color: var(--success); }
    .status-pending { background: var(--warning-soft); color: var(--warning); }
    .status-rejected { background: var(--critical-soft); color: var(--critical); }

    .action-btn {
        font-size: 0.7rem;
        padding: 0.2rem 0.5rem;
        border-radius: 4px;
        font-weight: 600;
        border: 1px solid;
        cursor: pointer;
    }

    .action-btn.accept {
        background: var(--success-soft);
        color: var(--success);
        border-color: var(--success);
    }

    .action-btn.reject {
        background: var(--critical-soft);
        color: var(--critical);
        border-color: var(--critical);
    }

    .action-btn.delete {
        background: var(--bg);
        color: var(--text-muted);
        border-color: var(--border);
    }
</style>
