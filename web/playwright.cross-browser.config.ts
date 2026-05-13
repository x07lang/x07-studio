import { defineConfig, devices } from '@playwright/test';

const daemonAddr = process.env.LOOM_DAEMON_ADDR ?? '127.0.0.1:7729';
const webPort = Number(process.env.X07_STUDIO_WEB_PORT ?? 5181);

export default defineConfig({
	testDir: './tests-connected',
	testMatch: /connected-zv-cross-browser\.spec\.ts/,
	timeout: 60_000,
	workers: 1,
	fullyParallel: false,
	expect: {
		timeout: 10_000
	},
	webServer: [
		{
			command: `python3 ../scripts/serve_connected_e2e_daemon.py --workspace target/connected-e2e-workspace --bin-dir target/connected-e2e-bin --addr ${daemonAddr}`,
			url: `http://${daemonAddr}/v1/health`,
			reuseExistingServer: false,
			timeout: 120_000
		},
		{
			command: `LOOM_DAEMON_ORIGIN=http://${daemonAddr} npm run dev -- --host 127.0.0.1 --port ${webPort} --strictPort`,
			url: `http://127.0.0.1:${webPort}`,
			reuseExistingServer: false,
			timeout: 60_000
		}
	],
	use: {
		baseURL: `http://127.0.0.1:${webPort}`,
		trace: 'on-first-retry'
	},
	projects: [
		{
			name: 'chromium',
			use: { ...devices['Desktop Chrome'] }
		},
		{
			name: 'firefox',
			use: { ...devices['Desktop Firefox'] }
		},
		{
			name: 'webkit',
			use: { ...devices['Desktop Safari'], ignoreHTTPSErrors: true }
		}
	]
});
