import { describe, it, expect, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';

vi.mock('$lib/stores/server', () => ({
	getActiveServer: vi.fn(() => 'https://test.komun.buzz'),
	setActiveServer: vi.fn(),
	servers: { subscribe: vi.fn() }
}));

import { auth, isAuthenticated, getToken, getDisplayName, logout, isSuperadmin } from '$lib/stores/auth';

describe('auth store', () => {
	beforeEach(() => {
		auth.set({ keypair: null, servers: {} });
		sessionStorage.clear();
		localStorage.clear();
	});

	describe('isAuthenticated', () => {
		it('returns false when no session', () => {
			expect(isAuthenticated()).toBe(false);
		});

		it('returns true when server has token', () => {
			auth.set({
				keypair: null,
				servers: {
					'https://test.komun.buzz': {
						token: 'test-jwt',
						userId: 'abc-123',
						displayName: 'Test User',
						role: 'member'
					}
				}
			});
			expect(isAuthenticated()).toBe(true);
		});
	});

	describe('getToken', () => {
		it('returns null when not authenticated', () => {
			expect(getToken()).toBeNull();
		});

		it('returns the JWT token', () => {
			auth.set({
				keypair: null,
				servers: {
					'https://test.komun.buzz': {
						token: 'eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjMifQ.abc',
						userId: 'abc-123',
						displayName: 'Test',
						role: 'member'
					}
				}
			});
			expect(getToken()).toBe('eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjMifQ.abc');
		});
	});

	describe('getDisplayName', () => {
		it('returns null when not authenticated', () => {
			expect(getDisplayName()).toBeNull();
		});

		it('returns the display name', () => {
			auth.set({
				keypair: null,
				servers: {
					'https://test.komun.buzz': {
						token: 'jwt',
						userId: 'id',
						displayName: 'Alice',
						role: 'member'
					}
				}
			});
			expect(getDisplayName()).toBe('Alice');
		});
	});

	describe('isSuperadmin', () => {
		it('returns false when not authenticated', () => {
			expect(isSuperadmin()).toBe(false);
		});

		it('returns false for member role', () => {
			auth.set({
				keypair: null,
				servers: {
					'https://test.komun.buzz': {
						token: 'jwt', userId: 'id', displayName: 'User', role: 'member'
					}
				}
			});
			expect(isSuperadmin()).toBe(false);
		});

		it('returns true for superadmin role', () => {
			auth.set({
				keypair: null,
				servers: {
					'https://test.komun.buzz': {
						token: 'jwt', userId: 'id', displayName: 'Admin', role: 'superadmin'
					}
				}
			});
			expect(isSuperadmin()).toBe(true);
		});
	});

	describe('logout', () => {
		it('removes server from auth state', () => {
			auth.set({
				keypair: null,
				servers: {
					'https://test.komun.buzz': {
						token: 'jwt', userId: 'id', displayName: 'User', role: 'member'
					}
				}
			});
			logout();
			expect(isAuthenticated()).toBe(false);
		});

		it('clears session storage', () => {
			sessionStorage.setItem('komun_auth_session', 'test');
			auth.set({
				keypair: null,
				servers: {
					'https://test.komun.buzz': {
						token: 'jwt', userId: 'id', displayName: 'User', role: 'member'
					}
				}
			});
			logout();
			expect(sessionStorage.getItem('komun_auth_session')).toBeNull();
		});
	});

	describe('auth store reactivity', () => {
		it('updates correctly when setting new server', () => {
			auth.set({
				keypair: null,
				servers: {
					'https://test.komun.buzz': {
						token: 'token1', userId: 'u1', displayName: 'User1', role: 'member'
					}
				}
			});
			expect(get(auth).servers['https://test.komun.buzz']?.displayName).toBe('User1');
		});

		it('handles multiple servers independently', () => {
			auth.set({
				keypair: null,
				servers: {
					'https://a.komun.buzz': {
						token: 'ta', userId: 'ua', displayName: 'Alice', role: 'member'
					},
					'https://b.komun.buzz': {
						token: 'tb', userId: 'ub', displayName: 'Bob', role: 'admin'
					}
				}
			});
			const state = get(auth);
			expect(state.servers['https://a.komun.buzz']?.displayName).toBe('Alice');
			expect(state.servers['https://b.komun.buzz']?.role).toBe('admin');
			expect(Object.keys(state.servers)).toHaveLength(2);
		});
	});
});
