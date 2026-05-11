import { expect, test, type Page } from '@playwright/test';

function inspectOperation(page: Page, op: string) {
	return page.getByRole('button', { name: new RegExp(`Inspect( operation)? ${escapeRegex(op)}`) }).first();
}

function escapeRegex(value: string) {
	return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

test('connected Studio drives a simple XTAL session through Loom', async ({ page }) => {
	await page.setViewportSize({ width: 1440, height: 920 });
	await page.goto('/');

	await expect(page.getByRole('heading', { name: 'x07 Studio' })).toBeVisible();
	await expect(page.locator('.rail-status')).toContainText('Loom online');
	await expect(page.locator('footer')).toContainText('Connected to Loom daemon');
	const setupReadiness = page.getByLabel('Setup readiness');
	await expect(setupReadiness.locator('div', { hasText: 'x07 CLI' }).first()).toContainText('Ready');
	await expect(setupReadiness.locator('div', { hasText: 'x07-wasm' }).first()).toContainText('Ready');
	await expect(setupReadiness.locator('div', { hasText: 'x07 platform' }).first()).toContainText('Ready');

	await page.getByLabel('Project title').fill('Connected XTAL sorter');
	await page.getByRole('button', { name: 'New Session', exact: true }).click();
	await expect(page.locator('footer')).toContainText('Created simple project: Connected XTAL sorter');

	await page.getByRole('button', { name: 'Polish Intent' }).click();
	await expect(page.getByText('Awaiting Approval', { exact: true })).toBeVisible();
	await expect(page.getByLabel('Spec approval preview')).toContainText('toy.sorter');

	await page.getByRole('button', { name: 'Approve and Run' }).click();
	await expect(page.locator('footer')).toContainText('Verify passed and trust review opened', {
		timeout: 30_000
	});
	await expect(inspectOperation(page, 'project.init.xtal-pure')).toBeVisible();
	await expect(inspectOperation(page, 'spec.scaffold')).toBeVisible();
	await expect(inspectOperation(page, 'tests.gen.write')).toBeVisible();
	await expect(inspectOperation(page, 'impl.sync.write')).toBeVisible();
	await expect(inspectOperation(page, 'xtal.verify')).toBeVisible();
	await expect(page.getByLabel('Operation artifacts')).toContainText('target/xtal/verify/summary.json');

	await page.getByLabel('Active binding').selectOption('spec.check');
	await page.getByRole('button', { name: 'Run Binding' }).click();
	await expect(page.locator('footer')).toContainText('Ran spec.check');
	await expect(inspectOperation(page, 'spec.check')).toBeVisible();
});
