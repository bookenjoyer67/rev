import { writable, get } from 'svelte/store';
import { themes, buildThemeColors, themeBySystem, THEME_STORAGE_KEY } from '$lib/themes';
import type { Theme } from '$lib/themes';

export const themeName = writable<string>('Komun Dark');

export function initTheme(): void {
	const stored = typeof localStorage !== 'undefined' ? localStorage.getItem(THEME_STORAGE_KEY) : null;
	const name = stored || themeBySystem();
	const valid = themes.find((t) => t.name === name);
	const final = valid ? valid.name : themeBySystem();
	themeName.set(final);
	applyTheme(final);
}

export function setTheme(name: string): void {
	const t = themes.find((t) => t.name === name);
	if (!t) return;
	themeName.set(name);
	applyTheme(name);
	if (typeof localStorage !== 'undefined') {
		localStorage.setItem(THEME_STORAGE_KEY, name);
	}
}

function applyTheme(name: string): void {
	const t = themes.find((t) => t.name === name);
	if (!t) return;

	const root = document.documentElement;
	root.dataset.theme = name;

	const colors = buildThemeColors(t);
	for (const [key, value] of Object.entries(colors)) {
		root.style.setProperty(`--${key}`, value);
	}

	const meta = document.querySelector('meta[name="theme-color"]');
	if (meta) {
		meta.setAttribute('content', t.meta.themeColor);
	}
}

export function getTheme(): Theme {
	const name = get(themeName);
	return themes.find((t) => t.name === name) || themes[0];
}

export function getAll(): Theme[] {
	return themes;
}
