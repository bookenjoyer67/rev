import { writable, get } from 'svelte/store';

interface LocationState {
	name: string;
	lat: number | null;
	lon: number | null;
}

const STORAGE_KEY = 'komun_location';

function loadFromStorage(): LocationState {
	if (typeof localStorage === 'undefined') return { name: '', lat: null, lon: null };
	const raw = localStorage.getItem(STORAGE_KEY);
	if (!raw) return { name: '', lat: null, lon: null };
	try {
		return JSON.parse(raw);
	} catch {
		return { name: '', lat: null, lon: null };
	}
}

function saveToStorage(state: LocationState) {
	if (typeof localStorage === 'undefined') return;
	localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
}

export const location = writable<LocationState>(loadFromStorage());

location.subscribe(saveToStorage);

export function hasLocation(): boolean {
	const loc = get(location);
	return loc.lat !== null && loc.lon !== null;
}

export function getLocation(): LocationState {
	return get(location);
}

export async function geocode(query: string): Promise<boolean> {
	try {
		const encoded = encodeURIComponent(query);
		const res = await fetch(
			`https://nominatim.openstreetmap.org/search?q=${encoded}&format=json&limit=1`,
			{ headers: { 'User-Agent': 'Komun/0.1 (mutual-aid-app)' } }
		);
		if (!res.ok) return false;

		const results = await res.json();
		if (results.length === 0) return false;

		const result = results[0];
		location.set({
			name: result.display_name.split(',').slice(0, 2).join(',').trim(),
			lat: parseFloat(result.lat),
			lon: parseFloat(result.lon),
		});
		return true;
	} catch {
		return false;
	}
}

export function clearLocation() {
	location.set({ name: '', lat: null, lon: null });
}
