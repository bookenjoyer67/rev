import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import OfferPanel from '$lib/components/OfferPanel.svelte';
import DealReviewModal from '$lib/components/DealReviewModal.svelte';
import { createOffer, agreedOffer, canAccept, lastNumberedOffer, nextOfferKind, type Offer } from '$lib/api/offers';
import {
	ALREADY_REVIEWED_MESSAGE,
	ReviewApiError,
	countChars,
	createReview,
	validateBody,
	validateRating
} from '$lib/api/reviews';

/**
 * The API modules are mocked at the network edge only: the pure rules (`canAccept`,
 * `nextOfferKind`, `agreedOffer`, `validateRating`) stay real, so the tests pin the behaviour the
 * components depend on rather than a stub of it.
 */
vi.mock('$lib/stores/server', () => ({
	getActiveServer: vi.fn(() => 'https://test.komun.buzz')
}));

vi.mock('$lib/stores/auth', () => ({
	getToken: vi.fn(() => 'test-token')
}));

vi.mock('$lib/api/offers', async (importOriginal) => {
	const actual = await importOriginal<typeof import('$lib/api/offers')>();
	return { ...actual, createOffer: vi.fn() };
});

vi.mock('$lib/api/reviews', async (importOriginal) => {
	const actual = await importOriginal<typeof import('$lib/api/reviews')>();
	return { ...actual, createReview: vi.fn() };
});

let seq = 0;
function makeOffer(partial: Partial<Offer> = {}): Offer {
	seq += 1;
	return {
		id: `offer-${seq}`,
		match_id: 'match-1',
		actor_id: 'actor-a',
		kind: 'offer',
		amount_cents: 2500,
		currency: 'USD',
		note: null,
		created_at: new Date().toISOString(),
		...partial
	};
}

beforeEach(() => {
	vi.resetAllMocks();
});

describe('offer trail helpers', () => {
	it('treats the first number as an offer and every later one as a counter', () => {
		expect(nextOfferKind([])).toBe('offer');
		expect(nextOfferKind([makeOffer({ kind: 'offer' })])).toBe('counter');
	});

	it('reads the last offer/counter as the number on the table', () => {
		const offers = [
			makeOffer({ kind: 'offer', amount_cents: 2500 }),
			makeOffer({ kind: 'counter', amount_cents: 2100 }),
			makeOffer({ kind: 'decline', amount_cents: null })
		];
		expect(lastNumberedOffer(offers)?.amount_cents).toBe(2100);
		expect(agreedOffer(offers)).toBeNull();
	});

	it('reads the agreed amount off the accept row', () => {
		const offers = [
			makeOffer({ kind: 'offer', amount_cents: 2500 }),
			makeOffer({ kind: 'accept', amount_cents: 2100, currency: 'EUR' })
		];
		const agreed = agreedOffer(offers);
		expect(agreed?.amount_cents).toBe(2100);
		expect(agreed?.currency).toBe('EUR');
	});

	it('allows accept only when the last number is the counterparty’s', () => {
		expect(canAccept([makeOffer({ actor_id: 'them' })], 'me')).toBe(true);
		expect(canAccept([makeOffer({ actor_id: 'me' })], 'me')).toBe(false);
		expect(canAccept([], 'me')).toBe(false);
	});
});

