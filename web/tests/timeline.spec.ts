import { expect, test } from '@playwright/test';

test('timeline shell builds a prompt and exposes run/try surfaces in demo mode', async ({ page }) => {
	await page.setViewportSize({ width: 1280, height: 900 });
	await page.goto('/');

	await expect(page.getByRole('heading', { name: 'x07 Studio' })).toBeVisible();
	await expect(page.getByTestId('composer')).toBeVisible();
	await expect(page.getByTestId('timeline')).toBeVisible();
	await expect(page.getByTestId('now-panel')).toBeVisible();
	await expect(page.getByTestId('studio-mode-toggle')).toHaveCount(0);

	await page
		.getByTestId('composer-input')
		.fill('Build a stable sorter for byte arrays that rejects empty input.');
	await page.getByTestId('composer-submit').click();

	await expect(page.getByTestId('turn-user_intent')).toBeVisible();
	await page.getByTestId('approve-build').click();

	await expect(page.getByTestId('turn-verified')).toBeVisible();
	await expect(page.getByTestId('run-invocation')).toContainText('x07 run');
	await expect(page.getByTestId('followups')).toBeVisible();

	await page.getByTestId('try-inline-input').fill('[3,1,2]');
	await page.getByTestId('try-inline-run').click();
	await expect(page.getByTestId('turn-verified')).toContainText('demo output');
});

test('expert query opens evidence drawers without restoring mode toggle', async ({ page }) => {
	await page.goto('/?mode=expert');
	await expect(page.getByTestId('studio-mode-toggle')).toHaveCount(0);
	await page.getByTestId('composer-input').fill('Build a tiny echo tool.');
	await page.getByTestId('composer-submit').click();
	await expect(page.getByText('Show evidence').first()).toBeVisible();
});
