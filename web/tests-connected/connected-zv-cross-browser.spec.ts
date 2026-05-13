import { expect, test } from '@playwright/test';

test('connected XTAL flow renders across browser engines', async ({ page, browserName }, testInfo) => {
	await page.goto('/');
	await expect(page.getByText('Connected to Loom daemon')).toBeVisible();
	await page.getByLabel('Auto').uncheck();

	const prompt = `Cross-browser sorter ${browserName} ${Date.now()}`;
	await page.getByTestId('composer-input').fill(prompt);
	await page.getByTestId('composer-submit').click();
	await expect(page.getByTestId('turn-user_intent')).toContainText(prompt, { timeout: 20_000 });
	await expect(page.getByTestId('turn-agent_clarify')).toBeVisible({ timeout: 20_000 });

	const emptyInputQuestion = page.locator('[data-testid="clarify-card"]').first();
	await emptyInputQuestion.getByRole('button').first().click();
	await emptyInputQuestion.getByTestId('clarify-answer-submit').click();
	await expect(emptyInputQuestion.getByTestId('clarify-answer-locked')).toBeVisible({
		timeout: 10_000
	});

	await page.getByTestId('approve-build').click();
	await expect(page.getByTestId('turn-verified').first()).toBeVisible({ timeout: 60_000 });
	await expect(page.getByTestId('trust-card')).toContainText(/solve-pure|proof coverage/i);

	await page.screenshot({
		path: testInfo.outputPath(`cross-browser-${browserName}.png`),
		fullPage: true
	});
});