describe('OfferPanel', () => {
	it('renders the ordered list with kind, amount, currency, note and author', () => {
		render(OfferPanel, {
			props: {
				matchId: 'match-1',
				postKind: 'listing',
				status: 'proposed',
				myUserId: 'viewer',
				names: { 'actor-a': 'Ana', 'actor-b': 'Bob' },
				offers: [
					makeOffer({ actor_id: 'actor-a', kind: 'offer', amount_cents: 2500, currency: 'USD' }),
					makeOffer({
						actor_id: 'actor-b',
						kind: 'counter',
						amount_cents: 2100,
						currency: 'USD',
						note: 'Can collect Sunday'
					})
				]
			}
		});

		expect(screen.getByText('Offer', { selector: '.offer-kind' })).toBeInTheDocument();
		expect(screen.getByText('$25.00')).toBeInTheDocument();
		expect(screen.getByText('Counter', { selector: '.offer-kind' })).toBeInTheDocument();
		expect(screen.getByText('$21.00')).toBeInTheDocument();
		expect(screen.getByText('Can collect Sunday')).toBeInTheDocument();
		expect(screen.getByText('Ana')).toBeInTheDocument();
		expect(screen.getByText('Bob')).toBeInTheDocument();
	});

	it('shows the agreed price once an offer has been accepted', () => {
		render(OfferPanel, {
			props: {
				matchId: 'match-1',
				postKind: 'listing',
				status: 'accepted',
				myUserId: 'viewer',
				offers: [
					makeOffer({ kind: 'offer', amount_cents: 2500 }),
					makeOffer({ kind: 'accept', amount_cents: 2100, currency: 'EUR' })
				]
			}
		});

		expect(screen.getByText(/Agreed price:/)).toHaveTextContent('€21.00');
	});

	it('does not offer Accept to the author of the last offer', () => {
		render(OfferPanel, {
			props: {
				matchId: 'match-1',
				postKind: 'listing',
				status: 'proposed',
				myUserId: 'me',
				offers: [makeOffer({ actor_id: 'me', kind: 'offer' })]
			}
		});

		expect(screen.queryByRole('button', { name: 'Accept' })).not.toBeInTheDocument();
	});

	it('offers Accept when the last offer is the counterparty’s', () => {
		render(OfferPanel, {
			props: {
				matchId: 'match-1',
				postKind: 'listing',
				status: 'proposed',
				myUserId: 'me',
				offers: [makeOffer({ actor_id: 'them', kind: 'offer' })]
			}
		});

		expect(screen.getByRole('button', { name: 'Accept' })).toBeInTheDocument();
	});

	it('shows the server’s 409 sentence instead of swallowing it', async () => {
		const user = userEvent.setup();
		vi.mocked(createOffer).mockRejectedValueOnce(
			new Error('you cannot accept your own offer: the other party is the one who agrees to it')
		);

		render(OfferPanel, {
			props: {
				matchId: 'match-1',
				postKind: 'listing',
				status: 'proposed',
				myUserId: 'me',
				names: { them: 'Them' },
				offers: [makeOffer({ actor_id: 'them', kind: 'offer' })]
			}
		});

		await user.click(screen.getByRole('button', { name: 'Accept' }));

		expect(await screen.findByRole('alert')).toHaveTextContent('you cannot accept your own offer');
	});

	it('never shows the panel on an aid thread', () => {
		render(OfferPanel, {
			props: {
				matchId: 'match-1',
				postKind: 'need',
				status: 'proposed',
				myUserId: 'me',
				offers: [makeOffer()]
			}
		});

		expect(screen.queryByText('Offers')).not.toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Decline' })).not.toBeInTheDocument();
	});
});

describe('review validation', () => {
	it('refuses a rating outside 1–5 before anything is sent', () => {
		expect(validateRating(0)).toMatch(/1 to 5/);
		expect(validateRating(6)).toMatch(/1 to 5/);
		expect(validateRating(4.5)).toMatch(/1 to 5/);
		expect(validateRating(3)).toBeNull();
	});

	it('bounds the body at 2000 characters, counted by code point', () => {
		expect(validateBody('a'.repeat(2000))).toBeNull();
		expect(validateBody('a'.repeat(2001))).toMatch(/2000/);
		expect(countChars('💰')).toBe(1);
	});
});

describe('DealReviewModal', () => {
	it('refuses to send when no rating was chosen', async () => {
		const user = userEvent.setup();
		render(DealReviewModal, { props: { matchId: 'match-1', onClose: vi.fn() } });

		await user.click(screen.getByRole('button', { name: 'Post review' }));

		expect(createReview).not.toHaveBeenCalled();
		expect(screen.getByRole('alert')).toHaveTextContent(/Choose a rating/);
	});

	it('counts the characters typed into the review box', async () => {
		const user = userEvent.setup();
		render(DealReviewModal, { props: { matchId: 'match-1', onClose: vi.fn() } });

		await user.type(screen.getByRole('textbox'), 'Great trade');

		expect(screen.getByTestId('review-counter')).toHaveTextContent('11 / 2000');
	});

	it('renders the server’s 409 message when the deal is not completed', async () => {
		const user = userEvent.setup();
		vi.mocked(createReview).mockRejectedValueOnce(
			new ReviewApiError('this deal is not completed (status: accepted)', 409)
		);

		render(DealReviewModal, { props: { matchId: 'match-1', revieweeName: 'Bob', onClose: vi.fn() } });

		await user.click(screen.getByRole('radio', { name: /5 stars/ }));
		await user.click(screen.getByRole('button', { name: 'Post review' }));

		expect(await screen.findByRole('alert')).toHaveTextContent(
			'this deal is not completed (status: accepted)'
		);
	});

	it('shows the already-reviewed state without offering the form again', async () => {
		const user = userEvent.setup();
		vi.mocked(createReview).mockRejectedValueOnce(new ReviewApiError(ALREADY_REVIEWED_MESSAGE, 409));

		render(DealReviewModal, { props: { matchId: 'match-1', onClose: vi.fn() } });

		await user.click(screen.getByRole('radio', { name: /5 stars/ }));
		await user.click(screen.getByRole('button', { name: 'Post review' }));

		expect(await screen.findByText('You have already reviewed this deal.')).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Post review' })).not.toBeInTheDocument();
	});
});
