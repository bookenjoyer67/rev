import { writable, get } from 'svelte/store';

const STORAGE_KEY = 'komun_directories';
const DEFAULT_DIRECTORIES = [import.meta.env.VITE_DEFAULT_DIRECTORY || 'https://api.komun.buzz'];

function loadFromStorage(): string[] {
	if (typeof localStorage === 'undefined') return DEFAULT_DIRECTORIES;
	const raw = localStorage.getItem(STORAGE_KEY);
	if (!raw) return DEFAULT_DIRECTORIES;
	try {
		const parsed = JSON.parse(raw);
		if (!Array.isArray(parsed) || parsed.length === 0) return DEFAULT_DIRECTORIES;
		return parsed;
	} catch {
		return DEFAULT_DIRECTORIES;
	}
}

function saveToStorage(dirs: string[]) {
	if (typeof localStorage === 'undefined') return;
	localStorage.setItem(STORAGE_KEY, JSON.stringify(dirs));
}

export const directories = writable<string[]>(loadFromStorage());

directories.subscribe(saveToStorage);

export function addDirectory(url: string) {
	const normalized = url.replace(/\/+$/, '');
	directories.update((dirs) => {
		if (dirs.includes(normalized)) return dirs;
		return [...dirs, normalized];
	});
}

export function removeDirectory(url: string) {
	directories.update((dirs) => dirs.filter((d) => d !== url));
}

export function getDirectories(): string[] {
	return get(directories);
}
