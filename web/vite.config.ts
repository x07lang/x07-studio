import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	server: {
		proxy: {
			'/v1': 'http://127.0.0.1:7719'
		}
	},
	test: {
		environment: 'jsdom',
		setupFiles: ['./src/tests/setup.ts'],
		include: ['src/**/*.test.ts'],
		clearMocks: true
	}
});
