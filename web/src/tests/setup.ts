import '@testing-library/jest-dom/vitest';
import { vi } from 'vitest';

class MemoryStorage implements Storage {
	private store = new Map<string, string>();
	get length(): number { return this.store.size; }
	clear(): void { this.store.clear(); }
	getItem(key: string): string | null { return this.store.get(key) ?? null; }
	key(index: number): string | null { return Array.from(this.store.keys())[index] ?? null; }
	removeItem(key: string): void { this.store.delete(key); }
	setItem(key: string, value: string): void { this.store.set(key, value); }
}

if (typeof globalThis.sessionStorage === 'undefined') {
	globalThis.sessionStorage = new MemoryStorage();
}
if (typeof globalThis.localStorage === 'undefined') {
	globalThis.localStorage = new MemoryStorage();
}

vi.mock('$app/navigation', () => ({
	goto: vi.fn(),
	afterNavigate: vi.fn(),
	beforeNavigate: vi.fn()
}));

vi.mock('$app/stores', () => ({
	page: { subscribe: vi.fn() },
	navigating: { subscribe: vi.fn() },
	updated: { subscribe: vi.fn() }
}));
