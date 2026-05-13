import { expect, test, type Page } from '@playwright/test';

async function useManualMode(page: Page) {
	await page.getByLabel('Auto').uncheck();
}

test('connected arch check is a Shareable rung gate', async ({ page }) => {
	const prompt = `Cycle 5 arch check ${Date.now()}`;
	await page.goto('/');
	await expect(page.getByText('Connected to Loom daemon')).toBeVisible();
	await useManualMode(page);
	await page.getByTestId('composer-input').fill(prompt);
	await page.getByTestId('composer-submit').click();
	await expect(page.getByTestId('turn-user_intent')).toContainText(prompt, { timeout: 20_000 });
	await page.getByTestId('approve-build').click();
	await expect(page.getByTestId('turn-verified').first()).toBeVisible({ timeout: 60_000 });

	const sessionsResponse = await page.request.get('/v1/sessions');
	const sessions = (await sessionsResponse.json()) as Array<{ session_id: string; title: string }>;
	const session = sessions.find((item) => item.title === prompt.slice(0, 80));
	expect(session).toBeTruthy();

	const archResponse = await page.request.get(`/v1/sessions/${session!.session_id}/arch-check`);
	expect(archResponse.ok()).toBe(true);
	const arch = (await archResponse.json()) as { passed: boolean };
	expect(arch.passed).toBe(true);

	const ladderResponse = await page.request.get(`/v1/sessions/${session!.session_id}/ladder`);
	expect(ladderResponse.ok()).toBe(true);
	const ladder = (await ladderResponse.json()) as {
		rungs: Array<{ id: string; gates: Array<{ id: string; currently_satisfied: boolean }> }>;
	};
	const shareable = ladder.rungs.find((rung) => rung.id === 'shareable');
	expect(shareable?.gates.some((gate) => gate.id === 'arch-check' && gate.currently_satisfied)).toBe(
		true
	);
	await expect(page.getByTestId('shipping-ladder')).toContainText('Architecture check');
});
