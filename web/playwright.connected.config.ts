import { defineConfig, devices } from '@playwright/test';

const daemonAddr = '127.0.0.1:7729';
const webPort = 5179;

export default defineConfig({
	testDir: './tests-connected',
	timeout: 45_000,
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
			command: `LOOM_DAEMON_ORIGIN=http://${daemonAddr} npm run dev -- --host 127.0.0.1 --port ${webPort}`,
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
		}
	]
});
