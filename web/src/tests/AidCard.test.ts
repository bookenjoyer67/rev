import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import AidCard from '$lib/components/AidCard.svelte';
import { auth } from '$lib/stores/auth';

vi.mock('$lib/stores/server', () => ({
	getActiveServer: vi.fn(() => 'https://test.komun.buzz'),
	setActiveServer: vi.fn(),
	servers: { subscribe: vi.fn() }
}));

vi.mock('$lib/api/discovery', () => ({}));

const makePost = (overrides = {}) => ({
	id: 'post-1',
	community_id: 'comm-1',
	author_id: 'author-1',
	kind: 'need',
	category: 'food',
	title: 'Need groceries',
	body: 'Can someone help with groceries this week?',
	urgency: 'high',
	status: 'active',
	created_at: new Date(Date.now() - 3600000).toISOString(),
	updated_at: new Date().toISOString(),
	community_name: 'Mutual Aid STL',
	community_slug: 'stl',
	server_name: 'stl.komun.buzz',
	server_url: 'https://stl.komun.buzz',
	server_location: 'St. Louis, MO',
	...overrides
});

describe('AidCard', () => {
	beforeEach(() => {
		auth.set({ keypair: null, servers: {} });
	});

	it('renders post title', () => {
		render(AidCard, { props: { post: makePost() } });
		expect(screen.getByText('Need groceries')).toBeInTheDocument();
	});

	it('renders post body', () => {
		render(AidCard, { props: { post: makePost() } });
		expect(screen.getByText('Can someone help with groceries this week?')).toBeInTheDocument();
	});

	it('renders kind label', () => {
		render(AidCard, { props: { post: makePost({ kind: 'need' }) } });
		expect(screen.getByText('Need')).toBeInTheDocument();
	});

	it('shows Offer kind label', () => {
		render(AidCard, { props: { post: makePost({ kind: 'offer' }) } });
		expect(screen.getByText('Offer')).toBeInTheDocument();
	});

	it('shows Resource kind label', () => {
		render(AidCard, { props: { post: makePost({ kind: 'resource' }) } });
		expect(screen.getByText('Resource')).toBeInTheDocument();
	});

	it('shows community name and server', () => {
		render(AidCard, { props: { post: makePost() } });
		expect(screen.getByText('Mutual Aid STL')).toBeInTheDocument();
		expect(screen.getByText('stl.komun.buzz')).toBeInTheDocument();
	});

	it('shows "I can help" button for need post by other author', () => {
		render(AidCard, { props: { post: makePost({ kind: 'need', author_id: 'other-author' }) } });
		expect(screen.getByText('I can help')).toBeInTheDocument();
	});

	it('shows "Request this" button for offer post by other author', () => {
		render(AidCard, { props: { post: makePost({ kind: 'offer', author_id: 'other-author' }) } });
		expect(screen.getByText('Request this')).toBeInTheDocument();
	});

	it('shows "Your post" for own posts', () => {
		auth.set({
			keypair: null,
			servers: {
				'https://test.komun.buzz': { token: 'jwt', userId: 'my-id', displayName: 'Me', role: 'member' }
			}
		});
		render(AidCard, {
			props: {
				post: makePost({
					author_id: 'my-id',
					server_url: 'https://test.komun.buzz'
				})
			}
		});
		expect(screen.getByText('Your post')).toBeInTheDocument();
	});

	it('does not show respond button for own posts', () => {
		auth.set({
			keypair: null,
			servers: {
				'https://test.komun.buzz': { token: 'jwt', userId: 'my-id', displayName: 'Me', role: 'member' }
			}
		});
		render(AidCard, {
			props: {
				post: makePost({
					author_id: 'my-id',
					server_url: 'https://test.komun.buzz'
				})
			}
		});
		expect(screen.queryByText('I can help')).not.toBeInTheDocument();
		expect(screen.queryByText('Request this')).not.toBeInTheDocument();
	});

	it('opens respond modal on button click', async () => {
		const user = userEvent.setup();
		render(AidCard, { props: { post: makePost({ kind: 'need', author_id: 'other' }) } });
		await user.click(screen.getByText('I can help'));
		expect(screen.getByPlaceholderText('What should people call you?')).toBeInTheDocument();
	});

	it('hides body when not provided', () => {
		render(AidCard, { props: { post: makePost({ body: null }) } });
		expect(screen.queryByText('Can someone help with groceries this week?')).not.toBeInTheDocument();
	});
});
