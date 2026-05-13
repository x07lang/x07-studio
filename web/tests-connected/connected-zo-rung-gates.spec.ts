import { expect, test, type Page } from '@playwright/test';

async function useManualMode(page: Page) {
	await page.getByLabel('Auto').uncheck();
}

async function startSession(page: Page, prompt: string) {
	await page.goto('/');
	await expect(page.getByText('Connected to Loom daemon')).toBeVisible();
	await useManualMode(page);
	await page.getByTestId('composer-input').fill(prompt);
	await page.getByTestId('composer-submit').click();
	await expect(page.getByTestId('turn-user_intent')).toContainText(prompt, { timeout: 20_000 });

	const sessionsResponse = await page.request.get('/v1/sessions');
	const sessions = (await sessionsResponse.json()) as Array<{ session_id: string; title: string }>;
	const session = sessions.find((item) => item.title === prompt.slice(0, 80));
	expect(session).toBeTruthy();
	return session!;
}

test('connected shipping ladder exposes rung gates after verify', async ({ page }) => {
	const prompt = `Cycle 4 rung gates ${Date.now()}`;
	const session = await startSession(page, prompt);

	await page.getByTestId('approve-build').click();
	await expect(page.getByTestId('turn-verified').first()).toBeVisible({ timeout: 60_000 });
	await expect(page.getByTestId('shipping-ladder')).toContainText('XTAL verify');
	await expect(page.getByTestId('rung-gates').first()).toContainText('Solve-world default');

	const ladderResponse = await page.request.get(`/v1/sessions/${session.session_id}/ladder`);
	expect(ladderResponse.ok()).toBe(true);
	const ladder = (await ladderResponse.json()) as {
		rungs: Array<{ id: string; gates: Array<{ id: string; currently_satisfied: boolean }> }>;
	};
	const local = ladder.rungs.find((rung) => rung.id === 'local_preview');
	expect(local?.gates.some((gate) => gate.id === 'xtal-verify' && gate.currently_satisfied)).toBe(
		true
	);
});
