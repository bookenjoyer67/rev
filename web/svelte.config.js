import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	kit: {
		adapter: adapter({
			pages: 'build',
			assets: 'build',
			fallback: 'index.html'
		}),
		csp: {
			mode: 'hash',
			directives: {
				'default-src': ['self'],
				'script-src': ['self', 'wasm-unsafe-eval'],
				'style-src': ['self', 'unsafe-inline'],
				'img-src': [
					'self',
					'data:',
					'blob:',
					// Map tiles (LocationMap / /map). An operator pointing
					// [map] tile_url at another tile host must add it here too.
					'https://tile.openstreetmap.org',
					'https://*.tile.openstreetmap.org'
				],
				'frame-src': ['self', 'https://localhost:5174', 'https://app.piggpin.space', 'https://www.openstreetmap.org'],
				'connect-src': ['self', 'http://localhost:*', 'https://localhost:5174', 'https://*.piggpin.space', 'wss://*.piggpin.space'],
				'font-src': ['self'],
				'object-src': ['none'],
				'base-uri': ['self'],
				'form-action': ['self']
			}
		}
	}
};

export default config;
