import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import path from 'path';

export default defineConfig({
	plugins: [svelte()],
	test: {
		environment: 'jsdom',
		globals: true,
		setupFiles: ['./src/tests/setup.ts'],
		include: ['src/**/*.test.ts'],
		alias: {
			'komun-wasm': '/src/tests/__mocks__/komun-wasm.ts'
		}
	},
	resolve: {
		alias: {
			$lib: path.resolve('./src/lib'),
			$app: path.resolve('./src/tests/__mocks__/app')
		},
		conditions: ['browser']
	}
});
