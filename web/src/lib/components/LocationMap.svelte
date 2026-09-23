<script lang="ts">
	// Leaflet ships without TypeScript declarations in this image and no
	// @types/leaflet is installed (and none can be — no egress), so keep the
	// import untyped rather than adding a diagnostic to the frozen baseline.
	// @ts-ignore
	import * as L from 'leaflet';
	import 'leaflet/dist/leaflet.css';

	interface MapMarker {
		lat: number;
		lon: number;
		label?: string;
	}

	interface Props {
		lat: number;
		lon: number;
		zoom?: number;
		tileUrl?: string;
		attribution?: string;
		markers?: MapMarker[];
	}

	let {
		lat,
		lon,
		zoom = 13,
		tileUrl = 'https://tile.openstreetmap.org/{z}/{x}/{y}.png',
		attribution = '&copy; OpenStreetMap contributors',
		markers = []
	}: Props = $props();

	let container = $state<HTMLDivElement | null>(null);

	$effect(() => {
		if (!container) return;

		const instance = L.map(container, { attributionControl: false });
		instance.setView([lat, lon], zoom);

		// Attribution is rendered unconditionally, so it is present even when
		// no tile ever loads (bogus URL, offline, blocked host).
		L.control
			.attribution({ prefix: false })
			.addAttribution(attribution)
			.addTo(instance);

		try {
			L.tileLayer(tileUrl, { attribution: '' }).addTo(instance);
		} catch (err) {
			// A malformed tile URL must degrade to an empty map, never a broken page.
			console.warn('[LocationMap] invalid tile URL, showing an empty map', err);
		}

		const icon = L.divIcon({
			className: 'komun-map-marker',
			iconSize: [14, 14],
			iconAnchor: [7, 7]
		});
		for (const marker of markers) {
			L.marker([marker.lat, marker.lon], { icon })
				.bindPopup(marker.label ?? '')
				.addTo(instance);
		}

		return () => {
			instance.remove();
		};
	});
</script>

<div class="location-map" bind:this={container} data-attribution={attribution}></div>

<style>
	.location-map {
		width: 100%;
		height: 100%;
		min-height: 320px;
		border-radius: var(--radius, 8px);
		overflow: hidden;
		background: var(--bg-surface, #eee);
	}

	/* Leaflet builds these nodes itself, so the selector must be global. */
	:global(.komun-map-marker) {
		width: 14px;
		height: 14px;
		border-radius: 50%;
		background: var(--accent, #d95d39);
		border: 2px solid #fff;
		box-shadow: 0 1px 4px rgba(0, 0, 0, 0.4);
	}
</style>
