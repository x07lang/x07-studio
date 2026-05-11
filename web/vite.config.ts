import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, type Plugin } from 'vite';

const loomDaemonOrigin = process.env.LOOM_DAEMON_ORIGIN ?? 'http://127.0.0.1:7719';

function loomHealthEndpoint(): Plugin {
	return {
		name: 'loom-health-endpoint',
		configureServer(server) {
			server.middlewares.use('/v1/health', async (req, res, next) => {
				if (req.method !== 'GET') {
					next();
					return;
				}
				try {
					const upstream = await fetch(`${loomDaemonOrigin}/v1/health`, {
						signal: AbortSignal.timeout(250)
					});
					res.statusCode = upstream.status;
					res.setHeader('content-type', upstream.headers.get('content-type') ?? 'application/json');
					res.end(await upstream.text());
				} catch {
					res.statusCode = 503;
					res.setHeader('content-type', 'application/json');
					res.end(JSON.stringify({ ok: false, error: 'loom daemon unavailable' }));
				}
			});
		}
	};
}

export default defineConfig({
	plugins: [loomHealthEndpoint(), sveltekit()],
	server: {
		proxy: {
			'/v1': loomDaemonOrigin
		}
	},
	test: {
		environment: 'jsdom',
		setupFiles: ['./src/tests/setup.ts'],
		include: ['src/**/*.test.ts'],
		clearMocks: true
	}
});
