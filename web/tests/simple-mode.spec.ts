import { expect, test } from '@playwright/test';

// Demo-mode Simple Mode walkthrough. The daemon is not running for this
// suite, so `clarifyIntent` returns null and the flow short-circuits to the
// "no questions" branch — exactly the path a new user sees when they first
// open the browser without any backing services. The connected E2E covers
// the full Claude/Codex clarify round.

test('simple mode lands a non-engineer prompt through approve-and-build', async ({ page }) => {
	await page.setViewportSize({ width: 1280, height: 900 });
	await page.goto('/');

	await expect(page.getByTestId('studio-mode-toggle')).toBeVisible();
	await expect(page.getByTestId('simple-start')).toBeVisible();
	await expect(page.getByRole('heading', { name: 'What do you want to build?' })).toBeVisible();

	await page
		.getByTestId('simple-start-prompt')
		.fill(
			'Build a stable sorter for byte arrays that rejects empty input and keeps equal items in their original order.'
		);
	await page.getByTestId('simple-start-begin').click();

	const clarify = page.getByTestId('simple-clarify');
	await expect(clarify).toBeVisible();
	await expect(page.getByTestId('clarify-empty')).toBeVisible();

	await page.getByTestId('simple-clarify-build').click();

	await expect(page.getByTestId('simple-build-progress')).toBeVisible();
	await expect(page.getByTestId('simple-build-stages')).toBeVisible();
	await expect(page.getByTestId('build-stage-design')).toBeVisible();
	await expect(page.getByTestId('build-stage-verify')).toBeVisible();
});

test('URL query forces Expert mode and the Simple landing is gated by localStorage', async ({ page }) => {
	await page.goto('/?mode=expert');
	await expect(page.getByTestId('simple-start')).toHaveCount(0);

	await page.goto('/');
	// After unforced reload, mode should fall back to localStorage; in a fresh
	// context that means Simple Mode wins.
	await expect(page.getByTestId('simple-start')).toBeVisible();
});

test('Open Expert link in Simple Mode toggles the mode and persists', async ({ page }) => {
	const pageErrors: string[] = [];
	page.on('pageerror', (err) => pageErrors.push(err.message));
	await page.goto('/');
	await expect(page.getByTestId('simple-start')).toBeVisible();
	// SvelteKit's adapter-static prerenders the SSR shell; click handlers are
	// only wired after hydration finishes. Wait until network is idle (vite
	// has stopped proxying /v1 calls) before driving the toggle.
	await page.waitForLoadState('networkidle').catch(() => undefined);
	await page.waitForTimeout(200);
	await page.getByTestId('simple-start-expert').click();
	await page.waitForFunction(
		() => localStorage.getItem('x07-studio-mode') === 'expert',
		null,
		{ timeout: 5000 }
	);
	await expect(page.getByTestId('simple-start')).toHaveCount(0);
	await page.reload();
	await expect(page.getByTestId('simple-start')).toHaveCount(0);
	expect(pageErrors).toEqual([]);
});
