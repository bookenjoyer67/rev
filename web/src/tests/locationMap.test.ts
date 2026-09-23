import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
import LocationMap, { clampLatLon } from '$lib/components/LocationMap.svelte';

describe('LocationMap pick mode', () => {
	it('clamps coordinates to valid latitude/longitude', () => {
		expect(clampLatLon(200, 300)).toEqual({ lat: 90, lon: 180 });
		expect(clampLatLon(-200, -300)).toEqual({ lat: -90, lon: -180 });
		expect(clampLatLon(37.5, -122.25)).toEqual({ lat: 37.5, lon: -122.25 });
	});

	it('ignores nonsense coordinates instead of pinning NaN', () => {
		expect(clampLatLon(NaN, 10)).toBeNull();
		expect(clampLatLon(10, Infinity)).toBeNull();
		expect(clampLatLon(undefined, undefined)).toBeNull();
		expect(clampLatLon('not a number', 0)).toBeNull();
	});

	it('draws a marker at the picked coordinates in pick mode', () => {
		const { container } = render(LocationMap, {
			props: { lat: 0, lon: 0, pickable: true, pickedLat: 12.34, pickedLon: 56.78 }
		});

		expect(container.querySelectorAll('.komun-map-marker').length).toBe(1);
	});

	it('stays read-only by default: no picked marker without `pickable`', () => {
		const { container } = render(LocationMap, {
			props: { lat: 0, lon: 0, pickedLat: 12.34, pickedLon: 56.78 }
		});

		expect(container.querySelectorAll('.komun-map-marker').length).toBe(0);
	});
});
