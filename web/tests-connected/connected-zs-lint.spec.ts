import { expect, test, type Page } from '@playwright/test';

async function useManualMode(page: Page) {
	await page.getByLabel('Auto').uncheck();
}

test('connected lint loop opens diagnostics and applies quickfixes', async ({ page }) => {
	await page.goto('/');
	await expect(page.getByText('Connected to Loom daemon')).toBeVisible();
	await useManualMode(page);

	await page.getByTestId('composer-input').fill(`Cycle 5 lint loop ${Date.now()}`);
	await page.getByTestId('composer-submit').click();
	await expect(page.getByTestId('turn-user_intent')).toBeVisible({ timeout: 20_000 });
	await page.getByTestId('approve-build').click();
	await expect(page.getByTestId('turn-verified').first()).toBeVisible({ timeout: 60_000 });

	const lintTurn = page.getByTestId('turn-lint').last();
	await expect(lintTurn).toContainText('X07-LINT-0042', { timeout: 20_000 });
	await lintTurn.getByRole('button', { name: 'Open lint report' }).click();
	await expect(page.getByTestId('lint-report')).toContainText('1 diagnostics');
	await page.getByTestId('lint-report').getByRole('button', { name: 'Apply quickfix' }).first().click();
	await expect(page.getByTestId('lint-report')).toContainText('0 diagnostics', { timeout: 20_000 });
});
