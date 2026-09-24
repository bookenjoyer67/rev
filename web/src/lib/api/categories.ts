/**
 * The category taxonomy — fetched from the server, never compiled into the client.
 *
 * SPEC 1.6 / decision B8: `categories` is a seeded table with a `scope` column, and
 * the market UI is a consumer of it, not an owner of a copy. There is deliberately
 * no hardcoded category list anywhere in the market code: the table is the source
 * of truth. Against the seed in `migrations/001_schema.sql`, `scope=market` is
 * 15 market-only + 6 `both` = 21 rows.
 */
import { getActiveServer } from '$lib/stores/server';

export type CategoryScope = 'aid' | 'market' | 'both';

export interface Category {
	slug: string;
	label: string;
	scope: CategoryScope;
}

/**
 * `scope` is a union, not an equality: asking for `market` also keeps the `both`
 * rows, because a `both` category is usable from the market form. This mirrors
 * `db::categories::list` (`WHERE scope = $2 OR scope = 'both'`); the public
 * `/api/categories?scope=market` route already applies it, so this is the same
 * rule expressed once more for callers that hold raw rows.
 */
export function categoriesForScope(rows: Category[], scope: CategoryScope): Category[] {
	return rows.filter((row) => row.scope === scope || row.scope === 'both');
}

function isCategory(value: unknown): value is Category {
	if (typeof value !== 'object' || value === null) return false;
	const candidate = value as Record<string, unknown>;
	return (
		typeof candidate.slug === 'string' &&
		typeof candidate.label === 'string' &&
		(candidate.scope === 'aid' || candidate.scope === 'market' || candidate.scope === 'both')
	);
}

/** `GET /api/categories?scope=` — public, active rows only. */
export async function fetchCategories(scope: CategoryScope): Promise<Category[]> {
	const base = getActiveServer();
	if (!base) throw new Error('Not connected to a server');

	const res = await fetch(`${base}/api/categories?scope=${scope}`);
	if (!res.ok) {
		const err = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(err.error || `Failed to load categories (${res.status})`);
	}

	const rows: unknown = await res.json();
	if (!Array.isArray(rows)) throw new Error('categories response was not a list');
	return rows.filter(isCategory);
}

/**
 * The market taxonomy, fetched once per page load. `/market` and `/market/new`
 * share the same fetch; the promise is cached so navigating between them does
 * not hit the route twice. A failed fetch clears the cache so the next caller
 * retries rather than re-throwing a stale rejection forever.
 */
let marketCategories: Promise<Category[]> | null = null;

export function fetchMarketCategories(): Promise<Category[]> {
	if (!marketCategories) {
		marketCategories = fetchCategories('market')
			.then((rows) => categoriesForScope(rows, 'market'))
			.catch((err: unknown) => {
				marketCategories = null;
				throw err;
			});
	}
	return marketCategories;
}

/** Test/teardown hook; the production path never needs to drop the cache. */
export function resetMarketCategoriesCache(): void {
	marketCategories = null;
}
