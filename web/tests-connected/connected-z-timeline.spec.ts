import { expect, test } from '@playwright/test';

test.describe.configure({ mode: 'serial' });

test('connected timeline formalizes, clarifies, builds, tries, and scans incidents', async ({ page }) => {
	await page.setViewportSize({ width: 1280, height: 900 });
	await page.goto('/');

	await expect(page.getByRole('heading', { name: 'x07 Studio' })).toBeVisible();
	await expect(page.getByText('Connected to Loom daemon')).toBeVisible();

	await page
		.getByTestId('composer-input')
		.fill('Build a stable sorter for byte arrays. Equal items must keep order. Reject empty input.');
	await page.getByTestId('composer-submit').click();

	await expect(page.getByTestId('turn-user_intent')).toBeVisible({ timeout: 15_000 });
	await expect(page.getByTestId('turn-agent_clarify')).toBeVisible({ timeout: 20_000 });
	await expect(page.getByTestId('clarify-card')).toHaveCount(2, { timeout: 20_000 });

	const empty = page.locator('[data-testid="clarify-card"][data-question-id="q-empty-input"]');
	await empty.getByRole('button', { name: 'Reject with an error' }).click();
	await empty.getByTestId('clarify-answer-submit').click();
	await expect(empty.getByTestId('clarify-answer-locked')).toBeVisible({ timeout: 10_000 });

	await page.getByTestId('approve-build').click();
	await expect(page.getByTestId('turn-verified')).toBeVisible({ timeout: 45_000 });
	await expect(page.getByTestId('run-invocation')).toContainText('x07 run');
	await expect(page.getByTestId('shipping-ladder')).toContainText('Local preview');

	await page.getByTestId('try-it-panel').getByRole('textbox').first().fill('[2,1]');
	await page.getByTestId('try-it-panel').getByRole('button', { name: 'Run it' }).click();
	await expect(page.getByTestId('try-it-panel')).toContainText('Latest verify evidence', {
		timeout: 15_000
	});
});
