import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
import LocationMap from '$lib/components/LocationMap.svelte';

describe('LocationMap', () => {
	it('renders the OpenStreetMap attribution', () => {
		const { container } = render(LocationMap, {
			props: { lat: 37.7749, lon: -122.4194, zoom: 12 }
		});

		const attribution = container.querySelector('.leaflet-control-attribution');
		expect(attribution).not.toBeNull();
		expect(attribution?.textContent).toContain('OpenStreetMap');
	});

	it('does not throw when the tile URL is bogus', () => {
		expect(() =>
			render(LocationMap, {
				props: { lat: 0, lon: 0, tileUrl: 'this is not a tile url' }
			})
		).not.toThrow();

		const attribution = document.querySelector('.leaflet-control-attribution');
		expect(attribution?.textContent).toContain('OpenStreetMap');
	});

	it('renders one marker per located post', () => {
		const { container } = render(LocationMap, {
			props: {
				lat: 0,
				lon: 0,
				markers: [
					{ lat: 1, lon: 2, label: 'One' },
					{ lat: 3, lon: 4, label: 'Two' }
				]
			}
		});

		expect(container.querySelectorAll('.komun-map-marker').length).toBe(2);
	});
});
