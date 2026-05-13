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

test('connected semantic diff is available from the timeline and API', async ({ page }) => {
	const prompt = `Cycle 4 semantic diff ${Date.now()}`;
	const session = await startSession(page, prompt);

	await page.getByRole('button', { name: 'Compare' }).first().click();
	await page.getByRole('menuitem', { name: 'With current' }).first().click();
	await expect(page.getByTestId('semantic-diff')).toContainText('solve-pure');

	const diffResponse = await page.request.post(`/v1/sessions/${session.session_id}/diff`, {
		data: {
			schema_version: 'x07.studio.semantic_diff_request@0.1.0',
			from: { kind: 'current' },
			to: { kind: 'current' },
			mode: 'project'
		}
	});
	expect(diffResponse.ok()).toBe(true);
	const diff = (await diffResponse.json()) as { headline: string; trust_delta_color: string };
	expect(diff.headline).toContain('solve-pure');
	expect(diff.trust_delta_color).toBe('green');
});
