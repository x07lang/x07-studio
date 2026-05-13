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

test('connected certificate viewer opens after the team rung is certified', async ({ page }) => {
	test.setTimeout(90_000);
	const prompt = `Cycle 4 certificate ${Date.now()}`;
	const session = await startSession(page, prompt);

	await page.getByTestId('approve-build').click();
	await expect(page.getByTestId('turn-verified').first()).toBeVisible({ timeout: 60_000 });

	await page.getByRole('button', { name: 'Climb to Shareable' }).click();
	await expect(page.getByTestId('shipping-ladder')).toContainText('Shareable', {
		timeout: 20_000
	});
	await page.getByRole('button', { name: 'Climb to Team' }).click();
	await expect(page.getByTestId('view-certificate')).toBeVisible({ timeout: 20_000 });
	await page.getByTestId('view-certificate').click();
	await expect(page.getByTestId('certificate-view')).toContainText('verified_core_pure_v1');

	const certResponse = await page.request.get(`/v1/sessions/${session.session_id}/certificate`);
	expect(certResponse.ok()).toBe(true);
	const certificate = (await certResponse.json()) as { profile: string; signature: string };
	expect(certificate.profile).toContain('verified_core_pure_v1');
	expect(certificate.signature).toBeTruthy();
});
