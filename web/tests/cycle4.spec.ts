import { expect, test } from '@playwright/test';

test('cycle 4 welcome, command palette, proof, and compare surfaces render in demo mode', async ({
	page
}) => {
	await page.setViewportSize({ width: 1280, height: 900 });
	await page.goto('/');

	await expect(page.getByTestId('welcome-recipes')).toBeVisible();
	await expect(page.getByText('Demo projection active')).toBeVisible();
	await expect(page.getByRole('button', { name: /text-core \/ text-utils/ })).toBeVisible();
	await expect(page.getByTestId('trust-card')).toContainText('Trust posture pending');

	await page.getByLabel('Open command palette').click();
	await expect(page.getByTestId('command-palette')).toBeVisible();
	await page.getByPlaceholder('Command').fill('sync');
	await expect(page.getByTestId('command-palette')).toContainText('Continue elsewhere');
	await page.keyboard.press('Escape');

	await page
		.getByTestId('composer-input')
		.fill('Build a stable sorter for byte arrays that rejects empty input.');
	await page.getByTestId('composer-submit').click();
	await expect(page.getByTestId('turn-user_intent')).toBeVisible();
	await page.getByTestId('approve-build').click();

	await expect(page.getByTestId('turn-verified')).toBeVisible();
	await page.locator('.promise-button').first().click();
	await expect(page.getByTestId('proof-explorer')).toBeVisible();
	await page.getByTestId('proof-explorer').getByRole('button', { name: 'Close' }).click();

	await page.getByRole('button', { name: 'Compare' }).first().click();
	await page.getByRole('menuitem', { name: 'With current' }).first().click();
	await expect(page.getByTestId('semantic-diff')).toContainText('stays solve-pure');
});

test('canonical recipe click starts a session and opens the AGENT.md contract', async ({ page }) => {
	await page.goto('/');
	await page.getByRole('button', { name: /text-core \/ text-utils/ }).click();
	await expect(page.getByTestId('turn-user_intent')).toContainText('docs/examples/agent-gate/text-core/text-utils', {
		timeout: 15_000
	});
	await expect(page.getByTestId('agent-contract-editor')).toBeVisible();
	await expect(page.getByTestId('agent-contract-editor')).toContainText('AGENT.md');
});
