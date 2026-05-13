import { expect, test } from '@playwright/test';

test('connected health row renders doctor lockfile and migrate status', async ({ page }) => {
	await page.goto('/');
	await expect(page.getByText('Connected to Loom daemon')).toBeVisible();
	await page.getByRole('button', { name: 'Refresh' }).click();
	const health = page.getByTestId('health-row');
	await expect(health).toContainText('Doctor');
	await expect(health).toContainText('ready');
	await expect(health).toContainText('Lockfile');
	await expect(health).toContainText('verified');
	await expect(health.getByText('Migrate')).toHaveCount(0);
});
