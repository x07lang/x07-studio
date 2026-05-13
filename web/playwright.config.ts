import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
	testDir: './tests',
	timeout: 30_000,
	expect: {
		timeout: 5_000
	},
	webServer: {
		command: 'LOOM_DAEMON_ORIGIN=demo npm run dev -- --host 127.0.0.1 --port 5178 --strictPort',
		url: 'http://127.0.0.1:5178',
		reuseExistingServer: !process.env.CI,
		timeout: 60_000
	},
	use: {
		baseURL: 'http://127.0.0.1:5178',
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
